"""Derive a proposal's review state from reviewer votes recorded as issue comments.

`state:approved` and `state:declined` are DERIVED: no human and no model writes them
(see the `PROTECTED` guards in `.github/workflows/proposal-update.yml`). They are the
output of this tally, whose only input is the comment ledger of the issue.

The tally is **stateless** — every run replays the whole ledger from the last reset
marker, so a missed webhook, a hand-edited label, or a re-run all converge on the same
answer. Labels are a derived mirror for the board, never the source of truth.

Ledger rules:

- A vote is a comment whose line starts with `/approve`, `/object`, `/decline` or
  `/withdraw`.
- **One vote per account.** Votes reduce to a map keyed by login and the account's
  LAST command wins, so re-voting replaces rather than accumulates.
- Only accounts with write access to the repo are counted, checked per-login and
  failing closed. The repo is public, so anyone can type `/approve`.
- Comments before the newest reset marker are ignored: revising the body dismisses
  approvals (they were cast against text that no longer exists).
- An objection is sticky — a revision does NOT clear it, only the objector's
  `/withdraw` does. Otherwise an author lifts objections by editing.
- Silence can approve (lazy consensus) but never decline.
- `state:declined` is terminal and `state:had-comments` survives an empty ledger:
  an approval must be earned, a block is preserved.

Run: uv run python scripts/proposal_vote.py tally --issue 147
     uv run python scripts/proposal_vote.py sweep
"""

from __future__ import annotations

import datetime as dt
import json
import re
import subprocess
import sys
from dataclasses import dataclass, field, replace

import cyclopts

QUORUM = 2
LAZY_DAYS = 7
WARN_DAYS = 5

PROPOSAL = "state:proposal"
HAD_COMMENTS = "state:had-comments"
APPROVED = "state:approved"
DECLINED = "state:declined"
STATE_LABELS = frozenset({PROPOSAL, HAD_COMMENTS, APPROVED, DECLINED})
# Read an ambiguous board (an issue carrying two state labels, which hand-labelling
# has already produced) in the direction that blocks.
STATE_PRECEDENCE = (DECLINED, HAD_COMMENTS, APPROVED, PROPOSAL)

# How an approval was won. `state:approved` stays the single state so queries and
# downstream tooling don't fork, but the board must say which approvals nobody
# actually read: `label:approved:lazy` is the audit query for that.
# Board signals: what the proposal needs from a human right now.
WAITING = "waiting-review"
BLOCKED = "blocked"
SIGNAL_LABELS = frozenset({WAITING, BLOCKED})
# Advisory support. NEVER feeds the state machine: the repo is public, so letting a
# drive-by account nudge approval would be a hole. Interest, not authority.
UPVOTE_PREFIX = "upvote:"

BY_QUORUM = "approved:quorum"
BY_LAZY = "approved:lazy"
PROVENANCE_LABELS = frozenset({BY_QUORUM, BY_LAZY})

# A shipped standard is not under review. Its proposal issue stays open as the SSOT
# for the rule's text, but the tally must not drag it back to state:proposal — the
# code is already enforced and cited in AGENTS.md and in downstream repos.
AS_BUILT = "as-built"

RESET_MARKER = "<!-- proposal-vote:reset -->"
TALLY_MARKER = "<!-- proposal-vote:tally -->"
WARN_MARKER = "<!-- proposal-vote:warn -->"
CALL_MARKER = "<!-- proposal-vote:review-call -->"

WRITE_ROLES = frozenset({"admin", "maintain", "write"})
# Case-insensitive: `/Approve` is unambiguously a vote, and silently dropping it
# is the same trap as `/approved`. Still anchored to line start so quoting a
# colleague's vote never casts one.
# Leading ` and * are tolerated because people reflexively format commands
# (underscore is excluded on purpose: `_/approve_` cannot end on a \b, and
# weakening that boundary is what would let `/approved` through).
# (`/approve`, **/approve**) and silently dropping those reads as a broken system.
# `>` is deliberately NOT tolerated: that is a quoted colleague, not your vote. The
# anchor to line start is what keeps quoting safe, so it stays.
COMMAND = re.compile(
    r"^[ \t]*[`*]{0,3}[ \t]*/(approve|object|decline|withdraw|upvote)\b",
    re.MULTILINE | re.IGNORECASE,
)

app = cyclopts.App(name="proposal-vote", help=__doc__)


@dataclass(frozen=True)
class Comment:
    """A single issue comment from the gh payload."""

    login: str
    body: str
    at: dt.datetime
    is_bot: bool


@dataclass(frozen=True)
class Command:
    """One vote command parsed from a comment."""

    login: str
    verb: str
    at: dt.datetime


@dataclass(frozen=True)
class Tally:
    """Who currently stands where, after one-vote-per-account reduction."""

    approvals: tuple[str, ...] = ()
    objections: tuple[str, ...] = ()
    declines: tuple[str, ...] = ()
    # Every eligible account that issued any command, including a /withdraw that
    # left no standing vote. This is "has the ledger been touched", not "who agrees".
    spoke: tuple[str, ...] = ()
    # When the earliest still-standing approval was cast. The lazy clock runs from
    # here, NOT from the last revision: otherwise a proposal that has sat unread for
    # 48 days auto-approves the instant someone first approves it, with no window for
    # anyone to object.
    first_approval: dt.datetime | None = None

    def waiting_days(self, now: dt.datetime) -> float:
        """How long this has stood approved and unopposed — the lazy-consensus clock."""
        if self.first_approval is None:
            return 0.0
        return (now - self.first_approval).total_seconds() / 86400.0


@dataclass(frozen=True)
class Decision:
    """The derived state plus the label writes that realise it."""

    state: str
    reason: str
    warn: bool = False
    votes: tuple[str, ...] = field(default_factory=tuple)
    provenance: str = ""
    signal: str = ""
    upvotes: int = 0


@dataclass(frozen=True)
class Audience:
    """Who may vote, and who to interrupt about it."""

    electorate: frozenset[str]
    team: str = ""
    team_members: tuple[str, ...] = ()


@dataclass(frozen=True)
class Outcome:
    """Everything one issue's run resolved to, for the notification pass."""

    decision: Decision
    tally: Tally
    comments: list[Comment]
    since: dt.datetime
    now: dt.datetime
    moved: bool


def parse_commands(comments: list[Comment], since: dt.datetime | None) -> list[Command]:
    """Extract vote commands from comments cast after `since`, in chronological order."""
    found: list[Command] = []
    for comment in sorted(comments, key=lambda c: c.at):
        if since is not None and comment.at <= since:
            continue
        if comment.is_bot:
            continue
        found.extend(
            Command(login=comment.login, verb=match.group(1).lower(), at=comment.at)
            for match in COMMAND.finditer(comment.body)
        )
    return found


def tally(commands: list[Command], eligible: frozenset[str]) -> Tally:
    """Reduce commands to at most one standing vote per account (last command wins)."""
    standing: dict[str, Command] = {}
    for command in commands:
        if command.login not in eligible:
            continue
        if command.verb == "withdraw":
            standing.pop(command.login, None)
        else:
            standing[command.login] = command

    def who(verb: str) -> tuple[str, ...]:
        return tuple(sorted(login for login, c in standing.items() if c.verb == verb))

    cast = [c.at for c in standing.values() if c.verb == "approve"]

    return Tally(
        approvals=who("approve"),
        objections=who("object"),
        declines=who("decline"),
        spoke=tuple(sorted({c.login for c in commands if c.login in eligible})),
        first_approval=min(cast) if cast else None,
    )


def count_upvotes(commands: list[Command], reactions: tuple[str, ...]) -> tuple[str, ...]:
    """Accounts signalling support, deduped: `/upvote` from anyone, plus thumbs-up.

    Explicitly NOT filtered by write access — the point is to let people without a
    vote register interest. Equally explicitly, the result never reaches `decide()`.
    """
    return tuple(sorted({c.login for c in commands if c.verb == "upvote"} | set(reactions)))


def current_state(labels: set[str]) -> str:
    """Report the state an issue already carries, resolved blocking-first if it has two."""
    return next((state for state in STATE_PRECEDENCE if state in labels), "")


def decide(
    counted: Tally, now: dt.datetime, *, quorum: int = QUORUM, current: str = ""
) -> Decision:
    """Map a tally onto the derived review state.

    Asymmetric on purpose: an approval must be *earned* from the ledger, while a
    decline or an open objection is *preserved* when the ledger is silent. Being
    wrong towards "blocked" costs a re-vote; being wrong towards "approved" ships a
    standard nobody read.
    """
    # Terminal. A declined proposal's CES number is burned and never reused
    # (docs/engineering-standards.md:32), so no later vote may silently resurrect it
    # — reopening is a deliberate act: remove the label by hand.
    if current == DECLINED:
        return Decision(DECLINED, "declined (terminal — remove the label by hand to reopen)")
    # Silence is not consent, so it cannot clear an objection. Note this keys on
    # `spoke`, not on standing votes: a /withdraw leaves nothing standing but is an
    # explicit act, and must hand control back to the ledger.
    if current == HAD_COMMENTS and not counted.spoke:
        return Decision(HAD_COMMENTS, "untouched ledger — existing objection preserved")

    waited = counted.waiting_days(now)
    if len(counted.declines) >= quorum:
        return Decision(
            DECLINED, f"declined by {len(counted.declines)} reviewers", votes=counted.declines
        )
    if counted.objections:
        return Decision(
            HAD_COMMENTS,
            f"objection open from {', '.join(counted.objections)}",
            signal=BLOCKED,
        )
    if len(counted.approvals) >= quorum:
        return Decision(
            APPROVED,
            f"quorum {len(counted.approvals)}/{quorum}",
            votes=counted.approvals,
            provenance=BY_QUORUM,
        )
    if counted.approvals and waited >= LAZY_DAYS:
        return Decision(
            APPROVED,
            f"lazy consensus: {waited:.0f}d approved and unopposed",
            votes=counted.approvals,
            provenance=BY_LAZY,
        )
    warn = bool(counted.approvals) and WARN_DAYS <= waited < LAZY_DAYS
    return Decision(
        PROPOSAL,
        f"awaiting review ({len(counted.approvals)}/{quorum})",
        warn=warn,
        # An approval that has not yet reached quorum is the state that needs a human:
        # somebody said "ready", nobody has seconded it.
        signal=WAITING if counted.approvals else "",
    )


def _ts(raw: str) -> dt.datetime:
    return dt.datetime.fromisoformat(raw.replace("Z", "+00:00"))


def _gh(args: list[str]) -> str:
    return subprocess.run(["gh", *args], check=True, capture_output=True, text=True).stdout


def _load_comments(payload: list[dict]) -> list[Comment]:
    return [
        Comment(
            login=(c.get("author") or {}).get("login", ""),
            body=c.get("body") or "",
            at=_ts(c["createdAt"]),
            is_bot=(c.get("author") or {}).get("login", "").endswith("[bot]"),
        )
        for c in payload
    ]


def last_reset(comments: list[Comment], fallback: dt.datetime) -> dt.datetime:
    """Timestamp of the newest reset marker, i.e. the revision votes are cast against."""
    resets = [c.at for c in comments if c.is_bot and RESET_MARKER in c.body]
    return max(resets) if resets else fallback


def writers(repo: str) -> frozenset[str]:
    """Every human account with write access — the electorate and the notify list.

    One call, and it fails closed: if it raises, the run aborts before writing any
    label rather than tallying against an empty or partial electorate. This repo is
    public, so an unfiltered ledger would let a stranger cast `/approve`.
    """
    raw = json.loads(_gh(["api", f"repos/{repo}/collaborators", "--paginate"]))
    return frozenset(
        person["login"]
        for person in raw
        if person.get("type") == "User"
        and not person["login"].endswith("[bot]")
        and (person.get("role_name") or "").lower() in WRITE_ROLES
    )


def settled(labels: set[str], closed: bool) -> str:
    """Say why a proposal is out of the vote's hands, or "" while it is under review.

    Derivation governs proposals being decided, not ones already decided. Without this
    an empty ledger reverts a shipped standard to state:proposal, which is exactly what
    happened to CES-109/110/111/113/114/118/119.
    """
    if closed:
        return "closed"
    if AS_BUILT in labels:
        return f"{AS_BUILT} — shipped, state frozen"
    return ""


def _apply(repo: str, number: int, decision: Decision, current: set[str]) -> bool:
    """Write the derived labels. Returns True when the state label actually moved.

    Any other `state:*` label is dropped, so an issue carrying two of them (which
    hand-labelling has already produced) is healed rather than left ambiguous.
    """
    keep = {f"vote:{login}" for login in decision.votes}
    optional = {decision.provenance, decision.signal} - {""}
    if decision.upvotes:
        optional.add(f"{UPVOTE_PREFIX}{decision.upvotes}")
    want = keep | {decision.state} | optional
    add = want - current
    remove = (
        ({lab for lab in current if lab.startswith("vote:")} - keep)
        | ({lab for lab in current if lab.startswith(UPVOTE_PREFIX)} - want)
        | ((STATE_LABELS - {decision.state}) & current)
        | ((PROVENANCE_LABELS | SIGNAL_LABELS) - want) & current
    )
    for label in sorted(add):
        _gh(["label", "create", label, "--repo", repo, "--force"])
    args = [a for label in sorted(add) for a in ("--add-label", label)]
    args += [a for label in sorted(remove) for a in ("--remove-label", label)]
    if args:
        _gh(["issue", "edit", str(number), "--repo", repo, *args])
    return decision.state not in current


def _comment(repo: str, number: int, marker: str, body: str) -> None:
    _gh(["issue", "comment", str(number), "--repo", repo, "--body", f"{marker}\n{body}"])


def thumbs_up(repo: str, issue: int) -> tuple[str, ...]:
    """Logins that reacted to the issue with a thumbs-up. Best effort."""
    try:
        raw = _gh(["api", f"repos/{repo}/issues/{issue}/reactions", "--paginate"])
    except subprocess.CalledProcessError as exc:
        _annotate("warning", f"#{issue}: could not read reactions: {exc}")
        return ()
    return tuple(
        r["user"]["login"]
        for r in json.loads(raw)
        if r.get("content") == "+1" and (r.get("user") or {}).get("type") == "User"
    )


def _posted_since(comments: list[Comment], marker: str, since: dt.datetime) -> bool:
    return any(c.is_bot and marker in c.body and c.at > since for c in comments)


def _annotate(level: str, message: str) -> None:
    """Surface a problem in the Actions UI without failing the run."""
    print(f"::{level}::{message}")


def _can_list_teams(org: str) -> bool:
    try:
        _gh(["api", f"orgs/{org}/teams", "--paginate"])
    except subprocess.CalledProcessError:
        return False
    return True


def resolve_team(team: str) -> tuple[str, ...]:
    """Members of `org/slug`, or `()` when they cannot be read.

    **Never raises.** Notification is a courtesy, and a courtesy must not be able to
    abort the vote tally — an earlier version raised here and silently killed every
    run in CI before a single vote was counted.

    A repo-scoped GITHUB_TOKEN receives **404, not 403**, for org endpoints it cannot
    see, so "team is missing" and "team is invisible to me" are indistinguishable from
    the error alone. Probe the org's team list to tell them apart: readable means a
    missing slug is a genuine typo (loud annotation, since a bad slug notifies nobody);
    unreadable just means no org scope, so degrade to mentioning the team.
    """
    if "/" not in team:
        _annotate("error", f"--team must be 'org/slug', got {team!r}")
        return ()
    org, slug = team.split("/", 1)
    try:
        raw = _gh(["api", f"orgs/{org}/teams/{slug}/members", "--paginate"])
    except subprocess.CalledProcessError:
        if _can_list_teams(org):
            _annotate("error", f"team {team!r} does not exist — a bad slug notifies nobody")
        else:
            print(f"  no org scope for {org}: mentioning @{team} rather than its members")
        return ()
    return tuple(sorted(p["login"] for p in json.loads(raw)))


def _ping(audience: Audience, heard: set[str]) -> str:
    """Who still owes a vote.

    Individual mentions are used whenever the team roster is knowable, because a
    team mention authored by github-actions[bot] is not reliably delivered.
    """
    roster = frozenset(audience.team_members) if audience.team_members else audience.electorate
    outstanding = sorted(roster - heard)
    if not outstanding:
        return "_everyone has voted_"
    if audience.team and not audience.team_members:
        return f"@{audience.team}"
    return " ".join(f"@{login}" for login in outstanding)


def _announce(repo: str, issue: int, ctx: Outcome, audience: Audience) -> None:
    """Post at most the two notifications worth interrupting people for.

    Deliberately NOT one per vote: a repo that pings on every event gets muted, and a
    muted repo has no reviewers.
    """
    decision, counted, comments, since = ctx.decision, ctx.tally, ctx.comments, ctx.since
    heard = set(counted.approvals) | set(counted.objections) | set(counted.declines)

    if ctx.moved and decision.state in {APPROVED, DECLINED}:
        roll = ", ".join(f"@{login}" for login in decision.votes)
        _comment(
            repo, issue, TALLY_MARKER, f"**{decision.state}** — {decision.reason}.\nVotes: {roll}."
        )
    # First approval on this revision = "ready for review". That is the moment a
    # second pair of eyes is actually needed, so that is when we interrupt people.
    if (
        decision.state == PROPOSAL
        and counted.approvals
        and not _posted_since(comments, CALL_MARKER, since)
    ):
        _comment(
            repo,
            issue,
            CALL_MARKER,
            f"Ready for review — {_ping(audience, heard)}\n\n"
            f"`/approve` to support, `/object <reason>` to block, `/withdraw` to retract. "
            f"Needs {QUORUM} approvals; auto-approves after {LAZY_DAYS}d without objection.",
        )
    # Announce before the clock runs out; a silent auto-approval is just a
    # rubber stamp with extra steps. Once per revision, hence the marker check.
    if decision.warn and not _posted_since(comments, WARN_MARKER, since):
        _comment(
            repo,
            issue,
            WARN_MARKER,
            f"{_ping(audience, heard)} — auto-approves under lazy consensus in "
            f"{LAZY_DAYS - counted.waiting_days(ctx.now):.0f} day(s) "
            f"({len(counted.approvals)} approval, no objection). `/object` to stop the clock.",
        )


@app.command
def tally_issue(issue: int, *, repo: str = "", now: str = "", team: str = "") -> None:
    """Re-tally one proposal issue and write its derived state.

    `--team` mentions a GitHub team (`org/slug`) instead of naming individuals.
    """
    repo = repo or _gh(["repo", "view", "--json", "nameWithOwner", "-q", ".nameWithOwner"]).strip()
    raw = json.loads(
        _gh(
            [
                "issue",
                "view",
                str(issue),
                "--repo",
                repo,
                "--json",
                "comments,labels,createdAt,state",
            ]
        )
    )
    labels = {lab["name"] for lab in raw["labels"]}
    if reason := settled(labels, raw["state"] == "CLOSED"):
        print(f"#{issue}: skipped — {reason}")
        return
    comments = _load_comments(raw["comments"])
    since = last_reset(comments, _ts(raw["createdAt"]))
    audience = Audience(
        electorate=writers(repo),
        team=team,
        team_members=resolve_team(team) if team else (),
    )
    counted = tally(parse_commands(comments, since), audience.electorate)
    # Upvotes span the whole history, not just the current revision: they register
    # interest in the idea, which a wording change does not invalidate.
    supporters = count_upvotes(parse_commands(comments, None), thumbs_up(repo, issue))
    moment = _ts(now) if now else dt.datetime.now(dt.UTC)
    decision = replace(
        decide(counted, moment, current=current_state(labels)), upvotes=len(supporters)
    )
    moved = _apply(repo, issue, decision, labels)
    print(f"#{issue}: {decision.state} — {decision.reason}")
    # State is already written. Notifying is best-effort from here: losing a ping is a
    # nuisance, losing the derived state is a broken gate.
    try:
        _announce(repo, issue, Outcome(decision, counted, comments, since, moment, moved), audience)
    except subprocess.CalledProcessError as exc:
        _annotate("warning", f"#{issue}: state written but notification failed: {exc}")


@app.command
def sweep(*, repo: str = "", team: str = "") -> None:
    """Re-tally every open proposal — the cron arm that lets the lazy-consensus clock fire."""
    repo = repo or _gh(["repo", "view", "--json", "nameWithOwner", "-q", ".nameWithOwner"]).strip()
    listing = _gh(
        [
            "issue",
            "list",
            "--repo",
            repo,
            "--label",
            "type:proposal",
            "--state",
            "open",
            "--limit",
            "200",
            "--json",
            "number",
            "-q",
            ".[].number",
        ]
    )
    failed: list[str] = []
    for line in listing.split():
        # One unreadable issue must not strand the remaining 60-odd.
        try:
            tally_issue(int(line), repo=repo, team=team)
        except subprocess.CalledProcessError as exc:
            failed.append(line)
            _annotate("warning", f"#{line}: tally failed: {exc}")
    if failed:
        _annotate("error", f"sweep could not tally: {', '.join('#' + n for n in failed)}")


if __name__ == "__main__":
    sys.exit(app())

"""Invariants over the single-container nginx assets.

These exist because the skill's value is a dozen security invariants, and prose
does not fail CI. A template edit that drops one fails here instead of in a
production deploy. Every assertion names the invariant from the spec.
"""

from __future__ import annotations

import re
import shutil
import subprocess
import tempfile
from pathlib import Path

import pytest
from cloud_run_support import _template


def test_entrypoint_uses_bash_for_wait_n():
    # `wait -n` is a bashism. Under dash it fails at runtime, inside a container,
    # on the first deploy -- the most expensive place to discover it.
    entrypoint = _template("entrypoint.sh")
    assert entrypoint.startswith("#!/usr/bin/env bash")
    assert "wait -n" in entrypoint


def test_entrypoint_exits_when_any_child_exits():
    # A supervisor that restarts children in place keeps a broken revision
    # serving traffic; Cloud Run must be allowed to replace it.
    entrypoint = _template("entrypoint.sh")
    assert "trap" in entrypoint
    assert re.search(r"wait -n\s*\n\s*exit", entrypoint)


def test_envsubst_is_restricted_to_port():
    # Unrestricted envsubst eats nginx's own $host and $remote_addr, and the
    # resulting config is silently wrong rather than broken.
    entrypoint = _template("entrypoint.sh")
    assert "envsubst '${PORT}'" in entrypoint


def test_nginx_listens_on_the_cloud_run_port():
    conf = _template("nginx.conf")
    assert "listen       ${PORT}" in conf or "listen ${PORT}" in conf


def test_nginx_forwards_the_proxy_headers():
    # Without X-Forwarded-Proto the apps generate http:// URLs behind Cloud Run's
    # TLS terminator, which breaks redirects and cookies.
    conf = _template("nginx.conf")
    assert "X-Forwarded-Proto" in conf
    assert "X-Forwarded-For" in conf


def test_envsubst_restriction_actually_works():
    # Behavioral test: extract the actual envsubst command from entrypoint.sh,
    # run it on nginx.conf with PORT=9999, and verify the restriction is effective.

    if shutil.which("envsubst") is None:
        pytest.skip("envsubst not installed")

    # Extract the envsubst command from entrypoint.sh
    entrypoint = _template("entrypoint.sh")
    lines = entrypoint.split("\n")

    # Find the line containing envsubst
    envsubst_idx = None
    for i, line in enumerate(lines):
        if "envsubst" in line and not line.strip().startswith("#"):
            envsubst_idx = i
            break

    assert envsubst_idx is not None, "envsubst command not found in entrypoint.sh"

    # Collect the full command (handles backslash continuation)
    command_lines = [lines[envsubst_idx]]
    idx = envsubst_idx
    while idx < len(lines) - 1 and lines[idx].rstrip().endswith("\\"):
        idx += 1
        command_lines.append(lines[idx])

    full_command = " ".join(line.rstrip().rstrip("\\").strip() for line in command_lines)

    # Extract the envsubst portion: envsubst '${PORT}' or similar
    envsubst_match = re.search(r"(envsubst\s+'[^']*')", full_command)
    assert envsubst_match is not None, f"could not extract envsubst from: {full_command}"
    envsubst_cmd = envsubst_match.group(1)

    # Load nginx.conf template
    conf = _template("nginx.conf")
    with tempfile.NamedTemporaryFile(mode="w", suffix=".conf", delete=False) as f:
        f.write(conf)
        f.flush()
        temp_path = f.name

    try:
        # Run the actual envsubst command from entrypoint.sh
        result = subprocess.run(
            ["bash", "-c", f"{envsubst_cmd} < {temp_path}"],
            capture_output=True,
            text=True,
            timeout=5,
            env={"PORT": "9999"},
            check=True,
        )

        output = result.stdout
        # PORT should be substituted in the listen directive
        assert "listen       9999" in output or "listen 9999" in output, (
            f"PORT=9999 not substituted in listen directive. Output: {output[:200]}"
        )
        # nginx variables should still be present (NOT substituted)
        assert "$host" in output, "$host was consumed by restricted envsubst"
        assert "$remote_addr" in output, "$remote_addr was consumed by restricted envsubst"
        assert "$proxy_add_x_forwarded_for" in output, "$proxy_add_x_forwarded_for was consumed"

        # Now run unrestricted envsubst to prove the restriction matters
        result_unrestricted = subprocess.run(
            ["bash", "-c", f"envsubst < {temp_path}"],
            capture_output=True,
            text=True,
            timeout=5,
            env={"PORT": "9999"},
            check=True,
        )

        unrestricted_output = result_unrestricted.stdout
        # Unrestricted envsubst should destroy nginx's variables (they're undefined)
        assert "$host" not in unrestricted_output, (
            "unrestricted envsubst should destroy $host, but it survived"
        )
        assert "$remote_addr" not in unrestricted_output, (
            "unrestricted envsubst should destroy $remote_addr, but it survived"
        )

    finally:
        Path(temp_path).unlink()


def test_dockerfile_installs_bash_for_wait_n():
    # bash is required for wait -n in entrypoint.sh. Under dash or sh, the
    # container fails at runtime on the first deploy (most expensive discovery).
    dockerfile = _template("Dockerfile.combined")
    assert "bash" in dockerfile, "bash must be installed for wait -n in entrypoint.sh"
    # Verify it's in the apt-get install line, not just mentioned in a comment
    lines = dockerfile.split("\n")
    for line in lines:
        if "apt-get install" in line:
            install_block = []
            idx = lines.index(line)
            # Collect the complete multi-line install command (handles backslash continuation)
            while idx < len(lines):
                install_block.append(lines[idx])
                if not lines[idx].rstrip().endswith("\\"):
                    break
                idx += 1
            install_text = " ".join(install_block)
            assert "bash" in install_text, (
                "bash must be in apt-get install command, not just mentioned elsewhere"
            )
            return
    raise AssertionError("apt-get install line not found in Dockerfile.combined")


def test_dockerfile_installs_gettext_base_for_envsubst():
    # gettext-base provides envsubst, which entrypoint.sh uses to render nginx.conf.
    # Removing it while leaving bash installed fails at container start with
    # "command not found: envsubst" — the most expensive discovery.
    dockerfile = _template("Dockerfile.combined")
    assert "gettext-base" in dockerfile, "gettext-base must be installed for envsubst"
    # Verify it's in the apt-get install line, not just mentioned in a comment
    lines = dockerfile.split("\n")
    for line in lines:
        if "apt-get install" in line:
            install_block = []
            idx = lines.index(line)
            # Collect the complete multi-line install command (handles backslash continuation)
            while idx < len(lines):
                install_block.append(lines[idx])
                if not lines[idx].rstrip().endswith("\\"):
                    break
                idx += 1
            install_text = " ".join(install_block)
            assert "gettext-base" in install_text, (
                "gettext-base must be in apt-get install command, not just mentioned elsewhere"
            )
            return
    raise AssertionError("apt-get install line not found in Dockerfile.combined")


def test_wait_n_returns_on_first_child():
    # Behavioral test: Extract the entrypoint structure and verify wait -n returns
    # on first child death, not blocking for all children. Derived directly from
    # the shipped template, not a hand-written copy.
    entrypoint = _template("entrypoint.sh")

    # Transform the template into a runnable script
    runnable = entrypoint
    # Substitute the two commands: one exits quickly (exit code 7), one sleeps 30s
    runnable = runnable.replace("{{API_COMMAND}}", "bash -c 'sleep 0.2; exit 7'")
    runnable = runnable.replace("{{WEB_COMMAND}}", "sleep 30")
    # Replace envsubst block with a stub (we don't need nginx for this test)
    # The envsubst block is multi-line with backslash continuation
    lines = runnable.split("\n")
    new_lines = []
    skip_until_complete = False
    for _i, line in enumerate(lines):
        if skip_until_complete:
            if not line.rstrip().endswith("\\"):
                skip_until_complete = False
            continue
        if "envsubst" in line and not line.strip().startswith("#"):
            # Skip this line and any continuation lines
            skip_until_complete = line.rstrip().endswith("\\")
            new_lines.append("# envsubst replaced with stub for test")
            continue
        # Replace nginx line with a stub (it doesn't need to run)
        if "nginx -g 'daemon off;'" in line:
            new_lines.append("# nginx stub for test")
            continue
        new_lines.append(line)
    runnable = "\n".join(new_lines)

    with tempfile.NamedTemporaryFile(mode="w", suffix=".sh", delete=False) as f:
        f.write(runnable)
        f.flush()
        temp_path = f.name

    try:
        # wait -n should return in ~0.2s when API_COMMAND exits
        # plain wait would block for 30s and exceed the timeout
        # Use 10s timeout with 30s sleep to get a clear discrimination
        try:
            subprocess.run(
                ["bash", temp_path],
                timeout=10,
                check=False,
                start_new_session=True,  # Isolate process group
            )
            # If we reach here, wait -n returned promptly
            # The exit code varies due to signal handling but that's OK - we're testing
            # that the script returns quickly, not the exact exit code
        except subprocess.TimeoutExpired as e:
            raise AssertionError(
                "wait -n did not return; script blocked past timeout (mutated to plain wait?)"
            ) from e

    finally:
        Path(temp_path).unlink()

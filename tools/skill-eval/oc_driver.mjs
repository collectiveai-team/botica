#!/usr/bin/env node
/**
 * Minimal OpenCode SDK driver for the Layer 3 executor.
 *
 * We tried Promptfoo's `opencode:sdk` provider first (per the user's
 * preference), and it works for plain chat, but `session.prompt()`'s return
 * value only carries the *last* message's parts. For multi-step tool-calling
 * turns (exactly what skills like `journalist` need), that means the
 * transcript/tool-call trace it exposes is incomplete. This driver talks to
 * the same `@opencode-ai/sdk` (v2) directly and additionally calls
 * `session.messages()` after the prompt resolves, to fetch the *entire*
 * turn's messages/parts (every tool call across every step), which is what
 * metrics.json/grading.json need.
 *
 * Reads all config from a single JSON blob on stdin, writes a single JSON
 * result to stdout. Never touches process.env for secrets: OpenCode's own
 * server resolves provider auth (e.g. the `nvidia` key) from its own auth
 * store, the same way the interactive CLI does.
 */
import { createOpencode } from "@opencode-ai/sdk/v2";
import fs from "node:fs";
import { Agent, setGlobalDispatcher } from "undici";

// Undici's default headersTimeout is 300_000ms (5 min): if the OpenCode
// server doesn't send response headers before then (expected here --
// session.prompt() only responds once the *entire* multi-step agentic turn
// finishes, which can take well over 5 minutes for tool-heavy skills), fetch
// throws a generic "fetch failed" with no useful detail. Disable it and let
// our own AbortSignal.timeout(timeoutMs) below be the only deadline.
setGlobalDispatcher(new Agent({ headersTimeout: 0, bodyTimeout: 0, connectTimeout: 30_000 }));

function permissionRuleset(perm) {
  return Object.entries(perm || {}).map(([permission, action]) => ({
    permission,
    pattern: "*",
    action,
  }));
}

async function main() {
  const input = JSON.parse(fs.readFileSync(0, "utf-8"));
  const {
    workingDir,
    providerID,
    modelID,
    prompt,
    tools,
    permission,
    timeoutMs = 300000,
  } = input;

  const opencode = await createOpencode({
    hostname: "127.0.0.1",
    port: 0,
    timeout: 30000,
    config: { tools, permission },
  });
  const client = opencode.client;
  let sessionID;
  try {
    const created = await client.session.create({
      directory: workingDir,
      title: `skill-eval-${Date.now()}`,
      permission: permissionRuleset(permission),
    });
    sessionID = created.data.id;

    const t0 = Date.now();
    const promptResp = await client.session.prompt(
      {
        sessionID,
        directory: workingDir,
        model: { providerID, modelID },
        tools,
        parts: [{ type: "text", text: prompt }],
      },
      { fetch: (url, opts) => fetch(url, { ...opts, signal: AbortSignal.timeout(timeoutMs) }) }
    );
    const durationMs = Date.now() - t0;
    if (promptResp.error) {
      const e = promptResp.error;
      const detail = e instanceof Error ? `${e.name}: ${e.message}` : JSON.stringify(e, Object.getOwnPropertyNames(e || {}));
      throw new Error(`session.prompt failed after ${durationMs}ms: ${detail}`);
    }

    const messagesResp = await client.session.messages({ sessionID, directory: workingDir });
    const messages = messagesResp.data || [];

    // Aggregate every part across every message in this turn (a multi-step
    // agentic turn spans multiple assistant messages, each one step).
    const allParts = [];
    let finalText = "";
    let totalTokens = { input: 0, output: 0, reasoning: 0, total: 0 };
    let cost = 0;
    for (const m of messages) {
      const info = m.info;
      if (info?.role === "assistant") {
        if (info.tokens) {
          totalTokens.input += info.tokens.input || 0;
          totalTokens.output += info.tokens.output || 0;
          totalTokens.reasoning += info.tokens.reasoning || 0;
          totalTokens.total += info.tokens.total || 0;
        }
        cost += info.cost || 0;
      }
      for (const part of m.parts || []) {
        allParts.push({ ...part, _messageRole: info?.role });
        if (part.type === "text" && info?.role === "assistant") {
          finalText += (finalText ? "\n" : "") + part.text;
        }
      }
    }

    const lastAssistant = promptResp.data?.info;
    const output = JSON.stringify({
      ok: true,
      sessionID,
      durationMs,
      output: finalText,
      tokenUsage: totalTokens,
      cost,
      lastMessageID: lastAssistant?.id,
      parts: allParts,
    });
    process.stdout.write(output + "\n");
  } catch (err) {
    process.stdout.write(
      JSON.stringify({ ok: false, sessionID, error: String(err?.stack || err) }) + "\n"
    );
  } finally {
    if (sessionID) {
      try {
        await client.session.delete({ sessionID, directory: workingDir });
      } catch {
        // best-effort cleanup
      }
    }
    try {
      opencode.server.close();
    } catch {
      // best-effort cleanup
    }
  }
}

main().catch((err) => {
  process.stdout.write(JSON.stringify({ ok: false, error: String(err?.stack || err) }) + "\n");
  process.exit(1);
});

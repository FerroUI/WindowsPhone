import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import { resolveReuseRun, validateArtifact, validateRun, verifyRuntimeSources } from "./verify-reuse.mjs";

const runId = "45378305962";
const repository = "FerroUI/WindowsPhone";
const sha = "1a2046672e0e82ffd3fe978fdae8ff4c0b5c8366";
const push = commitMessage => resolveReuseRun({ eventName: "push", commitMessage });
const sourceRun = () => ({
  id: Number(runId), repository: { full_name: repository }, head_repository: { full_name: repository },
  head_branch: "main", path: ".github/workflows/pages.yml", status: "completed", event: "push",
  head_sha: sha, conclusion: "failure",
});
const sourceArtifact = () => ({
  id: 12345, name: "compiled-app", expired: false,
  workflow_run: { id: Number(runId), head_sha: sha },
});

test("ordinary builds need no reuse request or Actions credentials", () => {
  assert.equal(push("Refine rendered checks"), null);
  assert.equal(resolveReuseRun({ eventName: "workflow_dispatch", input: " " }), null);
  const output = execFileSync(process.execPath, ["scripts/verify-reuse.mjs"], {
    encoding: "utf8",
    env: { ...process.env, PHONE_REUSE_RUN: "", PHONE_REUSE_COMMIT_MESSAGE: "Ordinary push",
      GITHUB_EVENT_NAME: "push", GITHUB_TOKEN: "", GITHUB_OUTPUT: "", GITHUB_REPOSITORY: "" },
  });
  assert.match(output, /No compiled artifact reuse requested/);
});

test("push markers and dispatch inputs resolve the same canonical run ID", () => {
  assert.equal(push(`Recheck native input\n\n[reuse-run:${runId}]`), runId);
  assert.equal(push(`Literal shell text $(false) and \`false\`\n[reuse-run:${runId}]`), runId);
  assert.equal(resolveReuseRun({ eventName: "workflow_dispatch", input: ` ${runId}\n` }), runId);
});

test("malformed and unsafe reuse markers fail instead of starting a normal build", () => {
  for (const marker of [
    "[reuse-run]", "[reuse-run:]", "[reuse-run:0]", "[reuse-run:01]", "[reuse-run:-1]",
    "[reuse-run:1.5]", "[reuse-run:1e3]", "[reuse-run:9007199254740992]",
    "[reuse-run:1", "[reuse-run:1 extra]", "[reuse-run: 1]", "[ reuse-run:1]", "[REUSE-RUN:1]",
  ]) assert.throws(() => push(`Recheck\n${marker}`), /Malformed reuse marker|reuse_run must be a run ID/);
  for (const input of ["0", "01", "-1", "1.5", "1e3", "9007199254740992", "1\n2"])
    assert.throws(() => resolveReuseRun({ eventName: "workflow_dispatch", input }), /reuse_run must be a run ID/);
});

test("duplicate and conflicting reuse requests are rejected", () => {
  for (const other of [runId, "123"])
    assert.throws(() => push(`[reuse-run:${runId}]\n[reuse-run:${other}]`), /Duplicate or conflicting/);
  for (const input of [runId, "123"])
    assert.throws(() => resolveReuseRun({ eventName: "workflow_dispatch", input,
      commitMessage: `[reuse-run:${runId}]` }), /Duplicate or conflicting/);
});

test("reuse request mechanisms remain tied to their supported workflow events", () => {
  for (const eventName of ["workflow_dispatch", "pull_request", "schedule", ""])
    assert.throws(() => resolveReuseRun({ eventName, commitMessage: `[reuse-run:${runId}]` }), /requires push/);
  for (const eventName of ["push", "pull_request", "schedule", ""])
    assert.throws(() => resolveReuseRun({ eventName, input: runId }), /requires workflow_dispatch/);
});

test("reuse requires the exact completed main run from this repository and Pages workflow", () => {
  // A completed run whose render gate failed may supply a compiled artifact:
  // the current workflow still revalidates source identity and reruns rendering.
  assert.equal(validateRun(sourceRun(), repository, runId), sha);
  for (const patch of [
    { id: 123 }, { repository: { full_name: "Other/WindowsPhone" } },
    { head_repository: { full_name: "Fork/WindowsPhone" } }, { head_branch: "feature" },
    { path: ".github/workflows/android.yml" }, { status: "in_progress" },
    { event: "pull_request" }, { head_sha: "HEAD; false" },
  ]) assert.throws(() => validateRun({ ...sourceRun(), ...patch }, repository, runId));
});

test("reuse requires one unexpired artifact from the validated run and commit", () => {
  const artifact = sourceArtifact();
  assert.equal(validateArtifact([artifact], sourceRun()), artifact.id);
  for (const artifacts of [
    [], [artifact, artifact], [{ ...artifact, expired: true }], [{ ...artifact, name: "ui-verification" }],
    [{ ...artifact, id: 0 }], [{ ...artifact, id: Number.MAX_SAFE_INTEGER + 1 }],
    [{ ...artifact, workflow_run: { id: 123, head_sha: sha } }],
    [{ ...artifact, workflow_run: { id: Number(runId), head_sha: "0".repeat(40) } }],
  ]) assert.throws(() => validateArtifact(artifacts, sourceRun()));
});

test("source reuse allows harness changes but rejects every compiled-input boundary and framework pin changes", () => {
  const directory = mkdtempSync(join(tmpdir(), "phone-reuse-"));
  const origin = join(directory, "origin"), checkout = join(directory, "checkout");
  const git = (cwd, ...args) => execFileSync("git", ["-c", "user.name=Reuse test",
    "-c", "user.email=reuse-test@example.invalid", ...args], { cwd, encoding: "utf8", stdio: "pipe" }).trim();
  const write = (root, path, text) => {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), text);
  };
  try {
    mkdirSync(origin);
    git(origin, "init", "--quiet");
    const compiled = ["crates/example/src/lib.rs", "Cargo.toml", "Cargo.lock", ".cargo/config.toml",
      ".gitmodules", "rust-toolchain.toml", "scripts/build-browser.sh", "scripts/prepare-site.mjs"];
    const harness = ["scripts/ui-metro-controls.mjs", ".github/workflows/pages.yml", "docs/evidence.json"];
    for (const path of [...compiled, ...harness]) write(origin, path, "# original\n");
    git(origin, "add", ".");
    git(origin, "commit", "--quiet", "-m", "Fixture files");
    const frameworkPin = git(origin, "rev-parse", "HEAD");
    git(origin, "update-index", "--add", "--cacheinfo", `160000,${frameworkPin},FerroUI`);
    git(origin, "commit", "--quiet", "-m", "Pin framework");
    const sourceSha = git(origin, "rev-parse", "HEAD");
    git(directory, "clone", "--quiet", origin, checkout);
    for (const path of harness) write(checkout, path, "corrected verification\n");
    git(checkout, "add", ".");
    git(checkout, "commit", "--quiet", "-m", "Harness correction");
    const harnessSha = git(checkout, "rev-parse", "HEAD");
    assert.doesNotThrow(() => verifyRuntimeSources(sourceSha, checkout));
    for (const path of compiled) {
      git(checkout, "reset", "--hard", harnessSha);
      write(checkout, path, "# changed runtime input\n");
      git(checkout, "add", path);
      git(checkout, "commit", "--quiet", "-m", "Runtime change");
      assert.throws(() => verifyRuntimeSources(sourceSha, checkout), /Runtime sources differ/, path);
    }
    git(checkout, "reset", "--hard", harnessSha);
    git(checkout, "update-index", "--cacheinfo", `160000,${sourceSha},FerroUI`);
    git(checkout, "commit", "--quiet", "-m", "Different framework pin");
    assert.throws(() => verifyRuntimeSources(sourceSha, checkout), /Runtime sources differ/);
    assert.throws(() => verifyRuntimeSources("HEAD", checkout), /Invalid source commit/);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

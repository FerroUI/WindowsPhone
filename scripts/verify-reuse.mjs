import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { appendFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

const runtimeInputs = [
  "crates", "Cargo.toml", "Cargo.lock", ".cargo", "FerroUI", ".gitmodules",
  "rust-toolchain.toml", "scripts/build-browser.sh", "scripts/prepare-site.mjs",
];

export function resolveReuseRun({ input = "", commitMessage = "", eventName = "" }) {
  const inputRun = input.trim();
  // Recognize malformed variants too, so a broken request cannot silently
  // fall back to a normal build. Only the exact marker syntax is accepted.
  const starts = [...commitMessage.matchAll(/\[\s*reuse-run/gi)];
  assert(starts.length <= 1, "Duplicate or conflicting reuse-run markers");
  let markerRun = "";
  if (starts.length) {
    const marker = /^\[reuse-run:([1-9][0-9]*)\]/.exec(commitMessage.slice(starts[0].index));
    assert(marker, "Malformed reuse marker; expected [reuse-run:RUN_ID]");
    markerRun = marker[1];
  }
  assert(!(inputRun && markerRun), "Duplicate or conflicting reuse_run input and commit marker");
  const runId = inputRun || markerRun;
  if (!runId) return null;
  assert(/^[1-9][0-9]*$/.test(runId) && Number.isSafeInteger(Number(runId)), "reuse_run must be a run ID");
  assert.equal(eventName, inputRun ? "workflow_dispatch" : "push",
    "Reuse input requires workflow_dispatch; a reuse marker requires push");
  return runId;
}

export function validateRun(run, repository, runId) {
  assert.equal(String(run.id), runId, "The API returned a different run");
  assert.equal(run.repository?.full_name, repository, "Run belongs to another repository");
  assert.equal(run.head_repository?.full_name, repository, "Fork artifacts cannot be reused");
  assert.equal(run.head_branch, "main", "Only main-branch artifacts can be reused");
  assert.equal(run.path, ".github/workflows/pages.yml", "Artifact must come from the Pages build");
  assert.equal(run.status, "completed", "Wait for the original run to complete");
  assert(["push", "workflow_dispatch"].includes(run.event), "Unsupported source workflow event");
  assert(/^[0-9a-f]{40}$/.test(run.head_sha), "Invalid source commit");
  return run.head_sha;
}

export function verifyRuntimeSources(sha, cwd = process.cwd()) {
  assert(/^[0-9a-f]{40}$/.test(sha), "Invalid source commit");
  execFileSync("git", ["fetch", "--no-tags", "--depth=1", "origin", sha], { cwd, stdio: "pipe" });
  try {
    execFileSync("git", ["diff", "--quiet", sha, "HEAD", "--", ...runtimeInputs], { cwd, stdio: "pipe" });
  } catch {
    throw new Error("Runtime sources differ or could not be compared; run a normal build instead.");
  }
}

export function validateArtifact(artifacts, run) {
  const matches = artifacts.filter(artifact => artifact.name === "compiled-app" && !artifact.expired);
  assert.equal(matches.length, 1, "Expected one unexpired compiled-app artifact");
  const artifact = matches[0];
  assert(Number.isSafeInteger(artifact.id) && artifact.id > 0, "Invalid artifact ID");
  assert.equal(artifact.workflow_run?.id, run.id, "Artifact belongs to another run");
  assert.equal(artifact.workflow_run?.head_sha, run.head_sha, "Artifact commit does not match");
  return artifact.id;
}

async function main() {
  const runId = resolveReuseRun({
    input: process.env.PHONE_REUSE_RUN || "",
    commitMessage: process.env.PHONE_REUSE_COMMIT_MESSAGE || "",
    eventName: process.env.GITHUB_EVENT_NAME || "",
  });
  if (runId === null) {
    console.log("No compiled artifact reuse requested.");
    return;
  }
  const repository = process.env.GITHUB_REPOSITORY || "";
  assert(/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository), "Invalid repository");
  assert.equal(process.env.GITHUB_REF, "refs/heads/main", "Replay must run from main");
  assert(process.env.GITHUB_TOKEN && process.env.GITHUB_OUTPUT, "Missing Actions credentials or output");
  const api = async endpoint => {
    const response = await fetch(`https://api.github.com/repos/${repository}/${endpoint}`, {
      headers: {
        Authorization: `Bearer ${process.env.GITHUB_TOKEN}`,
        Accept: "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
      },
      signal: AbortSignal.timeout(20000),
    });
    assert(response.ok, `GitHub API returned ${response.status} for ${endpoint}`);
    return response.json();
  };
  const run = await api(`actions/runs/${runId}`);
  const sha = validateRun(run, repository, runId);
  verifyRuntimeSources(sha);
  const listed = await api(`actions/runs/${runId}/artifacts?name=compiled-app&per_page=100`);
  const artifactId = validateArtifact(listed.artifacts, run);
  await appendFile(process.env.GITHUB_OUTPUT, `run_id=${runId}\nartifact_id=${artifactId}\nsource_sha=${sha}\n`);
  console.log(`Verified compiled-app ${artifactId} from run ${runId} (${sha}); runtime sources match HEAD.`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await main();
}

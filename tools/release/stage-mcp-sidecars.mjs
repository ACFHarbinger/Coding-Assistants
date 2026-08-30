#!/usr/bin/env node

/**
 * Stage the Creative Tools MCP bridge binaries under the filenames Tauri's
 * `bundle.externalBin` convention consumes. Cargo produces an unsuffixed
 * binary; Tauri requires `<name>-<target-triple>[.exe]` as its build input.
 *
 * Usage: node tools/release/stage-mcp-sidecars.mjs <target-triple> [source-dir]
 *
 * `source-dir` defaults to `target/<target-triple>/release`, which matches the
 * `cargo build --release --target <target-triple>` command in the release flow.
 */
import { copyFile, mkdir, stat } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const bridgeNames = [
  "coding-assistants-mcp-blender",
  "coding-assistants-mcp-krita",
  "coding-assistants-mcp-godot",
  "coding-assistants-mcp-aseprite",
  "coding-assistants-mcp-unreal",
  "coding-assistants-mcp-unity",
  "coding-assistants-mcp-opentoonz",
];

const [targetTriple, sourceDirArgument] = process.argv.slice(2);
if (!targetTriple || !/^[A-Za-z0-9_.-]+$/.test(targetTriple)) {
  console.error("usage: stage-mcp-sidecars.mjs <target-triple> [source-dir]");
  process.exit(2);
}

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDir, "../..");
const sourceDir = sourceDirArgument
  ? path.resolve(sourceDirArgument)
  : path.join(repositoryRoot, "target", targetTriple, "release");
const destinationDir = path.join(repositoryRoot, "src-tauri", "binaries");
const extension = targetTriple.includes("windows") ? ".exe" : "";

await mkdir(destinationDir, { recursive: true });

const missing = [];
for (const name of bridgeNames) {
  const source = path.join(sourceDir, `${name}${extension}`);
  try {
    if (!(await stat(source)).isFile()) missing.push(source);
  } catch {
    missing.push(source);
  }
}

if (missing.length > 0) {
  console.error("MCP sidecars were not built:");
  for (const file of missing) console.error(`  ${file}`);
  process.exit(1);
}

for (const name of bridgeNames) {
  const source = path.join(sourceDir, `${name}${extension}`);
  const destination = path.join(destinationDir, `${name}-${targetTriple}${extension}`);
  await copyFile(source, destination);
  console.log(`staged ${path.relative(repositoryRoot, destination)}`);
}

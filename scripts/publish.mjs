#!/usr/bin/env node
// Publishes platform packages first, then the main package, so the main
// package never points at platform versions that do not exist yet.
// Usage: node scripts/publish.mjs [--dry-run]
import { execFileSync } from "node:child_process";
import { readdirSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const platformsDir = path.join(root, "npm", "platforms");
const mainPkgDir = path.join(root, "npm", "delim-doctor");

const dryRun = process.argv.slice(2).includes("--dry-run");
const npmArgs = ["publish", "--access", "public"];
if (dryRun) npmArgs.push("--dry-run");

function publish(dir, label) {
  console.log(`publish: ${label}${dryRun ? " (dry-run)" : ""}`);
  execFileSync("npm", npmArgs, { cwd: dir, stdio: "inherit" });
}

const platformDirs = readdirSync(platformsDir).filter((name) => {
  try {
    return statSync(path.join(platformsDir, name)).isDirectory();
  } catch {
    return false;
  }
});

for (const name of platformDirs) {
  publish(path.join(platformsDir, name), `@rcoplolabs/${name}`);
}

publish(mainPkgDir, "delim-doctor");
console.log("publish: done");

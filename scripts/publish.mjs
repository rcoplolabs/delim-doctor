#!/usr/bin/env node
// Publishes platform packages first, then the main package, so the main
// package never points at platform versions that do not exist yet.
// Usage: node scripts/publish.mjs [--dry-run]
import { execFileSync } from "node:child_process";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const platformsDir = path.join(root, "npm", "platforms");
const mainPkgDir = path.join(root, "npm", "delim-doctor");

const dryRun = process.argv.slice(2).includes("--dry-run");
const npmArgs = ["publish", "--access", "public"];
if (dryRun) npmArgs.push("--dry-run");

function readMeta(dir) {
  const json = JSON.parse(readFileSync(path.join(dir, "package.json"), "utf8"));
  return { name: json.name, version: json.version };
}

async function alreadyPublished(name, version) {
  const url = `https://registry.npmjs.org/${name.replace("/", "%2F")}`;
  try {
    const res = await fetch(url);
    if (!res.ok) return false;
    const data = await res.json();
    return Boolean(data.versions && data.versions[version]);
  } catch {
    return false;
  }
}

async function publish(dir) {
  const { name, version } = readMeta(dir);
  if (await alreadyPublished(name, version)) {
    console.log(`skip: ${name}@${version} already published`);
    return;
  }
  console.log(`publish: ${name}@${version}${dryRun ? " (dry-run)" : ""}`);
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
  await publish(path.join(platformsDir, name));
}

await publish(mainPkgDir);
console.log("publish: done");

#!/usr/bin/env node
// Builds the Rust binary and copies it into the matching npm platform package.
// Usage: node scripts/build-all.mjs --only current
import { execFileSync } from "node:child_process";
import { chmodSync, copyFileSync, existsSync, mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const PLATFORMS_DIR = path.join(root, "npm", "platforms");

function linuxLibc() {
  if (process.platform !== "linux") return null;
  try {
    const report = process.report && process.report.getReport();
    return report && report.header && report.header.glibcVersionRuntime
      ? "gnu"
      : "musl";
  } catch {
    return "gnu";
  }
}

function platformSuffix() {
  const { platform, arch } = process;
  if (platform === "linux") return `linux-${arch}-${linuxLibc()}`;
  return `${platform}-${arch}`;
}

function binaryName() {
  return process.platform === "win32" ? "delim-doctor.exe" : "delim-doctor";
}

const args = process.argv.slice(2);
const onlyIdx = args.indexOf("--only");
const only = onlyIdx >= 0 ? args[onlyIdx + 1] : "current";
if (onlyIdx >= 0 && only !== "current") {
  console.error(`build-all: unsupported --only value '${only}' (expected 'current')`);
  process.exit(1);
}

const suffix = platformSuffix();
const pkgDir = path.join(PLATFORMS_DIR, suffix);
if (!existsSync(path.join(pkgDir, "package.json"))) {
  console.error(`build-all: no platform package for '${suffix}' at npm/platforms/${suffix}`);
  process.exit(1);
}

console.log(`build-all: building release for ${suffix} ...`);
execFileSync("cargo", ["build", "--release"], { cwd: root, stdio: "inherit" });

const bin = binaryName();
const src = path.join(root, "target", "release", bin);
if (!existsSync(src)) {
  console.error(`build-all: built binary not found at ${src}`);
  process.exit(1);
}

const destDir = path.join(pkgDir, "bin");
mkdirSync(destDir, { recursive: true });
const dest = path.join(destDir, bin);
copyFileSync(src, dest);
if (process.platform !== "win32") chmodSync(dest, 0o755);

console.log(`build-all: copied to ${path.relative(root, dest)}`);

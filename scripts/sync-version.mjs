#!/usr/bin/env node
// Single source of truth for the version: reads crates' Cargo.toml [package]
// version and writes it into the main npm package and every platform package.
import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function readCargoVersion() {
  const cargo = readFileSync(path.join(root, "Cargo.toml"), "utf8");
  const section = cargo.match(/\[package\][\s\S]*?(?=\n\[|$)/);
  if (!section) throw new Error("Cargo.toml: [package] section not found");
  const m = section[0].match(/^\s*version\s*=\s*"([^"]+)"/m);
  if (!m) throw new Error("Cargo.toml: [package] version not found");
  return m[1];
}

function writeVersion(pkgJsonPath, version) {
  const pkg = JSON.parse(readFileSync(pkgJsonPath, "utf8"));
  if (pkg.version === version) {
    console.log(`sync-version: ${path.relative(root, pkgJsonPath)} already ${version}`);
    return;
  }
  pkg.version = version;
  if (pkg.optionalDependencies) {
    for (const dep of Object.keys(pkg.optionalDependencies)) {
      pkg.optionalDependencies[dep] = version;
    }
  }
  writeFileSync(pkgJsonPath, JSON.stringify(pkg, null, 2) + "\n");
  console.log(`sync-version: ${path.relative(root, pkgJsonPath)} -> ${version}`);
}

const version = readCargoVersion();
console.log(`sync-version: source version is ${version}`);

writeVersion(path.join(root, "npm", "delim-doctor", "package.json"), version);

const platformsDir = path.join(root, "npm", "platforms");
for (const name of readdirSync(platformsDir)) {
  const pkgJsonPath = path.join(platformsDir, name, "package.json");
  try {
    if (!statSync(pkgJsonPath).isFile()) continue;
  } catch {
    continue;
  }
  writeVersion(pkgJsonPath, version);
}

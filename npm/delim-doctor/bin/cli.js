#!/usr/bin/env node
"use strict";

const { spawn } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

// Detect glibc vs musl on Linux so the correct platform package is selected.
function linuxLibc() {
  try {
    const report = process.report && process.report.getReport();
    return report && report.header && report.header.glibcVersionRuntime
      ? "gnu"
      : "musl";
  } catch {
    return "gnu";
  }
}

// Maps the running platform to the @rcoplolabs/delim-doctor-<suffix> package name.
function platformSuffix() {
  const { platform, arch } = process;
  if (platform === "linux") {
    return `linux-${arch}-${linuxLibc()}`;
  }
  return `${platform}-${arch}`;
}

const SUFFIX = platformSuffix();
const BIN_NAME =
  process.platform === "win32" ? "delim-doctor.exe" : "delim-doctor";
const PKG_NAME = `@rcoplolabs/delim-doctor-${SUFFIX}`;

// The platform package is an optionalDependency, so `npm install --omit=optional`
// (or --no-optional) leaves it out. Fall back to a clear, actionable message.
function failMissingPlatformPackage() {
  console.error(`delim-doctor: no prebuilt binary for ${SUFFIX}.`);
  console.error(``);
  console.error(`The platform package ${PKG_NAME} is not installed. This usually means:`);
  console.error(`  1. npm install ran with --omit=optional or --no-optional, which skips`);
  console.error(`     optionalDependencies (some CI configs and corporate proxies do this);`);
  console.error(`  2. this platform has no prebuilt package.`);
  console.error(``);
  console.error(`To fix, install the platform package explicitly:`);
  console.error(`  npm install ${PKG_NAME}`);
  console.error(`or reinstall without omitting optional dependencies:`);
  console.error(`  npm install delim-doctor`);
  console.error(`To build from source instead, see the project repository.`);
  process.exit(1);
}

function failMissingBinary(binaryPath) {
  console.error(`delim-doctor: binary not found at ${binaryPath}`);
  console.error(``);
  console.error(`The platform package ${PKG_NAME} is installed but its binary is missing,`);
  console.error(`so the installation is corrupt or incomplete.`);
  console.error(``);
  console.error(`To fix, reinstall it:`);
  console.error(`  npm install ${PKG_NAME} --force`);
  console.error(`or download the binary manually from:`);
  console.error(`  https://github.com/rcoplolabs/delim-doctor/releases`);
  process.exit(1);
}

let binPath;
try {
  const pkgJson = require.resolve(`${PKG_NAME}/package.json`);
  binPath = path.join(path.dirname(pkgJson), "bin", BIN_NAME);
} catch {
  failMissingPlatformPackage();
}

if (!fs.existsSync(binPath)) {
  failMissingBinary(binPath);
}

const child = spawn(binPath, process.argv.slice(2), { stdio: "inherit" });

child.on("error", (err) => {
  console.error(`delim-doctor: failed to start binary ${binPath}`);
  console.error(`error: ${err.message}`);
  if (err.code === "ENOENT") {
    failMissingBinary(binPath);
  } else if (err.code === "EACCES") {
    console.error(`Insufficient permissions; check file permissions or run as administrator.`);
  }
  process.exit(1);
});

child.on("exit", (code, signal) => {
  if (signal) {
    console.error(`delim-doctor: process terminated by signal ${signal}`);
    process.exit(1);
  }
  process.exit(code ?? 1);
});

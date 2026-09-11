#!/usr/bin/env node
"use strict";

const { spawn } = require("node:child_process");
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

// Maps the running platform to the @rcoplolabs/<suffix> package name.
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

let binPath;
try {
  const pkg = `@rcoplolabs/${SUFFIX}`;
  const pkgJson = require.resolve(`${pkg}/package.json`);
  binPath = path.join(path.dirname(pkgJson), "bin", BIN_NAME);
} catch {
  console.error(`delim-doctor: unsupported platform ${SUFFIX}.`);
  console.error(`Possible causes:`);
  console.error(`  1. npm install used --omit=optional, skipping the platform package`);
  console.error(`  2. no prebuilt package exists for this platform; build from source`);
  console.error(`  3. open an issue in the project repository`);
  process.exit(1);
}

const child = spawn(binPath, process.argv.slice(2), { stdio: "inherit" });

child.on("error", (err) => {
  console.error(`delim-doctor: failed to start binary ${binPath}`);
  console.error(`error: ${err.message}`);
  if (err.code === "ENOENT") {
    console.error(`Binary file is missing; the platform package may be corrupt, please reinstall.`);
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

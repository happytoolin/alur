#!/usr/bin/env node
"use strict";

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const PLATFORM_PACKAGES = {
  "darwin:arm64": "@happytoolin/hni-darwin-arm64",
  "darwin:x64": "@happytoolin/hni-darwin-x64",
  "linux:arm64": "@happytoolin/hni-linux-arm64-musl",
  "linux:x64": "@happytoolin/hni-linux-x64-musl",
  "win32:arm64": "@happytoolin/hni-win32-arm64-msvc",
  "win32:x64": "@happytoolin/hni-win32-x64-msvc",
};

function resolveBinaryPath() {
  const ext = process.platform === "win32" ? ".exe" : "";
  const packageRoot = path.resolve(__dirname, "..");

  const packageName = PLATFORM_PACKAGES[`${process.platform}:${process.arch}`];
  if (packageName) {
    try {
      const packageJsonPath = require.resolve(`${packageName}/package.json`, {
        paths: [packageRoot],
      });
      const packageDir = path.dirname(packageJsonPath);

      for (const candidate of [
        path.join(packageDir, `hni${ext}`),
        path.join(packageDir, "bin", `hni${ext}`),
      ]) {
        if (fs.existsSync(candidate)) {
          return candidate;
        }
      }
    } catch {
      // optional dependency for this platform may not be installed in local dev.
    }
  }

  const override = process.env.HNI_NATIVE_BIN;
  if (override && fs.existsSync(override)) {
    return override;
  }

  const localBuild = path.join(packageRoot, "target", "release", `hni${ext}`);
  if (fs.existsSync(localBuild)) {
    return localBuild;
  }

  return null;
}

function run(invocation) {
  const binaryPath = resolveBinaryPath();
  if (!binaryPath) {
    console.error(
      "[hni] native binary not found. Reinstall `hni` for your platform or set HNI_NATIVE_BIN.",
    );
    process.exit(1);
  }

  const userArgs = process.argv.slice(2);
  const args = invocation === "hni" ? userArgs : [invocation, ...userArgs];

  const result = spawnSync(binaryPath, args, {
    stdio: "inherit",
    env: process.env,
    windowsHide: false,
  });

  if (result.error) {
    console.error(`[hni] failed to execute native binary: ${result.error.message}`);
    process.exit(1);
  }

  if (typeof result.status === "number") {
    process.exit(result.status);
  }

  process.exit(1);
}

module.exports = { run };

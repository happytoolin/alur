#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const aliasesPath = path.join(repoRoot, "aliases.json");

function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

function withFileContext(label, action, filePath, callback) {
  try {
    return callback();
  } catch (error) {
    throw new Error(
      `${label}: failed to ${action} ${filePath}: ${errorMessage(error)}`,
      { cause: error },
    );
  }
}

function readJson(filePath, label) {
  const raw = withFileContext(label, "read", filePath, () =>
    fs.readFileSync(filePath, "utf8")
  );
  return withFileContext(label, "parse JSON from", filePath, () =>
    JSON.parse(raw)
  );
}

function writeText(filePath, value, label, options = "utf8") {
  withFileContext(label, "write", filePath, () =>
    fs.writeFileSync(filePath, value, options)
  );
}

function writeJson(filePath, value, label) {
  const payload = withFileContext(label, "serialize JSON for", filePath, () =>
    `${JSON.stringify(value, null, 2)}\n`
  );
  writeText(filePath, payload, label);
}

function makeExecutable(filePath, label) {
  withFileContext(label, "chmod", filePath, () => fs.chmodSync(filePath, 0o755));
}

function wrapperSource(alias) {
  return `#!/usr/bin/env node

const { run } = require("./binary");
process.argv.splice(2, 0, ${JSON.stringify(alias)});
run("alur");
`;
}

function main() {
  const packageDir = process.argv[2];
  if (!packageDir) {
    throw new Error("usage: postprocess-cargo-dist-npm-package.mjs <generated-package-dir>");
  }

  const packageJsonPath = path.join(packageDir, "package.json");
  const packageJson = readJson(packageJsonPath, "generated package manifest");
  const aliases = readJson(aliasesPath, "alias manifest").alur ?? [];

  if (!packageJson.bin?.alur) {
    throw new Error("generated npm package is missing bin.alur");
  }

  packageJson.bin.alur = "run-alur.js";
  for (const alias of aliases) {
    const wrapper = `run-${alias}.js`;
    packageJson.bin[alias] = wrapper;
    const wrapperPath = path.join(packageDir, wrapper);
    writeText(wrapperPath, wrapperSource(alias), `alias wrapper ${alias}`, {
      mode: 0o755,
    });
    makeExecutable(wrapperPath, `alias wrapper ${alias}`);
  }

  writeJson(packageJsonPath, packageJson, "generated package manifest");
}

try {
  main();
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  process.stderr.write(`${message}\n`);
  process.exitCode = 1;
}

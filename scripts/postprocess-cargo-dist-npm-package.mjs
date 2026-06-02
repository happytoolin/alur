#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const aliasesPath = path.join(repoRoot, "aliases.json");

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function writeJson(filePath, value) {
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

function wrapperSource(alias) {
  return `#!/usr/bin/env node

const { run } = require("./binary");
process.argv.splice(2, 0, ${JSON.stringify(alias)});
run("hni");
`;
}

function main() {
  const packageDir = process.argv[2];
  if (!packageDir) {
    throw new Error("usage: postprocess-cargo-dist-npm-package.mjs <generated-package-dir>");
  }

  const packageJsonPath = path.join(packageDir, "package.json");
  const packageJson = readJson(packageJsonPath);
  const aliases = readJson(aliasesPath).hni ?? [];

  if (!packageJson.bin?.hni) {
    throw new Error("generated npm package is missing bin.hni");
  }

  packageJson.bin.hni = "run-hni.js";
  for (const alias of aliases) {
    const wrapper = `run-${alias}.js`;
    packageJson.bin[alias] = wrapper;
    const wrapperPath = path.join(packageDir, wrapper);
    fs.writeFileSync(wrapperPath, wrapperSource(alias), { mode: 0o755 });
    fs.chmodSync(wrapperPath, 0o755);
  }

  writeJson(packageJsonPath, packageJson);
}

try {
  main();
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  process.stderr.write(`${message}\n`);
  process.exitCode = 1;
}

import { dirname, join } from "jsr:@std/path@^1.1.4";

const REPO = "happytoolin/hni";
const VERSION = "0.0.3";
const TAG = VERSION.startsWith("v") ? VERSION : `v${VERSION}`;
const DEFAULT_DOWNLOAD_ROOT = "https://happytoolin.com/hni/releases/download";
const DEFAULT_FALLBACK_DOWNLOAD_ROOT =
  `https://github.com/${REPO}/releases/download`;

export const INVOCATIONS = [
  "hni",
  "ni",
  "nr",
  "nlx",
  "nun",
  "nci",
  "np",
  "ns",
] as const;

export type Invocation = typeof INVOCATIONS[number];

interface TargetInfo {
  target: string;
  ext: string;
}

export async function runInvocation(
  invocation: Invocation,
  rawArgs: string[] = Deno.args,
): Promise<never> {
  const { binaryPath } = await ensureBinary();
  const args = invocation === "hni" ? rawArgs : [invocation, ...rawArgs];
  const command = new Deno.Command(binaryPath, {
    args,
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit",
  });
  const { code } = await command.output();
  Deno.exit(code);
}

export async function ensureBinary(): Promise<{ binaryPath: string }> {
  const targetInfo = resolveTarget();
  const installDir = resolveInstallDir();
  const binaryPath = join(installDir, `hni${targetInfo.ext}`);
  const markerPath = join(installDir, ".version");
  const marker = `${TAG}:${targetInfo.target}`;

  await Deno.mkdir(installDir, { recursive: true });

  if (!(await isCurrentInstall(binaryPath, markerPath, marker))) {
    const primaryRoot = trimTrailingSlash(downloadRoot());
    const fallbackRoot = trimTrailingSlash(fallbackDownloadRoot());
    const rawAsset = `hni-${TAG}-${targetInfo.target}${targetInfo.ext}`;
    const rawPrimaryUrl = `${primaryRoot}/${TAG}/${rawAsset}`;
    const rawFallbackUrl = `${fallbackRoot}/${TAG}/${rawAsset}`;
    const rawPayload = await downloadWithFallback(
      rawPrimaryUrl,
      rawFallbackUrl,
    );

    if (rawPayload) {
      await Deno.writeFile(binaryPath, rawPayload);
      if (targetInfo.ext !== ".exe") {
        await Deno.chmod(binaryPath, 0o755);
      }
    } else {
      const archiveExt = targetInfo.ext === ".exe" ? ".zip" : ".tar.gz";
      const archiveAsset = `hni-${TAG}-${targetInfo.target}${archiveExt}`;
      const archivePrimaryUrl = `${primaryRoot}/${TAG}/${archiveAsset}`;
      const archiveFallbackUrl = `${fallbackRoot}/${TAG}/${archiveAsset}`;
      const archivePayload = await downloadWithFallback(
        archivePrimaryUrl,
        archiveFallbackUrl,
      );

      if (!archivePayload) {
        throw new Error(`failed to download ${rawAsset} or ${archiveAsset}`);
      }

      await installFromArchive(
        archivePayload,
        archiveExt,
        binaryPath,
        targetInfo.ext,
      );
    }

    await Deno.writeTextFile(markerPath, marker);
  }

  return { binaryPath };
}

function downloadRoot(): string {
  return Deno.env.get("HNI_DOWNLOAD_ROOT") ?? DEFAULT_DOWNLOAD_ROOT;
}

function fallbackDownloadRoot(): string {
  return Deno.env.get("HNI_FALLBACK_DOWNLOAD_ROOT") ??
    DEFAULT_FALLBACK_DOWNLOAD_ROOT;
}

function resolveTarget(): TargetInfo {
  if (Deno.build.os === "darwin") {
    if (Deno.build.arch === "x86_64") {
      return { target: "x86_64-apple-darwin", ext: "" };
    }
    if (Deno.build.arch === "aarch64") {
      return { target: "aarch64-apple-darwin", ext: "" };
    }
  }

  if (Deno.build.os === "linux") {
    if (Deno.build.arch === "x86_64") {
      return { target: "x86_64-unknown-linux-musl", ext: "" };
    }
    if (Deno.build.arch === "aarch64") {
      return { target: "aarch64-unknown-linux-musl", ext: "" };
    }
  }

  if (Deno.build.os === "windows") {
    if (Deno.build.arch === "x86_64") {
      return { target: "x86_64-pc-windows-msvc", ext: ".exe" };
    }
    if (Deno.build.arch === "aarch64") {
      return { target: "aarch64-pc-windows-msvc", ext: ".exe" };
    }
  }

  throw new Error(
    `unsupported platform/arch: ${Deno.build.os}/${Deno.build.arch}`,
  );
}

function resolveInstallDir(): string {
  const override = Deno.env.get("HNI_INSTALL_DIR");
  if (override) {
    return override;
  }

  if (Deno.build.os === "windows") {
    const localAppData = Deno.env.get("LOCALAPPDATA");
    if (localAppData) {
      return join(localAppData, "hni", "deno");
    }
    const userProfile = Deno.env.get("USERPROFILE");
    if (userProfile) {
      return join(userProfile, ".hni", "deno");
    }
    return join(dirname(Deno.execPath()), "hni");
  }

  const xdgCache = Deno.env.get("XDG_CACHE_HOME");
  if (xdgCache) {
    return join(xdgCache, "hni");
  }

  const home = Deno.env.get("HOME");
  if (home) {
    return join(home, ".cache", "hni");
  }

  return join(dirname(Deno.execPath()), "hni");
}

async function isCurrentInstall(
  binaryPath: string,
  markerPath: string,
  marker: string,
): Promise<boolean> {
  try {
    await Deno.stat(binaryPath);
    const found = await Deno.readTextFile(markerPath);
    return found.trim() === marker;
  } catch {
    return false;
  }
}

async function downloadWithFallback(
  primaryUrl: string,
  fallbackUrl: string,
): Promise<Uint8Array | null> {
  try {
    return await fetchBinary(primaryUrl);
  } catch (_error) {
    try {
      return await fetchBinary(fallbackUrl);
    } catch (_fallbackError) {
      return null;
    }
  }
}

async function fetchBinary(url: string): Promise<Uint8Array> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`download failed (${response.status}): ${url}`);
  }
  return new Uint8Array(await response.arrayBuffer());
}

function trimTrailingSlash(value: string): string {
  return value.endsWith("/") ? value.slice(0, -1) : value;
}

async function installFromArchive(
  payload: Uint8Array,
  archiveExt: ".tar.gz" | ".zip",
  binaryPath: string,
  binaryExt: string,
): Promise<void> {
  const tempRoot = await Deno.makeTempDir({ prefix: "hni-jsr-" });
  const archivePath = join(tempRoot, `hni${archiveExt}`);
  const extractDir = join(tempRoot, "extract");

  try {
    await Deno.mkdir(extractDir, { recursive: true });
    await Deno.writeFile(archivePath, payload);

    if (archiveExt === ".tar.gz") {
      await runCommand("tar", ["-xzf", archivePath, "-C", extractDir]);
    } else {
      const psScript =
        `Expand-Archive -Path "${archivePath}" -DestinationPath "${extractDir}" -Force`;
      try {
        await runCommand("powershell", ["-NoProfile", "-Command", psScript]);
      } catch (_error) {
        await runCommand("pwsh", ["-NoProfile", "-Command", psScript]);
      }
    }

    const extractedBinary = join(extractDir, `hni${binaryExt}`);
    await Deno.copyFile(extractedBinary, binaryPath);
    if (binaryExt !== ".exe") {
      await Deno.chmod(binaryPath, 0o755);
    }
  } finally {
    await Deno.remove(tempRoot, { recursive: true }).catch(() => {});
  }
}

async function runCommand(cmd: string, args: string[]): Promise<void> {
  const result = await new Deno.Command(cmd, {
    args,
    stdin: "null",
    stdout: "null",
    stderr: "null",
  }).output();

  if (result.code !== 0) {
    throw new Error(`command failed: ${cmd} ${args.join(" ")}`);
  }
}

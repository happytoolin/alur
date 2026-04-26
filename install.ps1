param(
  [string]$Version = "latest",
  [string]$InstallDir = "$env:LOCALAPPDATA\hni\bin"
)

$ErrorActionPreference = "Stop"
$Repo = "happytoolin/hni"
$DownloadRoot = "https://happytoolin.com"
$FallbackDownloadRoot = "https://github.com/$Repo"
$Aliases = @("ni", "nr", "nlx", "nru", "nun", "nci", "na", "np", "ns")

function Write-Log {
  param([string]$Message)
  Write-Host "[hni] $Message"
}

function Resolve-Tag {
  param([string]$RequestedVersion)

  if ($RequestedVersion -eq "latest") {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
    if (-not $release.tag_name) {
      throw "Unable to resolve latest release tag."
    }
    return "$($release.tag_name)"
  }

  if ($RequestedVersion.StartsWith("v")) {
    return $RequestedVersion
  }

  return "v$RequestedVersion"
}

function Resolve-Target {
  $arch = $env:PROCESSOR_ARCHITECTURE

  switch ($arch) {
    "AMD64" { return "x86_64-pc-windows-msvc" }
    "ARM64" { return "aarch64-pc-windows-msvc" }
    default { throw "Unsupported architecture: $arch" }
  }
}

function Download-Asset {
  param(
    [string]$Tag,
    [string]$Target,
    [string]$OutputPath
  )

  $asset = "hni-$Tag-$Target.zip"
  $primaryUrl = "$($DownloadRoot.TrimEnd('/'))/hni/releases/download/$Tag/$asset"
  $fallbackUrl = "$FallbackDownloadRoot/releases/download/$Tag/$asset"

  try {
    Invoke-WebRequest -Uri $primaryUrl -OutFile $OutputPath
    Write-Log "Downloaded $asset from $DownloadRoot"
  } catch {
    Write-Log "Primary URL failed, falling back to GitHub releases"
    Invoke-WebRequest -Uri $fallbackUrl -OutFile $OutputPath
  }
}

function Ensure-PathEntry {
  param([string]$PathEntry)

  $current = [Environment]::GetEnvironmentVariable("Path", "User")
  if (-not $current) {
    [Environment]::SetEnvironmentVariable("Path", $PathEntry, "User")
    return
  }

  $parts = $current.Split(';') | Where-Object { $_ -ne "" }
  if ($parts -contains $PathEntry) {
    return
  }

  [Environment]::SetEnvironmentVariable("Path", "$current;$PathEntry", "User")
}

function Write-ShellSetup {
  Write-Log "Shell setup:"
  Write-Log "  Add the hni init line at the end of your shell config, after nvm/mise/asdf/fnm/volta init."
  Write-Log '  PowerShell ($PROFILE): Invoke-Expression (& hni init powershell)'
  Write-Log '  bash (~/.bashrc): eval "$(hni init bash)"'
  Write-Log '  zsh (~/.zshrc): eval "$(hni init zsh)"'
  Write-Log '  fish (~/.config/fish/config.fish): hni init fish | source'
  Write-Log '  nushell (~/.config/nushell/config.nu):'
  Write-Log '    hni init nushell | save --force ~/.config/nushell/hni.nu'
  Write-Log '    source ~/.config/nushell/hni.nu'
}

$tag = Resolve-Tag -RequestedVersion $Version
$target = Resolve-Target

Write-Log "Installing $Repo $tag for $target"

$tmpDir = Join-Path $env:TEMP ("hni-install-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tmpDir | Out-Null

try {
  $archive = Join-Path $tmpDir "hni.zip"
  Download-Asset -Tag $tag -Target $target -OutputPath $archive
  Expand-Archive -Path $archive -DestinationPath $tmpDir -Force

  $sourceExe = Join-Path $tmpDir "hni.exe"
  if (-not (Test-Path $sourceExe)) {
    throw "Archive does not contain hni.exe"
  }

  New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null

  $targetExe = Join-Path $InstallDir "hni.exe"
  Copy-Item -Path $sourceExe -Destination $targetExe -Force

  foreach ($alias in $Aliases) {
    Copy-Item -Path $targetExe -Destination (Join-Path $InstallDir "$alias.exe") -Force
  }

  Ensure-PathEntry -PathEntry $InstallDir
} finally {
  Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Log "Installed to $InstallDir"
Write-ShellSetup
Write-Log "Restart your terminal to use the command immediately."

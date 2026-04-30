param(
  [string]$Version,
  [string]$Repository,
  [string]$InstallDir
)

$ErrorActionPreference = "Stop"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$headers = @{ "User-Agent" = "pack-install-ps1" }
if ($env:GITHUB_TOKEN) {
  $headers["Authorization"] = "Bearer $($env:GITHUB_TOKEN)"
}

if (-not $Version) {
  $Version = $env:PACK_VERSION
}
if (-not $Repository) {
  $Repository = if ($env:PACK_REPO) { $env:PACK_REPO } else { "Blu3Ph4ntom/pack" }
}
if (-not $InstallDir) {
  $InstallDir = if ($env:PACK_INSTALL_DIR) { $env:PACK_INSTALL_DIR } else { Join-Path $HOME ".local\bin" }
}

if (-not $Version) {
  $latest = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repository/releases/latest" -Headers $headers
  if (-not $latest.tag_name) {
    throw "Could not resolve latest release tag. Set PACK_VERSION=x.y.z and retry."
  }
  $Version = $latest.tag_name.TrimStart("v")
}

$arch = [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString().ToLowerInvariant()
$os = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription.ToLowerInvariant()

if ($os -notmatch "windows") {
  throw "install.ps1 is for Windows. Use install.sh on Linux/macOS."
}

switch ($arch) {
  "x64" { $target = "x86_64-pc-windows-msvc"; break }
  default { throw "Unsupported Windows architecture: $arch" }
}

$asset = "pack-$target.exe"
$baseUrl = "https://github.com/$Repository/releases/download/v$Version"
$assetUrl = "$baseUrl/$asset"
$sumUrl = "$baseUrl/SHA256SUMS"

$tmpExe = Join-Path $env:TEMP "pack-$Version.exe"
$tmpSums = Join-Path $env:TEMP "pack-$Version-SHA256SUMS.txt"

Write-Host "Installing Pack v$Version ($target)..."
function Download-WithRetry {
  param(
    [Parameter(Mandatory = $true)][string]$Uri,
    [Parameter(Mandatory = $true)][string]$OutFile
  )

  $attempt = 0
  while ($true) {
    $attempt++
    try {
      Invoke-WebRequest -Uri $Uri -OutFile $OutFile -UseBasicParsing -Headers $headers
      return
    } catch {
      if ($attempt -ge 3) {
        throw
      }
      Start-Sleep -Seconds (2 * $attempt)
    }
  }
}

Download-WithRetry -Uri $assetUrl -OutFile $tmpExe
Download-WithRetry -Uri $sumUrl -OutFile $tmpSums

$expectedLine = Select-String -Path $tmpSums -Pattern "  $([regex]::Escape($asset))$" | Select-Object -First 1
if (-not $expectedLine) {
  throw "Checksum entry for $asset not found."
}

$expected = ($expectedLine.Line -split "\s+")[0].Trim().ToLowerInvariant()
if (Get-Command Get-FileHash -ErrorAction SilentlyContinue) {
  $actual = (Get-FileHash -Algorithm SHA256 -Path $tmpExe).Hash.ToLowerInvariant()
} else {
  $certUtilOutput = & certutil -hashfile $tmpExe SHA256 2>$null
  if ($LASTEXITCODE -ne 0 -or -not $certUtilOutput) {
    throw "Checksum verification failed: neither Get-FileHash nor certutil produced a SHA256 hash."
  }

  $actual = ($certUtilOutput |
    Where-Object { $_ -match '^[0-9a-fA-F ]+$' } |
    Select-Object -First 1)

  if (-not $actual) {
    throw "Checksum verification failed: could not parse SHA256 from certutil output."
  }

  $actual = ($actual -replace ' ', '').ToLowerInvariant()
}
if ($expected -ne $actual) {
  throw "Checksum verification failed."
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
$dest = Join-Path $InstallDir "pack.exe"
Move-Item -Force -Path $tmpExe -Destination $dest
Remove-Item -Force -ErrorAction SilentlyContinue $tmpSums

Write-Host "Pack installed to $dest"

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$InstallDir*") {
  $newPath = if ($userPath) { "$userPath;$InstallDir" } else { $InstallDir }
  [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
  Write-Host "Added $InstallDir to your user PATH. Restart terminal to use 'pack'."
}

& $dest --version

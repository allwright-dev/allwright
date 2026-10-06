param(
    [string]$Version = $env:ALLWRIGHT_VERSION,
    [string]$InstallDir = $env:ALLWRIGHT_INSTALL_DIR,
    [string]$Repository = $(if ($env:ALLWRIGHT_REPOSITORY) { $env:ALLWRIGHT_REPOSITORY } else { "allwright-dev/allwright" })
)

$ErrorActionPreference = "Stop"

function Get-DefaultInstallDir {
    # WindowsApps is on the user PATH by default and needs no admin rights.
    return (Join-Path $env:LOCALAPPDATA "Microsoft\WindowsApps")
}

if (-not $Version -or $Version.Trim() -eq "" -or $Version.Trim() -eq "latest") {
    $latest = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repository/releases/latest"
    $Version = $latest.tag_name
}

if (-not $InstallDir -or $InstallDir.Trim() -eq "") {
    $InstallDir = Get-DefaultInstallDir
}

switch ($env:PROCESSOR_ARCHITECTURE) {
    "AMD64" { $target = "x86_64-pc-windows-msvc" }
    "ARM64" { $target = "aarch64-pc-windows-msvc" }
    default { throw "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE" }
}

$assetName = "allwright-$Version-$target.zip"
$downloadUrl = "https://github.com/$Repository/releases/download/$Version/$assetName"
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("allwright-" + [System.Guid]::NewGuid().ToString("N"))
$archivePath = Join-Path $tempRoot $assetName
$extractPath = Join-Path $tempRoot "extract"

New-Item -ItemType Directory -Path $tempRoot | Out-Null
New-Item -ItemType Directory -Path $extractPath | Out-Null
New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null

try {
    Write-Host "Downloading $downloadUrl"
    Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath
    Expand-Archive -Path $archivePath -DestinationPath $extractPath -Force
    Copy-Item (Join-Path $extractPath "bin\\allwright.exe") (Join-Path $InstallDir "allwright.exe") -Force
}
finally {
    if (Test-Path $tempRoot) {
        Remove-Item $tempRoot -Recurse -Force
    }
}

$installedPath = Join-Path $InstallDir "allwright.exe"
Write-Host "Installed allwright to $installedPath"
$installedVersion = (& $installedPath --version).Trim()
$expectedVersion = $Version.TrimStart("v")
if ($installedVersion -ne "allwright $expectedVersion") {
    throw "Installed binary reports '$installedVersion'; expected 'allwright $expectedVersion'"
}

# Remove copies left by earlier installer versions (checked for presence first).
$legacyDirs = @(
    "$env:ProgramFiles\allwright\bin",
    "$env:LOCALAPPDATA\Programs\allwright\bin"
)
foreach ($legacyDir in $legacyDirs) {
    if (-not $legacyDir -or $legacyDir -eq $InstallDir) { continue }
    $legacyExe = Join-Path $legacyDir "allwright.exe"
    if (-not (Test-Path $legacyExe -PathType Leaf)) { continue }
    Write-Host "Removing previously installed $legacyExe"
    try {
        Remove-Item $legacyExe -Force -ErrorAction Stop
        if (-not (Get-ChildItem $legacyDir -Force -ErrorAction SilentlyContinue)) {
            Remove-Item $legacyDir -Force -ErrorAction SilentlyContinue
        }
    }
    catch {
        Write-Warning "Could not remove $legacyExe (administrator rights may be required): $($_.Exception.Message)"
    }
}

$resolved = Get-Command allwright -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
if ($resolved -and $resolved.Source -ne $installedPath) {
    Write-Warning "Your shell resolves allwright to $($resolved.Source), so $installedPath may be shadowed by an older binary. Move $InstallDir earlier on PATH or remove the stale executable."
}
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not $userPath) {
    $userPath = ""
}
if (-not (($userPath -split ';') -contains $InstallDir)) {
    Write-Host "This install directory is not on your user PATH."
    Write-Host "Add $InstallDir to PATH if the command is not available in a new shell."
}

Write-Host "Future releases can be installed with: allwright update"

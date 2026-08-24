param(
    [string]$Version = $env:ALLWRIGHT_VERSION,
    [string]$InstallDir = $env:ALLWRIGHT_INSTALL_DIR,
    [string]$Repository = $(if ($env:ALLWRIGHT_REPOSITORY) { $env:ALLWRIGHT_REPOSITORY } else { "allwright-dev/allwright" })
)

$ErrorActionPreference = "Stop"

if (-not $Version -or $Version.Trim() -eq "") {
    $latest = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repository/releases/latest"
    $Version = $latest.tag_name
}

if (-not $InstallDir -or $InstallDir.Trim() -eq "") {
    $InstallDir = Join-Path $env:LOCALAPPDATA "Programs\allwright\bin"
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
New-Item -ItemType Directory -Path $InstallDir | Out-Null

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

Write-Host "Installed allwright to $(Join-Path $InstallDir 'allwright.exe')"
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not $userPath) {
    $userPath = ""
}
if (-not (($userPath -split ';') -contains $InstallDir)) {
    Write-Host "Add $InstallDir to your PATH to run allwright directly."
}

param(
    [string]$Version = $env:ALLWRIGHT_VERSION,
    [string]$InstallDir = $env:ALLWRIGHT_INSTALL_DIR,
    [string]$Repository = $(if ($env:ALLWRIGHT_REPOSITORY) { $env:ALLWRIGHT_REPOSITORY } else { "allwright-dev/allwright" })
)

$ErrorActionPreference = "Stop"

function Get-DefaultInstallDir {
    $preferredDirs = @(
        "$env:ProgramFiles\allwright\bin",
        "$env:LOCALAPPDATA\Programs\allwright\bin"
    )

    foreach ($dir in $preferredDirs) {
        if (-not $dir -or $dir.Trim() -eq "") {
            continue
        }
        $parent = Split-Path -Parent $dir
        if (-not $parent) {
            continue
        }
        try {
            if (-not (Test-Path $dir) -and -not (Test-Path $parent)) {
                continue
            }
            if (-not (Test-Path $dir) -and -not (Test-Path $parent -PathType Container)) {
                continue
            }
            if (Test-Path $dir) {
                $probe = Join-Path $dir ("allwright-write-test-" + [System.Guid]::NewGuid().ToString("N"))
                New-Item -ItemType Directory -Path $probe | Out-Null
                Remove-Item $probe -Force
                return $dir
            }
            $probe = Join-Path $parent ("allwright-write-test-" + [System.Guid]::NewGuid().ToString("N"))
            New-Item -ItemType Directory -Path $probe | Out-Null
            Remove-Item $probe -Force
            return $dir
        }
        catch {
            continue
        }
    }

    $pathEntries = ($env:PATH -split ';') | Where-Object { $_ -and $_.Trim() -ne "" }
    foreach ($entry in $pathEntries) {
        if (-not (Test-Path $entry)) {
            continue
        }
        if ($entry -match '\\pnpm\\' -or
            $entry -match '\\npm\\' -or
            $entry -match '\\Yarn\\' -or
            $entry -match '\\Volta\\' -or
            $entry -match '\\cargo\\' -or
            $entry -match '\\go\\bin' -or
            $entry -match '\\bun\\bin') {
            continue
        }
        try {
            $probe = Join-Path $entry ("allwright-write-test-" + [System.Guid]::NewGuid().ToString("N"))
            New-Item -ItemType Directory -Path $probe | Out-Null
            Remove-Item $probe -Force
            return $entry
        }
        catch {
            continue
        }
    }

    return (Join-Path $env:LOCALAPPDATA "Programs\allwright\bin")
}

if (-not $Version -or $Version.Trim() -eq "") {
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
    Write-Host "This install directory is not on your user PATH."
    Write-Host "Add $InstallDir to PATH if the command is not available in a new shell."
}

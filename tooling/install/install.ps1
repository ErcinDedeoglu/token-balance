# Install token-balance.exe + tb.exe from the latest GitHub release.
# Usage: irm https://raw.githubusercontent.com/ErcinDedeoglu/token-balance/main/tooling/install/install.ps1 | iex
param(
    [switch]$PrintTarget,
    [string]$OsArch = ""
)
$ErrorActionPreference = "Stop"
$Repo = if ($env:TOKEN_BALANCE_REPO) { $env:TOKEN_BALANCE_REPO } else { "ErcinDedeoglu/token-balance" }
$BinDir = if ($env:TOKEN_BALANCE_BIN_DIR) { $env:TOKEN_BALANCE_BIN_DIR } else { Join-Path $env:LOCALAPPDATA "token-balance\bin" }

if ($PrintTarget) {
    if ($OsArch -eq "Arm64" -or $OsArch -eq "ARM64") {
        throw "token-balance install: Windows ARM64 is not in GitHub releases yet. Use: cargo install --git https://github.com/$Repo --locked"
    }
    Write-Output "x86_64-pc-windows-msvc"
    exit 0
}

$arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
if ($arch -eq "Arm64") {
    throw "token-balance install: Windows ARM64 is not in GitHub releases yet. Use: cargo install --git https://github.com/$Repo --locked"
}
$target = "x86_64-pc-windows-msvc"
$url = "https://github.com/$Repo/releases/latest/download/token-balance-$target.zip"

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("tb-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
    $zip = Join-Path $tmp "tb.zip"
    Write-Host "downloading $url"
    Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
    Expand-Archive -Path $zip -DestinationPath $tmp -Force
    New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    Copy-Item (Join-Path $tmp "token-balance.exe") (Join-Path $BinDir "token-balance.exe") -Force
    Copy-Item (Join-Path $tmp "tb.exe") (Join-Path $BinDir "tb.exe") -Force
} finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not $userPath) { $userPath = "" }
$parts = $userPath -split ";" | Where-Object { $_ -ne "" }
if ($parts -notcontains $BinDir) {
    [Environment]::SetEnvironmentVariable("Path", ($userPath.TrimEnd(";") + ";" + $BinDir), "User")
    $env:Path = $env:Path + ";" + $BinDir
    Write-Host "added $BinDir to user PATH (new terminals pick this up)"
}

Write-Host "installed $BinDir\token-balance.exe and $BinDir\tb.exe"
& (Join-Path $BinDir "tb.exe") --version

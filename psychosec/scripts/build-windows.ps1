<#
 Builds Psychosec.exe and the installer on Windows.
 Prereqs: Rust (stable, MSVC), Inno Setup 6 (iscc on PATH), Node 20+ (only for release signing).
 Usage:   pwsh scripts/build-windows.ps1 -Version 1.0.0 [-ApiUrl https://api.psychosec.org] [-UpdatePubKey <base64>]
#>
param(
  [Parameter(Mandatory)][string]$Version,
  [string]$ApiUrl = "https://api.psychosec.org",
  [string]$UpdatePubKey = $env:PSYCHOSEC_UPDATE_PUBKEY
)
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")
if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw "Version must look like 1.2.3" }
if (-not $UpdatePubKey) { Write-Warning "PSYCHOSEC_UPDATE_PUBKEY not set: this build will refuse all updates." }

# Keep Cargo.toml in sync with the release version.
(Get-Content apps/cli/Cargo.toml -Raw) -replace '(?m)^version = ".*"$', "version = `"$Version`"" | Set-Content apps/cli/Cargo.toml -NoNewline

$env:PSYCHOSEC_API_URL = $ApiUrl
$env:PSYCHOSEC_UPDATE_PUBKEY = $UpdatePubKey
# --remap-path-prefix keeps build-machine paths out of the binary.
$env:RUSTFLAGS = "--remap-path-prefix=$((Get-Location).Path)=. --remap-path-prefix=$env:USERPROFILE=~ -C target-feature=+crt-static"
cargo build --release --locked -p psychosec
if ($LASTEXITCODE) { throw "cargo build failed" }

$exe = "target\release\Psychosec.exe"
if (-not (Test-Path $exe)) { throw "missing $exe" }
iscc "/DAppVersion=$Version" "/DSourceExe=..\$exe" installer\psychosec.iss
if ($LASTEXITCODE) { throw "iscc failed" }
Copy-Item "installer\Output\Psychosec-Setup-v$Version.exe" "installer\Output\Psychosec-Setup.exe" -Force
Get-FileHash "installer\Output\Psychosec-Setup-v$Version.exe" -Algorithm SHA256 | Format-List

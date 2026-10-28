$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$configurationPath = Join-Path $repositoryRoot "src-tauri\tauri.conf.json"
$releaseExecutable = Join-Path $repositoryRoot "src-tauri\target\release\kohlrausch.exe"
$licensePath = Join-Path $repositoryRoot "src-tauri\LICENSE"

if (-not (Test-Path -LiteralPath $releaseExecutable -PathType Leaf)) {
  throw "Release executable not found. Build it with 'cargo tauri build' from src-tauri first."
}

$version = (Get-Content -LiteralPath $configurationPath -Raw | ConvertFrom-Json).version
$distributionDirectory = Join-Path $repositoryRoot "dist"
$portableDirectory = Join-Path $distributionDirectory "Kohlrausch-Konfig-$version-portable"
$portableExecutable = Join-Path $portableDirectory "Kohlrausch-Konfig.exe"
$portableArchive = Join-Path $distributionDirectory "Kohlrausch-Konfig-$version-portable.zip"

New-Item -ItemType Directory -Path $portableDirectory -Force | Out-Null
Copy-Item -LiteralPath $releaseExecutable -Destination $portableExecutable -Force
Copy-Item -LiteralPath $licensePath -Destination (Join-Path $portableDirectory "LICENSE") -Force

@'
Kohlrausch Konfig — Portable

Run Kohlrausch-Konfig.exe. This no-install package requires the Microsoft Edge WebView2 Runtime; Windows 10 and Windows 11 normally include it.

Profiles, configurations, macros, and logs remain in the normal user profile storage (%APPDATA%\Kohlrausch), not beside this executable.
'@ | Set-Content -LiteralPath (Join-Path $portableDirectory "README.txt") -Encoding utf8

Compress-Archive -Path (Join-Path $portableDirectory "*") -DestinationPath $portableArchive -Force
Copy-Item -LiteralPath $portableExecutable -Destination (Join-Path $distributionDirectory "Kohlrausch-Konfig-$version-portable.exe") -Force

Get-Item -LiteralPath $portableArchive, (Join-Path $distributionDirectory "Kohlrausch-Konfig-$version-portable.exe") |
  Select-Object FullName, Length

$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $PSScriptRoot
$bridgeRoot = Join-Path $PSScriptRoot "prospero-bridge"
$vendor = Join-Path $bridgeRoot "vendor"
$source = Join-Path $vendor "LibProsperoPKG-src"
$repository = "https://github.com/SvenGDK/LibProsperoPKG.git"
$commit = "748eabf1b7d17819528cabf367d8e27109d8fce3"

New-Item -ItemType Directory -Force $vendor | Out-Null
if (-not (Test-Path (Join-Path $source ".git"))) {
    git clone --filter=blob:none --no-checkout $repository $source
    if ($LASTEXITCODE -ne 0) { throw "No se pudo descargar LibProsperoPKG" }
}
git -C $source fetch --depth 1 origin $commit
if ($LASTEXITCODE -ne 0) { throw "No se pudo obtener el commit fijado de LibProsperoPKG" }
git -C $source checkout --detach $commit
if ($LASTEXITCODE -ne 0) { throw "No se pudo seleccionar el commit fijado de LibProsperoPKG" }
$actualCommit = (git -C $source rev-parse HEAD).Trim()
if ($actualCommit -ne $commit) { throw "El commit de LibProsperoPKG no coincide" }

Copy-Item (Join-Path $source "LICENSE") (Join-Path $vendor "LICENSE") -Force
Copy-Item (Join-Path $source "NOTICE") (Join-Path $vendor "NOTICE") -Force
$publish = Join-Path $bridgeRoot "publish\win-x64"
dotnet publish (Join-Path $bridgeRoot "ProsperoBridge.csproj") -c Release -r win-x64 --self-contained true -o $publish
if ($LASTEXITCODE -ne 0) { throw "No se pudo compilar ROMForge Prospero Bridge" }

$destination = Join-Path $projectRoot "src-tauri\binaries\romforge-prospero-bridge.exe"
Copy-Item (Join-Path $publish "romforge-prospero-bridge.exe") $destination -Force
Copy-Item (Join-Path $bridgeRoot "THIRD_PARTY_NOTICES.txt") (Join-Path $projectRoot "src-tauri\binaries\LICENCIA-Prospero-Bridge.txt") -Force
Copy-Item (Join-Path $vendor "LICENSE") (Join-Path $projectRoot "src-tauri\binaries\GPL-3.0-LibProsperoPKG.txt") -Force
Copy-Item (Join-Path $vendor "NOTICE") (Join-Path $projectRoot "src-tauri\binaries\NOTICE-LibProsperoPKG.txt") -Force
Write-Host "Prospero Bridge: $destination"

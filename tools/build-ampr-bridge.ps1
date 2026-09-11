$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $PSScriptRoot
$bridgeRoot = Join-Path $PSScriptRoot "ampr-bridge"
$vendor = Join-Path $bridgeRoot "vendor"
$source = Join-Path $vendor "ampr_emu"
$runtime = Join-Path $vendor "libSceAmpr.sprx"
$commit = "cfa85df379f6eeeb165d7badf9b648e266fe77b7"
$runtimeSha256 = "69E6C4D5E4F5FB83C9E01815DB5861C4C75734ACBF4595CAFA50D4C218116D1A"
$runtimeUrl = "https://github.com/drakmor/ampr_emu/releases/download/0.4.2.1/libSceAmpr.sprx-0.4.2.1-test-pack"

function Get-Sha256([string]$Path) {
    $stream = [System.IO.File]::OpenRead($Path)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $bytes = $sha.ComputeHash($stream)
        return ([System.BitConverter]::ToString($bytes)).Replace("-", "")
    }
    finally {
        $sha.Dispose()
        $stream.Dispose()
    }
}

New-Item -ItemType Directory -Force $vendor | Out-Null
if (-not (Test-Path -LiteralPath (Join-Path $source ".git"))) {
    git clone --filter=blob:none https://github.com/drakmor/ampr_emu.git $source
}
git -C $source fetch --depth 1 origin $commit
git -C $source checkout --detach $commit
if ((git -C $source rev-parse HEAD).Trim() -ne $commit) {
    throw "No se pudo fijar ampr_emu en $commit"
}

if (-not (Test-Path -LiteralPath $runtime) -or (Get-Sha256 $runtime) -ne $runtimeSha256) {
    Invoke-WebRequest -UseBasicParsing -Uri $runtimeUrl -OutFile $runtime
}
if ((Get-Sha256 $runtime) -ne $runtimeSha256) {
    throw "El SHA-256 del runtime AMPR 0.4.2.1 no coincide"
}

$venv = Join-Path $bridgeRoot ".venv-build"
if (-not (Test-Path -LiteralPath (Join-Path $venv "Scripts\python.exe"))) {
    python -m venv $venv
}
$python = Join-Path $venv "Scripts\python.exe"
& $python -m pip install --disable-pip-version-check "lz4==4.4.4" "pyinstaller==6.20.0"
if ($LASTEXITCODE -ne 0) { throw "No se pudieron instalar las dependencias de AMPR Bridge" }

$dist = Join-Path $bridgeRoot "dist\windows"
$work = Join-Path $bridgeRoot "build\windows"
& $python -m PyInstaller --noconfirm --clean --onefile `
    --name romforge-ampr-bridge `
    --distpath $dist --workpath $work --specpath $work `
    --paths (Join-Path $source "tools") `
    --add-data "$runtime;." `
    (Join-Path $bridgeRoot "romforge_ampr_bridge.py")
if ($LASTEXITCODE -ne 0) { throw "No se pudo compilar ROMForge AMPR Bridge" }

$destination = Join-Path $projectRoot "src-tauri\binaries\romforge-ampr-bridge.exe"
Copy-Item (Join-Path $dist "romforge-ampr-bridge.exe") $destination -Force
Copy-Item (Join-Path $bridgeRoot "THIRD_PARTY_NOTICES.txt") (Join-Path $projectRoot "src-tauri\binaries\LICENCIA-AMPR-Bridge.txt") -Force
Copy-Item (Join-Path $source "LICENSE") (Join-Path $projectRoot "src-tauri\binaries\GPL-3.0-AMPR-Emu.txt") -Force
Copy-Item (Join-Path $source "third_party\lz4\LICENSE") (Join-Path $projectRoot "src-tauri\binaries\BSD-2-Clause-LZ4.txt") -Force
Write-Host "AMPR Bridge: $destination"

$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $PSScriptRoot
$bridgeRoot = Join-Path $PSScriptRoot "psp-bridge"
$source = Join-Path $bridgeRoot "vendor\LibOrbisPkg"
$commit = "643477263b2644e0803e0f58b8726ea4e3f3b7d4"

New-Item -ItemType Directory -Force (Split-Path -Parent $source) | Out-Null
if (-not (Test-Path -LiteralPath (Join-Path $source ".git"))) {
    git clone --filter=blob:none https://github.com/maxton/LibOrbisPkg.git $source
}
git -C $source fetch --depth 1 origin $commit
git -C $source checkout --detach $commit
if ((git -C $source rev-parse HEAD).Trim() -ne $commit) {
    throw "No se pudo fijar LibOrbisPkg en $commit"
}

$coreProject = Join-Path $source "LibOrbisPkg.Core\LibOrbisPkg.Core.csproj"
$projectText = Get-Content -LiteralPath $coreProject -Raw
$projectText = $projectText.Replace("<TargetFramework>netcoreapp3.0</TargetFramework>", "<TargetFramework>net8.0</TargetFramework>")
[System.IO.File]::WriteAllText($coreProject, $projectText)

$builder = Join-Path $source "LibOrbisPkg\PKG\PkgBuilder.cs"
$builderText = Get-Content -LiteralPath $builder -Raw
$builderText = $builderText.Replace(
    "e.DataSize = (uint)(pkgSize / 0x10000L) * 4;",
    "e.DataSize = (uint)(pkgSize / 0x10000L) * 4 + 0x10000; // ROMForge: soporte para paquetes grandes"
)
[System.IO.File]::WriteAllText($builder, $builderText)

$publish = Join-Path $bridgeRoot "publish\windows"
dotnet publish (Join-Path $bridgeRoot "PspBridge.csproj") -c Release -r win-x64 --self-contained true -o $publish
if ($LASTEXITCODE -ne 0) { throw "No se pudo compilar ROMForge PSP Bridge" }

$destination = Join-Path $projectRoot "src-tauri\binaries\romforge-psp-bridge.exe"
Copy-Item (Join-Path $publish "romforge-psp-bridge.exe") $destination -Force
Copy-Item (Join-Path $bridgeRoot "THIRD_PARTY_NOTICES.txt") (Join-Path $projectRoot "src-tauri\binaries\LICENCIA-PSP-Bridge.txt") -Force
Copy-Item (Join-Path $source "LICENSE.txt") (Join-Path $projectRoot "src-tauri\binaries\LGPL-3.0-LibOrbisPkg.txt") -Force
Write-Host "PSP Bridge: $destination"

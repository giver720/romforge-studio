$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $PSScriptRoot
$bridgeRoot = Join-Path $PSScriptRoot "prospero-bridge"
$vendor = Join-Path $bridgeRoot "vendor"
$archive = Join-Path $vendor "libprosperopkg-win-x64-v2.5.zip"
$expectedSha256 = "B4B3D290D5058A4FD408CA5C6E0207D8F4998EAB3849C36D51DC5D97592FC151"
$url = "https://github.com/SvenGDK/LibProsperoPKG/releases/download/v2.5/libprosperopkg-win-x64.zip"
$licenseUrl = "https://raw.githubusercontent.com/SvenGDK/LibProsperoPKG/3e159f2ed1d2c2c53c003f79ace378d987773fe3/LICENSE"
$noticeUrl = "https://raw.githubusercontent.com/SvenGDK/LibProsperoPKG/3e159f2ed1d2c2c53c003f79ace378d987773fe3/NOTICE"

function Get-Sha256([string]$Path) {
    $stream = [System.IO.File]::OpenRead($Path)
    try {
        $sha = [System.Security.Cryptography.SHA256]::Create()
        try { return ([System.BitConverter]::ToString($sha.ComputeHash($stream))).Replace("-", "") }
        finally { $sha.Dispose() }
    }
    finally { $stream.Dispose() }
}

New-Item -ItemType Directory -Force $vendor | Out-Null
if (-not (Test-Path $archive) -or (Get-Sha256 $archive) -ne $expectedSha256) {
    Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $archive
}
if ((Get-Sha256 $archive) -ne $expectedSha256) {
    throw "El SHA-256 de LibProsperoPKG v2.5 no coincide"
}

Expand-Archive -Path $archive -DestinationPath $vendor -Force
Invoke-WebRequest -UseBasicParsing -Uri $licenseUrl -OutFile (Join-Path $vendor "LICENSE")
Invoke-WebRequest -UseBasicParsing -Uri $noticeUrl -OutFile (Join-Path $vendor "NOTICE")
$publish = Join-Path $bridgeRoot "publish\win-x64"
dotnet publish (Join-Path $bridgeRoot "ProsperoBridge.csproj") -c Release -r win-x64 --self-contained true -o $publish
if ($LASTEXITCODE -ne 0) { throw "No se pudo compilar ROMForge Prospero Bridge" }

$destination = Join-Path $projectRoot "src-tauri\binaries\romforge-prospero-bridge.exe"
Copy-Item (Join-Path $publish "romforge-prospero-bridge.exe") $destination -Force
Copy-Item (Join-Path $bridgeRoot "THIRD_PARTY_NOTICES.txt") (Join-Path $projectRoot "src-tauri\binaries\LICENCIA-Prospero-Bridge.txt") -Force
Copy-Item (Join-Path $vendor "LICENSE") (Join-Path $projectRoot "src-tauri\binaries\GPL-3.0-LibProsperoPKG.txt") -Force
Copy-Item (Join-Path $vendor "NOTICE") (Join-Path $projectRoot "src-tauri\binaries\NOTICE-LibProsperoPKG.txt") -Force
Write-Host "Prospero Bridge: $destination"

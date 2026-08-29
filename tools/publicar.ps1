<#
    Prepara una version completa de CHD Studio:

      1. Descarga las herramientas nativas que van dentro del instalador
      2. Compila el instalador (setup.exe) firmado para el actualizador
      3. Arma la version portable en un .zip
      4. Genera latest.json, que es lo que lee el actualizador

    Requiere la clave privada de firma en %USERPROFILE%\.tauri\chd-studio.key

    Uso:  npm run release
#>

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$key = "$env:USERPROFILE\.tauri\chd-studio.key"
if (-not (Test-Path $key)) {
    throw "Falta la clave de firma en $key. Generala con:`n  npx tauri signer generate -w `"$key`" --password `"`""
}

$conf = Get-Content "src-tauri\tauri.conf.json" -Raw | ConvertFrom-Json
$version = $conf.version
Write-Host "  Preparando CHD Studio $version" -ForegroundColor Cyan

# --- 1. Herramientas -------------------------------------------------------
if (-not (Test-Path "src-tauri\binaries\chdman.exe")) {
    Write-Host "  Falta chdman, obteniendolo..." -ForegroundColor Yellow
    & "$PSScriptRoot\obtener-chdman.ps1"
}
Write-Host "  Actualizando herramientas nativas..." -ForegroundColor Cyan
& "$PSScriptRoot\obtener-herramientas.ps1" | Out-Null

# --- 2. Instalador ---------------------------------------------------------
Write-Host "  Compilando (esto tarda)..." -ForegroundColor Cyan
# Hay que pasar el CONTENIDO de la clave: si se le da la ruta, Tauri compila
# igual pero se salta la firma y luego no hay .sig que publicar.
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content $key -Raw).Trim()
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
npm run dist
if ($LASTEXITCODE -ne 0) { throw "Fallo la compilacion" }

$bundle = "src-tauri\target\release\bundle"
$setup = Get-ChildItem "$bundle\nsis" -Filter "*_${version}_x64-setup.exe" | Select-Object -First 1
if (-not $setup) { throw "No se genero el instalador" }

# --- 3. Portable -----------------------------------------------------------
Write-Host "  Armando la version portable..." -ForegroundColor Cyan
$out = Join-Path $root "release\v$version"
if (Test-Path $out) { Remove-Item $out -Recurse -Force }
New-Item -ItemType Directory -Force $out | Out-Null

# Se monta en la carpeta temporal y solo el .zip acaba en release/
$port = Join-Path $env:TEMP "CHD-Studio-$version-portable"
if (Test-Path $port) { Remove-Item $port -Recurse -Force }
New-Item -ItemType Directory -Force $port | Out-Null

Copy-Item "src-tauri\target\release\chd-studio.exe" (Join-Path $port "CHD Studio.exe")
Copy-Item "src-tauri\binaries" (Join-Path $port "binaries") -Recurse

# Este archivo es lo que activa el modo portable: los datos se quedan al lado
@"
La presencia de este archivo hace que CHD Studio guarde sus ajustes, las
herramientas que descargue y el entorno de Python en la carpeta 'datos', junto
al ejecutable, en vez de en %APPDATA%.

Borralo si prefieres que se comporte como la version instalada.
"@ | Out-File (Join-Path $port "portable.txt") -Encoding utf8

Copy-Item "README.md" $port -ErrorAction SilentlyContinue

$zip = Join-Path $out "CHD-Studio-$version-portable.zip"
Compress-Archive -Path "$port\*" -DestinationPath $zip -Force
Remove-Item $port -Recurse -Force

# --- 4. latest.json --------------------------------------------------------
Write-Host "  Generando latest.json..." -ForegroundColor Cyan
$sigFile = "$($setup.FullName).sig"
if (-not (Test-Path $sigFile)) { throw "No aparecio la firma $sigFile. Revisa que createUpdaterArtifacts este activo." }

# GitHub sustituye los espacios del nombre por puntos al subir el asset, asi
# que la URL tiene que llevarlos ya cambiados o el actualizador dara 404.
$assetName = $setup.Name -replace ' ', '.'

$manifest = [ordered]@{
    version   = $version
    notes     = "Consulta las notas de la release en GitHub."
    pub_date  = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    platforms = [ordered]@{
        "windows-x86_64" = [ordered]@{
            signature = (Get-Content $sigFile -Raw).Trim()
            url       = "https://github.com/giver720/chd-studio/releases/download/v$version/$assetName"
        }
    }
}
# Sin BOM: Out-File -Encoding utf8 mete EF BB BF y el actualizador de Tauri no
# sabe leer un JSON que empiece por ahi ("error decoding response body")
$json = $manifest | ConvertTo-Json -Depth 6
[System.IO.File]::WriteAllText(
    (Join-Path $out "latest.json"),
    $json,
    (New-Object System.Text.UTF8Encoding $false)
)

Copy-Item $setup.FullName $out -Force

Write-Host ""
Write-Host "  Listo. En la carpeta 'release\v$version':" -ForegroundColor Green
Get-ChildItem $out | Select-Object Name, @{N = "MB"; E = { [math]::Round($_.Length / 1MB, 2) } } | Format-Table -AutoSize
Write-Host "  Para publicarla:" -ForegroundColor Cyan
Write-Host "    gh release create v$version release\v$version\* --repo giver720/chd-studio --title `"CHD Studio $version`""

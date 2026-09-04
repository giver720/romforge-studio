<#
    Descarga la version oficial de MAME desde GitHub, saca chdman.exe y lo deja
    en src-tauri\binaries para que viaje dentro del instalador de ROMForge Studio.

    MAME no publica chdman por separado: hay que bajar el paquete de binarios
    (un autoextraible de 7-Zip de ~85 MB) y quedarse solo con esa herramienta.

    Uso:  powershell -ExecutionPolicy Bypass -File tools\obtener-chdman.ps1
#>

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$dest = Join-Path $root "src-tauri\binaries"
$work = Join-Path $env:TEMP "romforge-studio-mame"

function Say($msg) { Write-Host "  $msg" -ForegroundColor Cyan }

Say "Consultando la ultima version de MAME..."
$rel = Invoke-RestMethod "https://api.github.com/repos/mamedev/mame/releases/latest" `
    -Headers @{ "User-Agent" = "romforge-studio" }

# El paquete de binarios se llama mame0XXXb_x64.exe (o _arm64 en ARM)
$arch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "arm64" } else { "x64" }
$asset = $rel.assets | Where-Object { $_.name -match "^mame\d+b_$arch\.exe$" } | Select-Object -First 1

if (-not $asset) {
    throw "No se encontro el paquete de binarios en la version $($rel.tag_name)."
}

Say "$($rel.tag_name) - $($asset.name) ($([math]::Round($asset.size / 1MB, 1)) MB)"

New-Item -ItemType Directory -Force $work | Out-Null
$sfx = Join-Path $work $asset.name
$out = Join-Path $work "extract"

if (-not (Test-Path $sfx)) {
    Say "Descargando... (esto tarda un rato)"
    Invoke-WebRequest $asset.browser_download_url -OutFile $sfx -UseBasicParsing
}

# Comprobamos la firma SHA256 que publica el propio proyecto
$sumsAsset = $rel.assets | Where-Object { $_.name -eq "SHA256SUMS" } | Select-Object -First 1
if ($sumsAsset) {
    Say "Verificando SHA256..."
    $sums = (Invoke-WebRequest $sumsAsset.browser_download_url -UseBasicParsing).Content
    $line = ($sums -split "`n" | Where-Object { $_ -match [regex]::Escape($asset.name) }) | Select-Object -First 1
    if ($line) {
        $expected = ($line -split "\s+")[0].Trim().ToLower()
        $actual = (Get-FileHash $sfx -Algorithm SHA256).Hash.ToLower()
        if ($expected -ne $actual) {
            Remove-Item $sfx -Force
            throw "La descarga no coincide con la firma oficial. Se ha borrado el archivo."
        }
        Say "Firma correcta."
    }
}

Say "Extrayendo chdman.exe..."
if (Test-Path $out) { Remove-Item $out -Recurse -Force }
New-Item -ItemType Directory -Force $out | Out-Null

# El autoextraible de 7-Zip acepta -o<carpeta> -y
$p = Start-Process -FilePath $sfx -ArgumentList "-o`"$out`"", "-y" -Wait -PassThru
if ($p.ExitCode -ne 0) { throw "El autoextraible devolvio el codigo $($p.ExitCode)." }

$chdman = Get-ChildItem $out -Recurse -Filter "chdman.exe" | Select-Object -First 1
if (-not $chdman) { throw "chdman.exe no aparecio dentro del paquete." }

New-Item -ItemType Directory -Force $dest | Out-Null
Copy-Item $chdman.FullName (Join-Path $dest "chdman.exe") -Force

# La GPL obliga a acompanar el binario del aviso de licencia y del origen del codigo
@"
chdman.exe forma parte de MAME ($($rel.tag_name)) y se distribuye aqui sin modificar.

MAME esta bajo la licencia GNU General Public License, version 2 o posterior.
Texto completo: https://www.gnu.org/licenses/old-licenses/gpl-2.0.html

Codigo fuente de esta version exacta:
https://github.com/mamedev/mame/releases/tag/$($rel.tag_name)

ROMForge Studio se limita a ejecutar chdman como programa independiente; son obras
separadas que solo se distribuyen juntas por comodidad.
"@ | Out-File (Join-Path $dest "LICENCIA-chdman.txt") -Encoding utf8

Say "Limpiando archivos temporales..."
Remove-Item $out -Recurse -Force

$size = [math]::Round((Get-Item (Join-Path $dest "chdman.exe")).Length / 1MB, 2)
Write-Host ""
Write-Host "  Listo: src-tauri\binaries\chdman.exe ($size MB)" -ForegroundColor Green
Write-Host "  Se incluira automaticamente en el proximo 'npm run dist'." -ForegroundColor Green
Write-Host ""
Write-Host "  El paquete descargado sigue en $work" -ForegroundColor DarkGray
Write-Host "  Puedes borrarlo si no piensas actualizar chdman pronto." -ForegroundColor DarkGray

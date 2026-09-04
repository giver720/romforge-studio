<#
    Descarga todas las herramientas nativas que ROMForge Studio empaqueta dentro de
    su instalador y las deja en src-tauri\binaries.

    Estos binarios NO se guardan en el repositorio: se bajan al preparar una
    version. Asi el repo queda limpio y cada compilacion coge la ultima version
    publicada por cada proyecto.

    chdman se obtiene aparte (viene dentro del paquete de MAME):
        npm run chdman

    Uso:  npm run tools:fetch
#>

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$dest = Join-Path $root "src-tauri\binaries"
$tmp = Join-Path $env:TEMP "romforge-studio-tools"

New-Item -ItemType Directory -Force $dest | Out-Null
New-Item -ItemType Directory -Force $tmp | Out-Null

# id, repositorio, texto que debe contener el asset, prefijo de etiqueta, nombre final
$tools = @(
    @{ id = "iso2god";  repo = "iliazeus/iso2god-rs";          asset = "windows";    tag = "";        exe = "iso2god.exe" },
    @{ id = "3dsconv";  repo = "ihaveamac/3dsconv";            asset = ".exe";       tag = "";        exe = "3dsconv.exe" },
    @{ id = "3dstool";  repo = "dnasdw/3dstool";               asset = "3dstool.zip"; tag = "";      exe = "3dstool.exe" },
    @{ id = "ctrtool";  repo = "3DSGuy/Project_CTR";           asset = "win_x64";    tag = "ctrtool"; exe = "ctrtool.exe" },
    @{ id = "makerom";  repo = "3DSGuy/Project_CTR";           asset = "win_x86_64"; tag = "makerom"; exe = "makerom.exe" },
    @{ id = "z3ds";     repo = "energeticokay/z3ds_compress";  asset = "windows";    tag = "";        exe = "z3ds_compressor.exe" },
    @{ id = "4nxci";    repo = "tetj/4NXCI-2026";              asset = ".exe";       tag = "";        exe = "4nxci.exe" },
    @{ id = "ps3iso";   repo = "bucanero/ps3iso-utils";        asset = "Win64";      tag = "";        exe = "extractps3iso.exe" },
    @{ id = "maxcso";   repo = "unknownbrackets/maxcso";       asset = "windows.7z"; tag = "";        exe = "maxcso.exe" },
    @{ id = "xiso";     repo = "XboxDev/extract-xiso";         asset = "Win64_Release"; tag = "";     exe = "extract-xiso.exe" }
)

$notas = @()

foreach ($t in $tools) {
    Write-Host "  $($t.id)..." -ForegroundColor Cyan -NoNewline

    $headers = @{ "User-Agent" = "romforge-studio" }
    if ($t.tag) {
        $rels = Invoke-RestMethod "https://api.github.com/repos/$($t.repo)/releases?per_page=100" -Headers $headers
        $rel = $rels | Where-Object { $_.tag_name.StartsWith($t.tag) } | Select-Object -First 1
    }
    else {
        $rel = Invoke-RestMethod "https://api.github.com/repos/$($t.repo)/releases/latest" -Headers $headers
    }
    if (-not $rel) { throw "$($t.id): no se encontro ninguna release" }

    $asset = $rel.assets | Where-Object { $_.name.ToLower().Contains($t.asset.ToLower()) } | Select-Object -First 1
    if (-not $asset) {
        $nombres = ($rel.assets | ForEach-Object { $_.name }) -join ", "
        throw "$($t.id): el filtro '$($t.asset)' no casa con nada. Hay: $nombres"
    }

    $work = Join-Path $tmp $t.id
    if (Test-Path $work) { Remove-Item $work -Recurse -Force }
    New-Item -ItemType Directory -Force $work | Out-Null

    $dl = Join-Path $work $asset.name
    Invoke-WebRequest $asset.browser_download_url -OutFile $dl -UseBasicParsing

    $tarExe = Join-Path $env:SystemRoot "System32\tar.exe"

    if ($asset.name.ToLower().EndsWith(".7z")) {
        # El tar de Windows es libarchive y sabe leer 7-Zip, asi que no hace
        # falta tener 7-Zip instalado
        & $tarExe -xf $dl -C $work
        if ($LASTEXITCODE -ne 0) { throw "$($t.id): fallo al abrir el .7z" }
        Get-ChildItem $work -Recurse -File |
            Where-Object { $_.Extension -in ".exe", ".dll" } |
            ForEach-Object { Copy-Item $_.FullName (Join-Path $dest $_.Name) -Force }
    }
    elseif ($asset.name.ToLower().EndsWith(".zip")) {
        Expand-Archive $dl -DestinationPath $work -Force

        # ps3iso-utils mete un tar.gz dentro del zip, con un programa por carpeta.
        # Se usa el tar de Windows a proposito: el de Git Bash toma "C:" por un
        # host remoto y falla.
        Get-ChildItem $work -Filter "*.tar.gz" | ForEach-Object {
            & $tarExe -xzf $_.FullName -C $work
            if ($LASTEXITCODE -ne 0) { throw "$($t.id): fallo al extraer $($_.Name)" }
            Remove-Item $_.FullName -Force
        }

        $exe = Get-ChildItem $work -Recurse -Filter $t.exe | Select-Object -First 1
        if (-not $exe) { throw "$($t.id): $($t.exe) no aparecio dentro del zip" }

        # Cuando la descarga trae varios programas se copian todos
        Get-ChildItem $work -Recurse -File |
            Where-Object { $_.Extension -in ".exe", ".dll" } |
            ForEach-Object { Copy-Item $_.FullName (Join-Path $dest $_.Name) -Force }
    }
    else {
        Copy-Item $dl (Join-Path $dest $t.exe) -Force
    }

    $notas += "$($t.exe)  -  $($t.repo)  ($($rel.tag_name))"
    Write-Host " $($rel.tag_name)" -ForegroundColor Green
}

# Aviso de procedencia y licencias, que algunas de estas herramientas son GPL
$cabecera = @"
Herramientas de terceros que ROMForge Studio distribuye sin modificar.
Cada una conserva su licencia original; consulta su repositorio.

"@
($cabecera + ($notas -join "`n")) | Out-File (Join-Path $dest "PROCEDENCIA.txt") -Encoding utf8

Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "  Listo. Contenido de src-tauri\binaries:" -ForegroundColor Green
Get-ChildItem $dest | Select-Object Name, @{N = "KB"; E = { [math]::Round($_.Length / 1KB, 1) } } | Format-Table -AutoSize

if (-not (Test-Path (Join-Path $dest "chdman.exe"))) {
    Write-Host "  Falta chdman.exe. Ejecuta:  npm run chdman" -ForegroundColor Yellow
}

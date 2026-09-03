# CHD Studio

Aplicación de escritorio para convertir imágenes de disco a **CHD** (Compressed Hunks of Data) usando
`chdman`, la herramienta oficial de MAME. Interfaz en español, arrastrar y soltar, cola por lotes y
progreso en tiempo real.

Disponible para **Windows 10/11** y **Ubuntu 22.04 o posterior (amd64)**.

## Qué cubre

| Familia | Comando de chdman | Sistemas |
|---|---|---|
| CD-ROM (2352 B/sector) | `createcd` | PlayStation, Saturn, Dreamcast (GD-ROM), Mega-CD, PC Engine CD, Neo Geo CD, 3DO, CD-i, PC-FX, Amiga CD32 |
| DVD (2048 B/sector) | `createdvd` | PlayStation 2, PSP (UMD), Xbox, DVD-ROM de PC |
| Disco duro | `createhd` | Arcade con HDD, discos de MS-DOS/Win9x, Xbox HDD |
| Datos crudos | `createraw` | Volcados sin estructura |
| Arcade GD-ROM | `createcd` | NAOMI / NAOMI 2, Triforce, Chihiro, Atomiswave |

También hace el camino inverso (`extractcd`, `extractdvd`, `extracthd`, `extractraw`), verificación
(`verify`) e inspección de cabeceras (`info`).

### Formatos de entrada

- **Sí:** `.cue` + `.bin`, `.gdi`, `.toc`, `.nrg`, `.cdr`, `.iso`, `.img`, `.hdi`, `.vhd`, `.raw`
- **No:** `.cdi`, `.mdf/.mds`, `.ccd`, `.rvz/.wbfs/.nkit`, comprimidos. La app los detecta y avisa en
  vez de fallar en silencio.

## Instalar en Ubuntu

Descarga el archivo `.deb` desde la [última release](https://github.com/giver720/chd-studio/releases/latest)
e instálalo con APT para que también resuelva `mame-tools` y el resto de dependencias:

```bash
sudo apt install ./*.deb
```

El paquete crea la entrada **CHD Studio** en el menú de aplicaciones. `mame-tools` aporta `chdman`;
las demás herramientas se descargan o compilan desde **Ajustes → Herramientas** cuando una
conversión las necesita. El paquete recomienda Python, Git y las bibliotecas de compilación para
que esos botones funcionen sin preparación adicional. También instala el soporte exFAT que usa el
módulo de PS5.

## El motor: chdman

CHD Studio es la ventana; el trabajo lo hace `chdman`, la herramienta oficial de MAME.

### Incluirlo en el instalador de Windows

Ejecuta una vez:

```bash
npm run chdman
```

Descarga el paquete oficial de binarios de MAME desde GitHub, comprueba su firma SHA256, extrae
únicamente `chdman.exe` (~4 MB) y lo deja en `src-tauri/binaries/`. A partir de ahí, cada
`npm run dist` lo empaqueta dentro del instalador y **el usuario final no tiene que instalar nada**.

Junto al binario se genera `LICENCIA-chdman.txt` con el aviso de GPL-2.0-or-later y el enlace al
código fuente de esa versión exacta, como exige la licencia de MAME. CHD Studio ejecuta chdman como
programa independiente, así que son obras separadas distribuidas juntas por comodidad.

### Sin incluirlo

La app sigue funcionando y busca `chdman` por orden en:

1. La ruta que hayas elegido en Ajustes
2. Su carpeta interna (`%APPDATA%\chd-studio\bin` en Windows, `~/.config/chd-studio/bin` en Linux)
3. La copia empaquetada (`resources/binaries`)
4. El `PATH` del sistema
5. Instalaciones típicas de MAME

Si no aparece, en **Ajustes → Motor chdman** puedes importarlo. En Ubuntu, la instalación normal del
`.deb` ya instala `mame-tools`, así que no hace falta configurarlo a mano.

Con MAME 0.255 o superior se habilita el códec **zstd**, que el preset «Máxima» aprovecha.

## Presets de compresión

| Preset | CD | DVD / HDD |
|---|---|---|
| Máxima | `cdzs, cdlz, cdzl, cdfl` | `zstd, lzma, huff, flac` |
| Equilibrada | `cdlz, cdzl, cdfl` | `lzma, zlib, huff, flac` |
| Rápida | `cdzs, cdfl` | `zstd, huff` |

Sin soporte de zstd se usan los equivalentes clásicos.

## Verificación automática y salidas seguras

Cada conversión se comprueba antes de aparecer como terminada. La cola distingue dos niveles:

- **Verificación completa:** el formato se valida con su propia herramienta. Se usa `chdman verify`
  para CHD, la comprobación de hashes de NSZ y DolphinTool para imágenes de Wii cuando está
  disponible.
- **Validación estructural:** para formatos sin verificador independiente se comprueban existencia,
  tamaño, cabecera y estructura mínima. Esto incluye, entre otros, CSO/ZSO/DAX, juegos extraídos de
  Xbox y PS3, CCI y los NSP que pueda producir 4NXCI.

La conversión se escribe primero en una carpeta temporal situada junto al destino. Sólo después de
superar la comprobación se publica con un renombrado local. Si la herramienta falla, la verificación
rechaza el resultado o el usuario cancela, se retira la salida temporal y cualquier archivo anterior
permanece intacto. La opción **Sobrescribir** tampoco elimina el resultado antiguo hasta que el nuevo
ha sido validado.

## Nintendo Switch

| Conversión | Herramienta | Notas |
|---|---|---|
| NSP → NSZ | `nsz` | Compresión zstd, nivel 1–22 (18 por defecto) |
| NSZ → NSP | `nsz` | Reconstrucción bit a bit |
| XCI → XCZ | `nsz` | Comprime el volcado de cartucho |
| XCZ → XCI | `nsz` | Reconstrucción bit a bit |
| XCI → NSP | `4NXCI` | Cartucho a instalable; puede generar varios NSP |

**Requiere tus propias `prod.keys`** en `~/.switch/prod.keys`. CHD Studio no las incluye ni ayuda a
obtenerlas: solo comprueba si el archivo existe y avisa si falta.

`nsz` se instala con pip dentro de un entorno de Python privado de la app, sin tocar el Python del
sistema. En Linux, `4NXCI` se compila automáticamente desde su código fuente. Ambas cosas se hacen
desde **Ajustes → Herramientas** con un botón.

## Nintendo 3DS

| Conversión | Herramienta | Claves que exige |
|---|---|---|
| Comprimir a Z3DS | `z3ds_compressor` | ninguna |
| CIA → CCI | `cia-to-cci` | `~/.3ds/aes_keys.txt` |
| CCI/.3ds → CIA | `3dsconv` | `~/.3ds/boot9.bin` |

**Z3DS** es el formato comprimido que [Azahar](https://github.com/azahar-emu/azahar) admite desde la
versión 2123: zstd *seekable*, pensado para descomprimir rápido y poder saltar a cualquier punto sin
extraer el archivo entero. Según la extensión de entrada el resultado es `.zcci`, `.zcia`, `.zcxi` o
`.z3dsx`. Un `.3ds` es el mismo contenedor que un `.cci`, así que se le fuerza la salida `.zcci` para
que el emulador lo reconozca.

La compresión es de ida: no hay descompresor porque el emulador lee el archivo comprimido tal cual.

Las claves son tuyas y salen de tu propia consola. CHD Studio comprueba si están y avisa, pero no las
incluye ni ayuda a obtenerlas.

## Dónde guardas tus claves

No hace falta dejarlas en la carpeta por defecto. En las vistas de Switch y 3DS puedes señalar
**el archivo concreto o la carpeta que lo contiene**, y la ruta queda guardada:

| Archivo | Ruta por defecto | Se le pasa a la herramienta como |
|---|---|---|
| `prod.keys` | `~/.switch/` | `nsz --keys`, `4nxci -k` |
| `aes_keys.txt` | `~/.3ds/` | `cia-to-cci --keys` |
| `boot9.bin` | `~/.3ds/` | `3dsconv --boot9=` |

Las cuatro herramientas aceptan una ruta explícita, así que no se copia ni se mueve nada: se les
indica dónde mirar.

## Xbox 360

| Conversión | Herramienta | Notas |
|---|---|---|
| ISO → GOD | `iso2god` | Games On Demand, el formato de la tienda de la consola |
| ISO → carpeta | `extract-xiso` | Saca el `default.xex` y todos los archivos del juego |
| Carpeta → ISO | `extract-xiso` | El camino de vuelta |

**Cuál usar.** GOD ocupa menos y va en discos FAT32, pero **no todos los juegos arrancan en ese
formato**. La carpeta con el `default.xex` es la opción compatible: es el juego tal cual venía en el
disco, y los lanzadores como Aurora o Freestyle Dash lo leen directamente. Si un juego falla en GOD,
extráelo a carpeta.

`extract-xiso` reconoce los tres formatos de disco de Xbox (XGD1, XGD2 y XGD3), así que sirve tanto
para Xbox 360 como para Xbox original. Al extraer se omite `$SystemUpdate` por defecto, que es el
actualizador de firmware del disco y no hace falta para jugar.

GOD no es un archivo sino una carpeta con el juego troceado en partes de 1 GB, así que **cabe en
discos formateados en FAT32** (que no admiten archivos de más de 4 GB) y la consola lo reconoce sin
parchear nada.

La opción **«Recortar el espacio vacío»** (`--trim`, activada por defecto) es donde está casi todo el
ahorro: los discos de Xbox 360 llevan una zona de relleno que en muchos juegos son varios GB.

El botón **Analizar** ejecuta `--dry-run` para leer la ficha del juego sin convertir nada, útil para
comprobar que el ISO es válido antes de una conversión larga.

`iso2god` también acepta ISOs de Xbox original.

> **Sobre `.xex`:** no es un formato al que convertir, es el ejecutable que va *dentro* del ISO
> (`default.xex`). Lo que se hace es sacarlo del ISO junto al resto del juego, que es justo lo que
> hace «ISO → carpeta».

## PlayStation 3

Incluye dos perfiles que **no borran contenido del juego**:

| Perfil | Entrada | Resultado | Uso |
|---|---|---|---|
| PS3 real + RPCS3 | ISO o carpeta | ISO estándar compacto | CFW/HEN + Cobra/webMAN y RPCS3 |
| Máxima reducción RPCS3 | ISO descifrado o carpeta | Misma entrada, comprimida por el sistema de archivos | RPCS3 en PC |

### PS3 real + RPCS3

Con `extractps3iso` y `makeps3iso`, CHD Studio extrae el juego y vuelve a construir el ISO sin el
relleno físico del disco. No quita `PS3_UPDATE`, idiomas, vídeos ni ningún otro archivo. Después
reabre el ISO generado y compara la ruta y el tamaño de todos los archivos con el inventario
original antes de publicar el resultado. Opcionalmente puede producir fragmentos de 4 GB para
unidades FAT32.

La salida sigue siendo un ISO de PS3 normal. Para montarlo en una consola real hacen falta CFW o
HEN y soporte Cobra; una PS3 con firmware de fábrica no monta ISOs.

### Máxima reducción para RPCS3

Aplica compresión transparente sin cambiar el formato lógico: NTFS LZX en Windows o Btrfs zstd en
Linux. RPCS3 sigue abriendo el ISO descifrado o la carpeta normalmente y el sistema operativo
descomprime los bloques al leerlos. En Linux la unidad tiene que ser Btrfs y debe estar disponible
la utilidad `btrfs`.

Esta segunda compresión pertenece al disco del PC: no se conserva al copiar el archivo a otra
unidad y no sirve como formato para cargar directamente desde una PS3 real.

## PlayStation 5

El apartado **PlayStation 5** toma la carpeta raíz de un dump propio y crea una imagen estándar
`.exfat` con los archivos directamente en su raíz. La entrada debe contener `eboot.bin` y
`sce_sys/param.json`; no se admite una carpeta contenedora adicional. La imagen usa clústeres de
64 KiB, se vuelve a leer tras la copia y solo se publica si conserva todas las rutas y tamaños.

Este proceso **empaqueta, no comprime**: permite transportar la carpeta como un único archivo, pero
el resultado será algo mayor que los datos originales por el sistema de archivos y su margen de
seguridad. Está pensado para PS5 modificadas con un montador compatible como ShadowMountPlus; una
consola de fábrica no lo puede usar.

En Windows requiere [OSFMount](https://www.osforensics.com/tools/mount-disk-images.html) y ejecutar
CHD Studio como administrador durante la creación. En Ubuntu el `.deb` instala `exfatprogs`,
`exfat-fuse`, `fuse3` y PolicyKit; el escritorio muestra una autorización del sistema para montar y
desmontar temporalmente la imagen.

## Desarrollo

### Windows

```bash
npm install
npm run tools:fetch   # descarga las herramientas nativas a src-tauri/binaries
npm run chdman        # descarga chdman del paquete oficial de MAME
npm run app           # tauri dev
```

### Ubuntu/Debian

Instala primero las [dependencias de Tauri para Linux](https://v2.tauri.app/start/prerequisites/).
Después:

```bash
npm ci
npm run app           # desarrollo
./CREAR_LINUX.sh       # genera src-tauri/target/release/bundle/deb/*.deb
```

El workflow `Linux .deb` repite la compilación en Ubuntu 22.04, instala el paquete resultante y hace
una prueba de arranque bajo Xvfb. Construir sobre 22.04 mantiene compatibilidad con versiones nuevas
de Ubuntu sin exigir una glibc reciente.

Los binarios de terceros **no se guardan en el repositorio**: se descargan al preparar una versión.
Así el repo queda limpio y cada compilación coge la última versión de cada proyecto.

```bash
npm run check:tools   # comprueba que los assets de GitHub sigan existiendo
```

Ese último conviene ejecutarlo antes de publicar: los proyectos renombran sus archivos de una
release a otra, y cuando pasa, el botón «Instalar» falla sin decir gran cosa.

## Publicar una versión de Windows

```bash
npm run release
```

Descarga las herramientas, compila el instalador firmado, arma el `.zip` portable y genera el
`latest.json` que lee el actualizador. Todo queda en `release/vX.Y.Z/`, separado de las versiones
anteriores. Después:

```bash
gh release create vX.Y.Z release/vX.Y.Z/* --repo giver720/chd-studio
```

Hace falta la clave privada de firma en `%USERPROFILE%\.tauri\chd-studio.key`. **Si la pierdes, no
podrás volver a firmar actualizaciones** y habría que publicar una versión nueva a mano con otra
clave. No está en el repositorio y no debe estarlo.

### Instalador y portable

- **setup.exe** — instalador NSIS. Lleva dentro chdman, iso2god, 3dsconv, ctrtool, makerom,
  z3ds_compressor y 4NXCI (~23 MB), así que funciona sin descargar nada. La excepción es `nsz`
  (Switch), que es un paquete de Python y se instala al vuelo desde Ajustes.
- **portable.zip** — el ejecutable con sus herramientas y un `portable.txt` al lado. Mientras ese
  archivo exista, los ajustes, las herramientas descargadas y el entorno de Python se guardan en la
  subcarpeta `datos`, no en `%APPDATA%`. Sirve para llevarlo en un USB.
- **amd64.deb** — paquete para Ubuntu 22.04 o posterior. Declara `mame-tools` y el soporte exFAT de
  PS5 como dependencias, y se actualiza instalando el `.deb` de la siguiente release con APT.

### Actualizaciones automáticas

La app consulta las releases de GitHub. En Windows puede descargar e instalar la nueva versión desde
**Ajustes → Actualizaciones**. En Linux avisa de la versión nueva, pero el `.deb` se actualiza con APT
o instalando el paquete de la release, para no saltarse el gestor de paquetes del sistema.

## Estructura

```
src/                 interfaz React + Tailwind
  lib/profiles.ts    catálogo de sistemas por generación y códecs
  store.ts           estado global (zustand)
src-tauri/src/
  chdman.rs          localización y sondeo del ejecutable
  jobs.rs            cola, ejecución y parseo de progreso
  settings.rs        preferencias persistentes
  lib.rs             comandos expuestos al frontend
```

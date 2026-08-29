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
que esos botones funcionen sin preparación adicional.

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

Adelgaza juegos quitando lo que no usas. Con `ps3iso-utils` (las utilidades de Estwald).

| Paso | Herramienta |
|---|---|
| ISO → carpeta | `extractps3iso` |
| Analizar y borrar | interno |
| Carpeta → ISO | `makeps3iso` |
| Partir para FAT32 | `splitps3iso` |

**Qué detecta.** `PS3_UPDATE` (el actualizador de firmware del disco, entre 200 y 300 MB, que solo
sirve para instalar el sistema desde el juego) se marca solo. Los packs de idioma se reconocen por
el nombre de archivo (`_ES`, `SPA`, `spanish`, `_FR`, `GER`…) y se agrupan por idioma con su tamaño,
para que puedas quitar de golpe los que no vayas a jugar. Los vídeos y bancos de audio (`.pam`,
`.bik`, `.at3`, `.msf`…) se etiquetan aparte.

**Salvaguardas.** `EBOOT.BIN`, `PARAM.SFO`, `PS3_DISC.SFB` y `LICDIR` están bloqueados: no se
proponen y el backend se niega a borrarlos aunque se los pidan. Tampoco acepta rutas que salgan de
la carpeta del juego. Nada se borra sin una confirmación que dice cuántos archivos y cuánto espacio.

> ⚠️ **Haz copia antes.** Hay juegos que llevan un índice de sus propios archivos y se cuelgan si
> falta uno, aunque sea un vídeo en un idioma que no usas. La detección por nombre acierta casi
> siempre, pero no es infalible. Prueba el juego después de adelgazarlo.

Si juegas en formato carpeta (webMAN, Iris Manager), no hace falta reconstruir el ISO: ya has
terminado tras borrar.

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
- **amd64.deb** — paquete para Ubuntu 22.04 o posterior. Declara `mame-tools` como dependencia y se
  actualiza instalando el `.deb` de la siguiente release con APT.

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

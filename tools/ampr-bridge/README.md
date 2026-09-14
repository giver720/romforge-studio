# ROMForge AMPR Bridge

Adaptador de línea de comandos para el formato `AMPRPAK4` con bloques LZ4 de
`drakmor/ampr_emu`. Se distribuye como un proceso separado bajo GPL-3.0.

El comando `convert` trabaja sobre una copia, verifica todos los bloques contra
los archivos originales de esa copia y solo entonces retira de la salida los
archivos representados por el pack. Nunca modifica el dump indicado en
`--input`.

Si la entrada ya es una carpeta AMPRPAK4 completa, `convert` la restaura en un
área temporal junto al destino y la vuelve a comprimir con el perfil elegido.
Si solo quedaron índices o el runtime de un intento anterior, los sustituye en
la copia de salida sin tocar la carpeta original.

```text
romforge-ampr-bridge convert --input GAME --output GAME-lz4 --profile balanced
romforge-ampr-bridge verify --input GAME-lz4
romforge-ampr-bridge unpack --input GAME-lz4 --output GAME-restored
```

`unpack` verifica los bloques antes de restaurar y retira del resultado los
paquetes, índices y el runtime que el puente añadió al despliegue. El resultado
es otra vez el árbol normal del dump, no una segunda carpeta AMPR.

Perfiles disponibles:

- `fast`: compresión LZ4 rápida de nivel 1;
- `balanced`: LZ4 HC nivel 9;
- `maximum`: LZ4 HC nivel 12.

El resultado necesita un firmware, un cargador y una versión de
ShadowMountPlus compatibles. La verificación offline demuestra integridad y
reversibilidad, no compatibilidad de cada juego en hardware.

# ROMForge AMPR Bridge

Adaptador de línea de comandos para el formato `AMPRPAK4` con bloques LZ4 de
`drakmor/ampr_emu`. Se distribuye como un proceso separado bajo GPL-3.0.

El comando `convert` trabaja sobre una copia, verifica todos los bloques contra
los archivos originales de esa copia y solo entonces retira de la salida los
archivos representados por el pack. Nunca modifica el dump indicado en
`--input`.

```text
romforge-ampr-bridge convert --input GAME --output GAME-lz4 --profile balanced
romforge-ampr-bridge verify --input GAME-lz4
romforge-ampr-bridge unpack --input GAME-lz4 --output GAME-restored
```

El resultado necesita un firmware, un cargador y una versión de
ShadowMountPlus compatibles. La verificación offline demuestra integridad y
reversibilidad, no compatibilidad de cada juego en hardware.

# Formatos experimentales de PS5

ROMForge Studio integra FPKG nativo y AMPRPAK4/LZ4 como flujos experimentales separados. Esta separación evita confundir FPKG nativo de PS5 con FFPKG/UFS2 o con herramientas de PS4.

## FPKG nativo de PS5 (`ps5fpkg`)

Estado: integrado mediante el proceso externo ROMForge Prospero Bridge y LibProsperoPKG v2.5.

Para habilitar el botón en un dump se requiere:

- `contentId` válido en `sce_sys/param.json`;
- módulos ELF ya descifrados, en su ruta normal o reflejados bajo la subcarpeta configurable (por defecto `decrypted/`);
- que no quede ningún SELF cifrado sin su ELF correspondiente;
- el motor autocontenido incluido en la instalación de ROMForge.

El puente trabaja en una carpeta temporal, conserva intacto el dump original, rechaza módulos cifrados sin resolver y valida la imagen final mediante `ProsperoPkgValidator`. ROMForge repite esa validación antes de publicar el `.pkg` en la carpeta de salida.

El anuncio inicial indica compatibilidad prevista hasta firmware 11.40, pero esto debe volver a validarse con la herramienta corregida y el soporte del lado de la consola.

## AMPRPAK4/LZ4 (`ps5lz4`)

Estado: integrado mediante ROMForge AMPR Bridge y el encoder público de `drakmor/ampr_emu`.

No es un `.lz4` genérico. El puente crea una carpeta AMPRPAK4 con bloques LZ4 seekable, conserva sueltos los módulos y archivos requeridos por el sistema, verifica cada bloque y permite restaurar el árbol original.

La interfaz ofrece tres perfiles persistentes:

- `fast`: LZ4 rápido, menor uso de CPU y normalmente mayor tamaño;
- `balanced`: LZ4 HC nivel 9, recomendado para uso general;
- `maximum`: LZ4 HC nivel 12, más lento y orientado a reducir el tamaño.

La verificación offline demuestra integridad y reversibilidad. El montaje y la ejecución siguen dependiendo de una versión compatible de ShadowMountPlus, del firmware y de los payloads de la consola.

## Contrato de integración

`ps5fpkg` produce `.pkg` y `ps5lz4` produce una carpeta AMPRPAK4. Ambos modos son ejecutables, cancelables y usan una salida temporal: ROMForge solo publica el resultado después de validarlo y limpia los restos si ocurre un error.

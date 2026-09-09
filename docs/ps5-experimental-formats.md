# Formatos experimentales de PS5

ROMForge Studio reserva dos integraciones nuevas sin tratarlas todavía como conversiones funcionales. Esta separación evita confundir FPKG nativo de PS5 con FFPKG/UFS2 o con herramientas de PS4.

## FPKG nativo de PS5 (`ps5fpkg`)

Estado: integrado mediante el proceso externo ROMForge Prospero Bridge y LibProsperoPKG v2.5.

Para habilitar el botón en un dump se requiere:

- `contentId` válido en `sce_sys/param.json`;
- módulos ELF ya descifrados, en su ruta normal o reflejados bajo `decrypted/`;
- que no quede ningún SELF cifrado sin su ELF correspondiente;
- el motor autocontenido incluido en la instalación de ROMForge.

El puente trabaja en una carpeta temporal, conserva intacto el dump original, rechaza módulos cifrados sin resolver y valida la imagen final mediante `ProsperoPkgValidator`. ROMForge repite esa validación antes de publicar el `.pkg` en la carpeta de salida.

El anuncio inicial indica compatibilidad prevista hasta firmware 11.40, pero esto debe volver a validarse con la herramienta corregida y el soporte del lado de la consola.

## LZ4/Lizard (`ps5lz4`)

Estado: anunciado; no existe en ROMForge un encoder público verificado.

No se debe implementar como un `.lz4` genérico ni comprimir toda la carpeta con la biblioteca LZ4 común. Primero se necesita conocer el contenedor real, sus metadatos y la forma en que la PS5 lo monta.

Antes de habilitarlo se requiere:

- encoder o especificación pública del formato;
- firma o cabecera para identificarlo sin depender solo de la extensión;
- extractor o método de comprobación independiente;
- prueba de ida y vuelta sin pérdida;
- montaje y ejecución confirmados en hardware real.

## Contrato de integración

`ps5fpkg` es un modo ejecutable y produce `.pkg`; `ps5lz4` sigue reservado y `is_mode()` devuelve `false` únicamente para LZ4. La cola permite cancelar el proceso FPKG, limpia la salida temporal ante errores y solo publica un paquete aceptado.

## Mejoras del progreso de FPKG de PS5

- La cola actualiza el tiempo y el tamaño de salida cada segundo durante la creación del paquete.
- Las etapas de compresión Kraken, preparación de módulos y construcción del FPKG muestran descripciones claras.
- La barra de creación muestra actividad cuando el motor no proporciona un porcentaje real, sin presentar porcentajes inventados.
- Se añadió una prueba de regresión para estas etapas.

### Limitación conocida

El error `NAPS u2c next-base value ... exceeds the single-byte field` observado con Bendy sigue pendiente. Esta versión mejora el seguimiento del proceso, pero no incluye un arreglo confirmado para ese error ni garantiza la ejecución en consola.

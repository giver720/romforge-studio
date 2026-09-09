# ROMForge Prospero Bridge

Proceso externo que conecta ROMForge Studio con [LibProsperoPKG](https://github.com/SvenGDK/LibProsperoPKG). Convierte un backup descifrado de PS5 en un paquete debug FPKG y rechaza el resultado cuando quedan módulos cifrados sin resolver o falla la validación estructural.

Este puente se distribuye bajo GPL-3.0-or-later porque enlaza con LibProsperoPKG, que utiliza la misma licencia. ROMForge Studio lo ejecuta como un programa separado.

La compilación usa exactamente LibProsperoPKG v2.5. El código fuente correspondiente está disponible en el repositorio anterior y en la etiqueta `v2.5`.

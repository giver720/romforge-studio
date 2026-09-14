# ROMForge Prospero Bridge

Proceso externo que conecta ROMForge Studio con [LibProsperoPKG](https://github.com/SvenGDK/LibProsperoPKG). Convierte un backup descifrado de PS5 en un paquete debug FPKG y rechaza el resultado cuando quedan módulos cifrados sin resolver o falla la validación estructural.

Este puente se distribuye bajo GPL-3.0-or-later porque enlaza con LibProsperoPKG, que utiliza la misma licencia. ROMForge Studio lo ejecuta como un programa separado.

La compilación usa LibProsperoPKG 2.6.0 fijado al commit
`748eabf1b7d17819528cabf367d8e27109d8fce3`. El código fuente correspondiente
se descarga y compila junto con el puente para que el resultado sea reproducible.

## Correcciones de PlayStation 5

- FPKG nativo ahora acepta correctamente los modulos fake-SELF heredados usados por `libSceAmpr.sprx` y `libScePlayGo.sprx`.
- El motor FPKG se actualizo a ROMForge Prospero Bridge 1.1.0 con LibProsperoPKG 2.6.0.
- LZ4/AMPR ahora puede recomprimir una carpeta que ya contiene un despliegue AMPR completo: primero la restaura en un espacio temporal y luego aplica el perfil elegido.
- Los restos incompletos de `ampr_emu.index` y `fakelib/libSceAmpr.sprx` se recuperan sin modificar la carpeta original.
- Se agregaron pruebas automaticas de regresion para FPKG y LZ4 en Ubuntu 22.04.

La conversion sigue destinada exclusivamente a dumps propios y depende de la compatibilidad del firmware, los payloads y el entorno de cada consola.

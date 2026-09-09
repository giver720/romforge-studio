# Formatos experimentales de PS5

ROMForge Studio reserva dos integraciones nuevas sin tratarlas todavía como conversiones funcionales. Esta separación evita confundir FPKG nativo de PS5 con FFPKG/UFS2 o con herramientas de PS4.

## FPKG nativo de PS5 (`ps5fpkg`)

Estado: bloqueado en la interfaz y excluido del motor de trabajos.

Antes de habilitarlo se requiere:

- una build corregida que no produzca paquetes parcialmente corruptos;
- una CLI o API no interactiva con contrato estable;
- verificación estructural del paquete generado;
- instalación y ejecución confirmadas en una PS5 modificada compatible;
- licencia y canal de distribución claros para cualquier binario incluido.

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

Los identificadores están reservados en frontend y backend, pero `is_mode()` devuelve `false` para ambos. Cuando un motor sea seguro se añadirá de forma explícita a la cola, junto con detección de herramienta, extensión de salida, progreso, cancelación y verificación posterior.

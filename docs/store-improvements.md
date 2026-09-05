# Store: siete mejoras acordadas

Objetivo: completar las siete mejoras de la Store, conservando las ocho plataformas.

1. Catálogo en vivo: implementada consulta remota con timeout y límite de tamaño, validación del esquema, copia local y botón Actualizar. Pendiente prueba de interfaz empaquetada con y sin conexión.
2. Fichas: pendiente descripción completa, capturas cuando existan, autor, versión, fecha, requisitos y origen. Ampliar el importador con metadatos reales de cada fuente.
3. Descargas: pendiente selección de archivo/formato, distinguir datos adicionales, velocidad, progreso por trabajo, cancelación y reintento. Conservar verificación SHA-256 y estructura HBAS; revisar las rutas de manifiestos antes de ampliar el flujo.
4. Categorías: pendiente normalización de categorías de las fuentes, filtro y orden por nombre/fecha. No deducir fechas ambiguas sin conocer el formato de la fuente.
5. Compatibilidad: pendiente requisitos publicados por autor y mensaje «Compatibilidad sin confirmar» cuando falten. No inventar requisitos por plataforma.
6. Favoritos e historial: pendiente persistencia, abrir carpeta de descarga y detección de versiones diferentes respecto a las descargadas.
7. Presentación: pendiente tarjetas uniformes, imágenes progresivas, secciones de añadidos/actualizados con fechas verificables y paginación para evitar renderizar miles de tarjetas.

Verificación realizada para el punto 1: compilación TypeScript/Vite, cargo check, validación de las 2.448 entradas, rechazo de esquema incompatible y duplicados, ida/vuelta de caché. Aún no equivale a una prueba de uso dentro de Tauri.

Estos cambios son posteriores a 2.2.0 y aún no forman parte de una release.

## Avance verificado posterior

- Fichas con descripción, versión, autor, fecha, licencia, compatibilidad y selección de archivos; 739 entradas cuentan con capturas importadas de VitaDBtoo.
- Categorías de HBAS/OSC, fechas normalizadas por fuente, filtros y orden por nombre, actualización e incorporación.
- Favoritos e historial persistentes con carpeta de destino y comparación de versiones distintas.
- Cancelación nativa, exclusión de descargas concurrentes, reintento desde interfaz, velocidad y progreso del archivo actual. Paquetes HBAS en carpeta temporal y protección frente a rutas absolutas o ascendentes.
- Compilación frontend y cargo check correctos; prueba Rust de cancelación y liberación del trabajo correcta.
- Prueba visual mediante `tools/store-preview.html`: búsqueda de Deckis Platformer, apertura de ficha con cuatro capturas, cierre con Escape, favorito persistente tras recarga y filtro de favoritos comprobados.
- La vista de prueba usa el componente real pero carece de Tauri: su fallo de actualización es esperado y NO prueba la descarga ni la actualización nativas.
- Tendencias: ranking global y ranking por consola para las 1.748 entradas que publican un contador, con sección explícita para PS5 aunque sus 40 payloads actuales no publican contador.

Pendientes de cierre: prueba nativa de descargas y actualización/caché; revisar continuidad al cambiar de pestaña, datos remotos inválidos y secciones visuales de novedades; ampliar verificación de paquetes y limpiar cualquier defecto encontrado. No declarar completas las siete mejoras con la compilación sola.

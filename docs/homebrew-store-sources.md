# ROMForge Homebrew Store: fuentes y descubrimiento

## Objetivo

Construir un catálogo de aplicaciones homebrew legales para 3DS, Wii, Wii U,
PS Vita, PSP, PS4, PS5 y Switch. La Store debe descubrir metadatos y enlaces de
descarga sin redistribuir ROMs comerciales, claves, firmware propietario ni
copias de juegos.

## Fuentes iniciales

| Plataforma | Fuente | Tipo de datos | Prioridad |
| --- | --- | --- | --- |
| 3DS | [Universal-DB](https://db.universal-team.net/) / [full.json](https://db.universal-team.net/data/full.json) | Catálogo estructurado, iconos, versiones y descargas | Primaria |
| Wii U | [Homebrew App Store](https://github.com/fortheusers/hb-appstore) | Repositorio `get`, paquetes y metadatos | Primaria |
| Switch | [Homebrew App Store](https://github.com/fortheusers/hb-appstore) | Repositorio `get`, paquetes y metadatos | Primaria |
| Wii | [Open Shop Channel API](https://hbb1.oscwii.org/api/v3/contents) | Catálogo estructurado, ZIP, iconos y metadatos | Primaria |
| PS Vita | [VitaDB](https://github.com/Rinnegatamante/VitaDB-Downloader) y [VitaDBtoo](https://github.com/DrDecki/VitaDBtoo-db) | Catálogo, iconos, VPK y archivos de datos | Primaria/complementaria |
| PSP | VitaDB/VitaDBtoo y releases de GitHub | Homebrew, ports y utilidades | Complementaria |
| PS4 | [PS4-Store](https://github.com/LightningMods/PS4-Store), GoldHEN y PS4HEN | Releases oficiales de payloads y aplicaciones | Curada |
| PS5 | [PS5 Payloads Atlas](https://github.com/lucaszhongsj/ps5-payloads-atlas) y releases de autores | Catálogo de payloads ELF/BIN con checksums | Curada |

Universal-DB indica que sus datos se actualizan automáticamente y que su
`full.json` reúne la información obtenida de GitHub y otras fuentes. Homebrew
App Store usa repositorios estáticos y soporta Wii U y Switch, mientras que sus
ports para Wii y 3DS no deben tomarse como fuente principal. Para Vita/PSP,
VitaDBtoo permite recuperar metadatos e iconos cuando el servicio original no
está disponible.

## Cómo localizar nuevas aplicaciones

1. **Catálogo estructurado primero.** Descargar el índice oficial de cada
   fuente y registrar su `source_id`, fecha de consulta y licencia.
2. **GitHub/GitLab/Forgejo después.** Para fuentes curadas, leer solamente
   repositorios marcados por el proyecto o por un mantenedor. Usar sus releases
   y no enlaces construidos a partir de nombres de archivos.
3. **Resolver el artefacto.** Preferir el asset de release estable que coincida
   con la plataforma y arquitectura. Guardar todos los formatos disponibles
   (`.3dsx`, `.cia`, `.3ds`, `.dol`, `.elf`, `.vpk`, `.pkg`, `.bin`, `.nro`, etc.)
   como variantes del mismo producto.
4. **Validar antes de mostrar.** Exigir URL HTTPS, tamaño razonable, respuesta
   200, tipo de archivo permitido y checksum cuando la fuente lo publique.
5. **No duplicar.** Deduplicar por repositorio de origen y, cuando exista,
   `title_id`/identificador de aplicación. Conservar versiones anteriores para
   poder detectar actualizaciones.

## Registro normalizado

```json
{
  "id": "github:autor/repositorio",
  "platforms": ["switch"],
  "name": "Aplicación",
  "summary": "Descripción breve",
  "version": "1.2.3",
  "author": "Autor",
  "license": "MIT",
  "icon_url": "https://...",
  "screenshots": ["https://..."],
  "release_url": "https://github.com/.../releases/tag/v1.2.3",
  "downloads": [
    {
      "format": "nro",
      "url": "https://.../app.nro",
      "sha256": "...",
      "size": 123456
    }
  ],
  "source": "Universal-DB",
  "license_url": "https://...",
  "updated_at": "2026-09-04T00:00:00Z"
}
```

## Frecuencia y seguridad

- Sincronización rápida cada 6 horas para releases y cada 24 horas para
  catálogos completos.
- Catálogo firmado por ROMForge y cacheado localmente.
- Descargas a una carpeta temporal; verificar SHA-256 y tamaño antes de mover
  el archivo a la biblioteca del usuario.
- Mostrar siempre fuente, autor, licencia y versión.
- Permitir reportar un enlace caído o una retirada de contenido.
- Mantener una lista de exclusión para ROMs comerciales, BIOS, keys, firmware,
  dumps y enlaces que requieran evadir protecciones de copyright.

## Primera implementación recomendada

1. Importador de Universal-DB, Homebrew App Store y VitaDBtoo.
2. Adaptador de releases de GitHub para Wii, PSP, PS4 y PS5 con repositorios
   explícitamente curados.
3. Catálogo unificado firmado en GitHub Pages o Releases de ROMForge Studio.
4. Vista Store con búsqueda, filtros, portada, licencia, descarga y verificación
   local.

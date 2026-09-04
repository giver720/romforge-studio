"""
Comprueba que cada herramienta declarada en src-tauri/src/tools.rs siga
encontrando su archivo en GitHub.

Los proyectos renombran sus assets de una release a otra, y cuando eso pasa el
boton «Instalar» falla con un mensaje que no dice gran cosa. Este script lo
detecta antes de publicar una version.

    python tools/validar-herramientas.py
"""

import json
import re
import sys
import urllib.request
from pathlib import Path

TOOLS_RS = Path(__file__).resolve().parent.parent / "src-tauri" / "src" / "tools.rs"

SPEC = re.compile(
    r'id:\s*"([^"]+)",\s*name:[^\n]*\n\s*exe:[^\n]*\n\s*kind:\s*ToolKind::Github\s*\{\s*'
    r'repo:\s*"([^"]+)",(?:\s*//[^\n]*\n)?\s*asset:\s*"([^"]+)",\s*tag:\s*"([^"]*)",',
    re.S,
)


def api(url):
    req = urllib.request.Request(url, headers={"User-Agent": "romforge-studio"})
    return json.load(urllib.request.urlopen(req))


def main() -> int:
    src = TOOLS_RS.read_text(encoding="utf-8")
    specs = SPEC.findall(src)
    if not specs:
        print("No se encontro ninguna herramienta de GitHub en tools.rs")
        return 1

    print(f"{len(specs)} herramientas de GitHub declaradas\n")
    fallos = 0

    for tid, repo, asset, tag in specs:
        try:
            if tag:
                rels = api(f"https://api.github.com/repos/{repo}/releases?per_page=100")
                rel = next((r for r in rels if r["tag_name"].startswith(tag)), None)
                if rel is None:
                    print(f'{tid:10s} FALLA  no hay release cuya etiqueta empiece por "{tag}"')
                    fallos += 1
                    continue
            else:
                rel = api(f"https://api.github.com/repos/{repo}/releases/latest")

            hit = next(
                (a for a in rel["assets"] if asset.lower() in a["name"].lower()), None
            )
            if hit:
                print(f'{tid:10s} OK     {rel["tag_name"]:20s} {hit["name"]}')
            else:
                nombres = ", ".join(a["name"] for a in rel["assets"]) or "(sin assets)"
                print(f'{tid:10s} FALLA  el filtro "{asset}" no casa. Hay: {nombres}')
                fallos += 1
        except Exception as e:  # noqa: BLE001 - queremos ver cualquier fallo de red
            print(f"{tid:10s} ERROR  {e}")
            fallos += 1

    print()
    if fallos:
        print(f"{fallos} herramienta(s) con problemas: corrige el filtro en tools.rs")
        return 1
    print("Todas las herramientas se pueden descargar.")
    return 0


if __name__ == "__main__":
    sys.exit(main())

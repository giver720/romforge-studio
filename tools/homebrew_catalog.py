"""Build a normalized, legal-homebrew catalog from configured public sources."""

from __future__ import annotations

import argparse
import json
import re
import sys
import urllib.request
from pathlib import Path
from typing import Any


USER_AGENT = "ROMForge-Homebrew-Catalog/1.0"
ALLOWED_FORMATS = {
    ".3dsx", ".cia", ".3ds", ".nds", ".dol", ".elf", ".rpx", ".wuhb",
    ".vpk", ".pbp", ".prx", ".nro", ".nsp", ".pkg", ".bin", ".zip",
}


def fetch_json(url: str) -> Any:
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(request, timeout=30) as response:
        if response.status != 200:
            raise RuntimeError(f"{url}: HTTP {response.status}")
        return json.load(response)


def safe_url(value: Any) -> str | None:
    if not isinstance(value, str) or not value.startswith("https://"):
        return None
    return value


def filename_format(filename: str) -> str | None:
    suffix = Path(filename).suffix.lower()
    return suffix[1:] if suffix in ALLOWED_FORMATS else None


def universal_db(source: dict[str, Any]) -> list[dict[str, Any]]:
    payload = fetch_json(source["index"])
    if not isinstance(payload, list):
        raise ValueError("Universal-DB index is not an array")
    result: list[dict[str, Any]] = []
    for item in payload:
        if not isinstance(item, dict):
            continue
        downloads: list[dict[str, Any]] = []
        raw_downloads = item.get("downloads") or {}
        if isinstance(raw_downloads, dict):
            for filename, raw in raw_downloads.items():
                raw = raw if isinstance(raw, dict) else {}
                url = safe_url(raw.get("url"))
                fmt = filename_format(str(filename))
                if not url or not fmt:
                    continue
                downloads.append({
                    "format": fmt,
                    "filename": str(filename),
                    "url": url,
                    "size": raw.get("size") if isinstance(raw.get("size"), int) else None,
                })
        if not downloads:
            continue
        github = item.get("github")
        app_id = f"universal-db:{github}" if github else f"universal-db:{item.get('name', 'unknown')}"
        result.append({
            "id": app_id,
            "platforms": [source["platform"]],
            "name": item.get("name") or item.get("title") or "Sin nombre",
            "summary": item.get("description") or "",
            "author": item.get("author") or "Desconocido",
            "version": item.get("version"),
            "license": item.get("license_name") or item.get("license"),
            "icon_url": safe_url(item.get("icon") or item.get("image")),
            "release_url": safe_url(item.get("download_page")),
            "downloads": downloads,
            "source": source["name"],
            "source_url": source["homepage"],
            "license_url": safe_url(item.get("license_url")),
            "updated_at": item.get("updated") or item.get("modified") or None,
        })
    return result


def build(config: dict[str, Any]) -> dict[str, Any]:
    entries: list[dict[str, Any]] = []
    errors: list[dict[str, str]] = []
    for source in config.get("sources", []):
        try:
            if source.get("type") == "universal_db":
                entries.extend(universal_db(source))
            else:
                errors.append({"source": source.get("id", "unknown"), "error": "unsupported source type"})
        except Exception as exc:  # Keep one broken source from hiding the others.
            errors.append({"source": source.get("id", "unknown"), "error": str(exc)})
    deduped = {entry["id"]: entry for entry in entries}
    return {
        "schema_version": 1,
        "entries": sorted(deduped.values(), key=lambda entry: entry["name"].lower()),
        "sources": config.get("sources", []),
        "errors": errors,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", default="tools/homebrew-sources.json")
    parser.add_argument("--output", default="public/store/catalog.json")
    args = parser.parse_args()
    config = json.loads(Path(args.config).read_text(encoding="utf-8"))
    catalog = build(config)
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(catalog, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"Generated {len(catalog['entries'])} entries from {len(catalog['sources'])} source(s)")
    if catalog["errors"]:
        print(f"Warnings: {len(catalog['errors'])} source(s) failed", file=sys.stderr)
    return 0 if catalog["entries"] else 1


if __name__ == "__main__":
    raise SystemExit(main())

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


def hbas_repo(source: dict[str, Any]) -> list[dict[str, Any]]:
    payload = fetch_json(source["index"])
    packages = payload.get("packages", []) if isinstance(payload, dict) else []
    result: list[dict[str, Any]] = []
    for item in packages:
        if not isinstance(item, dict):
            continue
        binary = str(item.get("binary") or "")
        if not binary.startswith("/") or binary == "/none":
            continue
        package = str(item.get("name") or Path(binary).parent.name)
        manifest_url = safe_url(f"{source['base'].rstrip('/')}/packages/{package}/manifest.install")
        if not manifest_url:
            continue
        # HBAS packages are assembled from manifest.install; the binary path is
        # metadata, not a standalone download URL.
        filename = f"{package}.hbas"
        fmt = "hbas"
        url = manifest_url
        if not url:
            continue
        upstream = item.get("url")
        app_name = item.get("title") or item.get("name") or "Sin nombre"
        result.append({
            "id": f"{source['id']}:{item.get('name', app_name)}",
            "platforms": [source["platform"]],
            "name": app_name,
            "summary": item.get("description") or "",
            "author": item.get("author") or "Desconocido",
            "version": item.get("version"),
            "license": item.get("license"),
            "icon_url": None,
            "release_url": safe_url(upstream),
            "downloads": [{
                "format": fmt,
                "filename": filename,
                "url": url,
                "size": int(item["extracted"]) if isinstance(item.get("extracted"), int) else None,
                "sha256": item.get("sha256"),
            }],
            "manifest_url": manifest_url,
            "source": source["name"],
            "source_url": source["homepage"],
            "license_url": None,
            "updated_at": item.get("updated"),
        })
    return result


def vitadbtoo(source: dict[str, Any]) -> list[dict[str, Any]]:
    payload = fetch_json(source["index"])
    if not isinstance(payload, list):
        raise ValueError("VitaDBtoo index is not an array")
    result: list[dict[str, Any]] = []
    for item in payload:
        if not isinstance(item, dict):
            continue
        url = safe_url(item.get("url"))
        if not url:
            continue
        filename = Path(url.split("?", 1)[0]).name or f"{item.get('id', 'app')}.zip"
        fmt = filename_format(filename) or "zip"
        icon = item.get("icon")
        icon_url = safe_url(f"{source['icon_base'].rstrip('/')}/{icon}") if icon else None
        downloads = [{
            "format": fmt,
            "filename": filename,
            "url": url,
            "size": int(item["size"]) if str(item.get("size", "")).isdigit() else None,
            "md5": item.get("hash") or None,
        }]
        data_url = safe_url(item.get("data"))
        if data_url:
            downloads.append({"format": "data", "filename": Path(data_url).name, "url": data_url})
        result.append({
            "id": f"{source['id']}:{item.get('id', item.get('name', filename))}",
            "platforms": [source["platform"]],
            "name": item.get("name") or filename,
            "summary": item.get("description") or "",
            "author": item.get("author") or "Desconocido",
            "version": item.get("version"),
            "title_id": item.get("titleid") or None,
            "license": None,
            "icon_url": icon_url,
            "release_url": safe_url(item.get("release_page") or item.get("source")),
            "downloads": downloads,
            "source": source["name"],
            "source_url": source["homepage"],
            "license_url": None,
            "updated_at": item.get("date"),
        })
    return result


def osc_api(source: dict[str, Any]) -> list[dict[str, Any]]:
    payload = fetch_json(source["index"])
    if not isinstance(payload, list):
        raise ValueError("Open Shop Channel index is not an array")
    result: list[dict[str, Any]] = []
    for item in payload:
        if not isinstance(item, dict) or not isinstance(item.get("url"), dict):
            continue
        zip_url = safe_url(item["url"].get("zip"))
        if not zip_url:
            continue
        slug = str(item.get("slug") or item.get("name") or "app")
        desc = item.get("description") if isinstance(item.get("description"), dict) else {}
        sizes = item.get("file_size") if isinstance(item.get("file_size"), dict) else {}
        result.append({
            "id": f"{source['id']}:{slug}",
            "platforms": [source["platform"]],
            "name": item.get("name") or slug,
            "summary": desc.get("short") or "",
            "description": desc.get("long") or "",
            "author": item.get("author") or "Desconocido",
            "version": item.get("version"),
            "title_id": (item.get("shop") or {}).get("title_id"),
            "license": None,
            "icon_url": safe_url(item["url"].get("icon")),
            "release_url": None,
            "downloads": [{
                "format": "zip",
                "filename": f"{slug}.zip",
                "url": zip_url,
                "size": sizes.get("zip_compressed"),
            }],
            "source": source["name"],
            "source_url": source["homepage"],
            "license_url": None,
            "updated_at": item.get("release_date"),
        })
    return result


def build(config: dict[str, Any]) -> dict[str, Any]:
    entries: list[dict[str, Any]] = []
    errors: list[dict[str, str]] = []
    for source in config.get("sources", []):
        try:
            if source.get("type") == "universal_db":
                entries.extend(universal_db(source))
            elif source.get("type") == "hbas_repo":
                entries.extend(hbas_repo(source))
            elif source.get("type") == "vitadbtoo":
                entries.extend(vitadbtoo(source))
            elif source.get("type") == "osc_api":
                entries.extend(osc_api(source))
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

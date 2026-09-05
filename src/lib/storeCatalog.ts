export type StoreDownload = { format: string; filename: string; url: string; size?: number | null; sha256?: string | null };
export type StoreEntry = {
  id: string; platforms: string[]; name: string; summary?: string; author?: string;
  version?: string | null; license?: string | null; icon_url?: string | null;
  release_url?: string | null; source_url?: string; downloads: StoreDownload[];
  source?: string; updated_at?: string | null;
  description?: string; screenshots?: string[]; requirements?: string;
  categories?: string[];
  added_at?: string | null;
  popularity?: number | null;
};
const CACHE_KEY = "romforge.store.catalog.v1";
export const CATALOG_URL = "https://raw.githubusercontent.com/giver720/romforge-studio/main/public/store/catalog.json";

export function parseCatalog(value: unknown): StoreEntry[] {
  const catalog = value as { schema_version?: unknown; entries?: unknown } | null;
  if (!catalog || catalog.schema_version !== 1 || !Array.isArray(catalog.entries) || !catalog.entries.length) {
    throw new Error("El catálogo recibido no es compatible");
  }
  const entries = catalog.entries as StoreEntry[];
  const ids = new Set<string>();
  for (const entry of entries) {
    // Version 1 catalogs shipped Unix seconds for OSC dates.
    const legacyDate = (entry as unknown as { updated_at?: unknown })?.updated_at;
    if (typeof legacyDate === "number" && Number.isFinite(legacyDate)) {
      const date = new Date(legacyDate * 1000);
      entry.updated_at = Number.isFinite(date.getTime()) ? date.toISOString() : null;
    }
    if (!entry || typeof entry.id !== "string" || ids.has(entry.id) || typeof entry.name !== "string" ||
        !Array.isArray(entry.platforms) || !entry.platforms.every(p => typeof p === "string") ||
        !Array.isArray(entry.downloads) || !entry.downloads.every(d => d && typeof d.filename === "string" &&
          typeof d.format === "string" && typeof d.url === "string" && d.url.startsWith("https://"))) {
      throw new Error("El catálogo contiene entradas inválidas");
    }
    ids.add(entry.id);
    for (const field of ["summary", "author", "version", "license", "icon_url", "release_url", "source_url", "source", "updated_at", "description", "requirements", "added_at"] as const) {
      if (entry[field] != null && typeof entry[field] !== "string") throw new Error(`Campo inválido: ${field}`);
    }
    for (const field of ["categories", "screenshots"] as const) {
      if (entry[field] != null && (!Array.isArray(entry[field]) || !entry[field]!.every(v => typeof v === "string"))) throw new Error(`Campo inválido: ${field}`);
    }
    if (entry.popularity != null && (!Number.isInteger(entry.popularity) || entry.popularity < 0)) throw new Error("Campo inválido: popularity");
  }
  return entries;
}

export function readCachedCatalog(): { entries: StoreEntry[]; checkedAt: string } | null {
  try {
    const cached = JSON.parse(localStorage.getItem(CACHE_KEY) || "null");
    if (!cached || typeof cached.checkedAt !== "string") return null;
    return { entries: parseCatalog(cached.catalog), checkedAt: cached.checkedAt };
  } catch { return null; }
}

export function cacheCatalog(entries: StoreEntry[], checkedAt: string): boolean {
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify({ catalog: { schema_version: 1, entries }, checkedAt }));
    return true;
  } catch { return false; }
}

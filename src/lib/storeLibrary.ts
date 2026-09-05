export type StoreHistory = { id: string; name: string; version: string | null; filename: string; directory: string; completedAt: string };
type Library = { favorites: string[]; history: StoreHistory[] };
const KEY = "romforge.store.library.v1";
export function readLibrary(): Library {
  try {
    const value = JSON.parse(localStorage.getItem(KEY) || "null");
    return {
      favorites: Array.isArray(value?.favorites) ? value.favorites.filter((v: unknown) => typeof v === "string") : [],
      history: Array.isArray(value?.history) ? value.history.filter((h: StoreHistory) => h &&
        [h.id, h.name, h.filename, h.directory, h.completedAt].every(v => typeof v === "string") &&
        (h.version === null || typeof h.version === "string")) : [],
    };
  } catch { return { favorites: [], history: [] }; }
}
export function saveLibrary(library: Library): boolean {
  try { localStorage.setItem(KEY, JSON.stringify(library)); return true; }
  catch { return false; }
}
export function latestDownloads(history: StoreHistory[]): Map<string, StoreHistory> {
  const latest = new Map<string, StoreHistory>();
  for (const item of history) if (!latest.has(item.id)) latest.set(item.id, item);
  return latest;
}

import { Download, ExternalLink, FolderOpen, Heart, Loader2, RefreshCw, Search, Store as StoreIcon } from "lucide-react";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useRef, useState } from "react";
import { api } from "../lib/api";
import { useStore } from "../store";
import { cacheCatalog, parseCatalog, readCachedCatalog } from "../lib/storeCatalog";
import { StoreDetails } from "./StoreDetails";
import { StoreImage } from "./StoreImage";
import { Trending } from "./Trending";
import type { StoreEntry, StoreDownload as DownloadEntry } from "../lib/storeCatalog";
import { latestDownloads, readLibrary, saveLibrary } from "../lib/storeLibrary";


const PLATFORMS = ["all", "3ds", "wii", "wiiu", "switch", "psvita", "psp", "ps4", "ps5"];
const LABELS: Record<string, string> = { all: "Todas", psvita: "PS Vita", psp: "PSP", ps4: "PS4", ps5: "PS5", wiiu: "Wii U" };

function size(value?: number | null) {
  if (!value) return "";
  const units = ["B", "KB", "MB", "GB"];
  let n = value;
  let i = 0;
  while (n >= 1024 && i < units.length - 1) { n /= 1024; i += 1; }
  return `${n.toFixed(n >= 10 || i === 0 ? 0 : 1)} ${units[i]}`;
}

export function StoreView() {
  const [entries, setEntries] = useState<StoreEntry[]>([]);
  const [library, setLibrary] = useState(readLibrary);
  const [shelf, setShelf] = useState("all");
  const [category, setCategory] = useState("all");
  const [order, setOrder] = useState("name");
  const highlights = useMemo(() => {
    const recent = (field: "updated_at" | "added_at") => entries.filter(e => {
      const raw = e[field];
      return raw && /^\d{4}-\d{2}-\d{2}(T|$)/.test(raw) && Number.isFinite(Date.parse(raw));
    }).sort((a, b) => Date.parse(b[field]!) - Date.parse(a[field]!)).slice(0, 4);
    return { updated: recent("updated_at"), added: recent("added_at") };
  }, [entries]);
  const downloaded = useMemo(() => latestDownloads(library.history), [library.history]);
  const changedVersions = useMemo(() => new Set(entries.filter(entry => {
    const previous = downloaded.get(entry.id);
    return previous?.version && entry.version && previous.version !== entry.version;
  }).map(entry => entry.id)), [entries, downloaded]);
  const [selected, setSelected] = useState<StoreEntry | null>(null);
  const [visibleCount, setVisibleCount] = useState(48);
  const [platform, setPlatform] = useState("all");
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [refreshing, setRefreshing] = useState(false);
  const [catalogStatus, setCatalogStatus] = useState("");
  const [downloading, setDownloading] = useState<string | null>(null);
  const downloadLock = useRef(false);
  const [progress, setProgress] = useState(0);
  const [transfer, setTransfer] = useState({ filename: "", received: 0, total: 0, speed: 0 });
  const sample = useRef({ filename: "", received: 0, time: 0 });
  const [retry, setRetry] = useState<{ entry: StoreEntry; item: DownloadEntry } | null>(null);
  const notify = useStore((s) => s.notify);

  function toggleFavorite(id: string) {
    const current = readLibrary();
    const next = { ...current, favorites: current.favorites.includes(id) ? current.favorites.filter(v => v !== id) : [...current.favorites, id] };
    if (!saveLibrary(next)) { notify("error", "No se pudo guardar el favorito"); return; }
    setLibrary(next);
  }

  useEffect(() => {
    let disposed = false;
    const cached = readCachedCatalog();
    if (cached) {
      setEntries(cached.entries);
      setLoading(false);
      setCatalogStatus(`Copia guardada · ${new Date(cached.checkedAt).toLocaleString()}`);
    }
    void (async () => {
      if (!cached) {
        try {
          const response = await fetch("/store/catalog.json");
          if (!response.ok) throw new Error(`HTTP ${response.status}`);
          const bundled = parseCatalog(await response.json());
          if (!disposed) { setEntries(bundled); setCatalogStatus("Catálogo incluido en la aplicación"); }
        } catch (e) { if (!disposed) setError(String(e)); }
        finally { if (!disposed) setLoading(false); }
      }
      if (!disposed) await refreshCatalog(() => disposed);
    })();
    return () => { disposed = true; };
  }, []);

  async function refreshCatalog(disposed = () => false) {
    setRefreshing(true);
    try {
      const fresh = parseCatalog(await api.fetchStoreCatalog());
      if (disposed()) return;
      const checkedAt = new Date().toISOString();
      const saved = cacheCatalog(fresh, checkedAt);
      setEntries(fresh);
      setError("");
      setCatalogStatus(`Actualizado · ${new Date(checkedAt).toLocaleTimeString()}${saved ? "" : " · No se pudo guardar la copia local"}`);
    } catch {
      if (!disposed()) setCatalogStatus("No se pudo actualizar. Mostrando la copia disponible.");
    } finally { if (!disposed()) setRefreshing(false); }
  }

  useEffect(() => {
    let stop: (() => void) | undefined;
    let disposed = false;
    listen<{ filename: string; received: number; total?: number | null }>("store://download", (event) => {
      const { filename, received, total } = event.payload;
      const now = performance.now();
      const previous = sample.current;
      const sameFile = previous.filename === filename && received >= previous.received;
      const elapsed = (now - previous.time) / 1000;
      const speed = sameFile && elapsed > 0 ? (received - previous.received) / elapsed : 0;
      if (elapsed >= 0.25 || !sameFile || received === total) {
        sample.current = { filename, received, time: now };
        setTransfer({ filename, received, total: total || 0, speed });
        setProgress(total ? Math.min(100, Math.round((received / total) * 100)) : 0);
      }
    }).then((unlisten) => { if (disposed) unlisten(); else stop = unlisten; }).catch(() => {});
    return () => { disposed = true; stop?.(); };
  }, []);

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return entries.filter((entry) =>
      (platform === "all" || entry.platforms.includes(platform)) &&
      (category === "all" || (category === "unknown" ? !entry.categories?.length : entry.categories?.includes(category))) &&
      (shelf !== "favorites" || library.favorites.includes(entry.id)) &&
      (shelf !== "updates" || changedVersions.has(entry.id)) &&
      (!needle || `${entry.name} ${entry.author ?? ""} ${entry.summary ?? ""}`.toLowerCase().includes(needle)),
    ).sort((a, b) => {
      const date = (entry: StoreEntry) => {
        const raw = order === "added" ? entry.added_at : entry.updated_at;
        return raw && /^\d{4}-\d{2}-\d{2}(T|$)/.test(raw) ? Date.parse(raw) || 0 : 0;
      };
      return (order === "name" ? 0 : date(b) - date(a)) || a.name.localeCompare(b.name);
    });
  }, [entries, platform, query, shelf, library.favorites, changedVersions, category, order]);

  useEffect(() => { setVisibleCount(48); }, [platform, query, shelf, category, order]);

  async function downloadEntry(entry: StoreEntry, item: DownloadEntry) {
    if (!item || downloadLock.current) return;
    downloadLock.current = true;
    setDownloading(entry.id);
    setProgress(0);
    setTransfer({ filename: item.filename, received: 0, total: 0, speed: 0 });
    sample.current = { filename: "", received: 0, time: performance.now() };
    setRetry(null);
    try {
      const destination = await open({ directory: true, multiple: false });
      if (typeof destination !== "string") return;
      if (item.format === "hbas") await api.downloadHbasPackage(item.url, destination, entry.name);
      else await api.downloadHomebrew(item.url, item.filename, destination, item.sha256);
      const current = readLibrary();
      const next = { ...current, history: [{ id: entry.id, name: entry.name, version: entry.version || null,
        filename: item.filename, directory: destination, completedAt: new Date().toISOString() }, ...current.history] };
      if (saveLibrary(next)) setLibrary(next);
      else notify("error", "Descarga completada, pero no se pudo guardar el historial");
      notify("ok", `${entry.name} descargado`);
    } catch (e) {
      setRetry({ entry, item });
      notify("error", `No se pudo descargar: ${String(e)}`);
    } finally { downloadLock.current = false; setDownloading(null); setProgress(0); }
  }

  return (
    <section className="scroll min-w-0 flex-1 p-6">
      <div className="mx-auto max-w-6xl">
        <div className="mb-5 flex items-start justify-between gap-4">
          <div>
            <p className="eyebrow">CATÁLOGO COMUNITARIO</p>
            <h1 className="mt-1 flex items-center gap-2 text-xl font-semibold"><StoreIcon size={20} style={{ color: "var(--accent)" }} /> Homebrew Store</h1>
            <p className="mt-1 text-sm text-[var(--color-muted)]">Aplicaciones, ports y utilidades con enlaces de sus autores.</p>
          </div>
          <button className="btn btn-ghost text-xs" disabled={refreshing} onClick={() => void refreshCatalog()}><RefreshCw size={14} className={refreshing ? "animate-spin" : ""} /> {refreshing ? "Actualizando…" : "Actualizar"}</button>
        </div>
        <p role="status" className="mb-4 text-xs text-[var(--color-muted)]">{entries.length} apps · {catalogStatus}</p>
        {downloading && <div className="glass mb-4 rounded-xl p-4" role="status">
          <div className="flex items-center justify-between gap-3"><p className="min-w-0 break-all text-sm">{transfer.filename}</p><button className="btn btn-ghost text-xs" onClick={() => void api.cancelStoreDownload().catch(e => notify("error", String(e)))}>Cancelar</button></div>
          <p className="mt-2 text-xs text-[var(--color-muted)]">{size(transfer.received) || "0 B"}{transfer.total ? ` / ${size(transfer.total)} · ${progress}%` : " · Tamaño pendiente"} · {size(transfer.speed) || "0 B"}/s · Archivo actual</p>
          <progress aria-label="Progreso del archivo actual" className="mt-2 h-2 w-full" max={100} value={transfer.total ? progress : undefined} />
        </div>}
        {!downloading && retry && <div className="glass mb-4 flex items-center justify-between gap-3 rounded-xl p-4 text-sm"><span>Descarga incompleta: {retry.entry.name}</span><button className="btn btn-primary text-xs" onClick={() => void downloadEntry(retry.entry, retry.item)}>Reintentar</button></div>}
        <div className="mb-4 flex flex-wrap gap-2">{[["all", "Explorar"], ["trending", "Tendencias"], ["favorites", `Favoritos (${library.favorites.length})`], ["updates", `Versiones distintas (${changedVersions.size})`], ["history", `Historial (${library.history.length})`]].map(([id, label]) => <button key={id} aria-pressed={shelf === id} onClick={() => setShelf(id)} className={`btn text-xs ${shelf === id ? "btn-primary" : "btn-ghost"}`}>{label}</button>)}</div>
        {shelf === "updates" && <p className="mb-4 text-xs text-[var(--color-muted)]">La versión publicada difiere de tu última descarga. Consulta la ficha para comprobar los cambios.</p>}
        {shelf === "trending" ? <Trending entries={entries} select={setSelected} /> : shelf === "history" ? <div className="space-y-3">
          {!library.history.length && <p className="glass rounded-xl p-6 text-sm">Tus descargas completadas aparecerán aquí.</p>}
          {library.history.slice(0, visibleCount).map((item, index) => <div key={`${item.completedAt}-${index}`} className="glass flex items-center gap-3 rounded-xl p-4">
            <div className="min-w-0 flex-1"><p className="text-sm font-semibold">{item.name} · {item.version || "Sin versión"}</p><p className="mt-1 break-all text-xs text-[var(--color-muted)]">{item.filename} · {new Date(item.completedAt).toLocaleString()}</p></div>
            <button className="btn btn-ghost text-xs" onClick={() => void openPath(item.directory).catch(e => notify("error", `No se pudo abrir la carpeta: ${String(e)}`))}><FolderOpen size={15} /> Carpeta</button>
          </div>)}
          {library.history.length > visibleCount && <button className="btn btn-ghost" onClick={() => setVisibleCount(n => n + 48)}>Mostrar más descargas</button>}
        </div> : <>
        {shelf === "all" && !query && platform === "all" && category === "all" && <div className="mb-5 grid gap-4 lg:grid-cols-2">
          {[["Actualizados recientemente", highlights.updated], ["Añadidos a la fuente", highlights.added]].map(([title, apps]) => <section key={title as string} className="glass rounded-xl p-4">
            <h2 className="mb-3 text-sm font-semibold">{title as string}</h2>
            {(apps as StoreEntry[]).map(entry => <button key={entry.id} onClick={() => setSelected(entry)} className="mb-2 flex w-full items-center gap-3 rounded-lg p-2 text-left hover:bg-white/5"><StoreImage key={entry.icon_url} url={entry.icon_url} name={entry.name} className="h-10 w-10" /><span className="min-w-0"><span className="block truncate text-xs font-medium">{entry.name}</span><span className="text-[0.65rem] text-[var(--color-muted)]">{entry.platforms.join(" · ").toUpperCase()}</span></span></button>)}
          </section>)}
        </div>}
        <div className="mb-5 flex flex-wrap gap-2">
          <label className="glass flex min-w-[240px] flex-1 items-center gap-2 rounded-xl px-3 py-2"><Search size={15} className="text-[var(--color-faint)]" /><input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Buscar homebrew..." className="min-w-0 flex-1 bg-transparent text-sm outline-none" /></label>
          {PLATFORMS.map((p) => <button key={p} onClick={() => setPlatform(p)} className={`rounded-xl px-3 py-2 text-xs font-medium ${platform === p ? "bg-[var(--accent)] text-[#0a0c12]" : "glass text-[var(--color-muted)]"}`}>{LABELS[p] ?? p.toUpperCase()}</button>)}
        </div>
        {loading && <div className="flex items-center gap-2 py-10 text-sm text-[var(--color-muted)]"><Loader2 size={16} className="animate-spin" /> Cargando catálogo...</div>}
        <div className="mb-4 flex flex-wrap gap-3 text-xs">
          <label>Categoría <select value={category} onChange={e => setCategory(e.target.value)} className="ml-2 rounded-lg bg-[#171923] p-2">
            {[["all", "Todas"], ["games", "Juegos"], ["emulators", "Emuladores"], ["utilities", "Utilidades"], ["multimedia", "Multimedia"], ["customization", "Personalización"], ["unknown", "Sin categoría"]].map(([id, name]) => <option key={id} value={id}>{name}</option>)}
          </select></label>
          <label>Orden <select value={order} onChange={e => setOrder(e.target.value)} className="ml-2 rounded-lg bg-[#171923] p-2"><option value="name">Nombre</option><option value="updated">Actualizados recientemente</option><option value="added">Añadidos a la fuente recientemente</option></select></label>
          {order !== "name" && <p className="self-center text-[var(--color-muted)]">Las entradas sin fecha confirmada aparecen al final.</p>}
        </div>
        {error && <div className="glass rounded-xl p-4 text-sm text-rose-300">No se pudo cargar el catálogo: {error}</div>}
        {!loading && !error && filtered.length === 0 && <div className="glass rounded-xl p-8 text-center text-sm text-[var(--color-muted)]">No hay resultados para esta búsqueda.</div>}
        <div className="grid grid-cols-1 gap-3 xl:grid-cols-2">
          {filtered.slice(0, visibleCount).map((entry) => {
            const artifact = entry.downloads[0];
            return <div key={entry.id} className="glass flex gap-3 rounded-xl p-3">
              <StoreImage key={entry.icon_url} url={entry.icon_url} name={entry.name} />
            <div className="min-w-0 flex-1">
              <button onClick={() => setSelected(entry)} className="block max-w-full truncate text-left text-sm font-semibold hover:underline">{entry.name}</button>
              <button aria-label={`Favorito: ${entry.name}`} aria-pressed={library.favorites.includes(entry.id)} onClick={() => toggleFavorite(entry.id)} className="btn btn-ghost mt-1 p-1"><Heart size={15} fill={library.favorites.includes(entry.id) ? "currentColor" : "none"} /></button>
              {changedVersions.has(entry.id) && <span className="ml-2 text-xs text-[var(--accent)]">Versión distinta disponible</span>}
              <p className="mt-1 text-[0.65rem] text-[var(--color-faint)]">{entry.platforms.map(p => LABELS[p] ?? p.toUpperCase()).join(" · ")}</p>
              <p className="mt-1 line-clamp-2 text-xs text-[var(--color-muted)]">{entry.summary || "Sin descripción"}</p>
              <p className="mt-1 text-[0.65rem] text-[var(--color-faint)]">{entry.author || "Autor desconocido"}{entry.version ? ` · ${entry.version}` : ""}{artifact?.size ? ` · ${size(artifact.size)}` : ""}</p>
              {downloading === entry.id && <div className="mt-2 h-1 overflow-hidden rounded-full bg-white/[0.08]"><div className="h-full rounded-full bg-[var(--accent)] transition-all" style={{ width: `${progress}%` }} /></div>}
              <div className="mt-2 flex gap-2">
                <button onClick={() => setSelected(entry)} className="btn btn-primary px-2.5 py-1 text-xs"><Download size={13} /> Ver ficha y archivos</button>
                {entry.release_url?.startsWith("https://") && <button onClick={() => void openUrl(entry.release_url!)} className="btn btn-ghost px-2.5 py-1 text-xs"><ExternalLink size={13} /> Origen</button>}
              </div>
            </div>
            </div>;
          })}
        </div>
        {filtered.length > visibleCount && <button className="btn btn-ghost mx-auto mt-5" onClick={() => setVisibleCount(n => n + 48)}>Mostrar más ({visibleCount} de {filtered.length})</button>}
        </>}
        {selected && <StoreDetails entry={selected} busy={downloading !== null} close={() => setSelected(null)} download={(entry, artifact) => void downloadEntry(entry, artifact)} />}
      </div>
    </section>
  );
}

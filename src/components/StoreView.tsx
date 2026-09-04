import { Download, ExternalLink, Loader2, Search, Store as StoreIcon } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useState } from "react";
import { api } from "../lib/api";
import { useStore } from "../store";

type DownloadEntry = { format: string; filename: string; url: string; size?: number | null; sha256?: string | null };
type StoreEntry = {
  id: string;
  platforms: string[];
  name: string;
  summary?: string;
  author?: string;
  version?: string | null;
  license?: string | null;
  icon_url?: string | null;
  release_url?: string | null;
  downloads: DownloadEntry[];
  source?: string;
};

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
  const [platform, setPlatform] = useState("all");
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [downloading, setDownloading] = useState<string | null>(null);
  const notify = useStore((s) => s.notify);

  useEffect(() => {
    fetch("/store/catalog.json", { cache: "no-cache" })
      .then((r) => { if (!r.ok) throw new Error(`HTTP ${r.status}`); return r.json(); })
      .then((catalog) => setEntries(Array.isArray(catalog.entries) ? catalog.entries : []))
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, []);

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return entries.filter((entry) =>
      (platform === "all" || entry.platforms.includes(platform)) &&
      (!needle || `${entry.name} ${entry.author ?? ""} ${entry.summary ?? ""}`.toLowerCase().includes(needle)),
    );
  }, [entries, platform, query]);

  async function downloadEntry(entry: StoreEntry) {
    const item = entry.downloads[0];
    if (!item) return;
    const destination = await open({ directory: true, multiple: false });
    if (typeof destination !== "string") return;
    setDownloading(entry.id);
    try {
      await api.downloadHomebrew(item.url, item.filename, destination, item.sha256);
      notify("ok", `${entry.name} descargado`);
    } catch (e) {
      notify("error", `No se pudo descargar: ${String(e)}`);
    } finally { setDownloading(null); }
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
          <span className="rounded-full border border-[var(--color-edge)] px-3 py-1 text-xs text-[var(--color-muted)]">{entries.length} apps</span>
        </div>
        <div className="mb-5 flex flex-wrap gap-2">
          <label className="glass flex min-w-[240px] flex-1 items-center gap-2 rounded-xl px-3 py-2"><Search size={15} className="text-[var(--color-faint)]" /><input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Buscar homebrew..." className="min-w-0 flex-1 bg-transparent text-sm outline-none" /></label>
          {PLATFORMS.map((p) => <button key={p} onClick={() => setPlatform(p)} className={`rounded-xl px-3 py-2 text-xs font-medium ${platform === p ? "bg-[var(--accent)] text-[#0a0c12]" : "glass text-[var(--color-muted)]"}`}>{LABELS[p] ?? p.toUpperCase()}</button>)}
        </div>
        {loading && <div className="flex items-center gap-2 py-10 text-sm text-[var(--color-muted)]"><Loader2 size={16} className="animate-spin" /> Cargando catálogo...</div>}
        {error && <div className="glass rounded-xl p-4 text-sm text-rose-300">No se pudo cargar el catálogo: {error}</div>}
        {!loading && !error && filtered.length === 0 && <div className="glass rounded-xl p-8 text-center text-sm text-[var(--color-muted)]">No hay resultados para esta búsqueda.</div>}
        <div className="grid grid-cols-1 gap-3 xl:grid-cols-2">
          {filtered.map((entry) => {
            const artifact = entry.downloads[0];
            return <div key={entry.id} className="glass flex gap-3 rounded-xl p-3">
              <div className="h-16 w-16 shrink-0 overflow-hidden rounded-xl bg-white/[0.06]">{entry.icon_url ? <img src={entry.icon_url} alt="" className="h-full w-full object-cover" loading="lazy" /> : <div className="flex h-full items-center justify-center text-xs text-[var(--color-faint)]">HB</div>}</div>
            <div className="min-w-0 flex-1"><div className="flex items-start justify-between gap-2"><h2 className="truncate text-sm font-semibold">{entry.name}</h2><span className="shrink-0 text-[0.65rem] text-[var(--color-faint)]">{entry.platforms.map((p) => LABELS[p] ?? p.toUpperCase()).join(" · ")}</span></div><p className="mt-0.5 line-clamp-2 text-xs text-[var(--color-muted)]">{entry.summary || "Sin descripción"}</p><p className="mt-1 text-[0.65rem] text-[var(--color-faint)]">{entry.author || "Autor desconocido"}{entry.version ? ` · ${entry.version}` : ""}{artifact?.size ? ` · ${size(artifact.size)}` : ""}</p><div className="mt-2 flex gap-2"><button disabled={!artifact || downloading === entry.id} onClick={() => void downloadEntry(entry)} className="btn btn-primary px-2.5 py-1 text-xs"><Download size={13} /> {downloading === entry.id ? "Descargando..." : `Descargar${artifact ? ` · ${artifact.format}` : ""}`}</button>{entry.release_url && <button onClick={() => openUrl(entry.release_url!)} className="btn btn-ghost px-2.5 py-1 text-xs"><ExternalLink size={13} /> Origen</button>}</div></div>
            </div>;
          })}
        </div>
      </div>
    </section>
  );
}

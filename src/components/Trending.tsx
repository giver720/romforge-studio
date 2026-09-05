import type { StoreEntry } from "../lib/storeCatalog";
import { StoreImage } from "./StoreImage";

const labels: Record<string, string> = { psvita: "PS Vita", psp: "PSP", wiiu: "Wii U" };
const platformName = (platform: string) => labels[platform] || platform.toUpperCase();
const count = (value?: number | null) => value == null ? "Sin contador publicado" : `${new Intl.NumberFormat("es").format(value)} descargas registradas`;
const dateValue = (value?: string | null) => value && /^\d{4}-\d{2}-\d{2}(T|$)/.test(value) ? Date.parse(value) || 0 : 0;

export function Trending({ entries, platform = "all", category = "all", select }: { entries: StoreEntry[]; platform?: string; category?: string; select: (entry: StoreEntry) => void }) {
  const scoped = entries.filter(entry =>
    (platform === "all" || entry.platforms.includes(platform)) &&
    (category === "all" || (category === "unknown" ? !entry.categories?.length : entry.categories?.includes(category))),
  );
  const ranked = scoped.filter(entry => entry.popularity != null).sort((a, b) => (b.popularity ?? 0) - (a.popularity ?? 0) || a.name.localeCompare(b.name));
  const recent = scoped.filter(entry => dateValue(entry.updated_at) > 0).sort((a, b) => dateValue(b.updated_at) - dateValue(a.updated_at) || a.name.localeCompare(b.name));
  const platforms = ["3ds", "wii", "wiiu", "switch", "psvita", "psp", "ps4", "ps5"];
  const title = platform === "all" ? "Más descargados con métrica de la fuente" : `Más descargados en ${platformName(platform)}`;
  return <div className="space-y-5"><section className="glass rounded-xl p-4"><h2 className="text-sm font-semibold">{title}</h2><p className="mt-1 text-xs text-[var(--color-muted)]">Popularidad publicada por la fuente. Las consolas sin contador, como PS5, se ordenan por novedades recientes.</p><div className="mt-3 grid gap-2 md:grid-cols-2">{(ranked.length ? ranked : recent).slice(0, 12).map((entry, index) => <button key={entry.id} onClick={() => select(entry)} className="flex items-center gap-3 rounded-lg p-2 text-left hover:bg-white/5"><span className="w-5 text-center text-xs text-[var(--accent)]">{index + 1}</span><StoreImage url={entry.icon_url} name={entry.name} className="h-10 w-10" /><span className="min-w-0"><span className="block truncate text-xs font-medium">{entry.name}</span><span className="text-[0.65rem] text-[var(--color-muted)]">{entry.platforms.map(platformName).join(" · ")} · {entry.popularity != null ? count(entry.popularity) : "Novedad reciente"}</span></span></button>)}</div>{!ranked.length && !recent.length && <p className="mt-3 text-xs text-[var(--color-muted)]">No hay métricas ni fechas publicadas para esta selección.</p>}</section>{platform === "all" && <div className="grid gap-4 lg:grid-cols-2">{platforms.map(platform => { const list = ranked.filter(entry => entry.platforms.includes(platform)).slice(0, 5); return <section key={platform} className="glass rounded-xl p-4"><h2 className="text-sm font-semibold">Populares en {platformName(platform)}</h2>{list.length ? list.map(entry => <button key={entry.id} onClick={() => select(entry)} className="mt-2 flex w-full items-center gap-3 rounded-lg p-2 text-left hover:bg-white/5"><StoreImage url={entry.icon_url} name={entry.name} className="h-9 w-9" /><span className="min-w-0"><span className="block truncate text-xs">{entry.name}</span><span className="text-[0.65rem] text-[var(--color-muted)]">{count(entry.popularity)}</span></span></button>) : <p className="mt-2 text-xs text-[var(--color-muted)]">La fuente no publica un contador para esta consola.</p>}</section>})}</div>}</div>;
}

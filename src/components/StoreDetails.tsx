import { useEffect, useRef } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Download, ExternalLink, X } from "lucide-react";
import type { StoreDownload, StoreEntry } from "../lib/storeCatalog";
import { StoreImage } from "./StoreImage";

export function StoreDetails({ entry, busy, close, download }: {
  entry: StoreEntry; busy: boolean; close: () => void;
  download: (entry: StoreEntry, artifact: StoreDownload) => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => { dialog.current?.showModal(); }, []);
  const source = entry.release_url || entry.source_url;
  return <dialog ref={dialog} onCancel={close} onClick={e => { if (e.target === e.currentTarget) close(); }}
    aria-labelledby="store-details-title"
    className="m-auto max-h-[85vh] w-[min(680px,90vw)] overflow-auto rounded-2xl border border-[var(--color-edge)] bg-[#11131c] p-6 text-[var(--color-text)] backdrop:bg-black/70">
    <div className="flex items-start justify-between gap-4">
      <StoreImage key={entry.icon_url} url={entry.icon_url} name={entry.name} />
      <div><p className="eyebrow">{entry.platforms.join(" · ").toUpperCase()}</p><h2 id="store-details-title" className="mt-2 text-xl font-semibold">{entry.name}</h2></div>
      <button autoFocus onClick={close} aria-label="Cerrar ficha" className="btn btn-ghost"><X size={18} /></button>
    </div>
    <p className="mt-2 text-sm text-[var(--color-muted)]">{entry.author || "Autor sin indicar"} · {entry.version || "Versión sin indicar"}</p>
    <p className="mt-5 whitespace-pre-wrap text-sm leading-relaxed">{entry.description || entry.summary || "El catálogo no proporciona una descripción."}</p>
    {!!entry.screenshots?.length && <div className="mt-4 flex gap-3 overflow-x-auto">{entry.screenshots.filter(url => url.startsWith("https://")).map(url => <StoreImage key={url} url={url} name={`Captura de ${entry.name}`} className="h-48 w-80" />)}</div>}
    <dl className="mt-5 grid grid-cols-2 gap-3 text-xs text-[var(--color-muted)]">
      <div><dt>Fecha publicada por la fuente</dt><dd className="mt-1">{entry.updated_at || "Sin fecha"}</dd></div>
      <div><dt>Licencia</dt><dd className="mt-1">{entry.license || "Sin indicar"}</dd></div>
    </dl>
    <div className="mt-5 rounded-xl bg-white/5 p-4 text-sm"><h3 className="font-semibold">Compatibilidad y requisitos</h3><p className="mt-2 whitespace-pre-wrap text-[var(--color-muted)]">{entry.requirements || "Compatibilidad sin confirmar. Consulta las instrucciones del autor para tu consola."}</p></div>
    <h3 className="mt-6 text-sm font-semibold">Archivos disponibles</h3>
    <p className="mt-1 text-xs text-[var(--color-muted)]">Elige el formato indicado por el autor. Los archivos marcados como datos complementarios se descargan por separado.</p>
    <div className="mt-3 space-y-2">{entry.downloads.map((artifact, index) => <div key={`${artifact.url}-${index}`} className="flex items-center gap-3 rounded-xl border border-[var(--color-edge)] p-3">
      <div className="min-w-0 flex-1"><p className="break-all text-sm">{artifact.filename}</p><p className="mt-1 text-xs text-[var(--color-muted)]">{artifact.format.toUpperCase()}{artifact.size ? ` · ${(artifact.size / 1048576).toFixed(1)} MB` : " · Tamaño sin indicar"}{artifact.format === "hbas" ? " · Paquete de varios archivos" : ""}</p></div>
      <button disabled={busy} onClick={() => download(entry, artifact)} className="btn btn-primary text-xs"><Download size={14} /> {artifact.format === "data" ? "Descargar datos complementarios" : "Descargar"}</button>
    </div>)}</div>
    {source?.startsWith("https://") && <button onClick={() => void openUrl(source)} className="btn btn-ghost mt-5 text-sm"><ExternalLink size={15} /> {entry.source || "Proyecto original"}</button>}
  </dialog>;
}

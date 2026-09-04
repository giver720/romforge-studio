import { open } from "@tauri-apps/plugin-dialog";
import { AnimatePresence, motion } from "framer-motion";
import { AlertTriangle, FolderInput, Package2, Play, Trash2, X } from "lucide-react";
import { useEffect } from "react";
import { api } from "../lib/api";
import { bytes } from "../lib/format";
import { PRESETS } from "../lib/profiles";
import { PSP_EXT, PSP_OPS, opsFor } from "../lib/psp";
import { useStore } from "../store";
import type { Preset } from "../lib/profiles";
import { DropZone } from "./DropZone";
import { Segmented } from "./ui";

export function PspView({ dragging }: { dragging: boolean }) {
  const {
    notify,
    refreshJobs,
    settings,
    patchSettings,
    tools,
    refreshTools,
    consoleFiles,
    addConsoleFiles,
    setConsoleOp,
    removeConsoleFile,
    clearConsoleFiles,
  } = useStore();
  const files = consoleFiles.psp;

  useEffect(() => {
    refreshTools();
  }, []);

  const maxcso = tools.find((t) => t.id === "maxcso");
  const missing = maxcso && !maxcso.found;

  async function handlePaths(paths: string[]) {
    const infos = await api.inspectPaths(paths);
    const ok = infos.filter((i) => PSP_EXT.includes(i.ext));
    if (!ok.length) {
      notify("warn", "Ahí no hay archivos .iso, .cso, .zso ni .dax");
      return;
    }
    addConsoleFiles("psp", ok);
  }

  async function run() {
    if (!files.length) return;
    await api.addJobs(files.map((f) => ({ input: f.path, mode: f.op, system: "psp" })));
    clearConsoleFiles("psp");
    await refreshJobs();
    notify("ok", `${files.length} ${files.length === 1 ? "trabajo encolado" : "trabajos encolados"}`);
  }

  return (
    <div className="scroll flex-1 p-5">
      <div className="mb-4 flex items-end justify-between gap-4">
        <div>
          <h1 className="text-lg font-semibold tracking-tight">
            PSP <span className="accent-text">UMD</span>
          </h1>
          <p className="mt-0.5 text-xs text-[var(--color-muted)]">
            Comprime tus ISO a CSO o ZSO sin perder nada. La consola y PPSSPP los leen tal cual.
          </p>
        </div>
        <div className="flex gap-2">
          {files.length > 0 && (
            <button className="btn btn-primary" onClick={run} disabled={!!missing}>
              <Play size={15} /> Convertir {files.length}
            </button>
          )}
        </div>
      </div>

      {missing && (
        <div className="mb-4 flex items-start gap-3 rounded-xl border border-amber-400/25 bg-amber-400/[0.07] p-3.5">
          <AlertTriangle size={17} className="mt-0.5 shrink-0 text-amber-400" />
          <div className="min-w-0 flex-1">
            <p className="text-[0.8rem] font-medium">Falta maxcso</p>
            <p className="mt-1 text-[0.7rem] leading-relaxed text-[var(--color-muted)]">
              Es un único ejecutable de 500 KB. ROMForge Studio puede descargarlo por ti.
            </p>
          </div>
          <button
            className="btn btn-ghost shrink-0"
            onClick={() => useStore.setState({ view: "settings" })}
          >
            <Package2 size={15} /> Instalar
          </button>
        </div>
      )}

      <DropZone
        dragging={dragging}
        compact={files.length > 0}
        title="Suelta aquí tus ISO, CSO, ZSO o DAX"
        hint="Comprimir y descomprimir es reversible: el ISO que sale es idéntico al que entró."
        extensions={PSP_EXT}
        onPaths={handlePaths}
      />

      <div className="glass mt-4 flex flex-wrap items-center gap-4 rounded-2xl p-3">
        <div className="min-w-[220px] flex-1">
          <label className="mb-1.5 block text-[0.68rem] font-medium text-[var(--color-muted)]">
            Esfuerzo de compresión
          </label>
          <Segmented<Preset>
            layoutId="psp-preset"
            value={settings.preset as Preset}
            onChange={(v) => patchSettings({ preset: v })}
            options={PRESETS.map((p) => ({ id: p.id, label: p.name, hint: p.desc }))}
          />
          <p className="mt-1.5 text-[0.64rem] leading-snug text-[var(--color-faint)]">
            En un UMD de 1,5 GB: Rápida deja el 33,3 % en 8 s; las otras dos, el 33,1 % en 100 s.
            Para PSP la diferencia es mínima, así que Rápida casi siempre compensa.
          </p>
        </div>
        <div className="min-w-[220px] flex-1">
          <label className="mb-1.5 block text-[0.68rem] font-medium text-[var(--color-muted)]">
            Carpeta de salida
          </label>
          <button
            className="btn btn-ghost w-full justify-start"
            onClick={async () => {
              const res = await open({ directory: true, multiple: false });
              if (res) await patchSettings({ output_dir: res as string });
            }}
          >
            <FolderInput size={15} className="shrink-0" />
            <span className="truncate">{settings.output_dir || "Junto al archivo original"}</span>
          </button>
        </div>
      </div>

      {files.length > 0 ? (
        <>
          <div className="mb-2 mt-5 flex items-center justify-between">
            <h2 className="text-[0.8rem] font-semibold">
              {files.length} {files.length === 1 ? "archivo" : "archivos"}
            </h2>
            <button
              className="btn btn-quiet btn-danger px-2 py-1 text-xs"
              onClick={() => clearConsoleFiles("psp")}
            >
              <Trash2 size={13} /> Vaciar
            </button>
          </div>
          <ul className="flex flex-col gap-1.5">
            <AnimatePresence initial={false}>
              {files.map((f) => (
                <motion.li
                  key={f.path}
                  layout
                  initial={{ opacity: 0, y: 8 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, x: -12, transition: { duration: 0.14 } }}
                  className="glass group flex items-center gap-3 rounded-xl px-3 py-2.5"
                >
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-[0.82rem] font-medium">{f.name}</span>
                    <span className="block truncate text-[0.68rem] text-[var(--color-faint)]">
                      {f.ext.toUpperCase()} · {bytes(f.size)}
                    </span>
                  </span>
                  <select
                    className="field w-[150px] shrink-0 py-1.5 text-xs"
                    value={f.op}
                    onChange={(e) => setConsoleOp("psp", f.path, e.target.value)}
                  >
                    {opsFor(f.ext).map((o) => (
                      <option key={o.id} value={o.id} title={o.desc}>
                        {o.label}
                      </option>
                    ))}
                  </select>
                  <button
                    onClick={() => removeConsoleFile("psp", f.path)}
                    className="btn btn-quiet btn-danger px-2 py-1 opacity-0 group-hover:opacity-100"
                  >
                    <X size={14} />
                  </button>
                </motion.li>
              ))}
            </AnimatePresence>
          </ul>
        </>
      ) : (
        <div className="mt-6">
          <h2 className="mb-2 text-[0.8rem] font-semibold">Qué formato elegir</h2>
          <div className="grid grid-cols-2 gap-2">
            {PSP_OPS.map((o) => (
              <div key={o.id} className="glass rounded-xl p-3">
                <p className="flex flex-wrap items-center gap-1.5 text-[0.78rem] font-semibold">
                  {o.label}
                  {o.experimental && <span className="chip">experimental</span>}
                </p>
                <p className="mt-1 text-[0.68rem] leading-snug text-[var(--color-muted)]">{o.desc}</p>
              </div>
            ))}
          </div>
          <p className="mt-3 text-[0.68rem] leading-relaxed text-[var(--color-faint)]">
            Si solo juegas en PPSSPP, mira también la pestaña <span className="text-[var(--color-ink)]">Convertir</span>:
            un CHD suele apretar algo más que un CSO. Pero CHD no funciona en una PSP de verdad, así
            que para la consola quédate con CSO, o con ZSO si tu CFW lo admite.
          </p>
        </div>
      )}
    </div>
  );
}

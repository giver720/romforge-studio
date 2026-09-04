import { open } from "@tauri-apps/plugin-dialog";
import { motion } from "framer-motion";
import { FolderInput, Play, Sparkles, Trash2, Wand2 } from "lucide-react";
import { GENERATIONS, PRESETS, codecsFor } from "../lib/profiles";
import { useStore } from "../store";
import type { Preset } from "../lib/profiles";
import { DropZone } from "./DropZone";
import { StagedList } from "./StagedList";
import { Segmented } from "./ui";

const ACCEPTED = ["cue", "gdi", "iso", "toc", "nrg", "cdr", "img", "raw", "hdi", "vhd", "hdd"];

function GenerationsGuide() {
  return (
    <div className="mt-5 grid grid-cols-2 gap-3">
      {GENERATIONS.map((g, i) => (
        <motion.div
          key={g.id}
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.05 * i, type: "spring", stiffness: 300, damping: 30 }}
          className="glass rounded-xl p-3.5"
        >
          <h4 className="text-[0.8rem] font-semibold">{g.title}</h4>
          <p className="mt-0.5 text-[0.66rem] leading-snug text-[var(--color-faint)]">{g.subtitle}</p>
          <div className="mt-2.5 flex flex-wrap gap-1.5">
            {g.systems.map((s) => (
              <span
                key={s.id}
                className="chip"
                style={{ borderColor: `${s.color}44`, background: `${s.color}14`, color: s.color }}
              >
                {s.name}
              </span>
            ))}
          </div>
        </motion.div>
      ))}
    </div>
  );
}

export function ConvertView({ dragging }: { dragging: boolean }) {
  const {
    staged,
    addPaths,
    clearStaged,
    setStaged,
    enqueueStaged,
    settings,
    patchSettings,
    notify,
    chdman,
  } = useStore();

  async function handlePaths(paths: string[]) {
    const { added, skipped } = await addPaths(paths);
    const bad = skipped.filter((s) => s.state !== "missing");
    if (added === 0 && bad.length) {
      notify("warn", bad[0].note ?? "Ese formato no lo admite chdman");
    } else if (bad.length) {
      notify("warn", `${added} añadidos · ${bad.length} descartados por formato`);
    }
  }

  async function pickOutput() {
    const res = await open({ directory: true, multiple: false });
    if (res) await patchSettings({ output_dir: res as string });
  }

  async function convert() {
    const n = await enqueueStaged();
    if (n) notify("ok", `${n} ${n === 1 ? "trabajo encolado" : "trabajos encolados"}`);
  }

  const totalSize = staged.reduce((a, s) => a + s.size, 0);

  // Cada modo tiene su familia de códecs; mostramos los del primer archivo del lote
  const codecPreview = staged.length
    ? codecsFor(staged[0].mode, settings.preset as Preset, chdman?.supports_zstd ?? false).join(", ")
    : "";

  return (
    <div className="scroll flex-1 p-5">
      <div className="mb-4 flex items-end justify-between gap-4">
        <div>
          <h1 className="text-lg font-semibold tracking-tight">
            Convertir a <span className="accent-text">CHD</span>
          </h1>
          <p className="mt-0.5 text-xs text-[var(--color-muted)]">
            Arrastra tus imágenes de disco y elige a qué sistema pertenecen. ROMForge Studio se encarga del resto.
          </p>
        </div>
        {staged.length > 0 && (
          <button className="btn btn-primary" onClick={convert}>
            <Play size={15} /> Convertir {staged.length}
          </button>
        )}
      </div>

      <DropZone
        dragging={dragging}
        compact={staged.length > 0}
        title="Suelta aquí tus ISO, CUE, GDI o IMG"
        hint="También puedes soltar una carpeta entera: se recorren las subcarpetas y se ignoran los archivos que chdman no sabe leer."
        extensions={ACCEPTED}
        onPaths={handlePaths}
      />

      {staged.length === 0 ? (
        <>
          <div className="mt-6 mb-2 flex items-center gap-2">
            <Sparkles size={14} style={{ color: "var(--accent)" }} />
            <h2 className="text-[0.8rem] font-semibold">Qué generaciones cubre CHD</h2>
          </div>
          <GenerationsGuide />
        </>
      ) : (
        <>
          <div className="glass mt-4 flex flex-wrap items-center gap-3 rounded-2xl p-3">
            <div className="min-w-[210px] flex-1">
              <label className="mb-1.5 block text-[0.68rem] font-medium text-[var(--color-muted)]">
                Compresión
              </label>
              <Segmented<Preset>
                layoutId="preset-seg"
                value={settings.preset as Preset}
                onChange={(v) => patchSettings({ preset: v })}
                options={PRESETS.map((p) => ({ id: p.id, label: p.name, hint: p.desc }))}
              />
            </div>

            <div className="min-w-[210px] flex-1">
              <label className="mb-1.5 block text-[0.68rem] font-medium text-[var(--color-muted)]">
                Aplicar sistema a todos
              </label>
              <select
                className="field py-2"
                value=""
                onChange={(e) => {
                  if (!e.target.value) return;
                  setStaged(
                    staged.map((s) => s.path),
                    { systemId: e.target.value },
                  );
                  e.target.value = "";
                }}
              >
                <option value="">Sin cambios…</option>
                {GENERATIONS.map((g) => (
                  <optgroup key={g.id} label={g.title}>
                    {g.systems.map((s) => (
                      <option key={s.id} value={s.id}>
                        {s.name}
                      </option>
                    ))}
                  </optgroup>
                ))}
              </select>
            </div>

            <div className="min-w-[210px] flex-1">
              <label className="mb-1.5 block text-[0.68rem] font-medium text-[var(--color-muted)]">
                Carpeta de salida
              </label>
              <button className="btn btn-ghost w-full justify-start" onClick={pickOutput}>
                <FolderInput size={15} className="shrink-0" />
                <span className="truncate">
                  {settings.output_dir || "Junto al archivo original"}
                </span>
              </button>
            </div>
          </div>

          <div className="mb-2 mt-5 flex items-center justify-between">
            <h2 className="text-[0.8rem] font-semibold">
              {staged.length} {staged.length === 1 ? "archivo" : "archivos"}
              <span className="ml-2 font-normal text-[var(--color-faint)]">
                {(totalSize / 1024 ** 3).toFixed(2)} GB en total
              </span>
            </h2>
            <button className="btn btn-quiet btn-danger px-2 py-1 text-xs" onClick={clearStaged}>
              <Trash2 size={13} /> Vaciar
            </button>
          </div>
          <StagedList />

          <p className="mt-4 flex items-start gap-2 rounded-xl border border-[var(--color-edge)] bg-white/[0.03] p-3 text-[0.7rem] leading-relaxed text-[var(--color-muted)]">
            <Wand2 size={14} className="mt-0.5 shrink-0" style={{ color: "var(--accent)" }} />
            <span>
              Códecs para este lote:{" "}
              <span className="mono text-[var(--color-ink)]">{codecPreview}</span>. El archivo original
              no se toca salvo que actives el borrado automático en Ajustes.
            </span>
          </p>
        </>
      )}
    </div>
  );
}

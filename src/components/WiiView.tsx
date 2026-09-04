import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { AnimatePresence, motion } from "framer-motion";
import {
  AlertTriangle,
  Download,
  FolderInput,
  FolderSearch,
  Loader2,
  Play,
  Trash2,
  X,
} from "lucide-react";
import { useEffect } from "react";
import { api } from "../lib/api";
import { bytes } from "../lib/format";
import { EXECUTABLE_FILTERS, executableName } from "../lib/platform";
import { WII_EXT, WII_OPS, opsFor } from "../lib/wii";
import { useStore } from "../store";
import { DropZone } from "./DropZone";
import { Toggle } from "./ui";

export function WiiView({ dragging }: { dragging: boolean }) {
  const {
    notify,
    refreshJobs,
    settings,
    patchSettings,
    tools,
    setTools,
    refreshTools,
    installTool,
    installingTool,
    consoleFiles,
    addConsoleFiles,
    setConsoleOp,
    removeConsoleFile,
    clearConsoleFiles,
  } = useStore();
  const files = consoleFiles.wii;

  useEffect(() => {
    refreshTools();
  }, []);

  // WBFS lo hace wit; el resto DolphinTool. Solo se avisa de la que haga falta.
  const needed = new Set<string>(
    files.map((f) => (WII_OPS.find((o) => o.id === f.op)?.needsWit ? "wit" : "dolphintool")),
  );
  if (!files.length) needed.add("dolphintool");
  const missingTools = tools.filter((t) => needed.has(t.id) && !t.found);
  const missing = missingTools.length > 0;
  const usaWbfs = files.some((f) => f.op === "iso2wbfs");

  async function handlePaths(paths: string[]) {
    const infos = await api.inspectPaths(paths);
    const ok = infos.filter((i) => WII_EXT.includes(i.ext));
    if (!ok.length) {
      notify("warn", "Ahí no hay archivos .iso, .rvz, .wia ni .gcz");
      return;
    }
    addConsoleFiles("wii", ok);
  }

  /** Estas dos vienen dentro de otros programas, así que se señalan a mano. */
  async function browse(id: string, nombre: string) {
    const res = await open({
      multiple: false,
      filters: EXECUTABLE_FILTERS?.map((filter) => ({ ...filter, name: nombre })),
    });
    if (!res) return;
    setTools(await api.setToolPath(id, res as string));
    notify("ok", `${nombre} configurado`);
  }

  async function run() {
    if (!files.length) return;
    await api.addJobs(files.map((f) => ({ input: f.path, mode: f.op, system: "wii" })));
    clearConsoleFiles("wii");
    await refreshJobs();
    notify("ok", `${files.length} ${files.length === 1 ? "trabajo encolado" : "trabajos encolados"}`);
  }

  return (
    <div className="scroll flex-1 p-5">
      <div className="mb-4 flex items-end justify-between gap-4">
        <div>
          <h1 className="text-lg font-semibold tracking-tight">
            Wii y <span className="accent-text">GameCube</span>
          </h1>
          <p className="mt-0.5 text-xs text-[var(--color-muted)]">
            Convierte a RVZ, el formato de Dolphin. Es donde más espacio se recupera de todo el
            programa.
          </p>
        </div>
        {files.length > 0 && (
          <button className="btn btn-primary" onClick={run} disabled={!!missing}>
            <Play size={15} /> Convertir {files.length}
          </button>
        )}
      </div>

      {missingTools.map((t) => {
        const esWit = t.id === "wit";
        return (
          <div
            key={t.id}
            className="mb-4 flex items-start gap-3 rounded-xl border border-amber-400/25 bg-amber-400/[0.07] p-3.5"
          >
            <AlertTriangle size={17} className="mt-0.5 shrink-0 text-amber-400" />
            <div className="min-w-0 flex-1">
              <p className="text-[0.8rem] font-medium">No se encontró {t.name}</p>
              <p className="mt-1 text-[0.7rem] leading-relaxed text-[var(--color-muted)]">
                {esWit
                  ? "Hace falta solo para crear WBFS. ROMForge Studio puede descargarlo de la web de su autor (GPL-2.0), o puedes señalarlo si ya lo tienes."
                  : "Viene dentro de Dolphin, y su proyecto no publica en GitHub, así que ROMForge Studio no puede descargarlo solo. Si ya tienes Dolphin, señálale la carpeta; si no, bájalo de su web y vuelve aquí."}
              </p>
              <div className="mt-2 flex flex-wrap gap-2">
                {esWit && (
                  <button
                    className="btn btn-primary px-2.5 py-1 text-xs"
                    onClick={() => installTool("wit")}
                    disabled={installingTool === "wit"}
                  >
                    {installingTool === "wit" ? (
                      <Loader2 size={13} className="animate-spin" />
                    ) : (
                      <Download size={13} />
                    )}
                    Descargar e instalar
                  </button>
                )}
                <button
                  className="btn btn-ghost px-2.5 py-1 text-xs"
                  onClick={() => browse(t.id, esWit ? "wit" : "DolphinTool")}
                >
                  <FolderSearch size={13} /> Señalar{" "}
                  {esWit ? executableName("wit") : executableName("DolphinTool")}
                </button>
                {!esWit && (
                  <button
                    className="btn btn-quiet px-2.5 py-1 text-xs"
                    onClick={() => openUrl("https://dolphin-emu.org/download/")}
                  >
                    <Download size={13} /> Descargar Dolphin
                  </button>
                )}
              </div>
            </div>
          </div>
        );
      })}

      <DropZone
        dragging={dragging}
        compact={files.length > 0}
        title="Suelta aquí tus ISO, RVZ, WIA o GCZ"
        hint="Sirve igual para Wii y para GameCube: es el mismo formato de disco."
        extensions={WII_EXT}
        onPaths={handlePaths}
      />

      <div className="glass mt-4 rounded-2xl p-3">
        <Toggle
          checked={settings.wii_scrub}
          onChange={(v) => patchSettings({ wii_scrub: v })}
          label="Quitar el relleno del disco"
          hint="Los discos de Wii van rellenos de datos basura para entorpecer la copia. Quitarlos adelgaza el juego mucho más que la propia compresión, y el juego funciona igual."
        />

        {usaWbfs && (
          <Toggle
            checked={settings.wii_wbfs_split}
            onChange={(v) => patchSettings({ wii_wbfs_split: v })}
            label="Partir el WBFS en trozos"
            hint="Necesario si tu disco USB está formateado en FAT32, que no admite archivos de más de 4 GB. En NTFS o exFAT puedes dejarlo apagado."
          />
        )}

        <div className="mt-2 flex flex-wrap items-center gap-4 px-3">
          <div className="min-w-[200px] flex-1">
            <label className="mb-1 block text-[0.68rem] font-medium text-[var(--color-muted)]">
              Compresión zstd: <span className="text-[var(--color-ink)]">{settings.wii_level}</span>
              {settings.wii_level === 5 && (
                <span className="ml-1 text-[var(--color-faint)]">· lo que recomienda Dolphin</span>
              )}
            </label>
            <input
              type="range"
              min={1}
              max={22}
              value={settings.wii_level}
              onChange={(e) => patchSettings({ wii_level: Number(e.target.value) })}
              className="w-full accent-[var(--accent)]"
            />
          </div>
          <div className="min-w-[200px] flex-1">
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
      </div>

      {files.length > 0 ? (
        <>
          <div className="mb-2 mt-5 flex items-center justify-between">
            <h2 className="text-[0.8rem] font-semibold">
              {files.length} {files.length === 1 ? "archivo" : "archivos"}
              <span className="ml-2 font-normal text-[var(--color-faint)]">
                {(files.reduce((a, f) => a + f.size, 0) / 1024 ** 3).toFixed(2)} GB
              </span>
            </h2>
            <button
              className="btn btn-quiet btn-danger px-2 py-1 text-xs"
              onClick={() => clearConsoleFiles("wii")}
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
                    <span className="text-[0.68rem] text-[var(--color-faint)]">
                      {f.ext.toUpperCase()} · {bytes(f.size)}
                    </span>
                  </span>
                  <select
                    className="field w-[150px] shrink-0 py-1.5 text-xs"
                    value={f.op}
                    onChange={(e) => setConsoleOp("wii", f.path, e.target.value)}
                  >
                    {opsFor(f.ext).map((o) => (
                      <option key={o.id} value={o.id} title={o.desc}>
                        {o.label}
                      </option>
                    ))}
                  </select>
                  <button
                    onClick={() => removeConsoleFile("wii", f.path)}
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
            {WII_OPS.map((o) => (
              <div
                key={o.id}
                className="glass rounded-xl p-3"
                style={
                  o.recommended
                    ? { borderColor: "color-mix(in srgb, var(--accent) 40%, transparent)" }
                    : undefined
                }
              >
                <p className="flex flex-wrap items-center gap-1.5 text-[0.78rem] font-semibold">
                  {o.label}
                  {o.needsWit && <span className="chip">consola real</span>}
                  {o.recommended && (
                    <span
                      className="chip"
                      style={{
                        borderColor: "color-mix(in srgb, var(--accent) 45%, transparent)",
                        background: "var(--accent-soft)",
                        color: "var(--accent)",
                      }}
                    >
                      recomendado
                    </span>
                  )}
                </p>
                <p className="mt-1 text-[0.68rem] leading-snug text-[var(--color-muted)]">{o.desc}</p>
              </div>
            ))}
          </div>
          <p className="mt-3 text-[0.68rem] leading-relaxed text-[var(--color-faint)]">
            Lo hace <span className="mono">DolphinTool</span>, que viene dentro de{" "}
            <button className="underline" onClick={() => openUrl("https://dolphin-emu.org/")}>
              Dolphin
            </button>
            . RVZ es su formato nativo: el emulador lo lee sin descomprimir nada. Para jugar en una
            Wii de verdad con un cargador USB harían falta otras herramientas, que de momento no
            están incluidas.
          </p>
        </div>
      )}
    </div>
  );
}

import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { AnimatePresence, motion } from "framer-motion";
import { AlertTriangle, FolderInput, Package2, Play, Trash2, X } from "lucide-react";
import { useEffect, useState } from "react";
import { api } from "../lib/api";
import { bytes } from "../lib/format";
import { THREEDS_EXT, THREEDS_OPS, opsFor, z3dsExt } from "../lib/threeds";
import { useStore } from "../store";
import type { ThreeDsKeys } from "../lib/types";
import { DropZone } from "./DropZone";
import { KeyPicker } from "./KeyPicker";

export function ThreeDsView({ dragging }: { dragging: boolean }) {
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
  const files = consoleFiles.threeds;
  const [keys, setKeys] = useState<ThreeDsKeys | null>(null);

  const refreshKeys = () => api.threeDsKeysStatus().then(setKeys);

  useEffect(() => {
    refreshKeys();
    refreshTools();
  }, []);

  const usedOps = files.map((f) => THREEDS_OPS.find((o) => o.id === f.op)!).filter(Boolean);
  const missingTools = [...new Set(usedOps.flatMap((o) => o.tools))]
    .map((id) => tools.find((t) => t.id === id))
    .filter((t) => t && !t.found);

  const needsBoot9 = usedOps.some((o) => o.needs === "boot9") && keys && !keys.boot9;
  const blocked = !!missingTools.length || !!needsBoot9;

  async function handlePaths(paths: string[]) {
    const infos = await api.inspectPaths(paths);
    const ok = infos.filter((i) => THREEDS_EXT.includes(i.ext));
    if (!ok.length) {
      notify("warn", "Ahí no hay archivos .cci, .3ds, .cia, .cxi ni .3dsx");
      return;
    }
    addConsoleFiles("threeds", ok);
  }

  async function run() {
    if (!files.length) return;
    await api.addJobs(files.map((f) => ({ input: f.path, mode: f.op, system: "3ds" })));
    clearConsoleFiles("threeds");
    await refreshJobs();
    notify("ok", `${files.length} ${files.length === 1 ? "trabajo encolado" : "trabajos encolados"}`);
  }

  return (
    <div className="scroll flex-1 p-5">
      <div className="mb-4 flex items-end justify-between gap-4">
        <div>
          <h1 className="text-lg font-semibold tracking-tight">
            Nintendo <span className="accent-text">3DS</span>
          </h1>
          <p className="mt-0.5 text-xs text-[var(--color-muted)]">
            Comprime al formato Z3DS que Azahar carga tal cual, y convierte entre CIA y CCI.
          </p>
        </div>
        {files.length > 0 && (
          <button className="btn btn-primary" onClick={run} disabled={blocked}>
            <Play size={15} /> Procesar {files.length}
          </button>
        )}
      </div>

      {keys && (
        <div className="mb-4 grid grid-cols-2 gap-2">
          <KeyPicker
            label={keys.boot9 ? "boot9.bin localizado" : "Falta boot9.bin"}
            file="boot9.bin"
            found={!!keys.boot9}
            path={keys.boot9}
            hint="La bootROM de tu propia consola. Hace falta para descifrar. Comprimir a Z3DS no la necesita."
            fallback={keys.expected_dir}
            settingKey="boot9_path"
            onChange={refreshKeys}
          />
          <KeyPicker
            label={keys.seeddb ? "seeddb.bin localizado" : "Falta seeddb.bin"}
            file="seeddb.bin"
            found={!!keys.seeddb}
            path={keys.seeddb}
            hint="Los juegos de eShop posteriores a 2015 llevan una semilla propia que no está en boot9. Sin este archivo no se pueden descifrar."
            fallback={keys.expected_dir}
            settingKey="seeddb_path"
            onChange={refreshKeys}
          />
        </div>
      )}

      {missingTools.length > 0 && (
        <div className="mb-4 flex items-start gap-3 rounded-xl border border-amber-400/25 bg-amber-400/[0.07] p-3.5">
          <AlertTriangle size={17} className="mt-0.5 shrink-0 text-amber-400" />
          <div className="min-w-0 flex-1">
            <p className="text-[0.8rem] font-medium">
              Falta {missingTools.map((m) => m!.name).join(" y ")}
            </p>
            <p className="mt-1 text-[0.7rem] leading-relaxed text-[var(--color-muted)]">
              ROMForge Studio puede instalarlo por ti desde Ajustes → Herramientas.
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
        title="Suelta aquí tus .cci, .3ds, .cia, .cxi o .3dsx"
        hint="Los .3ds y los .cci son el mismo contenedor, así que se tratan igual."
        extensions={THREEDS_EXT}
        onPaths={handlePaths}
      />

      <div className="glass mt-4 flex items-center gap-3 rounded-2xl p-3">
        <label className="text-[0.72rem] font-medium text-[var(--color-muted)]">
          Carpeta de salida
        </label>
        <button
          className="btn btn-ghost ml-auto max-w-[60%] justify-start"
          onClick={async () => {
            const res = await open({ directory: true, multiple: false });
            if (res) await patchSettings({ output_dir: res as string });
          }}
        >
          <FolderInput size={15} className="shrink-0" />
          <span className="truncate">{settings.output_dir || "Junto al archivo original"}</span>
        </button>
      </div>

      {files.length > 0 ? (
        <>
          <div className="mb-2 mt-5 flex items-center justify-between">
            <h2 className="text-[0.8rem] font-semibold">
              {files.length} {files.length === 1 ? "archivo" : "archivos"}
            </h2>
            <button
              className="btn btn-quiet btn-danger px-2 py-1 text-xs"
              onClick={() => clearConsoleFiles("threeds")}
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
                      {f.op === "z3dscompress" && ` → .${z3dsExt(f.ext)}`}
                    </span>
                  </span>
                  <select
                    className="field w-[178px] shrink-0 py-1.5 text-xs"
                    value={f.op}
                    onChange={(e) => setConsoleOp("threeds", f.path, e.target.value)}
                  >
                    {opsFor(f.ext).map((o) => (
                      <option key={o.id} value={o.id} title={o.desc}>
                        {o.label}
                      </option>
                    ))}
                  </select>
                  <button
                    onClick={() => removeConsoleFile("threeds", f.path)}
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
          <h2 className="mb-2 text-[0.8rem] font-semibold">Qué puede hacer</h2>
          <div className="grid grid-cols-3 gap-2">
            {THREEDS_OPS.map((o) => (
              <div key={o.id} className="glass rounded-xl p-3">
                <p className="flex flex-wrap items-center gap-1.5 text-[0.78rem] font-semibold">
                  {o.label}
                  {o.shrinks && (
                    <span className="chip border-emerald-400/25 bg-emerald-400/10 text-emerald-300">
                      ahorra
                    </span>
                  )}
                  {o.needs && <span className="chip">claves</span>}
                </p>
                <p className="mt-1 text-[0.68rem] leading-snug text-[var(--color-muted)]">{o.desc}</p>
              </div>
            ))}
          </div>
          <p className="mt-3 text-[0.68rem] leading-relaxed text-[var(--color-faint)]">
            Z3DS usa zstd «seekable»: prioriza descomprimir rápido y poder saltar a cualquier punto
            del archivo, para que el emulador lo lea sin extraerlo antes. Lo admite{" "}
            <button
              className="underline"
              onClick={() => openUrl("https://github.com/azahar-emu/azahar")}
            >
              Azahar
            </button>{" "}
            desde la versión 2123. La compresión es de ida: para volver atrás, el emulador ya lee el
            archivo comprimido directamente.
          </p>
        </div>
      )}
    </div>
  );
}

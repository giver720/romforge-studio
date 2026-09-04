import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { AnimatePresence, motion } from "framer-motion";
import { AlertTriangle, FolderInput, Package2, Play, Trash2, X } from "lucide-react";
import { useEffect, useState } from "react";
import { api } from "../lib/api";
import { bytes } from "../lib/format";
import { SWITCH_EXT, SWITCH_OPS, opsFor } from "../lib/switch";
import { useStore } from "../store";
import type { KeysStatus } from "../lib/types";
import { DropZone } from "./DropZone";
import { KeyPicker } from "./KeyPicker";

export function SwitchView({ dragging }: { dragging: boolean }) {
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
  const files = consoleFiles.switch;
  const [keys, setKeys] = useState<KeysStatus | null>(null);

  useEffect(() => {
    api.switchKeysStatus().then(setKeys);
    refreshTools();
  }, []);

  const nsz = tools.find((t) => t.id === "nsz");
  const nxci = tools.find((t) => t.id === "4nxci");
  const needed = new Set(files.map((f) => SWITCH_OPS.find((o) => o.id === f.op)?.tool));
  const missing = [nsz, nxci].filter((t) => t && needed.has(t.id) && !t.found);

  async function handlePaths(paths: string[]) {
    const infos = await api.inspectPaths(paths);
    const ok = infos.filter((i) => SWITCH_EXT.includes(i.ext));
    if (!ok.length) {
      notify("warn", "Ahí no hay archivos NSP, NSZ, XCI ni XCZ");
      return;
    }
    addConsoleFiles("switch", ok);
  }

  async function run() {
    if (!files.length) return;
    await api.addJobs(
      files.map((f) => ({ input: f.path, mode: f.op, system: "switch" })),
    );
    clearConsoleFiles("switch");
    await refreshJobs();
    notify("ok", `${files.length} ${files.length === 1 ? "trabajo encolado" : "trabajos encolados"}`);
  }

  return (
    <div className="scroll flex-1 p-5">
      <div className="mb-4 flex items-end justify-between gap-4">
        <div>
          <h1 className="text-lg font-semibold tracking-tight">
            Nintendo <span className="accent-text">Switch</span>
          </h1>
          <p className="mt-0.5 text-xs text-[var(--color-muted)]">
            Comprime NSP y XCI a NSZ/XCZ, vuelve atrás sin perder nada, y convierte cartuchos a
            instalables.
          </p>
        </div>
        {files.length > 0 && (
          <button className="btn btn-primary" onClick={run} disabled={!!missing.length || !keys?.found}>
            <Play size={15} /> Procesar {files.length}
          </button>
        )}
      </div>

      {keys && (
        <div className="mb-4">
          <KeyPicker
            label={keys.found ? "prod.keys localizadas" : "Faltan tus prod.keys"}
            file="prod.keys"
            found={keys.found}
            path={keys.path}
            hint="Estas herramientas necesitan las claves de tu propia consola para leer el contenido. ROMForge Studio no las incluye ni te ayuda a obtenerlas: tienes que volcarlas tú desde tu Switch."
            fallback={keys.expected}
            settingKey="switch_keys_path"
            onChange={() => api.switchKeysStatus().then(setKeys)}
          />
        </div>
      )}

      {missing.length > 0 && (
        <div className="mb-4 flex items-start gap-3 rounded-xl border border-amber-400/25 bg-amber-400/[0.07] p-3.5">
          <AlertTriangle size={17} className="mt-0.5 shrink-0 text-amber-400" />
          <div className="min-w-0 flex-1">
            <p className="text-[0.8rem] font-medium">
              Falta {missing.map((m) => m!.name).join(" y ")}
            </p>
            <p className="mt-1 text-[0.7rem] leading-relaxed text-[var(--color-muted)]">
              ROMForge Studio puede instalarlo por ti desde Ajustes → Herramientas.
            </p>
          </div>
          <button className="btn btn-ghost shrink-0" onClick={() => useStore.setState({ view: "settings" })}>
            <Package2 size={15} /> Instalar
          </button>
        </div>
      )}

      <DropZone
        dragging={dragging}
        compact={files.length > 0}
        title="Suelta aquí tus NSP, NSZ, XCI o XCZ"
        hint="Cada archivo lleva su propia conversión; puedes mezclar comprimir y descomprimir en el mismo lote."
        extensions={SWITCH_EXT}
        onPaths={handlePaths}
      />

      <div className="glass mt-4 flex flex-wrap items-center gap-4 rounded-2xl p-3">
        <div className="min-w-[190px] flex-1">
          <label className="mb-1 block text-[0.68rem] font-medium text-[var(--color-muted)]">
            Nivel de compresión:{" "}
            <span className="text-[var(--color-ink)]">{settings.nsz_level}</span>
            {settings.nsz_level >= 20 && (
              <span className="ml-1 text-[var(--color-faint)]">· muy lento</span>
            )}
          </label>
          <input
            type="range"
            min={1}
            max={22}
            value={settings.nsz_level}
            onChange={(e) => patchSettings({ nsz_level: Number(e.target.value) })}
            className="w-full accent-[var(--accent)]"
          />
        </div>
        <div className="min-w-[190px] flex-1">
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

      {files.length > 0 && (
        <>
          <div className="mb-2 mt-5 flex items-center justify-between">
            <h2 className="text-[0.8rem] font-semibold">
              {files.length} {files.length === 1 ? "archivo" : "archivos"}
            </h2>
            <button
              className="btn btn-quiet btn-danger px-2 py-1 text-xs"
              onClick={() => clearConsoleFiles("switch")}
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
                    className="field w-[168px] shrink-0 py-1.5 text-xs"
                    value={f.op}
                    onChange={(e) => setConsoleOp("switch", f.path, e.target.value)}
                  >
                    {opsFor(f.ext).map((o) => (
                      <option key={o.id} value={o.id} title={o.desc}>
                        {o.label}
                      </option>
                    ))}
                  </select>
                  <button
                    onClick={() => removeConsoleFile("switch", f.path)}
                    className="btn btn-quiet btn-danger px-2 py-1 opacity-0 group-hover:opacity-100"
                  >
                    <X size={14} />
                  </button>
                </motion.li>
              ))}
            </AnimatePresence>
          </ul>
        </>
      )}

      {files.length === 0 && (
        <div className="mt-6">
          <h2 className="mb-2 text-[0.8rem] font-semibold">Qué puede hacer</h2>
          <div className="grid grid-cols-2 gap-2">
            {SWITCH_OPS.map((o) => (
              <div key={o.id} className="glass rounded-xl p-3">
                <p className="flex items-center gap-2 text-[0.78rem] font-semibold">
                  {o.label}
                  {o.shrinks && (
                    <span className="chip border-emerald-400/25 bg-emerald-400/10 text-emerald-300">
                      ahorra
                    </span>
                  )}
                </p>
                <p className="mt-1 text-[0.68rem] leading-snug text-[var(--color-muted)]">{o.desc}</p>
              </div>
            ))}
          </div>
          <p className="mt-3 text-[0.68rem] leading-relaxed text-[var(--color-faint)]">
            NSZ y XCZ son formatos abiertos creados por{" "}
            <button className="underline" onClick={() => openUrl("https://github.com/nicoboss/nsz")}>
              nicoboss/nsz
            </button>
            . La compresión no toca el contenido: al descomprimir vuelves al archivo original idéntico.
          </p>
        </div>
      )}
    </div>
  );
}

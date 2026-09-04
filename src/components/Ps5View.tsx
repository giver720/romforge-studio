import { open } from "@tauri-apps/plugin-dialog";
import {
  AlertTriangle,
  CheckCircle2,
  FileArchive,
  FolderOpen,
  Gamepad2,
  HardDrive,
  Package2,
  ShieldCheck,
} from "lucide-react";
import { useEffect, useState } from "react";
import { api } from "../lib/api";
import { bytes } from "../lib/format";
import { IS_WINDOWS } from "../lib/platform";
import type { Ps5Scan } from "../lib/ps5";
import { useStore } from "../store";

export function Ps5View() {
  const { notify, refreshJobs, tools, refreshTools } = useStore();
  const [source, setSource] = useState<string | null>(null);
  const [scan, setScan] = useState<Ps5Scan | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    refreshTools();
  }, []);

  const support = tools.find((tool) => tool.id === "ps5exfat");
  const missing = support && !support.found;

  async function chooseFolder() {
    const result = (await open({ directory: true, multiple: false })) as string | null;
    if (!result) return;
    setBusy(true);
    try {
      const info = await api.ps5Scan(result);
      setSource(result);
      setScan(info);
      if (!info.valid) notify("error", info.error ?? "La carpeta no parece un dump de PS5");
    } catch (error) {
      notify("error", String(error));
    } finally {
      setBusy(false);
    }
  }

  async function enqueue() {
    if (!source || !scan?.valid) return;
    setBusy(true);
    try {
      await api.addJobs([{ input: source, mode: "ps5exfat", system: "ps5" }]);
      await refreshJobs();
      notify("ok", "Imagen exFAT de PS5 añadida a la cola");
      setSource(null);
      setScan(null);
    } catch (error) {
      notify("error", String(error));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="scroll flex-1 p-5">
      <div className="mb-4">
        <h1 className="text-lg font-semibold tracking-tight">
          PlayStation <span className="accent-text">5</span>
        </h1>
        <p className="mt-0.5 text-xs text-[var(--color-muted)]">
          Empaqueta un dump en carpeta como una imagen <span className="mono">.exfat</span> montable.
        </p>
      </div>

      <section className="glass rounded-2xl p-5">
        <div className="flex items-start gap-3">
          <span className="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-blue-400/10 text-blue-300">
            <FileArchive size={21} />
          </span>
          <div>
            <p className="text-[0.9rem] font-semibold">Carpeta de juego → un solo archivo exFAT</p>
            <p className="mt-1 max-w-2xl text-[0.7rem] leading-relaxed text-[var(--color-muted)]">
              Coloca los archivos del juego directamente en la raíz de la imagen, sin una carpeta
              adicional. Usa clústeres de 64 KiB para mantener buen rendimiento al montarla.
            </p>
          </div>
        </div>

        <div className="mt-4 grid gap-2 text-[0.7rem] text-[var(--color-muted)] sm:grid-cols-3">
          <p className="flex items-center gap-2"><Gamepad2 size={13} className="text-blue-300" /> Compatible con ShadowMountPlus</p>
          <p className="flex items-center gap-2"><ShieldCheck size={13} className="text-emerald-400" /> Verifica todos los archivos</p>
          <p className="flex items-center gap-2"><HardDrive size={13} className="text-violet-300" /> Imagen exFAT estándar</p>
        </div>

        <div className="mt-4 rounded-xl border border-sky-400/20 bg-sky-400/[0.06] p-3 text-[0.68rem] leading-relaxed text-[var(--color-muted)]">
          Esto empaqueta la carpeta en un archivo; no comprime los datos. La imagen será un poco
          mayor que el contenido original porque incluye el sistema de archivos y espacio de seguridad.
        </div>

        {missing && (
          <div className="mt-3 flex items-start gap-2 rounded-xl border border-amber-400/25 bg-amber-400/[0.07] p-3">
            <AlertTriangle size={15} className="mt-0.5 shrink-0 text-amber-400" />
            <div className="min-w-0 flex-1 text-[0.68rem] text-[var(--color-muted)]">
              {IS_WINDOWS
                ? "Hace falta OSFMount. Después abre ROMForge Studio como administrador para crear y formatear la unidad virtual."
                : "Hace falta exfatprogs, exfat-fuse y fuse3. En Ubuntu: sudo apt install exfatprogs exfat-fuse fuse3"}
            </div>
            <button className="btn btn-ghost shrink-0 px-2 py-1 text-xs" onClick={() => useStore.setState({ view: "settings" })}>
              <Package2 size={13} /> Configurar
            </button>
          </div>
        )}

        {scan && (
          <div className={`mt-4 rounded-xl border p-3 ${scan.valid ? "border-emerald-400/20 bg-emerald-400/[0.05]" : "border-rose-400/25 bg-rose-400/[0.06]"}`}>
            {scan.valid ? (
              <div className="flex flex-wrap items-center gap-x-5 gap-y-2 text-[0.7rem]">
                <CheckCircle2 size={15} className="text-emerald-400" />
                <span className="font-medium">{scan.title ?? scan.title_id ?? "Dump de PS5 válido"}</span>
                {scan.title_id && <span className="chip">{scan.title_id}</span>}
                {scan.version && <span className="text-[var(--color-faint)]">v{scan.version}</span>}
                <span className="text-[var(--color-muted)]">{scan.file_count.toLocaleString()} archivos · {bytes(scan.raw_bytes)}</span>
                <span className="text-[var(--color-muted)]">Imagen estimada: {bytes(scan.image_bytes)}</span>
              </div>
            ) : (
              <p className="flex items-center gap-2 text-[0.7rem] text-rose-300"><AlertTriangle size={15} /> {scan.error}</p>
            )}
          </div>
        )}

        <div className="mt-5 flex flex-wrap gap-2">
          <button className="btn btn-ghost" onClick={chooseFolder} disabled={busy}>
            <FolderOpen size={15} /> Seleccionar carpeta del juego
          </button>
          <button className="btn btn-primary" onClick={enqueue} disabled={busy || !!missing || !scan?.valid}>
            <FileArchive size={15} /> Crear .exfat
          </button>
        </div>
      </section>

      <div className="mt-3 flex items-start gap-3 rounded-xl border border-amber-400/20 bg-amber-400/[0.05] p-3.5">
        <AlertTriangle size={16} className="mt-0.5 shrink-0 text-amber-400" />
        <p className="text-[0.7rem] leading-relaxed text-[var(--color-muted)]">
          Solo para dumps propios. La carpeta elegida debe ser la raíz real del juego: debe contener
          <span className="mono"> eboot.bin</span> y <span className="mono">sce_sys/param.json</span> directamente.
          Una PS5 sin modificar no puede montar este archivo.
        </p>
      </div>
    </div>
  );
}

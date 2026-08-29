import { open } from "@tauri-apps/plugin-dialog";
import {
  AlertTriangle,
  CheckCircle2,
  Disc3,
  FolderOpen,
  Gamepad2,
  HardDrive,
  Package2,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { useEffect, useState } from "react";
import { api } from "../lib/api";
import { IS_WINDOWS } from "../lib/platform";
import { useStore } from "../store";
import { Toggle } from "./ui";

type InputKind = "iso" | "folder";

export function Ps3View() {
  const { notify, refreshJobs, settings, patchSettings, tools, refreshTools } = useStore();
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    refreshTools();
  }, []);

  const ps3iso = tools.find((tool) => tool.id === "ps3iso");
  const missing = ps3iso && !ps3iso.found;

  async function choose(kind: InputKind): Promise<string | null> {
    const result = await open(
      kind === "folder"
        ? { directory: true, multiple: false }
        : { multiple: false, filters: [{ name: "ISO de PS3", extensions: ["iso"] }] },
    );
    return (result as string | null) ?? null;
  }

  async function enqueue(mode: "ps3compact" | "ps3rpcs3", kind: InputKind) {
    const input = await choose(kind);
    if (!input) return;
    setBusy(true);
    try {
      if (kind === "folder") {
        const scan = await api.ps3Scan(input);
        if (!scan.valid) {
          notify("error", "La carpeta no parece un juego de PS3 extraído");
          return;
        }
      }
      await api.addJobs([{ input, mode, system: "ps3" }]);
      await refreshJobs();
      notify(
        "ok",
        mode === "ps3compact"
          ? "ISO compacto añadido a la cola; no se eliminará ningún archivo"
          : "Compresión transparente para RPCS3 añadida a la cola",
      );
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
          PlayStation <span className="accent-text">3</span>
        </h1>
        <p className="mt-0.5 text-xs text-[var(--color-muted)]">
          Reduce el espacio sin borrar idiomas, vídeos ni archivos del juego.
        </p>
      </div>

      <div className="grid gap-3 xl:grid-cols-2">
        <section className="glass flex flex-col rounded-2xl p-5">
          <div className="flex items-start gap-3">
            <span className="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-blue-400/10 text-blue-300">
              <Disc3 size={21} />
            </span>
            <div>
              <p className="text-[0.9rem] font-semibold">PS3 real + RPCS3</p>
              <p className="mt-1 text-[0.7rem] leading-relaxed text-[var(--color-muted)]">
                Reconstruye un ISO estándar sin el relleno físico del disco. Conserva todos los
                archivos y sigue siendo montable con CFW/HEN, Cobra y webMAN, además de RPCS3.
              </p>
            </div>
          </div>

          <div className="mt-4 space-y-2 text-[0.7rem] text-[var(--color-muted)]">
            <p className="flex items-center gap-2">
              <CheckCircle2 size={13} className="text-emerald-400" /> No elimina contenido
            </p>
            <p className="flex items-center gap-2">
              <ShieldCheck size={13} className="text-emerald-400" /> Reabre el ISO y compara el
              inventario completo
            </p>
            <p className="flex items-center gap-2">
              <Gamepad2 size={13} className="text-emerald-400" /> Salida ISO normal o fragmentos
              FAT32
            </p>
          </div>

          {missing && (
            <div className="mt-4 flex items-start gap-2 rounded-xl border border-amber-400/25 bg-amber-400/[0.07] p-3">
              <AlertTriangle size={15} className="mt-0.5 shrink-0 text-amber-400" />
              <div className="min-w-0 flex-1 text-[0.68rem] text-[var(--color-muted)]">
                Hace falta ps3iso-utils para extraer, reconstruir y verificar.
              </div>
              <button
                className="btn btn-ghost shrink-0 px-2 py-1 text-xs"
                onClick={() => useStore.setState({ view: "settings" })}
              >
                <Package2 size={13} /> Instalar
              </button>
            </div>
          )}

          <div className="mt-auto pt-5">
            <Toggle
              checked={settings.ps3_split_fat32}
              onChange={(value) => patchSettings({ ps3_split_fat32: value })}
              label="Crear fragmentos de 4 GB para FAT32"
              hint="Actívalo solo si llevarás el juego a la PS3 en una unidad FAT32."
            />
            <div className="mt-3 flex flex-wrap gap-2">
              <button
                className="btn btn-primary"
                onClick={() => enqueue("ps3compact", "iso")}
                disabled={busy || !!missing}
              >
                <Disc3 size={15} /> Optimizar ISO
              </button>
              <button
                className="btn btn-ghost"
                onClick={() => enqueue("ps3compact", "folder")}
                disabled={busy || !!missing}
              >
                <FolderOpen size={15} /> Desde carpeta
              </button>
            </div>
          </div>
        </section>

        <section className="glass flex flex-col rounded-2xl p-5">
          <div className="flex items-start gap-3">
            <span className="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-violet-400/10 text-violet-300">
              <Sparkles size={21} />
            </span>
            <div>
              <p className="text-[0.9rem] font-semibold">Máxima reducción para RPCS3</p>
              <p className="mt-1 text-[0.7rem] leading-relaxed text-[var(--color-muted)]">
                Comprime el ISO descifrado o la carpeta desde el sistema de archivos. RPCS3 lo ve
                en su formato normal y lo descomprime al leer, sin conversión adicional.
              </p>
            </div>
          </div>

          <div className="mt-4 space-y-2 text-[0.7rem] text-[var(--color-muted)]">
            <p className="flex items-center gap-2">
              <HardDrive size={13} className="text-violet-300" />
              {IS_WINDOWS ? "NTFS con compresión LZX" : "Btrfs con compresión zstd"}
            </p>
            <p className="flex items-center gap-2">
              <CheckCircle2 size={13} className="text-emerald-400" /> El archivo lógico no cambia
            </p>
            <p className="flex items-center gap-2">
              <CheckCircle2 size={13} className="text-emerald-400" /> Funciona con ISO descifrado o
              juego en carpeta
            </p>
          </div>

          <div className="mt-4 rounded-xl border border-sky-400/20 bg-sky-400/[0.06] p-3 text-[0.68rem] leading-relaxed text-[var(--color-muted)]">
            Este perfil es para el disco del PC. Si copias el resultado a un USB, la compresión del
            sistema de archivos no viaja con él. Para una PS3 real usa el perfil universal.
          </div>

          {!IS_WINDOWS && (
            <div className="mt-3 rounded-xl border border-amber-400/20 bg-amber-400/[0.06] p-3 text-[0.68rem] leading-relaxed text-[var(--color-muted)]">
              En Linux la unidad debe estar formateada como Btrfs y tener instalada la herramienta
              <span className="mono"> btrfs</span>.
            </div>
          )}

          <div className="mt-auto flex flex-wrap gap-2 pt-5">
            <button
              className="btn btn-primary"
              onClick={() => enqueue("ps3rpcs3", "iso")}
              disabled={busy}
            >
              <Disc3 size={15} /> Comprimir ISO
            </button>
            <button
              className="btn btn-ghost"
              onClick={() => enqueue("ps3rpcs3", "folder")}
              disabled={busy}
            >
              <FolderOpen size={15} /> Comprimir carpeta
            </button>
          </div>
        </section>
      </div>

      <div className="mt-3 flex items-start gap-3 rounded-xl border border-amber-400/20 bg-amber-400/[0.05] p-3.5">
        <AlertTriangle size={16} className="mt-0.5 shrink-0 text-amber-400" />
        <p className="text-[0.7rem] leading-relaxed text-[var(--color-muted)]">
          RPCS3 necesita un ISO descifrado. Una PS3 real necesita CFW o HEN con soporte Cobra para
          montar ISOs; una consola de fábrica no puede hacerlo.
        </p>
      </div>
    </div>
  );
}

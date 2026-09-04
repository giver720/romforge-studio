import { open } from "@tauri-apps/plugin-dialog";
import {
  AlertTriangle,
  ArchiveRestore,
  CheckCircle2,
  FileArchive,
  FolderOpen,
  Gauge,
  Gamepad2,
  HardDrive,
  Package2,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { api, type GameArtwork } from "../lib/api";
import { bytes } from "../lib/format";
import type { Ps5Scan } from "../lib/ps5";
import { useStore } from "../store";

type BuildMode = "ps5ffpkg" | "ps5exfat" | "ps5ffpfsc";

const formats: {
  mode: BuildMode;
  name: string;
  badge: string;
  description: string;
  tool: "mkpfs" | "ufs2tool";
  icon: typeof HardDrive;
}[] = [
  {
    mode: "ps5ffpkg",
    name: "FFPKG · UFS2",
    badge: "Recomendado",
    description: "Máximo rendimiento y formato preferido por ShadowMountPlus. En Windows muestra el permiso del sistema.",
    tool: "ufs2tool",
    icon: Gauge,
  },
  {
    mode: "ps5exfat",
    name: "exFAT · 64 KiB",
    badge: "Compatibilidad",
    description: "Para títulos que necesitan comportarse como contenido de una unidad externa.",
    tool: "mkpfs",
    icon: HardDrive,
  },
  {
    mode: "ps5ffpfsc",
    name: "FFPFSC · comprimido",
    badge: "Menor tamaño",
    description: "Contenedor comprimido; puede perder rendimiento en juegos que cargan datos sin parar.",
    tool: "mkpfs",
    icon: FileArchive,
  },
];

export function Ps5View() {
  const { notify, refreshJobs, tools, refreshTools } = useStore();
  const [source, setSource] = useState<string | null>(null);
  const [imageSource, setImageSource] = useState<string | null>(null);
  const [scan, setScan] = useState<Ps5Scan | null>(null);
  const [artwork, setArtwork] = useState<GameArtwork | null>(null);
  const [mode, setMode] = useState<BuildMode>("ps5ffpkg");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    refreshTools();
  }, []);

  const missingTools = useMemo(
    () => new Set(tools.filter((tool) => tool.family === "ps5" && !tool.found).map((tool) => tool.id)),
    [tools],
  );
  const selectedFormat = formats.find((format) => format.mode === mode)!;
  const selectedToolMissing = missingTools.has(selectedFormat.tool);
  const imageExt = imageSource?.split(".").pop()?.toLowerCase();
  const canCompress = imageExt === "exfat" || imageExt === "ffpkg";

  async function chooseFolder() {
    const result = (await open({ directory: true, multiple: false })) as string | null;
    if (!result) return;
    setBusy(true);
    setArtwork(null);
    try {
      const [info, cover] = await Promise.all([
        api.ps5Scan(result),
        api.gameArtwork(result, "ps5").catch(() => null),
      ]);
      setSource(result);
      setScan(info);
      setArtwork(cover);
      if (!info.valid) notify("error", info.error ?? "La carpeta no parece un dump de PS5");
    } catch (error) {
      notify("error", String(error));
    } finally {
      setBusy(false);
    }
  }

  async function chooseImage() {
    const result = (await open({
      directory: false,
      multiple: false,
      filters: [{ name: "Imágenes PS5", extensions: ["exfat", "ffpkg", "ffpfs", "ffpfsc"] }],
    })) as string | null;
    if (result) setImageSource(result);
  }

  async function enqueue(input: string, selectedMode: string, message: string) {
    setBusy(true);
    try {
      await api.addJobs([{ input, mode: selectedMode, system: "ps5" }]);
      await refreshJobs();
      notify("ok", message);
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
          Construye, comprime y recupera imágenes compatibles con ShadowMountPlus.
        </p>
      </div>

      <section className="glass rounded-2xl p-5">
        <div className="flex items-start gap-3">
          <span className="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-blue-400/10 text-blue-300">
            <Gamepad2 size={21} />
          </span>
          <div>
            <p className="text-[0.9rem] font-semibold">Carpeta de juego → imagen para PS5</p>
            <p className="mt-1 max-w-2xl text-[0.7rem] leading-relaxed text-[var(--color-muted)]">
              Elige la raíz real del dump. ROMForge lee sus metadatos, recomienda el formato y
              verifica el resultado antes de publicarlo.
            </p>
          </div>
        </div>

        <div className="mt-4 grid gap-2 lg:grid-cols-3">
          {formats.map((format) => {
            const Icon = format.icon;
            const selected = mode === format.mode;
            return (
              <button
                key={format.mode}
                onClick={() => setMode(format.mode)}
                className={`rounded-xl border p-3 text-left transition-colors ${
                  selected
                    ? "border-[var(--accent)] bg-[var(--accent-soft)]"
                    : "border-[var(--color-edge)] bg-white/[0.02] hover:bg-white/[0.05]"
                }`}
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="flex items-center gap-2 text-[0.76rem] font-semibold">
                    <Icon size={15} className="text-blue-300" /> {format.name}
                  </span>
                  <span className="chip text-[0.58rem]">{format.badge}</span>
                </div>
                <p className="mt-2 text-[0.65rem] leading-relaxed text-[var(--color-muted)]">
                  {format.description}
                </p>
              </button>
            );
          })}
        </div>

        {scan && (
          <div
            className={`mt-4 rounded-xl border p-3 ${
              scan.valid
                ? "border-emerald-400/20 bg-emerald-400/[0.05]"
                : "border-rose-400/25 bg-rose-400/[0.06]"
            }`}
          >
            {scan.valid ? (
              <div className="flex gap-3">
                {artwork?.data_url ? (
                  <img
                    src={artwork.data_url}
                    alt="Portada del juego"
                    className="h-20 w-16 shrink-0 rounded-lg border border-white/10 object-cover"
                  />
                ) : (
                  <span className="grid h-20 w-16 shrink-0 place-items-center rounded-lg border border-white/10 bg-blue-400/10 text-blue-300">
                    <Gamepad2 size={22} />
                  </span>
                )}
                <div className="min-w-0 flex-1 text-[0.7rem]">
                  <div className="flex flex-wrap items-center gap-2">
                    <CheckCircle2 size={15} className="text-emerald-400" />
                    <span className="font-medium">{scan.title ?? scan.title_id ?? "Dump de PS5 válido"}</span>
                    {scan.title_id && <span className="chip">{scan.title_id}</span>}
                    {scan.version && <span className="text-[var(--color-faint)]">v{scan.version}</span>}
                  </div>
                  <div className="mt-2 flex flex-wrap gap-x-5 gap-y-1 text-[var(--color-muted)]">
                    <span>{scan.file_count.toLocaleString()} archivos · {bytes(scan.raw_bytes)}</span>
                    <span>exFAT estimado: {bytes(scan.image_bytes)}</span>
                    <span className="text-violet-300">
                      FFPFSC estimado: {bytes(scan.compressed_estimate_bytes)} · ahorro ~{scan.estimated_savings_percent.toFixed(0)}%
                    </span>
                  </div>
                  <p className="mt-2 flex items-center gap-1.5 text-blue-300">
                    <Sparkles size={12} /> Recomendado para rendimiento: FFPKG · UFS2
                  </p>
                  {scan.warnings.map((warning) => (
                    <p key={warning} className="mt-1 text-amber-300">{warning}</p>
                  ))}
                </div>
              </div>
            ) : (
              <p className="flex items-center gap-2 text-[0.7rem] text-rose-300">
                <AlertTriangle size={15} /> {scan.error}
              </p>
            )}
          </div>
        )}

        {selectedToolMissing && (
          <div className="mt-3 flex items-start gap-2 rounded-xl border border-amber-400/25 bg-amber-400/[0.07] p-3">
            <AlertTriangle size={15} className="mt-0.5 shrink-0 text-amber-400" />
            <div className="min-w-0 flex-1 text-[0.68rem] text-[var(--color-muted)]">
              Falta {selectedFormat.tool === "mkpfs" ? "MkPFS" : "UFS2Tool"}. ROMForge puede descargarlo desde su proyecto oficial.
            </div>
            <button
              className="btn btn-ghost shrink-0 px-2 py-1 text-xs"
              onClick={() => useStore.setState({ view: "settings" })}
            >
              <Package2 size={13} /> Instalar
            </button>
          </div>
        )}

        <div className="mt-5 flex flex-wrap gap-2">
          <button className="btn btn-ghost" onClick={chooseFolder} disabled={busy}>
            <FolderOpen size={15} /> Seleccionar carpeta
          </button>
          <button
            className="btn btn-primary"
            onClick={() => source && enqueue(source, mode, `${selectedFormat.name} añadido a la cola`)}
            disabled={busy || selectedToolMissing || !source || !scan?.valid}
          >
            <FileArchive size={15} /> Crear {mode === "ps5ffpkg" ? ".ffpkg" : mode === "ps5exfat" ? ".exfat" : ".ffpfsc"}
          </button>
        </div>
        {mode === "ps5ffpkg" && (
          <p className="mt-2 text-[0.64rem] text-amber-300">
            Windows pedirá permiso de administrador al crear y verificar FFPKG; exFAT y FFPFSC no lo necesitan.
          </p>
        )}
      </section>

      <section className="glass mt-4 rounded-2xl p-5">
        <div className="flex items-start gap-3">
          <span className="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-violet-400/10 text-violet-300">
            <ArchiveRestore size={20} />
          </span>
          <div className="min-w-0 flex-1">
            <p className="text-[0.9rem] font-semibold">Comprimir o recuperar una imagen</p>
            <p className="mt-1 text-[0.68rem] leading-relaxed text-[var(--color-muted)]">
              Convierte una imagen exFAT/FFPKG a FFPFSC o extrae exFAT, FFPKG, FFPFS y FFPFSC de vuelta a una carpeta.
            </p>
            {imageSource && <p className="mono mt-2 truncate text-[0.65rem] text-blue-300">{imageSource}</p>}
          </div>
        </div>
        <div className="mt-4 flex flex-wrap gap-2">
          <button className="btn btn-ghost" onClick={chooseImage} disabled={busy}>
            <FileArchive size={15} /> Elegir imagen
          </button>
          <button
            className="btn btn-primary"
            disabled={busy || !imageSource || !canCompress || missingTools.has("mkpfs")}
            onClick={() => imageSource && enqueue(imageSource, "ps5compress", "Compresión FFPFSC añadida a la cola")}
          >
            <Sparkles size={15} /> Comprimir a .ffpfsc
          </button>
          <button
            className="btn btn-ghost"
            disabled={busy || !imageSource || (imageExt === "ffpkg" ? missingTools.has("ufs2tool") : missingTools.has("mkpfs"))}
            onClick={() => imageSource && enqueue(imageSource, "ps5extract", "Extracción añadida a la cola")}
          >
            <ArchiveRestore size={15} /> Extraer a carpeta
          </button>
        </div>
      </section>

      <div className="mt-3 flex items-start gap-3 rounded-xl border border-amber-400/20 bg-amber-400/[0.05] p-3.5">
        <ShieldCheck size={16} className="mt-0.5 shrink-0 text-amber-400" />
        <p className="text-[0.7rem] leading-relaxed text-[var(--color-muted)]">
          Solo para dumps propios. La raíz debe contener <span className="mono">eboot.bin</span> y
          <span className="mono"> sce_sys/param.json</span>. FFPFSC ahorra espacio, pero puede causar
          tirones en juegos que transmiten muchos datos. Una PS5 sin modificar no puede montar estas imágenes.
        </p>
      </div>
    </div>
  );
}

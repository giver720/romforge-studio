import { open } from "@tauri-apps/plugin-dialog";
import {
  AlertTriangle,
  ArchiveRestore,
  CheckCircle2,
  FileArchive,
  FolderInput,
  FlaskConical,
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
import {
  PS5_LAB_FORMATS,
  type Ps5ImageSpacePreflight,
  type Ps5OutputSpace,
  type Ps5Scan,
} from "../lib/ps5";
import { useStore } from "../store";
import { Toggle } from "./ui";

type BuildMode = "ps5ffpkg" | "ps5exfat" | "ps5ffpfsc";
type LibraryMode = BuildMode | "ps5fpkg" | "ps5lz4";

const libraryFormats: { mode: LibraryMode; label: string; tool: "mkpfs" | "ufs2tool" | "prospero" | "ampr" }[] = [
  { mode: "ps5ffpkg", label: "FFPKG · UFS2", tool: "ufs2tool" },
  { mode: "ps5exfat", label: "exFAT · 64 KiB", tool: "mkpfs" },
  { mode: "ps5ffpfsc", label: "FFPFSC · comprimido", tool: "mkpfs" },
  { mode: "ps5fpkg", label: "FPKG nativo", tool: "prospero" },
  { mode: "ps5lz4", label: "AMPR/LZ4", tool: "ampr" },
];

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

const GIB = 1024 * 1024 * 1024;

function withPs5WorkingMargin(payload: number) {
  return payload + Math.max(payload / 10, GIB);
}

function estimatedPs5WorkingSpace(mode: string, scan: Ps5Scan) {
  if (mode === "ps5exfat") return withPs5WorkingMargin(scan.image_bytes);
  if (mode === "ps5ffpkg") return withPs5WorkingMargin(scan.image_bytes + scan.raw_bytes);
  if (mode === "ps5ffpfsc") {
    return withPs5WorkingMargin(scan.compressed_estimate_bytes + scan.raw_bytes);
  }
  if (mode === "ps5fpkg") return withPs5WorkingMargin(scan.raw_bytes);
  if (mode === "ps5lz4") {
    return withPs5WorkingMargin(scan.raw_bytes + scan.compressed_estimate_bytes);
  }
  return 0;
}

function normalizeInternalPath(value: string) {
  const trimmed = value.trim();
  if (!trimmed) return "";
  const normalized = trimmed.replace(/\\/g, "/");
  const withoutLeadingSlash = normalized.startsWith("/") ? normalized.slice(1) : normalized;
  const relative = withoutLeadingSlash.endsWith("/")
    ? withoutLeadingSlash.slice(0, -1)
    : withoutLeadingSlash;
  if (
    !relative ||
    normalized.length > 1024 ||
    normalized.includes(":") ||
    normalized.includes("\0") ||
    relative.split("/").some((segment) => !segment || segment === "." || segment === "..")
  ) {
    return null;
  }
  return `/${relative}`;
}

export function Ps5View() {
  const { notify, refreshJobs, jobs, tools, refreshTools, settings, patchSettings } = useStore();
  const [source, setSource] = useState<string | null>(null);
  const [imageSource, setImageSource] = useState<string | null>(null);
  const [scan, setScan] = useState<Ps5Scan | null>(null);
  const [artwork, setArtwork] = useState<GameArtwork | null>(null);
  const [mode, setMode] = useState<BuildMode>("ps5ffpkg");
  const [libraryMode, setLibraryMode] = useState<LibraryMode>("ps5ffpkg");
  const [busy, setBusy] = useState(false);
  const [outputLocationError, setOutputLocationError] = useState<string | null>(null);
  const [checkingOutputLocation, setCheckingOutputLocation] = useState(false);
  const [scannedFpkgSubfolder, setScannedFpkgSubfolder] = useState<string | null>(null);
  const [checkingFpkgReadiness, setCheckingFpkgReadiness] = useState(false);
  const [outputSpace, setOutputSpace] = useState<Ps5OutputSpace | null>(null);
  const [outputSpaceError, setOutputSpaceError] = useState<string | null>(null);
  const [checkingOutputSpace, setCheckingOutputSpace] = useState(false);
  const [outputSpaceKey, setOutputSpaceKey] = useState<string | null>(null);
  const [imageSpace, setImageSpace] = useState<Ps5ImageSpacePreflight | null>(null);
  const [imageSpaceError, setImageSpaceError] = useState<string | null>(null);
  const [checkingImageSpace, setCheckingImageSpace] = useState(false);
  const [imageSpaceKey, setImageSpaceKey] = useState<string | null>(null);
  const [extractPath, setExtractPath] = useState("");

  useEffect(() => {
    refreshTools();
  }, []);

  const missingTools = useMemo(
    () => new Set(tools.filter((tool) => tool.family === "ps5" && !tool.found).map((tool) => tool.id)),
    [tools],
  );
  const selectedFormat = formats.find((format) => format.mode === mode)!;
  const selectedToolMissing = missingTools.has(selectedFormat.tool);
  const selectedLibraryFormat = libraryFormats.find((format) => format.mode === libraryMode)!;
  const libraryToolMissing = missingTools.has(selectedLibraryFormat.tool);
  const selectedFormatIncompatible = Boolean(
    mode === "ps5ffpfsc" && scan?.valid && !scan.pfs_compatible,
  );
  const prosperoMissing = missingTools.has("prospero");
  const amprMissing = missingTools.has("ampr");
  const imageExt = imageSource?.split(".").pop()?.toLowerCase();
  const canCompress = imageExt === "exfat" || imageExt === "ffpkg";
  const canInspectImage = ["exfat", "ffpkg", "ffpfs", "ffpfsc"].includes(imageExt ?? "");
  const normalizedExtractPath = normalizeInternalPath(extractPath);
  const selectiveExtract = imageExt === "ffpkg" && Boolean(normalizedExtractPath);
  const extractPathInvalid = imageExt === "ffpkg" && normalizedExtractPath === null;
  const effectiveOutputSetting = settings.ps5_output_dir || settings.output_dir || "";
  const currentOutputSpaceKey = source ? `${source}\u0000${effectiveOutputSetting}` : null;
  const currentImageSpaceKey = imageSource ? `${imageSource}\u0000${effectiveOutputSetting}` : null;
  const outputSpaceStale = Boolean(currentOutputSpaceKey && outputSpaceKey !== currentOutputSpaceKey);
  const imageSpaceStale = Boolean(currentImageSpaceKey && imageSpaceKey !== currentImageSpaceKey);
  const decryptedSubfolderTrimmed = settings.ps5_fpkg_decrypted_subfolder.trim();
  const decryptedSubfolderValid =
    decryptedSubfolderTrimmed.length > 0 &&
    decryptedSubfolderTrimmed.length <= 120 &&
    !decryptedSubfolderTrimmed.includes(":") &&
    !decryptedSubfolderTrimmed
      .split(/[\\/]/)
      .some((segment) => segment.length === 0 || segment === "." || segment === "..");
  const fpkgReadinessStale = Boolean(
    source &&
    scan?.valid &&
    decryptedSubfolderValid &&
    scannedFpkgSubfolder !== decryptedSubfolderTrimmed,
  );
  const requiredSpace = scan?.valid ? estimatedPs5WorkingSpace(mode, scan) : 0;
  const selectedSpaceInsufficient = Boolean(
    !outputSpaceStale && requiredSpace && outputSpace && outputSpace.available_bytes < requiredSpace,
  );

  function spaceInsufficientFor(targetMode: string) {
    return Boolean(
      scan?.valid &&
      !outputSpaceStale &&
      outputSpace &&
      outputSpace.available_bytes < estimatedPs5WorkingSpace(targetMode, scan),
    );
  }

  function imageSpaceInsufficientFor(targetMode: string) {
    const required = imageSpace?.required_bytes[targetMode];
    return Boolean(!imageSpaceStale && required && imageSpace && imageSpace.available_bytes < required);
  }

  async function chooseFolder() {
    const result = (await open({ directory: true, multiple: false })) as string | null;
    if (!result) return;
    setBusy(true);
    setArtwork(null);
    try {
      const [info, cover] = await Promise.all([
        api.ps5Scan(result, decryptedSubfolderTrimmed),
        api.gameArtwork(result, "ps5").catch(() => null),
      ]);
      setSource(result);
      setScan(info);
      setScannedFpkgSubfolder(decryptedSubfolderTrimmed);
      setArtwork(cover);
      if (!info.valid) notify("error", info.error ?? "La carpeta no parece un dump de PS5");
    } catch (error) {
      notify("error", String(error));
    } finally {
      setBusy(false);
    }
  }

  async function enqueueLibrary() {
    const result = (await open({
      directory: true,
      multiple: false,
      title: "Selecciona una carpeta con juegos de PS5",
    })) as string | null;
    if (!result) return;

    setBusy(true);
    try {
      const discovered = await api.ps5DiscoverGames(result);
      if (!discovered.length) {
        notify(
          "warn",
          "No se encontraron dumps PS5 en esa carpeta. Cada juego debe contener eboot.bin y sce_sys/param.json.",
        );
        return;
      }
      const active = new Set(
        jobs
          .filter((job) => job.mode === libraryMode && ["queued", "running"].includes(job.status))
          .map((job) => job.input.toLocaleLowerCase()),
      );
      const fresh = discovered.filter((path) => !active.has(path.toLocaleLowerCase()));
      if (!fresh.length) {
        notify("warn", "Todos los juegos detectados ya están activos en la cola.");
        return;
      }
      let libraryOptions: Record<string, string> = {};
      if (libraryMode === "ps5fpkg") {
        libraryOptions = {
          decrypted_subfolder: decryptedSubfolderTrimmed,
          embedded_right: String(settings.ps5_fpkg_embedded_right),
        };
      } else if (libraryMode === "ps5lz4") {
        libraryOptions = { profile: settings.ps5_lz4_profile };
      }
      await api.addJobs(fresh.map((input) => ({
        input,
        mode: libraryMode,
        system: "ps5",
        output_dir: settings.ps5_output_dir,
        options: libraryOptions,
      })));
      await refreshJobs();
      const skipped = discovered.length - fresh.length;
      notify(
        skipped ? "warn" : "ok",
        `${fresh.length} ${fresh.length === 1 ? "juego añadido" : "juegos añadidos"} a la cola${skipped ? ` · ${skipped} duplicados omitidos` : ""}.`,
      );
    } catch (error) {
      notify("error", String(error));
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    if (
      !source ||
      !scan?.valid ||
      !decryptedSubfolderValid ||
      scannedFpkgSubfolder === decryptedSubfolderTrimmed
    ) {
      setCheckingFpkgReadiness(false);
      return;
    }
    let active = true;
    setCheckingFpkgReadiness(true);
    const timer = window.setTimeout(() => {
      api.ps5FpkgReadiness(source, decryptedSubfolderTrimmed)
        .then((readiness) => {
          if (!active) return;
          setScan((current) => current ? {
            ...current,
            fpkg_ready: readiness.ready,
            fpkg_module_count: readiness.module_count,
            fpkg_blockers: readiness.blockers,
          } : current);
          setScannedFpkgSubfolder(decryptedSubfolderTrimmed);
        })
        .catch((error) => {
          if (active) notify("error", String(error));
        })
        .finally(() => {
          if (active) setCheckingFpkgReadiness(false);
        });
    }, 300);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [source, scan?.valid, decryptedSubfolderTrimmed, decryptedSubfolderValid, scannedFpkgSubfolder]);

  useEffect(() => {
    const output = settings.ps5_output_dir || settings.output_dir;
    if (!source || !output) {
      setOutputLocationError(null);
      setCheckingOutputLocation(false);
      return;
    }
    let active = true;
    setCheckingOutputLocation(true);
    api.ps5ValidateOutputLocation(source, output)
      .then(() => {
        if (active) setOutputLocationError(null);
      })
      .catch((error) => {
        if (active) setOutputLocationError(String(error));
      })
      .finally(() => {
        if (active) setCheckingOutputLocation(false);
      });
    return () => {
      active = false;
    };
  }, [source, settings.ps5_output_dir, settings.output_dir]);

  useEffect(() => {
    if (!source || !scan?.valid) {
      setOutputSpace(null);
      setOutputSpaceError(null);
      setCheckingOutputSpace(false);
      setOutputSpaceKey(null);
      return;
    }
    let active = true;
    setOutputSpace(null);
    setOutputSpaceError(null);
    setCheckingOutputSpace(true);
    setOutputSpaceKey(null);
    const requestKey = currentOutputSpaceKey;
    api.ps5OutputSpace(source, settings.ps5_output_dir)
      .then((space) => {
        if (active) {
          setOutputSpace(space);
          setOutputSpaceKey(requestKey);
        }
      })
      .catch((error) => {
        if (active) {
          setOutputSpaceError(String(error));
          setOutputSpaceKey(requestKey);
        }
      })
      .finally(() => {
        if (active) setCheckingOutputSpace(false);
      });
    return () => {
      active = false;
    };
  }, [source, scan?.valid, effectiveOutputSetting]);

  async function chooseImage() {
    const result = (await open({
      directory: false,
      multiple: false,
      filters: [{ name: "Imágenes PS5", extensions: ["exfat", "ffpkg", "ffpfs", "ffpfsc"] }],
    })) as string | null;
    if (result) {
      setImageSource(result);
      setExtractPath("");
    }
  }

  useEffect(() => {
    if (!imageSource || !canInspectImage) {
      setImageSpace(null);
      setImageSpaceError(null);
      setCheckingImageSpace(false);
      setImageSpaceKey(null);
      return;
    }
    let active = true;
    setImageSpace(null);
    setImageSpaceError(null);
    setCheckingImageSpace(true);
    setImageSpaceKey(null);
    const requestKey = currentImageSpaceKey;
    api.ps5ImageSpace(imageSource, settings.ps5_output_dir)
      .then((space) => {
        if (active) {
          setImageSpace(space);
          setImageSpaceKey(requestKey);
        }
      })
      .catch((error) => {
        if (active) {
          setImageSpaceError(String(error));
          setImageSpaceKey(requestKey);
        }
      })
      .finally(() => {
        if (active) setCheckingImageSpace(false);
      });
    return () => {
      active = false;
    };
  }, [imageSource, canInspectImage, effectiveOutputSetting]);

  async function enqueue(
    input: string,
    selectedMode: string,
    message: string,
    options: Record<string, string> = {},
  ) {
    setBusy(true);
    try {
      await api.addJobs([{
        input,
        mode: selectedMode,
        system: "ps5",
        options,
        output_dir: settings.ps5_output_dir,
      }]);
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
            const incompatible = Boolean(
              format.mode === "ps5ffpfsc" && scan?.valid && !scan.pfs_compatible,
            );
            return (
              <button
                key={format.mode}
                onClick={() => {
                  setMode(format.mode);
                  setLibraryMode(format.mode);
                }}
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
                {incompatible && (
                  <p className="mt-2 text-[0.61rem] leading-relaxed text-rose-300">
                    {scan?.pfs_blockers[0]}
                  </p>
                )}
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
                    <span className="text-blue-300">
                      Espacio de trabajo recomendado: {bytes(estimatedPs5WorkingSpace(mode, scan))}
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
            disabled={busy || checkingOutputLocation || checkingOutputSpace || outputSpaceStale || Boolean(outputLocationError) || selectedSpaceInsufficient || selectedFormatIncompatible || selectedToolMissing || !source || !scan?.valid}
            title={selectedFormatIncompatible ? scan?.pfs_blockers[0] : undefined}
          >
            <FileArchive size={15} /> Crear {mode === "ps5ffpkg" ? ".ffpkg" : mode === "ps5exfat" ? ".exfat" : ".ffpfsc"}
          </button>
        </div>
        <div className="mt-3 flex flex-wrap items-end gap-2 rounded-xl border border-white/10 bg-white/[0.02] p-3">
          <label className="min-w-[210px] text-[0.62rem] text-[var(--color-muted)]">
            Formato para la biblioteca
            <select
              className="field mt-1 w-full"
              value={libraryMode}
              onChange={(event) => setLibraryMode(event.target.value as LibraryMode)}
              disabled={busy}
            >
              {libraryFormats.map((format) => (
                <option key={format.mode} value={format.mode}>{format.label}</option>
              ))}
            </select>
          </label>
          <button
            className="btn btn-ghost"
            onClick={enqueueLibrary}
            disabled={busy || libraryToolMissing || (libraryMode === "ps5fpkg" && !decryptedSubfolderValid)}
            title={
              libraryToolMissing
                ? `Falta ${selectedLibraryFormat.label}`
                : "Detecta juegos en las subcarpetas inmediatas y los añade a la cola"
            }
          >
            <FolderInput size={15} /> Añadir biblioteca
          </button>
          <p className="min-w-[240px] flex-1 text-[0.62rem] leading-relaxed text-[var(--color-faint)]">
            Busca cada dump en las subcarpetas inmediatas, omite duplicados activos y aplica
            {` ${selectedLibraryFormat.label}`} a todos. Cada juego se valida antes de convertirlo.
          </p>
        </div>
        <div className="mt-3 flex items-center gap-3 rounded-xl border border-white/10 bg-white/[0.02] p-3">
          <FolderInput size={16} className="shrink-0 text-violet-300" />
          <span className="text-[0.68rem] text-[var(--color-muted)]">Carpeta de salida PS5</span>
          <button
            className="btn btn-quiet ml-auto min-w-0 max-w-[65%] justify-start"
            onClick={async () => {
              const result = await open({ directory: true, multiple: false });
              if (result) await patchSettings({ ps5_output_dir: result as string });
            }}
            disabled={busy}
            title="Las conversiones PS5 se guardarán en esta carpeta"
          >
            <span className="truncate">
              {settings.ps5_output_dir || settings.output_dir || "Junto a la entrada"}
            </span>
          </button>
          {settings.ps5_output_dir && (
            <button
              className="btn btn-quiet shrink-0 px-2"
              onClick={() => patchSettings({ ps5_output_dir: null })}
              disabled={busy}
              title="Volver a la salida general"
            >
              Restablecer
            </button>
          )}
        </div>
        <p className="mt-2 text-[0.62rem] leading-relaxed text-[var(--color-faint)]">
          {settings.ps5_output_dir
            ? "Esta ruta solo se aplica a PS5. En exFAT → FPKG, la extracción temporal también se crea aquí."
            : settings.output_dir
              ? "Usando la salida general. Elige otra carpeta para asignar una ruta exclusiva a PS5."
              : "En exFAT → FPKG, la extracción temporal se crea junto a la imagen para no llenar la unidad del sistema."}
        </p>
        {outputLocationError && (
          <p className="mt-2 flex items-start gap-1.5 text-[0.64rem] leading-relaxed text-rose-300">
            <AlertTriangle size={13} className="mt-0.5 shrink-0" /> {outputLocationError}
          </p>
        )}
        {scan?.valid && (
          <div
            className={`mt-2 flex items-start gap-2 rounded-lg border px-3 py-2 text-[0.64rem] ${
              selectedSpaceInsufficient
                ? "border-rose-400/25 bg-rose-400/[0.06] text-rose-300"
                : "border-white/10 bg-white/[0.02] text-[var(--color-muted)]"
            }`}
          >
            <HardDrive size={13} className="mt-0.5 shrink-0" />
            <span className="min-w-0">
              {checkingOutputSpace || outputSpaceStale
                ? "Consultando el espacio libre del destino…"
                : outputSpace
                  ? `${bytes(outputSpace.available_bytes)} libres · ${bytes(requiredSpace)} necesarios para ${selectedFormat.name}`
                  : outputSpaceError ?? "No se pudo consultar el espacio libre; la cola volverá a comprobarlo."}
              {outputSpace && !outputSpaceStale && (
                <span className="mt-0.5 block truncate text-[var(--color-faint)]" title={outputSpace.location}>
                  Comprobado en {outputSpace.location}
                </span>
              )}
            </span>
          </div>
        )}
        {mode === "ps5ffpkg" && (
          <p className="mt-2 text-[0.64rem] text-amber-300">
            Windows pedirá permiso de administrador al crear y verificar FFPKG; exFAT y FFPFSC no lo necesitan.
          </p>
        )}
      </section>

      <section className="glass mt-4 rounded-2xl p-5">
        <div className="flex items-start gap-3">
          <span className="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-fuchsia-400/10 text-fuchsia-300">
            <FlaskConical size={20} />
          </span>
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <p className="text-[0.9rem] font-semibold">Laboratorio PS5</p>
              <span className="chip text-[0.58rem]">Formatos recientes</span>
            </div>
            <p className="mt-1 max-w-3xl text-[0.68rem] leading-relaxed text-[var(--color-muted)]">
              FPKG nativo usa LibProsperoPKG. AMPR/LZ4 crea paquetes reversibles con el encoder
              público y ambos resultados se validan antes de publicarse.
            </p>
          </div>
        </div>

        <div
          className={`mt-4 flex items-center gap-2 rounded-xl border p-3 text-[0.68rem] ${
            scan?.valid
              ? "border-emerald-400/20 bg-emerald-400/[0.05] text-emerald-300"
              : "border-white/10 bg-white/[0.02] text-[var(--color-muted)]"
          }`}
        >
          {scan?.valid ? <CheckCircle2 size={15} /> : <FolderOpen size={15} />}
          <span className="min-w-0 truncate">
            {scan?.valid
              ? `Dump preparado: ${scan.title ?? scan.title_id ?? source}`
              : "Selecciona y valida un dump en el apartado superior para preparar estos formatos."}
          </span>
        </div>

        <div className="mt-3 grid gap-3 lg:grid-cols-2">
          {PS5_LAB_FORMATS.map((format) => (
            <article
              key={format.id}
              className="rounded-xl border border-[var(--color-edge)] bg-white/[0.02] p-4"
            >
              <div className="flex flex-wrap items-center justify-between gap-2">
                <p className="text-[0.78rem] font-semibold">{format.name}</p>
                <span className="chip text-[0.58rem]">{format.badge}</span>
              </div>
              <p className="mt-2 text-[0.66rem] leading-relaxed text-[var(--color-muted)]">
                {format.description}
              </p>
              {format.id === "fpkg" && scan?.valid && (
                <p className={`mt-2 text-[0.64rem] ${scan.fpkg_ready && !checkingFpkgReadiness && !fpkgReadinessStale ? "text-emerald-300" : "text-amber-300"}`}>
                  {checkingFpkgReadiness || fpkgReadinessStale
                    ? "Actualizando la preparación FPKG…"
                    : scan.fpkg_ready
                    ? `Dump listo · ${scan.fpkg_module_count} módulo${scan.fpkg_module_count === 1 ? "" : "s"} · ${scan.content_id}`
                    : "El dump necesita preparación antes de crear el FPKG."}
                </p>
              )}
              <ul className="mt-3 space-y-1.5 text-[0.63rem] text-[var(--color-muted)]">
                {(format.id === "fpkg" && scan?.valid && scan.fpkg_blockers.length > 0
                  ? scan.fpkg_blockers
                  : format.requirements
                ).map((requirement) => (
                  <li key={requirement} className="flex items-start gap-2">
                    <span
                      className={`mt-1 h-1.5 w-1.5 shrink-0 rounded-full ${
                        format.id === "fpkg" && scan?.valid && !scan.fpkg_ready
                          ? "bg-amber-300"
                          : "bg-fuchsia-300/70"
                      }`}
                    />
                    {requirement}
                  </li>
                ))}
              </ul>
              {format.id === "fpkg" && (
                <div className="mt-3 space-y-2 rounded-xl border border-white/10 bg-black/10 p-2.5">
                  <label className="block text-[0.62rem] text-[var(--color-muted)]">
                    Subcarpeta de módulos descifrados
                    <input
                      className="field mt-1 w-full"
                      value={settings.ps5_fpkg_decrypted_subfolder}
                      onChange={(event) => patchSettings({ ps5_fpkg_decrypted_subfolder: event.target.value })}
                      placeholder="decrypted"
                      maxLength={120}
                    />
                    {!decryptedSubfolderValid && (
                      <span className="mt-1 block text-[0.6rem] text-rose-300">
                        Usa una ruta relativa como decrypted o decrypted/modules, sin “..”.
                      </span>
                    )}
                  </label>
                  <Toggle
                    checked={settings.ps5_fpkg_embedded_right}
                    onChange={(value) => patchSettings({ ps5_fpkg_embedded_right: value })}
                    label="Usar right.sprx integrado"
                    hint="Actívalo solo para dumps que requieran el módulo incluido por LibProsperoPKG."
                  />
                </div>
              )}
              {scan?.valid && (
                <p className={`mt-2 text-[0.62rem] ${spaceInsufficientFor(format.mode) ? "text-rose-300" : "text-blue-300"}`}>
                  {checkingOutputSpace || outputSpaceStale
                    ? "Consultando espacio libre…"
                    : outputSpace
                      ? `${bytes(outputSpace.available_bytes)} libres · ${bytes(estimatedPs5WorkingSpace(format.mode, scan))} necesarios`
                      : `Espacio de trabajo recomendado: ${bytes(estimatedPs5WorkingSpace(format.mode, scan))}`}
                </p>
              )}
              {format.id === "lz4" && (
                <div className="mt-3 rounded-xl border border-white/10 bg-black/10 p-2.5">
                  <p className="text-[0.62rem] text-[var(--color-muted)]">Perfil de compresión LZ4</p>
                  <div className="mt-2 grid grid-cols-3 gap-1">
                    {([
                      ["fast", "Rápido"],
                      ["balanced", "Equilibrado"],
                      ["maximum", "Máximo"],
                    ] as const).map(([value, label]) => (
                      <button
                        key={value}
                        className={`rounded-lg border px-2 py-1.5 text-[0.61rem] transition-colors ${
                          settings.ps5_lz4_profile === value
                            ? "border-[var(--accent)] bg-[var(--accent-soft)] text-white"
                            : "border-white/10 text-[var(--color-muted)] hover:bg-white/5"
                        }`}
                        onClick={() => patchSettings({ ps5_lz4_profile: value })}
                        type="button"
                      >
                        {label}
                      </button>
                    ))}
                  </div>
                  <p className="mt-2 text-[0.59rem] leading-relaxed text-[var(--color-faint)]">
                    {settings.ps5_lz4_profile === "fast"
                      ? "Prioriza velocidad y usa menos CPU; el resultado puede ocupar más."
                      : settings.ps5_lz4_profile === "maximum"
                        ? "Busca el menor tamaño con LZ4 HC nivel 12; tardará más."
                        : "Recomendado: buen equilibrio entre tiempo, CPU y tamaño."}
                  </p>
                </div>
              )}
              <button
                className={`btn mt-4 w-full justify-center ${
                  format.id === "fpkg" ? "btn-primary" : "btn-ghost"
                }`}
                disabled={
                  busy ||
                  checkingOutputLocation ||
                  checkingOutputSpace ||
                  outputSpaceStale ||
                  Boolean(outputLocationError) ||
                  spaceInsufficientFor(format.mode) ||
                  (format.id === "fpkg" ? prosperoMissing : amprMissing) ||
                  !source ||
                  !scan?.valid ||
                  (format.id === "fpkg" && !decryptedSubfolderValid) ||
                  (format.id === "fpkg" && (checkingFpkgReadiness || fpkgReadinessStale)) ||
                  (format.id === "fpkg" && !scan.fpkg_ready)
                }
                title={
                  format.id === "fpkg"
                    ? scan?.fpkg_blockers[0] ?? (prosperoMissing ? "Falta LibProsperoPKG" : "Crear FPKG PS5")
                    : amprMissing ? "Falta el motor AMPR/LZ4 incluido" : "Crear una copia AMPRPAK4/LZ4"
                }
                onClick={() =>
                  source && enqueue(
                    source,
                    format.mode,
                    format.id === "fpkg" ? "FPKG nativo de PS5 añadido a la cola" : "Copia AMPR/LZ4 añadida a la cola",
                    format.id === "fpkg"
                      ? { decrypted_subfolder: decryptedSubfolderTrimmed, embedded_right: String(settings.ps5_fpkg_embedded_right) }
                      : { profile: settings.ps5_lz4_profile },
                  )
                }
              >
                <Package2 size={14} />
                {(format.id === "fpkg" ? prosperoMissing : amprMissing) ? "Motor no disponible" : format.actionLabel}
              </button>
            </article>
          ))}
        </div>

        <p className="mt-3 flex items-start gap-2 text-[0.64rem] leading-relaxed text-amber-300">
          <ShieldCheck size={14} className="mt-0.5 shrink-0" />
          FPKG exige módulos descifrados y se somete a una segunda validación estructural en la cola.
          La ejecución final también depende del firmware y los payloads instalados en la PS5.
        </p>
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
        {imageExt === "exfat" && (
          <div className="mt-3 grid gap-2 rounded-xl border border-white/10 bg-white/[0.02] p-3 sm:grid-cols-3">
            {[
              { label: "MkPFS 1.0", ready: !missingTools.has("mkpfs") },
              { label: "Motor FPKG", ready: !prosperoMissing },
              { label: "Destino", ready: Boolean(settings.ps5_output_dir || settings.output_dir), optional: true },
            ].map((requirement) => (
              <div
                key={requirement.label}
                className={`flex items-center gap-2 text-[0.64rem] ${
                  requirement.ready || requirement.optional ? "text-emerald-300" : "text-amber-300"
                }`}
              >
                {requirement.ready || requirement.optional
                  ? <CheckCircle2 size={13} />
                  : <AlertTriangle size={13} />}
                <span>
                  {requirement.label}: {requirement.ready
                    ? "listo"
                    : requirement.optional
                      ? "junto a la imagen"
                      : "falta instalar"}
                </span>
              </div>
            ))}
          </div>
        )}
        {imageExt === "ffpkg" && (
          <div className="mt-3 rounded-xl border border-violet-400/20 bg-violet-400/[0.04] p-3">
            <div className="flex flex-wrap items-end gap-2">
              <label className="min-w-[240px] flex-1 text-[0.64rem] text-[var(--color-muted)]">
                Ruta interna opcional
                <input
                  className="field mt-1 w-full"
                  value={extractPath}
                  onChange={(event) => setExtractPath(event.target.value)}
                  placeholder="Vacía para extraer todo · /sce_sys"
                  spellCheck={false}
                />
              </label>
              {["", "/sce_sys", "/eboot.bin"].map((path) => (
                <button
                  key={path || "all"}
                  type="button"
                  className="btn btn-ghost"
                  onClick={() => setExtractPath(path)}
                >
                  {path ? path.slice(1) : "Todo"}
                </button>
              ))}
            </div>
            {extractPathInvalid ? (
              <p className="mt-2 text-[0.61rem] text-rose-300">
                Usa una ruta interna segura; no se permiten segmentos “.”, “..”, rutas de Windows ni separadores dobles.
              </p>
            ) : (
              <p className="mt-2 text-[0.61rem] leading-relaxed text-[var(--color-faint)]">
                {selectiveExtract
                  ? `Solo se recuperará ${normalizedExtractPath}. El tamaño exacto lo determina UFS2Tool al extraer.`
                  : "La extracción completa conserva la verificación del dump. La selección sirve para recuperar archivos o carpetas concretas."}
                {" "}UFS2Tool puede fallar con archivos individuales mayores de 2 GiB.
              </p>
            )}
          </div>
        )}
        {imageSource && canInspectImage && (
          <div className="mt-3 rounded-xl border border-white/10 bg-white/[0.02] p-3">
            <div className="flex items-center gap-2 text-[0.64rem] text-[var(--color-muted)]">
              <HardDrive size={13} className="shrink-0" />
              {checkingImageSpace || imageSpaceStale
                ? "Calculando la recuperación y el espacio libre…"
                : imageSpace
                  ? `${bytes(imageSpace.available_bytes)} libres en el destino`
                  : imageSpaceError ?? "No se pudo consultar el espacio; la cola volverá a comprobarlo."}
            </div>
            {imageSpace && !imageSpaceStale && (
              <>
                <div className="mt-2 flex flex-wrap gap-2">
                  {([
                    ["ps5compress", "Comprimir"],
                    ["ps5extract", selectiveExtract ? "Extraer todo" : "Extraer"],
                    ["ps5fpkgexfat", "Crear FPKG"],
                  ] as const).map(([targetMode, label]) => {
                    const required = imageSpace.required_bytes[targetMode];
                    if (!required) return null;
                    const insufficient = imageSpace.available_bytes < required;
                    return (
                      <span
                        key={targetMode}
                        className={`rounded-md border px-2 py-1 text-[0.61rem] ${
                          insufficient
                            ? "border-rose-400/25 bg-rose-400/[0.06] text-rose-300"
                            : "border-emerald-400/20 bg-emerald-400/[0.05] text-emerald-300"
                        }`}
                      >
                        {label}: {bytes(required)} {insufficient ? "· insuficiente" : "· listo"}
                      </span>
                    );
                  })}
                </div>
                <p className="mt-1.5 truncate text-[0.58rem] text-[var(--color-faint)]" title={imageSpace.location}>
                  Comprobado en {imageSpace.location}
                </p>
              </>
            )}
          </div>
        )}
        <div className="mt-4 flex flex-wrap gap-2">
          <button className="btn btn-ghost" onClick={chooseImage} disabled={busy}>
            <FileArchive size={15} /> Elegir imagen
          </button>
          <button
            className="btn btn-primary"
            disabled={busy || checkingImageSpace || imageSpaceStale || imageSpaceInsufficientFor("ps5compress") || !imageSource || !canCompress || missingTools.has("mkpfs")}
            onClick={() => imageSource && enqueue(imageSource, "ps5compress", "Compresión FFPFSC añadida a la cola")}
          >
            <Sparkles size={15} /> Comprimir a .ffpfsc
          </button>
          <button
            className="btn btn-ghost"
            disabled={busy || extractPathInvalid || (!selectiveExtract && (checkingImageSpace || imageSpaceStale || imageSpaceInsufficientFor("ps5extract"))) || !imageSource || !canInspectImage || (imageExt === "ffpkg" ? missingTools.has("ufs2tool") : missingTools.has("mkpfs"))}
            onClick={() => imageSource && enqueue(
              imageSource,
              "ps5extract",
              selectiveExtract ? "Extracción selectiva añadida a la cola" : "Extracción añadida a la cola",
              selectiveExtract && normalizedExtractPath ? { extract_path: normalizedExtractPath } : {},
            )}
          >
            <ArchiveRestore size={15} /> {selectiveExtract ? "Extraer selección" : "Extraer a carpeta"}
          </button>
          <button
            className="btn btn-ghost"
            disabled={busy || !imageSource || !canInspectImage || (imageExt === "ffpkg" ? missingTools.has("ufs2tool") : missingTools.has("mkpfs"))}
            onClick={() => imageSource && enqueue(imageSource, "ps5verify", "Verificación de imagen añadida a la cola")}
            title="Comprueba la imagen en modo de solo lectura"
          >
            <ShieldCheck size={15} /> Verificar imagen
          </button>
          {imageExt === "exfat" && (
            <button
              className="btn btn-primary"
              disabled={busy || checkingImageSpace || imageSpaceStale || imageSpaceInsufficientFor("ps5fpkgexfat") || !imageSource || missingTools.has("mkpfs") || prosperoMissing || !decryptedSubfolderValid}
              title={
                missingTools.has("mkpfs")
                  ? "Falta MkPFS 1.0.0"
                  : prosperoMissing
                    ? "Falta el motor LibProsperoPKG"
                    : !decryptedSubfolderValid
                      ? "Corrige la subcarpeta de módulos descifrados"
                    : "Extraer, convertir y verificar el FPKG automáticamente"
              }
              onClick={() => imageSource && enqueue(
                imageSource,
                "ps5fpkgexfat",
                "Conversión exFAT → FPKG añadida a la cola",
                { decrypted_subfolder: decryptedSubfolderTrimmed, embedded_right: String(settings.ps5_fpkg_embedded_right) },
              )}
            >
              <Package2 size={15} /> Crear FPKG desde exFAT
            </button>
          )}
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

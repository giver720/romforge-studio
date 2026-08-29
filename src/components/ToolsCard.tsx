import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { CheckCircle2, Circle, Download, FolderSearch, Loader2, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import { api } from "../lib/api";
import { EXECUTABLE_FILTERS } from "../lib/platform";
import { useStore } from "../store";
import type { PythonStatus, ToolStatus } from "../lib/types";
import { Card } from "./ui";

const FAMILY_LABEL: Record<string, string> = {
  chd: "CHD",
  switch: "Nintendo Switch",
  "3ds": "Nintendo 3DS",
  ps3: "PlayStation 3",
  xbox360: "Xbox 360",
  psp: "PSP",
  wii: "Wii y GameCube",
};

const SOURCE_LABEL: Record<string, string> = {
  bundled: "incluida",
  manual: "ruta elegida por ti",
  app: "copia interna",
  venv: "entorno de Python de la app",
  tools: "descargada por la app",
  path: "encontrada en el PATH",
};

function kindLabel(t: ToolStatus): string {
  if (t.kind.type === "bundled") return "Viaja con CHD Studio";
  if (t.kind.type === "python") return `Paquete de Python · ${t.kind.package}`;
  if (t.kind.type === "external") return "Hay que instalarla aparte";
  if (t.kind.type === "web") return `Se descarga de ${t.kind.base.replace(/^https?:\/\//, "")}`;
  if (t.kind.type === "system") return t.kind.hint;
  if (t.kind.type === "pythonscript") return "Script de Python · lo prepara CHD Studio";
  if (t.kind.type === "source")
    return `No hay binario para Linux · CHD Studio la compila de ${t.kind.repo.replace(
      /^https?:\/\/(www\.)?github\.com\//,
      "",
    )}`;
  return `GitHub · ${t.kind.repo}`;
}

function ToolRow({ tool }: { tool: ToolStatus }) {
  const { installTool, installingTool, setTools, notify } = useStore();
  const busy = installingTool === tool.id;
  // Compilar lleva minutos y descargar segundos: conviene que el botón lo diga.
  const compila = tool.kind.type === "source";

  async function browse() {
    const res = await open({
      multiple: false,
      filters: EXECUTABLE_FILTERS,
    });
    if (!res) return;
    setTools(await api.setToolPath(tool.id, res as string));
    notify("ok", `${tool.name} configurada`);
  }

  return (
    <li className="flex items-start gap-3 rounded-xl border border-[var(--color-edge)] bg-white/[0.025] p-3">
      {tool.found ? (
        <CheckCircle2 size={16} className="mt-0.5 shrink-0 text-emerald-400" />
      ) : (
        <Circle size={16} className="mt-0.5 shrink-0 text-[var(--color-faint)]" />
      )}

      <div className="min-w-0 flex-1">
        <p className="flex items-center gap-2 text-[0.8rem] font-medium">
          {tool.name}
          <span className="chip">{tool.license}</span>
        </p>
        <p className="mt-0.5 text-[0.68rem] leading-snug text-[var(--color-muted)]">{tool.purpose}</p>
        {tool.found ? (
          <p className="selectable mono mt-1 truncate text-[0.64rem] text-[var(--color-faint)]" title={tool.path ?? ""}>
            {SOURCE_LABEL[tool.source ?? ""] ?? tool.source} · {tool.path}
          </p>
        ) : (
          <p className="mt-1 text-[0.64rem] text-[var(--color-faint)]">{kindLabel(tool)}</p>
        )}
      </div>

      <div className="flex shrink-0 gap-1">
        {!tool.found && tool.installable && (
          <button className="btn btn-ghost px-2.5 py-1 text-xs" onClick={() => installTool(tool.id)} disabled={busy}>
            {busy ? <Loader2 size={14} className="animate-spin" /> : <Download size={14} />}
            {busy ? (compila ? "Compilando…" : "Instalando…") : compila ? "Compilar" : "Instalar"}
          </button>
        )}
        {!tool.found && (
          <button className="btn btn-quiet px-2 py-1" title="Elegir del disco" onClick={browse}>
            <FolderSearch size={14} />
          </button>
        )}
      </div>
    </li>
  );
}

export function ToolsCard() {
  const { tools, refreshTools } = useStore();
  const [python, setPython] = useState<PythonStatus | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    refreshTools();
    api.pythonStatus().then(setPython);
  }, []);

  async function recheck() {
    setBusy(true);
    await refreshTools();
    setPython(await api.pythonStatus());
    setBusy(false);
  }

  const families = [...new Set(tools.map((t) => t.family))];
  const needsPython = tools.some(
    (t) => (t.kind.type === "python" || t.kind.type === "pythonscript") && !t.found,
  );

  return (
    <Card
      title="Herramientas"
      desc="Cada consola necesita su propio motor. CHD Studio los detecta e instala por ti."
      right={
        <button className="btn btn-quiet px-2 py-1" onClick={recheck} title="Volver a buscar">
          <RefreshCw size={15} className={busy ? "animate-spin" : ""} />
        </button>
      }
    >
      {needsPython && python && !python.found && (
        <div className="mb-3 flex items-start gap-3 rounded-xl border border-amber-400/25 bg-amber-400/[0.07] p-3">
          <div className="min-w-0 flex-1">
            <p className="text-[0.78rem] font-medium">Falta Python</p>
            <p className="mt-1 text-[0.68rem] leading-relaxed text-[var(--color-muted)]">
              Algunas herramientas son paquetes de Python. Instálalo una vez y CHD Studio se encarga
              del resto en un entorno propio, sin tocar el Python del sistema.
            </p>
          </div>
          <button className="btn btn-ghost shrink-0" onClick={() => openUrl("https://www.python.org/downloads/")}>
            <Download size={15} /> Obtener
          </button>
        </div>
      )}

      {python?.found && (
        <p className="mb-3 text-[0.66rem] text-[var(--color-faint)]">
          {python.version} · entorno de la app {python.venv_ready ? "listo" : "sin crear todavía"}
        </p>
      )}

      <div className="flex flex-col gap-3">
        {families.map((f) => (
          <div key={f}>
            <p className="mb-1.5 text-[0.66rem] font-semibold uppercase tracking-wider text-[var(--color-faint)]">
              {FAMILY_LABEL[f] ?? f}
            </p>
            <ul className="flex flex-col gap-1.5">
              {tools.filter((t) => t.family === f).map((t) => (
                <ToolRow key={t.id} tool={t} />
              ))}
            </ul>
          </div>
        ))}
      </div>
    </Card>
  );
}

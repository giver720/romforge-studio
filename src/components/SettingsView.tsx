import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  AlertTriangle,
  CheckCircle2,
  Download,
  FolderInput,
  FolderSearch,
  RefreshCw,
  Upload,
} from "lucide-react";
import { useState } from "react";
import { api } from "../lib/api";
import { EXECUTABLE_FILTERS, IS_WINDOWS, executableName } from "../lib/platform";
import { useStore } from "../store";
import { ToolsCard } from "./ToolsCard";
import { UpdateCard } from "./UpdateCard";
import { Card, Toggle } from "./ui";

const ACCENTS = [
  { id: "violet", color: "#a78bfa" },
  { id: "cyan", color: "#22d3ee" },
  { id: "emerald", color: "#34d399" },
  { id: "amber", color: "#f5a524" },
  { id: "rose", color: "#fb7185" },
];

const SOURCE_LABEL: Record<string, string> = {
  bundled: "incluido con CHD Studio",
  manual: "ruta elegida por ti",
  app: "copia interna de CHD Studio",
  path: "encontrado en el PATH",
  mame: "encontrado en una instalación de MAME",
};

export function SettingsView() {
  const { settings, patchSettings, chdman, refreshChdman, notify } = useStore();
  const [checking, setChecking] = useState(false);

  async function recheck() {
    setChecking(true);
    await refreshChdman();
    setChecking(false);
  }

  async function browseChdman() {
    const res = await open({
      multiple: false,
      filters: EXECUTABLE_FILTERS,
    });
    if (!res) return;
    await patchSettings({ chdman_path: res as string });
    await refreshChdman();
    notify("ok", "chdman configurado");
  }

  async function importChdman() {
    const res = await open({
      multiple: false,
      filters: EXECUTABLE_FILTERS,
    });
    if (!res) return;
    try {
      await api.installChdman(res as string);
      await refreshChdman();
      notify("ok", "chdman copiado dentro de CHD Studio");
    } catch (e) {
      notify("error", String(e));
    }
  }

  return (
    <div className="scroll flex-1 p-5">
      <h1 className="text-lg font-semibold tracking-tight">Ajustes</h1>
      <p className="mb-4 mt-0.5 text-xs text-[var(--color-muted)]">
        CHD Studio es la ventana; el trabajo pesado lo hace <span className="mono">chdman</span>, la
        herramienta oficial de MAME.
      </p>

      <div className="flex flex-col gap-3">
        <Card
          title="Motor chdman"
          desc="Se busca en la carpeta interna, en el PATH y en instalaciones de MAME."
          right={
            <button className="btn btn-quiet px-2 py-1" onClick={recheck} title="Volver a buscar">
              <RefreshCw size={15} className={checking ? "animate-spin" : ""} />
            </button>
          }
        >
          <div
            className="flex items-start gap-3 rounded-xl border p-3"
            style={{
              borderColor: chdman?.found ? "#34d39933" : "#f5a52433",
              background: chdman?.found ? "#34d3990d" : "#f5a5240d",
            }}
          >
            {chdman?.found ? (
              <CheckCircle2 size={18} className="mt-0.5 shrink-0 text-emerald-400" />
            ) : (
              <AlertTriangle size={18} className="mt-0.5 shrink-0 text-amber-400" />
            )}
            <div className="min-w-0 flex-1">
              {chdman?.found ? (
                <>
                  <p className="text-[0.8rem] font-medium">{chdman.version ?? "chdman"}</p>
                  <p className="selectable mono mt-1 truncate text-[0.68rem] text-[var(--color-muted)]">
                    {chdman.path}
                  </p>
                  <p className="mt-1 text-[0.68rem] text-[var(--color-faint)]">
                    {SOURCE_LABEL[chdman.source ?? ""] ?? ""}
                    {chdman.supports_zstd
                      ? " · admite zstd, así que el preset Máxima aprieta más"
                      : " · sin zstd; se usarán los códecs clásicos"}
                  </p>
                </>
              ) : (
                <>
                  <p className="text-[0.8rem] font-medium">No se encontró chdman</p>
                  <p className="mt-1 text-[0.7rem] leading-relaxed text-[var(--color-muted)]">
                    {IS_WINDOWS ? (
                      <>
                        Viene dentro del paquete de binarios de MAME. Descárgalo, descomprime{" "}
                        <span className="mono">chdman.exe</span> y tráelo aquí con «Importar».
                      </>
                    ) : (
                      <>
                        Instala el paquete del sistema con{" "}
                        <span className="mono">sudo apt install mame-tools</span> y vuelve a buscar.
                      </>
                    )}
                  </p>
                </>
              )}
            </div>
          </div>

          <div className="mt-3 flex flex-wrap gap-2">
            <button className="btn btn-ghost" onClick={importChdman}>
              <Upload size={15} /> Importar {executableName("chdman")}
            </button>
            <button className="btn btn-quiet" onClick={browseChdman}>
              <FolderSearch size={15} /> Usar uno del disco
            </button>
            <button
              className="btn btn-quiet"
              onClick={() => openUrl("https://github.com/mamedev/mame/releases/latest")}
            >
              <Download size={15} /> Descargar MAME
            </button>
          </div>
        </Card>

        <ToolsCard />

        <Card title="Salida" desc="Dónde se guardan los archivos generados.">
          <div className="flex items-center gap-2">
            <button
              className="btn btn-ghost min-w-0 flex-1 justify-start"
              onClick={async () => {
                const res = await open({ directory: true, multiple: false });
                if (res) await patchSettings({ output_dir: res as string });
              }}
            >
              <FolderInput size={15} className="shrink-0" />
              <span className="truncate">{settings.output_dir || "Junto al archivo original"}</span>
            </button>
            {settings.output_dir && (
              <button className="btn btn-quiet" onClick={() => patchSettings({ output_dir: null })}>
                Restablecer
              </button>
            )}
          </div>

          <div className="mt-2 flex flex-col">
            <Toggle
              checked={settings.overwrite}
              onChange={(v) => patchSettings({ overwrite: v })}
              label="Sobrescribir si ya existe"
              hint="Sin esto, chdman se niega a pisar un archivo que ya está ahí."
            />
            <Toggle
              checked={settings.verify_after}
              onChange={(v) => patchSettings({ verify_after: v })}
              label="Verificar después de convertir"
              hint="Comprueba el CHD recién creado. Tarda un poco más, pero da tranquilidad."
            />
            <Toggle
              checked={settings.delete_source}
              onChange={(v) => patchSettings({ delete_source: v })}
              label="Borrar el original al terminar"
              hint="Cuidado: elimina también las pistas .bin que acompañan a un .cue o .gdi."
            />
          </div>
        </Card>

        <Card title="Rendimiento" desc="chdman ya reparte el trabajo entre varios núcleos por su cuenta.">
          <label className="mb-1.5 block text-[0.7rem] font-medium text-[var(--color-muted)]">
            Conversiones a la vez: <span className="text-[var(--color-ink)]">{settings.parallel}</span>
          </label>
          <input
            type="range"
            min={1}
            max={6}
            value={settings.parallel}
            onChange={(e) => patchSettings({ parallel: Number(e.target.value) })}
            className="w-full accent-[var(--accent)]"
          />

          <label className="mb-1.5 mt-4 block text-[0.7rem] font-medium text-[var(--color-muted)]">
            Hilos por conversión:{" "}
            <span className="text-[var(--color-ink)]">
              {settings.threads === 0 ? "automático" : settings.threads}
            </span>
          </label>
          <input
            type="range"
            min={0}
            max={32}
            value={settings.threads}
            onChange={(e) => patchSettings({ threads: Number(e.target.value) })}
            className="w-full accent-[var(--accent)]"
          />
        </Card>

        <UpdateCard />

        <Card title="Apariencia" desc="Color de acento de la interfaz.">
          <div className="flex gap-2">
            {ACCENTS.map((a) => (
              <button
                key={a.id}
                onClick={() => patchSettings({ accent: a.id })}
                className="h-9 w-9 rounded-xl border-2 transition-transform hover:scale-110"
                style={{
                  background: a.color,
                  borderColor: settings.accent === a.id ? "#ffffff" : "transparent",
                }}
                aria-label={a.id}
              />
            ))}
          </div>
        </Card>
      </div>
    </div>
  );
}

import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { getVersion } from "@tauri-apps/api/app";
import { CheckCircle2, Download, Loader2, RefreshCw, Sparkles } from "lucide-react";
import { useEffect, useState } from "react";
import { api } from "../lib/api";
import { useStore } from "../store";
import { Card } from "./ui";

type Phase = "idle" | "checking" | "found" | "downloading" | "ready" | "error";

export function UpdateCard() {
  const { notify } = useStore();
  const [version, setVersion] = useState("");
  const [phase, setPhase] = useState<Phase>("idle");
  const [update, setUpdate] = useState<Update | null>(null);
  const [message, setMessage] = useState("");
  const [progress, setProgress] = useState(0);

  const [paths, setPaths] = useState<{
    portable: boolean;
    config_dir: string;
    can_update: boolean;
    update_hint: string | null;
  } | null>(null);

  // Instalada con .deb o .rpm no puede reemplazarse a sí misma: manda el gestor
  // de paquetes. Se comprueba igual, pero no se ofrece instalar.
  const autoinstala = paths?.can_update ?? true;

  useEffect(() => {
    getVersion().then(setVersion).catch(() => {});
    api.appPaths().then(setPaths).catch(() => {});
  }, []);

  async function look() {
    setPhase("checking");
    setMessage("");
    try {
      const found = await check();
      if (found) {
        setUpdate(found);
        setPhase("found");
      } else {
        setPhase("idle");
        setMessage("Ya tienes la última versión.");
      }
    } catch (e) {
      setPhase("error");
      // Con el repositorio privado, GitHub no deja descargar los archivos de
      // la release sin autenticación, así que este error es el esperado.
      setMessage(String(e));
    }
  }

  async function install() {
    if (!update) return;
    setPhase("downloading");
    let total = 0;
    let got = 0;
    try {
      await update.downloadAndInstall((ev) => {
        if (ev.event === "Started") total = ev.data.contentLength ?? 0;
        else if (ev.event === "Progress") {
          got += ev.data.chunkLength;
          if (total) setProgress((got / total) * 100);
        } else if (ev.event === "Finished") setProgress(100);
      });
      setPhase("ready");
    } catch (e) {
      setPhase("error");
      setMessage(String(e));
      notify("error", "No se pudo instalar la actualización");
    }
  }

  return (
    <Card
      title="Actualizaciones"
      desc={`Estás en la versión ${version || "…"}. Se comprueban contra las releases de GitHub.`}
      right={
        <button
          className="btn btn-quiet px-2 py-1"
          onClick={look}
          disabled={phase === "checking" || phase === "downloading"}
          title="Buscar ahora"
        >
          <RefreshCw size={15} className={phase === "checking" ? "animate-spin" : ""} />
        </button>
      }
    >
      {phase === "found" && update && (
        <div className="rounded-xl border border-[color:color-mix(in_srgb,var(--accent)_45%,transparent)] bg-[var(--accent-soft)] p-3">
          <p className="flex items-center gap-2 text-[0.8rem] font-medium">
            <Sparkles size={15} style={{ color: "var(--accent)" }} />
            Versión {update.version} disponible
          </p>
          {update.body && (
            <p className="selectable mt-1.5 max-h-24 overflow-auto text-[0.68rem] leading-relaxed text-[var(--color-muted)]">
              {update.body}
            </p>
          )}
          {autoinstala ? (
            <button className="btn btn-primary mt-2.5" onClick={install}>
              <Download size={15} /> Descargar e instalar
            </button>
          ) : (
            <p className="mt-2 text-[0.66rem] leading-relaxed text-[var(--color-faint)]">
              {paths?.update_hint}
            </p>
          )}
        </div>
      )}

      {phase === "downloading" && (
        <div>
          <div className="bar live">
            <i style={{ width: `${Math.max(3, progress)}%` }} />
          </div>
          <p className="mt-1.5 text-[0.68rem] text-[var(--color-muted)]">
            Descargando… {progress.toFixed(0)} %
          </p>
        </div>
      )}

      {phase === "ready" && (
        <div className="flex items-center gap-3 rounded-xl border border-emerald-400/25 bg-emerald-400/[0.07] p-3">
          <CheckCircle2 size={17} className="shrink-0 text-emerald-400" />
          <p className="flex-1 text-[0.78rem]">Instalada. Reinicia para usarla.</p>
          <button className="btn btn-ghost" onClick={() => relaunch()}>
            Reiniciar
          </button>
        </div>
      )}

      {phase === "checking" && (
        <p className="flex items-center gap-2 text-[0.72rem] text-[var(--color-muted)]">
          <Loader2 size={14} className="animate-spin" /> Buscando…
        </p>
      )}

      {phase === "error" && (
        <div className="rounded-xl border border-amber-400/25 bg-amber-400/[0.07] p-3">
          <p className="text-[0.78rem] font-medium">No se pudo comprobar</p>
          <p className="selectable mt-1 text-[0.68rem] leading-relaxed text-[var(--color-muted)]">
            {message}
          </p>
          <p className="mt-1.5 text-[0.66rem] leading-relaxed text-[var(--color-faint)]">
            Si el repositorio es privado, GitHub no permite descargar los archivos de una release sin
            autenticación. El actualizador empezará a funcionar en cuanto lo hagas público.
          </p>
        </div>
      )}

      {phase === "idle" && message && (
        <p className="flex items-center gap-2 text-[0.72rem] text-[var(--color-muted)]">
          <CheckCircle2 size={14} className="text-emerald-400" /> {message}
        </p>
      )}

      {paths && (
        <p className="mt-3 border-t border-[var(--color-edge)] pt-2.5 text-[0.66rem] text-[var(--color-faint)]">
          {paths.portable ? "Versión portable · " : ""}
          Tus datos están en{" "}
          <button
            className="selectable mono underline"
            onClick={() => api.reveal(paths.config_dir)}
            title="Abrir en el explorador"
          >
            {paths.config_dir}
          </button>
        </p>
      )}
    </Card>
  );
}

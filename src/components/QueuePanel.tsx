import { AnimatePresence, motion } from "framer-motion";
import {
  AlertCircle,
  CheckCircle2,
  FolderSearch,
  ListX,
  Loader2,
  RotateCcw,
  ShieldCheck,
  ShieldEllipsis,
  Trash2,
  X,
} from "lucide-react";
import { useEffect, useState } from "react";
import { api } from "../lib/api";
import { bytes, duration, savings } from "../lib/format";
import { MODE_LABELS, systemById } from "../lib/profiles";
import { useStore } from "../store";
import type { Job } from "../lib/types";
import { Empty } from "./ui";

function cleanGameName(name: string): string {
  return name
    .replace(/\.(cue|gdi|iso|chd|img|raw|nsp|nsz|xci|xcz|cia|cci|3ds|cxi|cso|zso|dax|rvz|wia|gcz|wbfs|exfat)$/i, "")
    .replace(/\.compact$/i, "")
    .trim();
}

function Artwork({
  job,
  color,
  onTitle,
}: {
  job: Job;
  color: string;
  onTitle: (title: string | null) => void;
}) {
  const [image, setImage] = useState<string | null>(null);
  const [loaded, setLoaded] = useState(false);
  const initials = systemById(job.system).name
    .split(/\s+/)
    .map((part) => part[0])
    .join("")
    .slice(0, 3)
    .toUpperCase();

  useEffect(() => {
    let active = true;
    setLoaded(false);
    api
      .gameArtwork(job.input, job.system)
      .then((artwork) => {
        if (active) {
          setImage(artwork.data_url);
          onTitle(artwork.title);
        }
      })
      .catch(() => {
        if (active) setImage(null);
      })
      .finally(() => {
        if (active) setLoaded(true);
      });
    return () => {
      active = false;
    };
  }, [job.input, job.system, onTitle]);

  return (
    <div
      className="relative h-[74px] w-[56px] shrink-0 overflow-hidden rounded-lg border border-white/10 shadow-lg"
      style={{ background: `linear-gradient(145deg, ${color}66, #111827 72%)` }}
    >
      {image ? (
        <img
          src={image}
          alt=""
          className="h-full w-full object-cover"
          onError={() => setImage(null)}
        />
      ) : (
        <div className="grid h-full place-items-center">
          <span className={`text-[0.7rem] font-black tracking-wider text-white/80 ${loaded ? "" : "animate-pulse"}`}>
            {initials}
          </span>
        </div>
      )}
      <span className="absolute inset-x-0 bottom-0 h-5 bg-gradient-to-t from-black/55 to-transparent" />
    </div>
  );
}

function JobRow({ job }: { job: Job }) {
  const { setJobs } = useStore();
  const sys = systemById(job.system);
  const running = job.status === "running";
  const done = job.status === "done";
  const failed = job.status === "error";
  const saved = done ? savings(job.input_size, job.output_size) : null;
  const elapsed =
    job.started_at && job.finished_at ? duration(job.finished_at - job.started_at) : null;
  const [detectedTitle, setDetectedTitle] = useState<string | null>(null);

  return (
    <motion.li
      layout
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, height: 0, marginBottom: 0, transition: { duration: 0.16 } }}
      transition={{ type: "spring", stiffness: 420, damping: 34 }}
      className="glass overflow-hidden rounded-xl p-3"
      style={running ? { borderColor: `${sys.color}66`, boxShadow: `0 12px 32px -24px ${sys.color}` } : undefined}
    >
      <div className="flex items-start gap-3">
        <Artwork job={job} color={sys.color} onTitle={setDetectedTitle} />

        <div className="min-w-0 flex-1 pt-0.5">
          <div className="flex items-start gap-2">
            <div className="min-w-0 flex-1">
              <p className="truncate text-[0.8rem] font-semibold" title={job.input}>
                {detectedTitle ?? cleanGameName(job.input_name)}
              </p>
              <p className="mt-0.5 flex flex-wrap items-center gap-x-1.5 text-[0.64rem] text-[var(--color-faint)]">
                <span style={{ color: sys.color }}>{sys.name}</span>
                <span>·</span>
                <span>{MODE_LABELS[job.mode] ?? job.mode}</span>
                {elapsed && (
                  <>
                    <span>·</span>
                    <span>{elapsed}</span>
                  </>
                )}
              </p>
            </div>

            <span className="mt-0.5 shrink-0">
          {running && <Loader2 size={15} className="animate-spin" style={{ color: "var(--accent)" }} />}
          {done && <CheckCircle2 size={15} className="text-emerald-400" />}
          {failed && <AlertCircle size={15} className="text-rose-400" />}
          {(job.status === "queued" || job.status === "canceled") && (
            <span
              className="block h-[15px] w-[15px] rounded-full border-2"
              style={{ borderColor: job.status === "queued" ? "var(--color-faint)" : "#ffffff22" }}
            />
          )}
            </span>
          </div>

          <div className="mt-2 flex gap-0.5">
          {done && (
            <button
              className="btn btn-quiet px-1.5 py-1"
              title="Mostrar en el explorador"
              onClick={() => api.reveal(job.output)}
            >
              <FolderSearch size={14} />
            </button>
          )}
          {failed && (
            <button
              className="btn btn-quiet px-1.5 py-1"
              title="Reintentar"
              onClick={async () => setJobs(await api.retryJob(job.id))}
            >
              <RotateCcw size={14} />
            </button>
          )}
          {(running || job.status === "queued") && (
            <button
              className="btn btn-quiet btn-danger px-1.5 py-1"
              title="Cancelar"
              onClick={() => api.cancelJob(job.id)}
            >
              <X size={14} />
            </button>
          )}
          {!running && (
            <button
              className="btn btn-quiet btn-danger px-1.5 py-1"
              title="Quitar de la lista"
              onClick={async () => setJobs(await api.removeJob(job.id))}
            >
              <Trash2 size={14} />
            </button>
          )}
          </div>
        </div>
      </div>

      {running && (
        <div className="mt-2.5">
          {/* Hay herramientas que no dicen por dónde van cuando su salida no es
              una consola. En ese caso se muestra lo que llevan escrito. */}
          {job.progress > 0 ? (
            <>
              <div className="bar live">
                <i style={{ width: `${Math.max(2, job.progress)}%` }} />
              </div>
              <p className="mt-1.5 flex justify-between text-[0.66rem] text-[var(--color-muted)]">
                <span>{job.phase}</span>
                <span className="mono">
                  {job.progress.toFixed(1)} %
                  {job.ratio != null && ` · ratio ${job.ratio.toFixed(0)} %`}
                </span>
              </p>
            </>
          ) : (
            <>
              <div className="bar indeterminate">
                <i />
              </div>
              <p className="mt-1.5 flex justify-between text-[0.66rem] text-[var(--color-muted)]">
                <span>{job.phase}</span>
                <span className="mono">
                  {job.output_size > 0 ? `${bytes(job.output_size)} escritos` : "trabajando…"}
                </span>
              </p>
            </>
          )}
        </div>
      )}

      {done && (
        <div className="mt-2 space-y-1.5">
          <p className="flex items-center gap-1.5 text-[0.68rem] text-[var(--color-muted)]">
            <span className="mono">{bytes(job.input_size)}</span>
            <span className="text-[var(--color-faint)]">→</span>
            <span className="mono">{bytes(job.output_size)}</span>
            {saved && (
              <span className="chip ml-auto border-emerald-400/25 bg-emerald-400/10 text-emerald-300">
                {saved}
              </span>
            )}
          </p>
          {job.verification === "passed" && (
            <p
              className="flex items-center gap-1.5 text-[0.66rem] text-emerald-300"
              title={job.verification_message ?? undefined}
            >
              <ShieldCheck size={13} /> Verificación completa
            </p>
          )}
          {job.verification === "basic" && (
            <p
              className="flex items-center gap-1.5 text-[0.66rem] text-sky-300"
              title={job.verification_message ?? undefined}
            >
              <ShieldEllipsis size={13} /> Validación estructural
            </p>
          )}
        </div>
      )}

      {failed && job.message && (
        <p className="selectable mt-2 rounded-lg bg-rose-400/10 px-2 py-1.5 text-[0.68rem] leading-snug text-rose-300">
          {job.message}
        </p>
      )}
    </motion.li>
  );
}

export function QueuePanel() {
  const { jobs, setJobs } = useStore();
  const active = jobs.filter((j) => j.status === "running" || j.status === "queued");
  const finished = jobs.filter((j) => j.status !== "running" && j.status !== "queued");
  const totalPct = active.length
    ? active.reduce((a, j) => a + j.progress, 0) / active.length
    : 0;

  return (
    <aside className="flex w-[360px] shrink-0 flex-col border-l border-[var(--color-edge)]">
      <header className="flex items-center justify-between gap-2 border-b border-[var(--color-edge)] px-4 py-3">
        <div>
          <h2 className="text-[0.82rem] font-semibold">Cola</h2>
          <p className="text-[0.68rem] text-[var(--color-faint)]">
            {active.length ? `${active.length} en marcha · ${totalPct.toFixed(0)} %` : "Sin trabajos activos"}
          </p>
        </div>
        <div className="flex gap-1">
          {active.length > 1 && (
            <button className="btn btn-quiet px-2 py-1" title="Cancelar todo" onClick={() => api.cancelAll()}>
              <ListX size={15} />
            </button>
          )}
          {finished.length > 0 && (
            <button
              className="btn btn-quiet px-2 py-1"
              title="Limpiar terminados"
              onClick={async () => setJobs(await api.clearFinished())}
            >
              <Trash2 size={15} />
            </button>
          )}
        </div>
      </header>

      {active.length > 0 && (
        <div className="border-b border-[var(--color-edge)] px-4 py-3">
          <div className="bar live">
            <i style={{ width: `${Math.max(2, totalPct)}%` }} />
          </div>
        </div>
      )}

      <div className="scroll flex-1 p-3">
        {jobs.length === 0 ? (
          <Empty
            icon={<Loader2 size={22} />}
            title="Todo tranquilo"
            desc="Los trabajos que lances aparecerán aquí con su progreso en tiempo real."
          />
        ) : (
          <ul className="flex flex-col gap-2">
            <AnimatePresence initial={false}>
              {[...active, ...finished].map((j) => (
                <JobRow key={j.id} job={j} />
              ))}
            </AnimatePresence>
          </ul>
        )}
      </div>
    </aside>
  );
}

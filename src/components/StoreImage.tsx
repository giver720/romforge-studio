import { useState } from "react";

export function StoreImage({ url, name, className = "h-20 w-20" }: { url?: string | null; name: string; className?: string }) {
  const [loaded, setLoaded] = useState(false);
  const [failed, setFailed] = useState(false);
  const valid = url?.startsWith("https://") && !failed;
  return <div className={`relative shrink-0 overflow-hidden rounded-xl bg-gradient-to-br from-indigo-500/20 to-cyan-500/10 ${className}`}>
    {!loaded && <div aria-hidden="true" className={`absolute inset-0 flex items-center justify-center text-lg font-semibold text-[var(--color-muted)] ${valid ? "animate-pulse" : ""}`}>{name.trim().slice(0, 2).toUpperCase() || "HB"}</div>}
    {valid && <img src={url!} alt={name} loading="lazy" decoding="async" onLoad={() => setLoaded(true)} onError={() => { setLoaded(false); setFailed(true); }} className={`h-full w-full object-contain transition-opacity ${loaded ? "opacity-100" : "opacity-0"}`} />}
  </div>;
}

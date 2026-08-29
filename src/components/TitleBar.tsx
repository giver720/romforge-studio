import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X } from "lucide-react";

export function TitleBar() {
  const win = getCurrentWindow();
  // `-webkit-app-region` (la clase .drag) solo lo entienden los motores de
  // Windows y macOS; en Linux WebKitGTK lo ignora y la ventana se quedaría
  // clavada. `data-tauri-drag-region` funciona en los tres.
  return (
    <header
      data-tauri-drag-region
      className="drag relative z-30 flex h-11 shrink-0 items-center justify-between border-b border-[var(--color-edge)] pl-4 pr-0"
    >
      <div className="flex items-center gap-2.5">
        <div
          className="grid h-[22px] w-[22px] place-items-center rounded-[7px] text-[11px] font-black text-[#0a0c12]"
          style={{ background: "linear-gradient(135deg, var(--accent), var(--accent-2))" }}
        >
          C
        </div>
        <span className="text-[0.82rem] font-semibold tracking-tight">
          CHD <span className="text-[var(--color-muted)] font-normal">Studio</span>
        </span>
      </div>

      <div className="no-drag flex h-full">
        <button
          onClick={() => win.minimize()}
          className="grid h-full w-12 place-items-center text-[var(--color-muted)] transition-colors hover:bg-white/10 hover:text-[var(--color-ink)]"
          aria-label="Minimizar"
        >
          <Minus size={15} />
        </button>
        <button
          onClick={() => win.toggleMaximize()}
          className="grid h-full w-12 place-items-center text-[var(--color-muted)] transition-colors hover:bg-white/10 hover:text-[var(--color-ink)]"
          aria-label="Maximizar"
        >
          <Square size={12} />
        </button>
        <button
          onClick={() => win.close()}
          className="grid h-full w-12 place-items-center text-[var(--color-muted)] transition-colors hover:bg-[#e81123] hover:text-white"
          aria-label="Cerrar"
        >
          <X size={16} />
        </button>
      </div>
    </header>
  );
}

import { motion } from "framer-motion";
import {
  AlertTriangle,
  CheckCircle2,
  Disc3,
  FileArchive,
  Gamepad2,
  Gauge,
  Layers,
  Package,
  Scissors,
  Search,
  Sparkles,
  Settings,
  Wand2,
} from "lucide-react";
import { useStore, type View } from "../store";

interface NavItem {
  id: View;
  label: string;
  icon: typeof Wand2;
  desc: string;
}

const SECTIONS: { title: string; items: NavItem[] }[] = [
  {
    title: "CHD",
    items: [
      { id: "convert", label: "Convertir", icon: Package, desc: "ISO, CUE, GDI → CHD" },
      { id: "extract", label: "Extraer", icon: Wand2, desc: "CHD → formato original" },
      { id: "inspect", label: "Inspeccionar", icon: Search, desc: "Datos y verificación" },
    ],
  },
  {
    title: "Consolas modernas",
    items: [
      { id: "switch", label: "Switch", icon: Gamepad2, desc: "NSP, NSZ, XCI, XCZ" },
      { id: "threeds", label: "3DS", icon: Layers, desc: "CIA, CCI y Z3DS" },
      { id: "xbox360", label: "Xbox 360", icon: Disc3, desc: "ISO → GOD o carpeta" },
      { id: "ps3", label: "PlayStation 3", icon: Scissors, desc: "ISO compacto · RPCS3" },
      { id: "ps5", label: "PlayStation 5", icon: FileArchive, desc: "Carpeta → imagen exFAT" },
      { id: "psp", label: "PSP", icon: Gauge, desc: "ISO ↔ CSO y ZSO" },
      { id: "wii", label: "Wii y GameCube", icon: Sparkles, desc: "ISO → RVZ" },
    ],
  },
];

function NavButton({ item }: { item: NavItem }) {
  const { view, setView, jobs } = useStore();
  const on = view === item.id;
  const Icon = item.icon;
  const active = jobs.filter((j) => j.status === "running" || j.status === "queued").length;

  return (
    <button
      onClick={() => setView(item.id)}
      className="relative flex items-center gap-3 rounded-xl px-3 py-2.5 text-left transition-colors"
    >
      {on && (
        <motion.span
          layoutId="nav-pill"
          className="absolute inset-0 rounded-xl border border-[#ffffff1f] bg-white/[0.07]"
          transition={{ type: "spring", stiffness: 460, damping: 38 }}
        />
      )}
      <Icon
        size={17}
        className="relative z-10 shrink-0"
        style={{ color: on ? "var(--accent)" : "var(--color-faint)" }}
      />
      <span className="relative z-10 min-w-0 flex-1">
        <span
          className="block truncate text-[0.83rem] font-medium"
          style={{ color: on ? "var(--color-ink)" : "var(--color-muted)" }}
        >
          {item.label}
        </span>
        <span className="block truncate text-[0.68rem] text-[var(--color-faint)]">{item.desc}</span>
      </span>
      {item.id === "convert" && active > 0 && (
        <span
          className="relative z-10 rounded-full px-1.5 py-0.5 text-[0.62rem] font-bold text-[#0a0c12]"
          style={{ background: "var(--accent)" }}
        >
          {active}
        </span>
      )}
    </button>
  );
}

export function Sidebar() {
  const { setView, chdman } = useStore();

  return (
    <nav className="scroll relative z-20 flex w-[214px] shrink-0 flex-col gap-1 border-r border-[var(--color-edge)] p-3">
      {SECTIONS.map((s) => (
        <div key={s.title} className="mb-1 flex flex-col gap-0.5">
          <p className="px-3 pb-1 pt-2 text-[0.62rem] font-semibold uppercase tracking-wider text-[var(--color-faint)]">
            {s.title}
          </p>
          {s.items.map((i) => (
            <NavButton key={i.id} item={i} />
          ))}
        </div>
      ))}

      <div className="mt-auto flex flex-col gap-1 pt-3">
        <NavButton
          item={{ id: "settings", label: "Ajustes", icon: Settings, desc: "Herramientas y opciones" }}
        />
        <button
          onClick={() => setView("settings")}
          className="glass flex w-full items-center gap-2.5 rounded-xl p-2.5 text-left transition-colors hover:bg-white/[0.08]"
        >
          {chdman?.found ? (
            <CheckCircle2 size={16} className="shrink-0 text-emerald-400" />
          ) : (
            <AlertTriangle size={16} className="shrink-0 text-amber-400" />
          )}
          <span className="min-w-0">
            <span className="block text-[0.72rem] font-semibold">
              {chdman?.found ? "chdman listo" : "Falta chdman"}
            </span>
            <span className="block truncate text-[0.65rem] text-[var(--color-faint)]">
              {chdman?.found
                ? chdman.supports_zstd
                  ? "Con soporte zstd"
                  : "Versión clásica"
                : "Toca para configurarlo"}
            </span>
          </span>
        </button>
      </div>
    </nav>
  );
}

import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { AnimatePresence, motion } from "framer-motion";
import { AlertTriangle, CheckCircle2, Info } from "lucide-react";
import { useEffect, useState } from "react";
import { ConvertView } from "./components/ConvertView";
import { ExtractView } from "./components/ExtractView";
import { InspectView } from "./components/InspectView";
import { Ps3View } from "./components/Ps3View";
import { Ps5View } from "./components/Ps5View";
import { PspView } from "./components/PspView";
import { QueuePanel } from "./components/QueuePanel";
import { SettingsView } from "./components/SettingsView";
import { StoreView } from "./components/StoreView";
import { Sidebar } from "./components/Sidebar";
import { SwitchView } from "./components/SwitchView";
import { ThreeDsView } from "./components/ThreeDsView";
import { TitleBar } from "./components/TitleBar";
import { WiiView } from "./components/WiiView";
import { XboxView } from "./components/XboxView";
import { api } from "./lib/api";
import type { InputInfo, Job } from "./lib/types";
import { FAMILY_EXT, familyOf, useStore, type ConsoleFamily, type View } from "./store";

function Toast() {
  const toast = useStore((s) => s.toast);
  const Icon = toast?.kind === "ok" ? CheckCircle2 : toast?.kind === "warn" ? AlertTriangle : Info;
  const tint =
    toast?.kind === "ok" ? "#34d399" : toast?.kind === "warn" ? "#f5a524" : toast?.kind === "error" ? "#fb7185" : "#a78bfa";

  return (
    <AnimatePresence>
      {toast && (
        <motion.div
          key={toast.id}
          initial={{ opacity: 0, y: 22, scale: 0.96 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          exit={{ opacity: 0, y: 12, scale: 0.97 }}
          transition={{ type: "spring", stiffness: 420, damping: 32 }}
          className="glass-strong fixed bottom-5 left-1/2 z-50 flex -translate-x-1/2 items-center gap-2.5 rounded-xl px-4 py-2.5 shadow-2xl"
        >
          <Icon size={16} style={{ color: tint }} />
          <span className="text-[0.78rem]">{toast.text}</span>
        </motion.div>
      )}
    </AnimatePresence>
  );
}

export default function App() {
  const {
    view,
    settings,
    loadSettings,
    refreshChdman,
    refreshJobs,
    upsertJob,
    setJobs,
    addPaths,
    addConsoleFiles,
    notify,
  } = useStore();
  const [dragging, setDragging] = useState(false);

  /**
   * Un archivo soltado va a la vista que le corresponde: un .cia no pinta nada
   * en la lista de CHD. Si vienen mezclados, gana el grupo más numeroso.
   */
  async function routeDrop(paths: string[]) {
    const infos = await api.inspectPaths(paths);
    if (!infos.length) return;

    // Si estás en una vista de consola y lo que sueltas le sirve, se queda ahí.
    // Esto resuelve la ambigüedad del .iso, que vale para CHD y para Xbox 360.
    const current = useStore.getState().view as ConsoleFamily;
    if (FAMILY_EXT[current]) {
      const mine = infos.filter((i) => FAMILY_EXT[current].includes(i.ext));
      if (mine.length) {
        const n = addConsoleFiles(current, mine);
        if (n) notify("ok", `${n} ${n === 1 ? "archivo listo" : "archivos listos"}`);
        return;
      }
    }

    const buckets: Record<ConsoleFamily, InputInfo[]> = {
      switch: [],
      threeds: [],
      xbox360: [],
      psp: [],
      wii: [],
    };
    const chd: InputInfo[] = [];
    for (const i of infos) {
      const fam = familyOf(i.ext);
      if (fam === "chd") chd.push(i);
      else buckets[fam].push(i);
    }

    const counts: [View, number][] = [
      ["convert", chd.filter((i) => i.state === "ok").length],
      ["switch", buckets.switch.length],
      ["threeds", buckets.threeds.length],
      ["psp", buckets.psp.length],
      ["wii", buckets.wii.length],
    ];
    const [winner, best] = counts.sort((a, b) => b[1] - a[1])[0];

    if (best === 0) {
      const bad = infos.find((i) => i.note);
      notify("warn", bad?.note ?? "Ninguno de esos archivos se puede procesar");
      return;
    }

    let added = 0;
    if (winner === "convert") {
      added = (await addPaths(paths)).added;
    } else {
      added = addConsoleFiles(winner as ConsoleFamily, buckets[winner as ConsoleFamily]);
    }

    if (added) {
      useStore.setState({ view: winner });
      notify("ok", `${added} ${added === 1 ? "archivo listo" : "archivos listos"}`);
    }
  }

  useEffect(() => {
    loadSettings();
    refreshChdman();
    refreshJobs();
  }, []);

  // El acento vive en un atributo del <html> para que lo lea el CSS
  useEffect(() => {
    document.documentElement.dataset.accent = settings.accent;
  }, [settings.accent]);

  useEffect(() => {
    const un1 = listen<Job>("job://update", (e) => upsertJob(e.payload));
    const un2 = listen<Job[]>("jobs://reset", (e) => setJobs(e.payload));
    return () => {
      un1.then((f) => f());
      un2.then((f) => f());
    };
  }, []);

  // Arrastrar y soltar sobre la ventana
  useEffect(() => {
    const un = getCurrentWebview().onDragDropEvent(async (e) => {
      if (e.payload.type === "over") setDragging(true);
      else if (e.payload.type === "leave") setDragging(false);
      else if (e.payload.type === "drop") {
        setDragging(false);
        const paths = e.payload.paths;
        if (!paths?.length) return;
        await routeDrop(paths);
      }
    });
    return () => {
      un.then((f) => f());
    };
  }, []);

  const showQueue = view !== "inspect" && view !== "settings";

  return (
    <div className="app-bg flex h-full flex-col">
      <TitleBar />
      <div className="relative z-10 flex min-h-0 flex-1">
        <Sidebar />
        <main className="flex min-w-0 flex-1">
          <AnimatePresence mode="wait">
            <motion.div
              key={view}
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -6 }}
              transition={{ duration: 0.16 }}
              className="flex min-w-0 flex-1"
            >
              {view === "convert" && <ConvertView dragging={dragging} />}
              {view === "extract" && <ExtractView dragging={dragging} />}
              {view === "switch" && <SwitchView dragging={dragging} />}
              {view === "threeds" && <ThreeDsView dragging={dragging} />}
              {view === "xbox360" && <XboxView dragging={dragging} />}
              {view === "ps3" && <Ps3View />}
              {view === "ps5" && <Ps5View />}
              {view === "psp" && <PspView dragging={dragging} />}
              {view === "wii" && <WiiView dragging={dragging} />}
              {view === "store" && <StoreView />}
              {view === "inspect" && <InspectView />}
              {view === "settings" && <SettingsView />}
            </motion.div>
          </AnimatePresence>
          {showQueue && <QueuePanel />}
        </main>
      </div>
      <Toast />
    </div>
  );
}

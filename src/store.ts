import { create } from "zustand";
import { api } from "./lib/api";
import { AUTO, ALL_SYSTEMS, codecsFor, type Preset } from "./lib/profiles";
import { SWITCH_EXT, defaultOp as switchDefaultOp } from "./lib/switch";
import { THREEDS_EXT, defaultOp as threeDsDefaultOp } from "./lib/threeds";
import { PSP_EXT, defaultOp as pspDefaultOp } from "./lib/psp";
import { WII_EXT, defaultOp as wiiDefaultOp } from "./lib/wii";
import type {
  ChdmanStatus,
  InputInfo,
  Job,
  Mode,
  Settings,
  StagedFile,
  ToolStatus,
} from "./lib/types";

export type View =
  | "convert"
  | "extract"
  | "inspect"
  | "switch"
  | "threeds"
  | "xbox360"
  | "ps3"
  | "ps5"
  | "psp"
  | "wii"
  | "settings";

export type ConsoleFamily = "switch" | "threeds" | "xbox360" | "psp" | "wii";

export interface ConsoleFile extends InputInfo {
  op: string;
}

/** Operación por defecto de un archivo según su módulo y extensión. */
function defaultOpFor(family: ConsoleFamily, ext: string): string {
  if (family === "switch") return switchDefaultOp(ext);
  if (family === "threeds") return threeDsDefaultOp(ext);
  if (family === "psp") return pspDefaultOp(ext);
  if (family === "wii") return wiiDefaultOp(ext);
  return "iso2god";
}

/** Extensiones que acepta cada vista de consola. */
export const FAMILY_EXT: Record<ConsoleFamily, string[]> = {
  switch: SWITCH_EXT,
  threeds: THREEDS_EXT,
  xbox360: ["iso"],
  psp: PSP_EXT,
  wii: WII_EXT,
};

/**
 * A qué módulo pertenece una extensión.
 *
 * `.iso` es ambiguo: lo usan tanto CHD como Xbox 360, así que por defecto va a
 * CHD y es la vista activa la que puede reclamarlo (ver `routeDrop`).
 */
export function familyOf(ext: string): ConsoleFamily | "chd" {
  if (SWITCH_EXT.includes(ext)) return "switch";
  if (THREEDS_EXT.includes(ext)) return "threeds";
  // Los comprimidos de PSP son inconfundibles; el .iso se queda en CHD
  if (ext === "cso" || ext === "zso" || ext === "dax") return "psp";
  if (ext === "rvz" || ext === "wia" || ext === "gcz" || ext === "gcm") return "wii";
  return "chd";
}

/** La vista con la que arranca la app. */
const INITIAL_VIEW: View = "convert";

interface AppStore {
  view: View;
  setView: (v: View) => void;

  settings: Settings;
  loadSettings: () => Promise<void>;
  patchSettings: (p: Partial<Settings>) => Promise<void>;

  chdman: ChdmanStatus | null;
  refreshChdman: () => Promise<void>;

  tools: ToolStatus[];
  setTools: (t: ToolStatus[]) => void;
  refreshTools: () => Promise<void>;
  installingTool: string | null;
  installTool: (id: string) => Promise<void>;

  /** Listas de espera de los módulos de consola, con la operación de cada archivo. */
  consoleFiles: Record<ConsoleFamily, ConsoleFile[]>;
  addConsoleFiles: (family: ConsoleFamily, infos: InputInfo[]) => number;
  setConsoleOp: (family: ConsoleFamily, path: string, op: string) => void;
  removeConsoleFile: (family: ConsoleFamily, path: string) => void;
  clearConsoleFiles: (family: ConsoleFamily) => void;

  staged: StagedFile[];
  addPaths: (paths: string[]) => Promise<{ added: number; skipped: InputInfo[] }>;
  setStaged: (paths: string[], patch: Partial<StagedFile>) => void;
  removeStaged: (path: string) => void;
  clearStaged: () => void;

  jobs: Job[];
  setJobs: (j: Job[]) => void;
  upsertJob: (j: Job) => void;
  refreshJobs: () => Promise<void>;
  enqueueStaged: (opts?: { outputDir?: string | null }) => Promise<number>;

  toast: { kind: string; text: string; id: number } | null;
  notify: (kind: string, text: string) => void;
}

const DEFAULT_SETTINGS: Settings = {
  chdman_path: null,
  output_dir: null,
  preset: "balanced",
  delete_source: false,
  overwrite: false,
  parallel: 1,
  threads: 0,
  accent: "violet",
  tool_paths: {},
  switch_keys_path: null,
  boot9_path: null,
  aes_keys_path: null,
  seeddb_path: null,
  nsz_level: 18,
  nsz_threads: 0,
  xbox_trim: true,
  xbox_skip_update: true,
  wii_scrub: true,
  wii_level: 5,
  wii_wbfs_split: true,
  ps3_split_fat32: false,
};

/** Elige el perfil de sistema más probable a partir de la extensión y el tamaño. */
function guessSystem(info: InputInfo): string {
  const ext = info.ext;
  if (ext === "gdi") return "dreamcast";
  if (info.suggested_mode === "createdvd") return "ps2";
  if (info.suggested_mode === "createhd") return "pchd";
  if (info.suggested_mode === "createcd") return "psx";
  return AUTO.id;
}

export const useStore = create<AppStore>((set, get) => ({
  view: INITIAL_VIEW,
  setView: (v) => set({ view: v }),

  settings: DEFAULT_SETTINGS,
  loadSettings: async () => set({ settings: await api.getSettings() }),
  patchSettings: async (p) => {
    const next = { ...get().settings, ...p };
    set({ settings: next });
    set({ settings: await api.setSettings(next) });
  },

  chdman: null,
  refreshChdman: async () => set({ chdman: await api.chdmanStatus() }),

  tools: [],
  setTools: (t) => set({ tools: t }),
  refreshTools: async () => set({ tools: await api.toolsStatus() }),
  installingTool: null,
  installTool: async (id) => {
    set({ installingTool: id });
    try {
      set({ tools: await api.installTool(id) });
      get().notify("ok", `${id} instalado`);
    } catch (e) {
      get().notify("error", String(e));
    } finally {
      set({ installingTool: null });
    }
  },

  consoleFiles: { switch: [], threeds: [], xbox360: [], psp: [], wii: [] },
  addConsoleFiles: (family, infos) => {
    const current = get().consoleFiles[family];
    const seen = new Set(current.map((c) => c.path));
    const fresh = infos
      .filter((i) => !seen.has(i.path))
      .map((i) => ({ ...i, op: defaultOpFor(family, i.ext) }));
    if (!fresh.length) return 0;
    set({ consoleFiles: { ...get().consoleFiles, [family]: [...current, ...fresh] } });
    return fresh.length;
  },
  setConsoleOp: (family, path, op) =>
    set({
      consoleFiles: {
        ...get().consoleFiles,
        [family]: get().consoleFiles[family].map((c) => (c.path === path ? { ...c, op } : c)),
      },
    }),
  removeConsoleFile: (family, path) =>
    set({
      consoleFiles: {
        ...get().consoleFiles,
        [family]: get().consoleFiles[family].filter((c) => c.path !== path),
      },
    }),
  clearConsoleFiles: (family) =>
    set({ consoleFiles: { ...get().consoleFiles, [family]: [] } }),

  staged: [],
  addPaths: async (paths) => {
    const infos = await api.inspectPaths(paths);
    const existing = new Set(get().staged.map((s) => s.path));
    // Los formatos de consola tienen su propia vista; aquí solo entra lo de CHD
    const mine = infos.filter((i) => familyOf(i.ext) === "chd");
    const ok = mine.filter((i) => i.state === "ok" && !existing.has(i.path));
    const skipped = mine.filter((i) => i.state !== "ok");

    const staged: StagedFile[] = ok.map((i) => ({
      ...i,
      systemId: guessSystem(i),
      mode: (i.suggested_mode || "createcd") as Mode,
    }));

    set({ staged: [...get().staged, ...staged] });
    return { added: staged.length, skipped };
  },
  setStaged: (paths, patch) =>
    set({
      staged: get().staged.map((s) => {
        if (!paths.includes(s.path)) return s;
        const next = { ...s, ...patch };
        // Cambiar de sistema arrastra consigo el modo de chdman correspondiente
        if (patch.systemId) {
          const sys = ALL_SYSTEMS.find((x) => x.id === patch.systemId);
          if (sys) next.mode = sys.mode;
        }
        return next;
      }),
    }),
  removeStaged: (path) => set({ staged: get().staged.filter((s) => s.path !== path) }),
  clearStaged: () => set({ staged: [] }),

  jobs: [],
  setJobs: (j) => set({ jobs: j }),
  upsertJob: (j) => {
    const jobs = get().jobs;
    const i = jobs.findIndex((x) => x.id === j.id);
    if (i === -1) set({ jobs: [...jobs, j] });
    else {
      const copy = jobs.slice();
      copy[i] = j;
      set({ jobs: copy });
    }
  },
  refreshJobs: async () => set({ jobs: await api.listJobs() }),

  enqueueStaged: async (opts) => {
    const { staged, settings, chdman } = get();
    if (!staged.length) return 0;
    const zstd = chdman?.supports_zstd ?? false;
    const specs = staged.map((s) => ({
      input: s.path,
      mode: s.mode,
      system: s.systemId,
      codecs: codecsFor(s.mode, settings.preset as Preset, zstd),
      hunk_size: null,
      unit_size: s.mode === "createraw" ? 512 : null,
      output_dir: opts?.outputDir ?? null,
    }));
    await api.addJobs(specs);
    set({ staged: [] });
    await get().refreshJobs();
    return specs.length;
  },

  toast: null,
  notify: (kind, text) => {
    set({ toast: { kind, text, id: Date.now() } });
    setTimeout(() => {
      if (get().toast?.text === text) set({ toast: null });
    }, 4200);
  },
}));

export type Mode =
  | "createcd"
  | "createdvd"
  | "createhd"
  | "createraw"
  | "extractcd"
  | "extractdvd"
  | "extracthd"
  | "extractraw"
  | "verify";

export type JobStatus = "queued" | "running" | "done" | "error" | "canceled";

export interface Job {
  id: string;
  input: string;
  input_name: string;
  output: string;
  output_extra: string | null;
  tool: string;
  mode: string;
  system: string;
  codecs: string[];
  hunk_size: number | null;
  unit_size: number | null;
  status: JobStatus;
  progress: number;
  phase: string;
  ratio: number | null;
  message: string | null;
  verification: "pending" | "running" | "passed" | "basic" | "failed" | "not_applicable";
  verification_message: string | null;
  log: string[];
  input_size: number;
  output_size: number;
  started_at: number | null;
  finished_at: number | null;
}

export interface InputInfo {
  path: string;
  name: string;
  ext: string;
  size: number;
  state: "ok" | "needs_cue" | "unsupported" | "missing";
  suggested_mode: string;
  note: string | null;
}

export interface ChdmanStatus {
  found: boolean;
  path: string | null;
  version: string | null;
  source: "manual" | "app" | "path" | "mame" | null;
  supports_zstd: boolean;
}

export type ToolKind =
  | { type: "bundled" }
  | { type: "python"; package: string }
  | { type: "github"; repo: string; asset: string }
  | { type: "external"; site: string }
  | { type: "web"; page: string; base: string; contains: string }
  // Solo en Linux: proyectos que no publican binario y hay que compilar, y
  // herramientas que reparte el gestor de paquetes de la distribución.
  | { type: "source"; repo: string; build: string; output: string; packages: string }
  | { type: "pythonscript"; repo: string; script: string; requires: string }
  | { type: "system"; hint: string };

export interface ToolStatus {
  id: string;
  name: string;
  purpose: string;
  family: "chd" | "switch" | "3ds" | "ps3" | "ps5" | "xbox360" | "psp" | "wii";
  license: string;
  kind: ToolKind;
  found: boolean;
  path: string | null;
  version: string | null;
  source: string | null;
  installable: boolean;
}

export interface PythonStatus {
  found: boolean;
  path: string | null;
  version: string | null;
  venv_ready: boolean;
}

export interface KeysStatus {
  found: boolean;
  path: string | null;
  expected: string;
  custom: boolean;
}

export interface ThreeDsKeys {
  boot9: string | null;
  aes_keys: string | null;
  seeddb: string | null;
  expected_dir: string;
}

export interface Settings {
  chdman_path: string | null;
  output_dir: string | null;
  preset: "max" | "balanced" | "fast";
  delete_source: boolean;
  overwrite: boolean;
  parallel: number;
  threads: number;
  accent: string;
  tool_paths: Record<string, string>;
  switch_keys_path: string | null;
  boot9_path: string | null;
  aes_keys_path: string | null;
  seeddb_path: string | null;
  nsz_level: number;
  nsz_threads: number;
  xbox_trim: boolean;
  xbox_skip_update: boolean;
  ps3_split_fat32: boolean;
  wii_scrub: boolean;
  wii_level: number;
  wii_wbfs_split: boolean;
}

/** Un archivo puesto en la mesa de trabajo, ya con su perfil asignado. */
export interface StagedFile extends InputInfo {
  systemId: string;
  mode: Mode;
}

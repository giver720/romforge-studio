import { invoke } from "@tauri-apps/api/core";
import type { Ps3Scan, TrimResult } from "./ps3";
import type {
  ChdmanStatus,
  InputInfo,
  Job,
  KeysStatus,
  PythonStatus,
  Settings,
  ThreeDsKeys,
  ToolStatus,
} from "./types";

export interface JobSpec {
  input: string;
  mode: string;
  system: string;
  codecs?: string[];
  hunk_size?: number | null;
  unit_size?: number | null;
  format?: string | null;
  output_dir?: string | null;
}

export const api = {
  inspectPaths: (paths: string[]) => invoke<InputInfo[]>("inspect_paths", { paths }),
  addJobs: (specs: JobSpec[]) => invoke<Job[]>("add_jobs", { specs }),
  listJobs: () => invoke<Job[]>("list_jobs"),
  cancelJob: (id: string) => invoke<void>("cancel_job", { id }),
  cancelAll: () => invoke<void>("cancel_all"),
  removeJob: (id: string) => invoke<Job[]>("remove_job", { id }),
  retryJob: (id: string) => invoke<Job[]>("retry_job", { id }),
  clearFinished: () => invoke<Job[]>("clear_finished"),
  getSettings: () => invoke<Settings>("get_settings"),
  setSettings: (value: Settings) => invoke<Settings>("set_settings", { value }),
  chdmanStatus: () => invoke<ChdmanStatus>("chdman_status"),
  installChdman: (path: string) => invoke<ChdmanStatus>("install_chdman", { path }),
  chdInfo: (path: string) => invoke<string>("chd_info", { path }),
  reveal: (path: string) => invoke<void>("reveal", { path }),

  toolsStatus: () => invoke<ToolStatus[]>("tools_status"),
  installTool: (id: string) => invoke<ToolStatus[]>("install_tool", { id }),
  setToolPath: (id: string, path: string | null) =>
    invoke<ToolStatus[]>("set_tool_path", { id, path }),
  pythonStatus: () => invoke<PythonStatus>("python_status"),
  switchKeysStatus: () => invoke<KeysStatus>("switch_keys_status"),
  threeDsKeysStatus: () => invoke<ThreeDsKeys>("threeds_keys_status"),
  xboxProbe: (path: string) => invoke<string>("xbox_probe", { path }),
  appPaths: () =>
    invoke<{
      portable: boolean;
      config_dir: string;
      can_update: boolean;
      update_hint: string | null;
    }>("app_paths"),
  ps3Scan: (dir: string) => invoke<Ps3Scan>("ps3_scan", { dir }),
  ps3Trim: (dir: string, paths: string[]) => invoke<TrimResult>("ps3_trim", { dir, paths }),
};

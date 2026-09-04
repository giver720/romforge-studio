export interface Ps5Scan {
  valid: boolean;
  title_id: string | null;
  title: string | null;
  version: string | null;
  file_count: number;
  directory_count: number;
  raw_bytes: number;
  image_bytes: number;
  compressed_estimate_bytes: number;
  estimated_savings_percent: number;
  recommended_format: "ffpkg" | "exfat" | "ffpfsc";
  warnings: string[];
  error: string | null;
}

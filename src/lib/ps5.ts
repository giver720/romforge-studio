export interface Ps5Scan {
  valid: boolean;
  title_id: string | null;
  title: string | null;
  version: string | null;
  file_count: number;
  directory_count: number;
  raw_bytes: number;
  image_bytes: number;
  error: string | null;
}

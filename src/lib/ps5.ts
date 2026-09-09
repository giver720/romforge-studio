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

export type Ps5LabFormatId = "fpkg" | "lz4";
export type Ps5LabMode = "ps5fpkg" | "ps5lz4";

export interface Ps5LabFormat {
  id: Ps5LabFormatId;
  mode: Ps5LabMode;
  name: string;
  badge: string;
  description: string;
  availability: "blocked-upstream" | "announced";
  requirements: string[];
  actionLabel: string;
}

/**
 * Formatos PS5 en preparación. No son modos ejecutables todavía: mantenerlos
 * fuera de la cola evita producir archivos dañados o con un formato supuesto.
 */
export const PS5_LAB_FORMATS: Ps5LabFormat[] = [
  {
    id: "fpkg",
    mode: "ps5fpkg",
    name: "FPKG nativo de PS5",
    badge: "Build de prueba",
    description:
      "Empaquetado FPKG para dumps nativos de PS5. Es distinto de FFPKG/UFS2 y del FPKG de PS4.",
    availability: "blocked-upstream",
    requirements: [
      "Build corregida sin corrupción parcial",
      "Interfaz automatizable y verificación del paquete",
      "Soporte de instalación y ejecución en la consola",
    ],
    actionLabel: "Esperando build corregida",
  },
  {
    id: "lz4",
    mode: "ps5lz4",
    name: "LZ4 · Lizard",
    badge: "Anunciado",
    description:
      "Nuevo formato compacto anunciado para juegos de PS5; no es un archivo LZ4 genérico renombrado.",
    availability: "announced",
    requirements: [
      "Encoder o especificación pública",
      "Detector fiable del contenedor",
      "Prueba de extracción y montaje en PS5",
    ],
    actionLabel: "Esperando encoder público",
  },
];

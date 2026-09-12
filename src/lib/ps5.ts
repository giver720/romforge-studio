export interface Ps5Scan {
  valid: boolean;
  title_id: string | null;
  title: string | null;
  version: string | null;
  content_id: string | null;
  file_count: number;
  directory_count: number;
  raw_bytes: number;
  image_bytes: number;
  compressed_estimate_bytes: number;
  estimated_savings_percent: number;
  recommended_format: "ffpkg" | "exfat" | "ffpfsc";
  fpkg_ready: boolean;
  fpkg_module_count: number;
  fpkg_blockers: string[];
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
  availability: "available" | "beta";
  requirements: string[];
  actionLabel: string;
}

/** Formatos PS5 recientes con motores públicos verificables. */
export const PS5_LAB_FORMATS: Ps5LabFormat[] = [
  {
    id: "fpkg",
    mode: "ps5fpkg",
    name: "FPKG nativo de PS5",
    badge: "Disponible",
    description:
      "Empaquetado FPKG actualizado para dumps nativos de PS5. Desde una carpeta crea el .pkg directamente; desde exFAT ROMForge encadena extracción, conversión y validación.",
    availability: "available",
    requirements: [
      "Dump con módulos ELF descifrados",
      "Las imágenes exFAT se procesan automáticamente desde el panel inferior",
      "contentId válido en sce_sys/param.json",
      "Firmware y payloads compatibles en la consola",
    ],
    actionLabel: "Crear .pkg",
  },
  {
    id: "lz4",
    mode: "ps5lz4",
    name: "LZ4 · AMPRPAK4",
    badge: "Beta pública",
    description:
      "Carpeta compacta con bloques LZ4 seekable y runtime AMPR oficial. Mantiene sueltos módulos y archivos del sistema.",
    availability: "beta",
    requirements: [
      "ShadowMountPlus y payloads compatibles",
      "Primera prueba recomendada desde USB o almacenamiento externo",
      "La verificación offline no sustituye la prueba completa del juego",
    ],
    actionLabel: "Crear carpeta LZ4",
  },
];

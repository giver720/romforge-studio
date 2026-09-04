import type { Mode } from "./types";

export interface SystemProfile {
  id: string;
  name: string;
  /** Fabricante o familia, se muestra pequeño debajo del nombre */
  maker: string;
  mode: Mode;
  /** Extensiones que este sistema suele traer */
  accepts: string[];
  /** Color de acento del perfil (clases Tailwind ya resueltas en runtime) */
  color: string;
  note?: string;
}

export interface Generation {
  id: string;
  title: string;
  subtitle: string;
  systems: SystemProfile[];
}

/**
 * chdman cubre tres familias de medio: CD (2352 bytes/sector), DVD (2048) y
 * disco duro. Agrupamos las consolas por generación para que se entienda de un
 * vistazo qué entra y qué no.
 */
export const GENERATIONS: Generation[] = [
  {
    id: "cd",
    title: "Era del CD-ROM",
    subtitle: "4.ª y 5.ª generación · sectores de 2352 bytes",
    systems: [
      {
        id: "psx",
        name: "PlayStation",
        maker: "Sony · 1994",
        mode: "createcd",
        accepts: ["cue", "bin", "iso", "toc"],
        color: "#8b95a5",
      },
      {
        id: "saturn",
        name: "Saturn",
        maker: "Sega · 1994",
        mode: "createcd",
        accepts: ["cue", "bin", "iso"],
        color: "#5b7cfa",
      },
      {
        id: "dreamcast",
        name: "Dreamcast",
        maker: "Sega · 1998",
        mode: "createcd",
        accepts: ["gdi", "cue", "bin"],
        color: "#f97362",
        note: "Usa el .gdi del GD-ROM. Los .cdi hay que convertirlos antes.",
      },
      {
        id: "segacd",
        name: "Mega-CD / Sega CD",
        maker: "Sega · 1991",
        mode: "createcd",
        accepts: ["cue", "bin", "iso"],
        color: "#4fb0c6",
      },
      {
        id: "pcecd",
        name: "PC Engine CD",
        maker: "NEC · 1988",
        mode: "createcd",
        accepts: ["cue", "bin", "toc"],
        color: "#e0a458",
      },
      {
        id: "neogeocd",
        name: "Neo Geo CD",
        maker: "SNK · 1994",
        mode: "createcd",
        accepts: ["cue", "bin", "iso"],
        color: "#d94f70",
      },
      {
        id: "3do",
        name: "3DO",
        maker: "Panasonic · 1993",
        mode: "createcd",
        accepts: ["cue", "bin", "iso"],
        color: "#9d7fe8",
      },
      {
        id: "cdi",
        name: "Philips CD-i",
        maker: "Philips · 1991",
        mode: "createcd",
        accepts: ["cue", "bin", "iso"],
        color: "#7fc99a",
      },
      {
        id: "pcfx",
        name: "PC-FX",
        maker: "NEC · 1994",
        mode: "createcd",
        accepts: ["cue", "bin"],
        color: "#c78ce0",
      },
      {
        id: "cd32",
        name: "Amiga CD32",
        maker: "Commodore · 1993",
        mode: "createcd",
        accepts: ["cue", "bin", "iso"],
        color: "#6fb1e0",
      },
    ],
  },
  {
    id: "dvd",
    title: "Era del DVD",
    subtitle: "6.ª y 7.ª generación · sectores de 2048 bytes",
    systems: [
      {
        id: "ps2",
        name: "PlayStation 2",
        maker: "Sony · 2000",
        mode: "createdvd",
        accepts: ["iso"],
        color: "#4a6cf7",
        note: "Los juegos en CD (los de disco azul) van mejor como CD, no DVD.",
      },
      {
        id: "ps2cd",
        name: "PlayStation 2 (CD)",
        maker: "Sony · disco azul",
        mode: "createcd",
        accepts: ["cue", "bin", "iso"],
        color: "#4a90f7",
      },
      {
        id: "psp",
        name: "PSP (UMD)",
        maker: "Sony · 2004",
        mode: "createdvd",
        accepts: ["iso"],
        color: "#7a8ff7",
      },
      {
        id: "xbox",
        name: "Xbox",
        maker: "Microsoft · 2001",
        mode: "createdvd",
        accepts: ["iso"],
        color: "#7bc043",
      },
      {
        id: "pcdvd",
        name: "PC / DVD-ROM",
        maker: "Datos genéricos",
        mode: "createdvd",
        accepts: ["iso"],
        color: "#8b95a5",
      },
    ],
  },
  {
    id: "hd",
    title: "Discos duros e imágenes crudas",
    subtitle: "Instalaciones de arcade, PC y consolas con HDD",
    systems: [
      {
        id: "arcadehd",
        name: "Arcade HDD",
        maker: "Naomi 2, Chihiro, Type X…",
        mode: "createhd",
        accepts: ["img", "raw", "hdd"],
        color: "#f0a04b",
      },
      {
        id: "pchd",
        name: "Disco duro de PC",
        maker: "MS-DOS, Win9x, ScummVM",
        mode: "createhd",
        accepts: ["img", "hdi", "vhd", "raw"],
        color: "#8b95a5",
      },
      {
        id: "xboxhd",
        name: "Xbox HDD",
        maker: "Microsoft",
        mode: "createhd",
        accepts: ["img", "raw"],
        color: "#7bc043",
      },
      {
        id: "raw",
        name: "Datos crudos",
        maker: "Cualquier volcado sin formato",
        mode: "createraw",
        accepts: ["raw", "bin", "img"],
        color: "#a0a8b8",
        note: "Necesita tamaño de hunk y de unidad manuales.",
      },
    ],
  },
  {
    id: "arcade",
    title: "Arcade en GD-ROM",
    subtitle: "Placas basadas en Dreamcast",
    systems: [
      {
        id: "naomi",
        name: "Sega NAOMI / NAOMI 2",
        maker: "Sega · 1998",
        mode: "createcd",
        accepts: ["gdi", "cue"],
        color: "#f97362",
      },
      {
        id: "triforce",
        name: "Triforce",
        maker: "Sega · Namco · Nintendo",
        mode: "createcd",
        accepts: ["gdi", "cue"],
        color: "#e05c8a",
      },
      {
        id: "chihiro",
        name: "Chihiro",
        maker: "Sega · Microsoft",
        mode: "createcd",
        accepts: ["gdi", "cue"],
        color: "#7bc043",
      },
      {
        id: "atomiswave",
        name: "Atomiswave",
        maker: "Sammy · 2003",
        mode: "createcd",
        accepts: ["gdi", "cue"],
        color: "#c9a227",
      },
    ],
  },
];

export const ALL_SYSTEMS: SystemProfile[] = GENERATIONS.flatMap((g) => g.systems);

export const AUTO: SystemProfile = {
  id: "auto",
  name: "Automático",
  maker: "Detecta el tipo por el archivo",
  mode: "createcd",
  accepts: [],
  color: "#a78bfa",
};

/** Perfil sintético para los trabajos que no vienen del catálogo de CHD. */
const SWITCH_PROFILE: SystemProfile = {
  id: "switch",
  name: "Switch",
  maker: "Nintendo · 2017",
  mode: "createcd",
  accepts: ["nsp", "nsz", "xci", "xcz"],
  color: "#e4404a",
};

const THREEDS_PROFILE: SystemProfile = {
  id: "3ds",
  name: "3DS",
  maker: "Nintendo · 2011",
  mode: "createcd",
  accepts: ["cci", "3ds", "cia", "cxi", "3dsx"],
  color: "#d13f5a",
};

const XBOX_PROFILE: SystemProfile = {
  id: "xbox360",
  name: "Xbox 360",
  maker: "Microsoft · 2005",
  mode: "createdvd",
  accepts: ["iso"],
  color: "#7bc043",
};

const PS3_PROFILE: SystemProfile = {
  id: "ps3",
  name: "PlayStation 3",
  maker: "Sony · 2006",
  mode: "createdvd",
  accepts: ["iso"],
  color: "#4a6cf7",
};

const PS5_PROFILE: SystemProfile = {
  id: "ps5",
  name: "PlayStation 5",
  maker: "Sony · 2020",
  mode: "createdvd",
  accepts: [],
  color: "#5b8cff",
};

const PSP_PROFILE: SystemProfile = {
  id: "psp",
  name: "PSP",
  maker: "Sony · 2004",
  mode: "createdvd",
  accepts: ["iso", "cso", "zso", "dax"],
  color: "#7a8ff7",
};

const WII_PROFILE: SystemProfile = {
  id: "wii",
  name: "Wii / GameCube",
  maker: "Nintendo · 2006",
  mode: "createdvd",
  accepts: ["iso", "rvz", "wia", "gcz"],
  color: "#4fb0c6",
};

export function systemById(id: string): SystemProfile {
  if (id === "wii") return WII_PROFILE;
  if (id === "psp") return PSP_PROFILE;
  if (id === "ps3") return PS3_PROFILE;
  if (id === "ps5") return PS5_PROFILE;
  if (id === "switch") return SWITCH_PROFILE;
  if (id === "3ds") return THREEDS_PROFILE;
  if (id === "xbox360") return XBOX_PROFILE;
  return ALL_SYSTEMS.find((s) => s.id === id) ?? AUTO;
}

export function generationOf(id: string): Generation | undefined {
  return GENERATIONS.find((g) => g.systems.some((s) => s.id === id));
}

/** Sistemas que encajan con la extensión y el modo sugerido de un archivo. */
export function suggestSystems(ext: string, mode: string): SystemProfile[] {
  return ALL_SYSTEMS.filter(
    (s) => s.mode === mode && (s.accepts.includes(ext) || s.accepts.length === 0),
  );
}

export type Preset = "max" | "balanced" | "fast";

export const PRESETS: { id: Preset; name: string; desc: string }[] = [
  { id: "max", name: "Máxima", desc: "El archivo más pequeño posible. Tarda bastante más." },
  { id: "balanced", name: "Equilibrada", desc: "Los códecs por defecto de MAME. La opción segura." },
  { id: "fast", name: "Rápida", desc: "Comprime menos pero va mucho más deprisa." },
];

/**
 * Los códecs de CD llevan prefijo `cd`; los de DVD/HD son los genéricos.
 * `cdzs`/`zstd` solo existen desde MAME 0.255, por eso se consulta antes.
 */
export function codecsFor(mode: Mode, preset: Preset, zstd: boolean): string[] {
  const isCd = mode === "createcd";
  if (isCd) {
    if (preset === "max") return zstd ? ["cdzs", "cdlz", "cdzl", "cdfl"] : ["cdlz", "cdzl", "cdfl"];
    if (preset === "fast") return zstd ? ["cdzs", "cdfl"] : ["cdzl", "cdfl"];
    return ["cdlz", "cdzl", "cdfl"];
  }
  if (preset === "max") return zstd ? ["zstd", "lzma", "huff", "flac"] : ["lzma", "zlib", "huff", "flac"];
  if (preset === "fast") return zstd ? ["zstd", "huff"] : ["zlib", "huff"];
  return ["lzma", "zlib", "huff", "flac"];
}

export const MODE_LABELS: Record<string, string> = {
  createcd: "CD → CHD",
  createdvd: "DVD → CHD",
  createhd: "HDD → CHD",
  createraw: "Crudo → CHD",
  extractcd: "CHD → CUE/BIN",
  extractdvd: "CHD → ISO",
  extracthd: "CHD → IMG",
  extractraw: "CHD → BIN",
  verify: "Verificar",
  nsp2nsz: "NSP → NSZ",
  nsz2nsp: "NSZ → NSP",
  xci2xcz: "XCI → XCZ",
  xcz2xci: "XCZ → XCI",
  xci2nsp: "XCI → NSP",
  z3dscompress: "Comprimir a Z3DS",
  cia2cci: "CIA → CCI",
  cci2cia: "CCI → CIA",
  iso2god: "ISO → GOD",
  iso2folder: "ISO → carpeta",
  folder2iso: "Carpeta → ISO",
  ps3extract: "ISO → carpeta",
  ps3build: "Carpeta → ISO",
  ps3split: "Partir para FAT32",
  ps3compact: "ISO compacto universal",
  ps3rpcs3: "Compresión transparente RPCS3",
  ps5exfat: "Carpeta → imagen exFAT",
  ps5ffpkg: "Carpeta → FFPKG UFS2",
  ps5ffpfsc: "Carpeta → FFPFSC",
  ps5compress: "Imagen → FFPFSC",
  ps5extract: "Imagen → carpeta",
  iso2cso: "ISO → CSO",
  iso2zso: "ISO → ZSO",
  iso2dax: "ISO → DAX",
  cso2iso: "→ ISO",
  iso2rvz: "ISO → RVZ",
  iso2wia: "ISO → WIA",
  iso2gcz: "ISO → GCZ",
  rvz2iso: "→ ISO",
  iso2wbfs: "ISO → WBFS",
};

export const IS_WINDOWS = navigator.userAgent.includes("Windows");

/** Filtro de ejecutables de Windows; en Linux no hay una extension obligatoria. */
export const EXECUTABLE_FILTERS = IS_WINDOWS
  ? [{ name: "Ejecutable", extensions: ["exe", ""] }]
  : undefined;

export function executableName(base: string): string {
  return IS_WINDOWS ? `${base}.exe` : base;
}

import { open } from "@tauri-apps/plugin-dialog";
import { AlertTriangle, CheckCircle2, FolderInput, Gamepad2, Image, Play, SlidersHorizontal, Trash2, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { api, type GameArtwork } from "../lib/api";
import { bytes } from "../lib/format";
import { useStore } from "../store";
import { DropZone } from "./DropZone";
import { Card, Segmented, Toggle } from "./ui";

const EXTENSIONS = ["iso"];
type Antialias = "off" | "MSAA4x" | "SSAA4x";

export function PspFpkgView({ dragging }: { dragging: boolean }) {
  const { consoleFiles, addConsoleFiles, removeConsoleFile, clearConsoleFiles, tools, refreshTools, refreshJobs, settings, patchSettings, notify } = useStore();
  const files = consoleFiles.pspfpkg;
  const [antialias, setAntialias] = useState<Antialias>("SSAA4x");
  const [multisaves, setMultisaves] = useState(true);
  const [secureSaves, setSecureSaves] = useState(false);
  const [textureReplacement, setTextureReplacement] = useState(false);
  const [autoArt, setAutoArt] = useState(true);
  const [acceptButton, setAcceptButton] = useState("");
  const [language, setLanguage] = useState("");
  const [logLevel, setLogLevel] = useState("");
  const [title, setTitle] = useState("");
  const [titleId, setTitleId] = useState("");
  const [icon, setIcon] = useState("");
  const [background, setBackground] = useState("");
  const [lua, setLua] = useState("");
  const [config, setConfig] = useState("");
  const [textures, setTextures] = useState("");
  const [emulatorDir, setEmulatorDir] = useState("");
  const [sets, setSets] = useState("");
  const [advanced, setAdvanced] = useState(false);
  const [artwork, setArtwork] = useState<GameArtwork | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => { refreshTools(); }, []);
  useEffect(() => {
    let active = true;
    setArtwork(null);
    if (files[0]) api.gameArtwork(files[0].path, "psp").then((value) => { if (active) setArtwork(value); }).catch(() => null);
    return () => { active = false; };
  }, [files[0]?.path]);

  const tool = tools.find((value) => value.id === "pspfpkg");
  const missing = !!tool && !tool.found;
  const invalidTitleId = titleId.trim() && !/^[A-Za-z]{4}[-_]?\d{5}$/.test(titleId.trim());
  const invalidFlag = useMemo(() => sets.split(/\r?\n/).map((value) => value.trim()).find((value) => value && (!value.includes("=") || value.startsWith("-"))), [sets]);

  async function handlePaths(paths: string[]) {
    const infos = await api.inspectPaths(paths);
    const accepted = infos.filter((value) => EXTENSIONS.includes(value.ext));
    if (!accepted.length) return notify("warn", "Elige una imagen ISO de PSP");
    addConsoleFiles("pspfpkg", accepted);
  }

  async function chooseFile(kind: "icon" | "background" | "lua" | "config") {
    const extensions = kind === "lua" ? ["lua"] : kind === "config" ? ["txt", "cfg"] : ["png", "jpg", "jpeg", "webp"];
    const result = await open({ multiple: false, filters: [{ name: kind === "lua" ? "Parche Lua" : kind === "config" ? "Configuración PSPHD" : "Imagen", extensions }] });
    if (!result) return;
    if (kind === "icon") setIcon(result as string);
    else if (kind === "background") setBackground(result as string);
    else if (kind === "lua") setLua(result as string);
    else setConfig(result as string);
  }

  async function chooseDirectory(kind: "textures" | "emulator") {
    const result = await open({ directory: true, multiple: false });
    if (!result) return;
    if (kind === "textures") setTextures(result as string); else setEmulatorDir(result as string);
  }

  async function run() {
    if (!files.length || invalidTitleId || invalidFlag) return;
    const options: Record<string, string> = {
      antialias, multisaves: String(multisaves), securesaves: String(secureSaves),
      texture_replacement: String(textureReplacement), auto_art: String(autoArt),
    };
    if (files.length === 1 && title.trim()) options.title = title.trim();
    if (files.length === 1 && titleId.trim()) options.title_id = titleId.trim();
    if (acceptButton) options.xobuttonmode = acceptButton;
    if (language) options.lang = language;
    if (logLevel) options.loglevel = logLevel;
    if (icon) options.icon = icon;
    if (background) options.background = background;
    if (lua) options.lua = lua;
    if (config) options.config = config;
    if (textures) options.textures = textures;
    if (emulatorDir) options.emulator_dir = emulatorDir;
    if (sets.trim()) options.sets = sets.trim();
    setBusy(true);
    try {
      await api.addJobs(files.map((file) => ({ input: file.path, mode: "pspfpkg", system: "psp", options })));
      clearConsoleFiles("pspfpkg");
      await refreshJobs();
      notify("ok", `${files.length} ${files.length === 1 ? "FPKG de PSP encolado" : "FPKG de PSP encolados"}`);
    } catch (error) {
      notify("error", String(error));
    } finally {
      setBusy(false);
    }
  }

  return <div className="scroll flex-1 p-5">
    <div className="mb-4 flex items-end justify-between gap-4">
      <div><h1 className="text-lg font-semibold">PSP <span className="accent-text">→ PS4 FPKG</span></h1><p className="mt-0.5 text-xs text-[var(--color-muted)]">Empaqueta tu ISO con PSPHD, ajustes de compatibilidad y portada por juego.</p></div>
      {files.length > 0 && <button className="btn btn-primary" disabled={busy || missing || !!invalidTitleId || !!invalidFlag} onClick={run}><Play size={15}/> {busy ? "Encolando…" : `Crear ${files.length}`}</button>}
    </div>

    {missing && <div className="mb-4 flex items-start gap-3 rounded-xl border border-amber-400/25 bg-amber-400/[0.07] p-3.5"><AlertTriangle size={17} className="mt-0.5 text-amber-400"/><div><p className="text-sm font-medium">Falta ROMForge PSP Bridge</p><p className="mt-1 text-xs text-[var(--color-muted)]">Reinstala ROMForge Studio o recompila los puentes incluidos.</p></div></div>}

    <DropZone dragging={dragging} compact={files.length > 0} title="Suelta tus juegos de PSP" hint="ISO de tus propias copias. El primer paquete descarga y verifica los recursos PSPHD." extensions={EXTENSIONS} onPaths={handlePaths}/>

    {files.length > 0 && <div className="mt-4 grid gap-3 xl:grid-cols-[1.05fr_.95fr]">
      <Card title="Emulador PSPHD" desc="SSAA 4× ofrece la mejor imagen; cambia el perfil si un juego presenta fallos.">
        <Segmented value={antialias} onChange={setAntialias} layoutId="psp-fpkg-aa" options={[{id:"off",label:"Nativo"},{id:"MSAA4x",label:"MSAA 4×"},{id:"SSAA4x",label:"SSAA 4×"}]}/>
        <div className="mt-3 grid grid-cols-3 gap-2">
          <label className="text-xs text-[var(--color-muted)]">Confirmar<select className="field mt-1 w-full" value={acceptButton} onChange={(e)=>setAcceptButton(e.target.value)}><option value="">Automático</option><option value="xenter">X</option><option value="oenter">Círculo</option></select></label>
          <label className="text-xs text-[var(--color-muted)]">Idioma<select className="field mt-1 w-full" value={language} onChange={(e)=>setLanguage(e.target.value)}><option value="">Sistema</option><option value="es">Español</option><option value="en">English</option><option value="jp">日本語</option><option value="fr">Français</option></select></label>
          <label className="text-xs text-[var(--color-muted)]">Registro<select className="field mt-1 w-full" value={logLevel} onChange={(e)=>setLogLevel(e.target.value)}><option value="">Normal</option><option value="error">Errores</option><option value="debug">Depuración</option></select></label>
        </div>
        <div className="mt-3 grid gap-2 md:grid-cols-2"><Toggle checked={multisaves} onChange={setMultisaves} label="Guardados múltiples" hint="Mantiene varias ranuras de guardado."/><Toggle checked={secureSaves} onChange={setSecureSaves} label="Guardados seguros" hint="Activa la protección PSPHD."/></div>
      </Card>

      <Card title="Portada y metadatos" desc="ROMForge busca el arte por el nombre del juego.">
        <div className="flex gap-3">{artwork?.data_url ? <img src={artwork.data_url} className="h-24 w-20 rounded-lg object-cover" alt="Portada PSP"/> : <span className="grid h-24 w-20 place-items-center rounded-lg bg-white/5"><Gamepad2/></span>}<div className="min-w-0 flex-1"><Toggle checked={autoArt} onChange={setAutoArt} label="Portada automática" hint="Crea icono y fondo con recorte correcto."/><input className="field mt-2 w-full" value={title} disabled={files.length !== 1} onChange={(e)=>setTitle(e.target.value)} placeholder="Título automático (opcional)"/></div></div>
        <div className="mt-2 grid grid-cols-2 gap-2"><button className="btn btn-quiet" onClick={()=>chooseFile("icon")}><Image size={14}/>{icon ? "Icono elegido" : "Icono propio"}</button><button className="btn btn-quiet" onClick={()=>chooseFile("background")}><Image size={14}/>{background ? "Fondo elegido" : "Fondo propio"}</button></div>
        <label className="mt-2 block text-xs text-[var(--color-muted)]">ID del juego (solo si no se detecta)<input className="field mt-1 w-full uppercase" value={titleId} disabled={files.length !== 1} onChange={(e)=>setTitleId(e.target.value)} placeholder="ULUS12345"/>{invalidTitleId && <span className="mt-1 block text-rose-300">Usa cuatro letras y cinco números.</span>}</label>
      </Card>
    </div>}

    {files.length > 0 && <Card title="Compatibilidad avanzada" desc="Lua, configuración y texturas para juegos que necesitan ajustes." right={<button className="btn btn-quiet" onClick={()=>setAdvanced(!advanced)}><SlidersHorizontal size={14}/>{advanced ? "Ocultar" : "Configurar"}</button>}>
      {advanced && <div className="grid gap-2 md:grid-cols-2">
        <button className="btn btn-ghost justify-start" onClick={()=>chooseFile("lua")}>{lua ? `Lua: ${lua.split(/[\\/]/).pop()}` : "Elegir parche .lua"}</button>
        <button className="btn btn-ghost justify-start" onClick={()=>chooseFile("config")}>{config ? `Config: ${config.split(/[\\/]/).pop()}` : "Añadir config PSPHD"}</button>
        <button className="btn btn-ghost justify-start" onClick={()=>chooseDirectory("textures")}>{textures ? `Texturas: ${textures.split(/[\\/]/).pop()}` : "Carpeta de texturas"}</button>
        <button className="btn btn-ghost justify-start" onClick={()=>chooseDirectory("emulator")}>{emulatorDir ? "PSPHD personalizado" : "PSPHD personalizado"}</button>
        <Toggle checked={textureReplacement} onChange={setTextureReplacement} label="Reemplazo de texturas" hint="Usa la carpeta seleccionada como texreplace."/>
        <label className="text-xs text-[var(--color-muted)]">Opciones extra, una por línea<textarea className="field mt-1 min-h-20 w-full resize-y" value={sets} onChange={(e)=>setSets(e.target.value)} placeholder={"fast-forward=0\nfoo=bar"}/>{invalidFlag && <span className="mt-1 block text-rose-300">Usa nombre=valor, sin guiones: {invalidFlag}</span>}</label>
      </div>}
    </Card>}

    {files.length > 0 && <><div className="mb-2 mt-5 flex justify-between"><h2 className="text-sm font-semibold">Juegos seleccionados</h2><button className="btn btn-quiet btn-danger" onClick={()=>clearConsoleFiles("pspfpkg")}><Trash2 size={13}/> Vaciar</button></div><ul className="space-y-2">{files.map((file)=><li key={file.path} className="glass flex items-center gap-3 rounded-xl p-3"><CheckCircle2 size={16} className="text-emerald-400"/><span className="min-w-0 flex-1"><span className="block truncate text-sm font-medium">{file.name}</span><span className="text-xs text-[var(--color-faint)]">ISO · {bytes(file.size)}</span></span><button className="btn btn-quiet btn-danger" onClick={()=>removeConsoleFile("pspfpkg", file.path)}><X size={14}/></button></li>)}</ul></>}

    <div className="glass mt-4 flex items-center gap-3 rounded-xl p-3"><FolderInput size={16}/><span className="text-xs text-[var(--color-muted)]">Salida</span><button className="btn btn-quiet min-w-0 flex-1 justify-start" onClick={async()=>{const value=await open({directory:true,multiple:false});if(value) await patchSettings({output_dir:value as string});}}><span className="truncate">{settings.output_dir || "Junto al juego original"}</span></button></div>
    <p className="mt-3 text-[0.68rem] text-[var(--color-faint)]">La compatibilidad depende del juego y de PSPHD. No se incluyen juegos, claves ni herramientas propietarias de Sony.</p>
  </div>;
}

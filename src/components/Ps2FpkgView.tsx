import { open } from "@tauri-apps/plugin-dialog";
import { AlertTriangle, CheckCircle2, FolderInput, Gamepad2, Image, Package2, Play, SlidersHorizontal, Trash2, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { api, type GameArtwork } from "../lib/api";
import { bytes } from "../lib/format";
import { useStore } from "../store";
import { DropZone } from "./DropZone";
import { Card, Segmented, Toggle } from "./ui";

const EXTENSIONS = ["iso", "chd", "7z", "zip", "rar"];
type Quality = "native" | "balanced" | "quality";
const QUALITY: Record<Quality, { uprender: string; upscale: string }> = {
  native: { uprender: "1x1", upscale: "none" },
  balanced: { uprender: "2x2", upscale: "EdgeSmooth" },
  quality: { uprender: "3x3", upscale: "EdgeSmooth" },
};

export function Ps2FpkgView({ dragging }: { dragging: boolean }) {
  const { consoleFiles, addConsoleFiles, removeConsoleFile, clearConsoleFiles, tools, refreshTools, refreshJobs, settings, patchSettings, notify } = useStore();
  const files = consoleFiles.ps2fpkg;
  const [quality, setQuality] = useState<Quality>("balanced");
  const [emu, setEmu] = useState("Jak v2");
  const [display, setDisplay] = useState("full");
  const [multitap, setMultitap] = useState("");
  const [autoArt, setAutoArt] = useState(true);
  const [title, setTitle] = useState("");
  const [icon, setIcon] = useState("");
  const [background, setBackground] = useState("");
  const [lua, setLua] = useState("");
  const [config, setConfig] = useState("");
  const [sets, setSets] = useState("");
  const [advanced, setAdvanced] = useState(false);
  const [artwork, setArtwork] = useState<GameArtwork | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => { refreshTools(); }, []);
  useEffect(() => {
    let active = true;
    setArtwork(null);
    if (files[0]) api.gameArtwork(files[0].path, "ps2").then((value) => { if (active) setArtwork(value); }).catch(() => null);
    return () => { active = false; };
  }, [files[0]?.path]);

  const tool = tools.find((value) => value.id === "ps2fpkg");
  const missing = !!tool && !tool.found;
  const invalidFlag = useMemo(() => sets.split(/\r?\n/).map((v) => v.trim()).find((v) => v && (!v.includes("=") || v.startsWith("-"))), [sets]);

  async function handlePaths(paths: string[]) {
    const infos = await api.inspectPaths(paths);
    const ok = infos.filter((value) => EXTENSIONS.includes(value.ext));
    if (!ok.length) return notify("warn", "Elige un ISO, CHD o archivo comprimido que contenga un ISO de PS2");
    addConsoleFiles("ps2fpkg", ok);
  }

  async function chooseImage(kind: "icon" | "background") {
    const result = await open({ multiple: false, filters: [{ name: "Imagen PNG", extensions: ["png"] }] });
    if (!result) return;
    if (kind === "icon") setIcon(result as string); else setBackground(result as string);
  }

  async function chooseExtra(kind: "lua" | "config") {
    const result = await open({ multiple: false, filters: [{ name: kind === "lua" ? "Parche Lua" : "Configuración", extensions: kind === "lua" ? ["lua"] : ["txt", "cfg"] }] });
    if (!result) return;
    if (kind === "lua") setLua(result as string); else setConfig(result as string);
  }

  async function run() {
    if (!files.length || invalidFlag) return;
    const options: Record<string, string> = {
      emu, uprender: QUALITY[quality].uprender, upscale: QUALITY[quality].upscale,
      display_mode: display, auto_art: String(autoArt),
    };
    if (multitap) options.multitap = multitap;
    if (files.length === 1 && title.trim()) options.title = title.trim();
    if (icon) options.icon = icon;
    if (background) options.background = background;
    if (lua) options.lua = lua;
    if (config) options.config = config;
    if (sets.trim()) options.sets = sets.trim();
    setBusy(true);
    try {
      await api.addJobs(files.map((file) => ({ input: file.path, mode: "ps2fpkg", system: "ps2", options })));
      clearConsoleFiles("ps2fpkg");
      await refreshJobs();
      notify("ok", `${files.length} ${files.length === 1 ? "FPKG encolado" : "FPKG encolados"}`);
    } catch (error) {
      notify("error", String(error));
    } finally {
      setBusy(false);
    }
  }

  return <div className="scroll flex-1 p-5">
    <div className="mb-4 flex items-end justify-between gap-4">
      <div><h1 className="text-lg font-semibold">PS2 <span className="accent-text">→ PS4 FPKG</span></h1><p className="mt-0.5 text-xs text-[var(--color-muted)]">Crea paquetes instalables con emulador, configuración y portada por juego.</p></div>
      {files.length > 0 && <button className="btn btn-primary" disabled={busy || missing || !!invalidFlag} onClick={run}><Play size={15}/> {busy ? "Encolando…" : `Crear ${files.length}`}</button>}
    </div>

    {missing && <div className="mb-4 flex items-start gap-3 rounded-xl border border-amber-400/25 bg-amber-400/[0.07] p-3.5"><AlertTriangle size={17} className="mt-0.5 text-amber-400"/><div className="flex-1"><p className="text-sm font-medium">Falta easy-ps2-fpkg</p><p className="mt-1 text-xs text-[var(--color-muted)]">ROMForge descarga el binario MIT para Windows o Ubuntu. En el primer FPKG, el motor descarga y guarda aparte los recursos del emulador.</p></div><button className="btn btn-ghost" onClick={() => useStore.setState({ view: "settings" })}><Package2 size={15}/> Instalar</button></div>}

    <DropZone dragging={dragging} compact={files.length > 0} title="Suelta tus juegos de PS2" hint="ISO y CHD, o 7Z/ZIP/RAR con un ISO. Usa únicamente tus propias copias." extensions={EXTENSIONS} onPaths={handlePaths}/>

    {files.length > 0 && <div className="mt-4 grid gap-3 xl:grid-cols-[1.05fr_.95fr]">
      <Card title="Emulador" desc="Equilibrada es el punto de partida recomendado.">
        <Segmented value={quality} onChange={setQuality} layoutId="ps2-quality" options={[{id:"native",label:"Nativa"},{id:"balanced",label:"Equilibrada"},{id:"quality",label:"3× calidad"}]}/>
        <div className="mt-3 grid grid-cols-3 gap-2">
          <label className="text-xs text-[var(--color-muted)]">Núcleo<select className="field mt-1 w-full" value={emu} onChange={(e)=>setEmu(e.target.value)}><option>Jak v2</option><option>Rogue v1</option></select></label>
          <label className="text-xs text-[var(--color-muted)]">Pantalla<select className="field mt-1 w-full" value={display} onChange={(e)=>setDisplay(e.target.value)}><option value="full">Completa</option><option value="fit">Ajustar</option><option value="original">Original</option></select></label>
          <label className="text-xs text-[var(--color-muted)]">Multitap<select className="field mt-1 w-full" value={multitap} onChange={(e)=>setMultitap(e.target.value)}><option value="">Desactivado</option><option value="1">Puerto 1</option><option value="2">Puerto 2</option><option value="both">Ambos</option></select></label>
        </div>
      </Card>
      <Card title="Portada y metadatos" desc="La portada automática se busca por el serial detectado.">
        <div className="flex gap-3">{artwork?.data_url ? <img src={artwork.data_url} className="h-24 w-20 rounded-lg object-cover" alt="Portada PS2"/> : <span className="grid h-24 w-20 place-items-center rounded-lg bg-white/5"><Gamepad2/></span>}<div className="min-w-0 flex-1"><Toggle checked={autoArt} onChange={setAutoArt} label="Portada automática" hint="Genera icono 512×512 y fondo 1920×1080."/><input className="field mt-2 w-full" value={title} disabled={files.length !== 1} onChange={(e)=>setTitle(e.target.value)} placeholder={files.length === 1 ? "Título automático (opcional)" : "Título automático para cada juego"}/></div></div>
        <div className="mt-2 flex gap-2"><button className="btn btn-quiet flex-1" onClick={()=>chooseImage("icon")}><Image size={14}/>{icon ? "Icono elegido" : "Icono propio"}</button><button className="btn btn-quiet flex-1" onClick={()=>chooseImage("background")}><Image size={14}/>{background ? "Fondo elegido" : "Fondo propio"}</button></div>
      </Card>
    </div>}

    {files.length > 0 && <Card title="Opciones avanzadas" desc="Parches de compatibilidad y banderas del emulador." right={<button className="btn btn-quiet" onClick={()=>setAdvanced(!advanced)}><SlidersHorizontal size={14}/>{advanced ? "Ocultar" : "Configurar"}</button>}>
      {advanced && <div className="grid gap-2 md:grid-cols-2"><button className="btn btn-ghost justify-start" onClick={()=>chooseExtra("lua")}>{lua ? `Lua: ${lua.split(/[\\/]/).pop()}` : "Elegir parche .lua"}</button><button className="btn btn-ghost justify-start" onClick={()=>chooseExtra("config")}>{config ? `Config: ${config.split(/[\\/]/).pop()}` : "Usar config-emu-ps4.txt"}</button><label className="md:col-span-2 text-xs text-[var(--color-muted)]">Banderas adicionales, una por línea<textarea className="field mt-1 min-h-20 w-full resize-y" value={sets} onChange={(e)=>setSets(e.target.value)} placeholder={"host-audio=1\ngs-interlace=1"}/>{invalidFlag && <span className="mt-1 block text-rose-300">Usa nombre=valor, sin guiones: {invalidFlag}</span>}</label></div>}
    </Card>}

    {files.length > 0 && <><div className="mb-2 mt-5 flex justify-between"><h2 className="text-sm font-semibold">Juegos seleccionados</h2><button className="btn btn-quiet btn-danger" onClick={()=>clearConsoleFiles("ps2fpkg")}><Trash2 size={13}/> Vaciar</button></div><ul className="space-y-2">{files.map((file)=><li key={file.path} className="glass flex items-center gap-3 rounded-xl p-3"><CheckCircle2 size={16} className="text-emerald-400"/><span className="min-w-0 flex-1"><span className="block truncate text-sm font-medium">{file.name}</span><span className="text-xs text-[var(--color-faint)]">{file.ext.toUpperCase()} · {bytes(file.size)}</span></span><button className="btn btn-quiet btn-danger" onClick={()=>removeConsoleFile("ps2fpkg", file.path)}><X size={14}/></button></li>)}</ul></>}

    <div className="glass mt-4 flex items-center gap-3 rounded-xl p-3"><FolderInput size={16}/><span className="text-xs text-[var(--color-muted)]">Salida</span><button className="btn btn-quiet min-w-0 flex-1 justify-start" onClick={async()=>{const value=await open({directory:true,multiple:false});if(value) await patchSettings({output_dir:value as string});}}><span className="truncate">{settings.output_dir || "Junto al juego original"}</span></button></div>
  </div>;
}

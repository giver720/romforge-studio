#!/usr/bin/env bash
# CHD Studio - crea el paquete .deb para Ubuntu/Debian.
#
# El equivalente de CREAR_EXE.bat en Linux. Comprueba antes las bibliotecas de
# desarrollo que necesita Tauri, porque el error que da cuando faltan no se
# entiende.
set -euo pipefail
cd "$(dirname "$0")"

echo
echo " == CHD Studio =="
echo " Generando los paquetes de Linux. La primera vez tarda varios minutos."
echo

falta=()
comprobar_paquete() { pkg-config --exists "$1" 2>/dev/null || falta+=("$2"); }

command -v cargo >/dev/null || { echo " Falta Rust. Instalalo desde https://rustup.rs"; exit 1; }
command -v npm   >/dev/null || { echo " Falta Node.js (npm)."; exit 1; }
command -v pkg-config >/dev/null || falta+=(pkg-config)

if command -v pkg-config >/dev/null; then
  comprobar_paquete webkit2gtk-4.1             libwebkit2gtk-4.1-dev
  comprobar_paquete gtk+-3.0                   libgtk-3-dev
  comprobar_paquete libssl                     libssl-dev
  comprobar_paquete ayatana-appindicator3-0.1  libayatana-appindicator3-dev
  comprobar_paquete librsvg-2.0                 librsvg2-dev
  comprobar_paquete xdo                         libxdo-dev
fi
command -v file >/dev/null || falta+=(file)

if [ ${#falta[@]} -gt 0 ]; then
  echo " Faltan dependencias de compilacion:"
  echo
  echo "   sudo apt install build-essential ${falta[*]}"
  echo
  echo " (en Fedora: webkit2gtk4.1-devel gtk3-devel openssl-devel libappindicator-gtk3-devel librsvg2-devel file)"
  exit 1
fi

if [ ! -d node_modules ]; then
  echo " [1/2] Instalando dependencias..."
  npm install --no-audit --no-fund
fi

echo " [2/2] Compilando..."
npm run dist:linux

echo
echo " Listo. Los paquetes estan en:"
echo "   src-tauri/target/release/bundle/deb"
echo
ls -1sh src-tauri/target/release/bundle/deb/*.deb 2>/dev/null || true

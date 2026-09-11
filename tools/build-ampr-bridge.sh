#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
bridge="$root/tools/ampr-bridge"
vendor="$bridge/vendor"
source_dir="$vendor/ampr_emu"
runtime="$vendor/libSceAmpr.sprx"
commit="cfa85df379f6eeeb165d7badf9b648e266fe77b7"
runtime_sha="69e6c4d5e4f5fb83c9e01815db5861c4c75734acbf4595cafa50d4c218116d1a"
runtime_url="https://github.com/drakmor/ampr_emu/releases/download/0.4.2.1/libSceAmpr.sprx-0.4.2.1-test-pack"

mkdir -p "$vendor"
if [ ! -d "$source_dir/.git" ]; then
  git clone --filter=blob:none https://github.com/drakmor/ampr_emu.git "$source_dir"
fi
git -C "$source_dir" fetch --depth 1 origin "$commit"
git -C "$source_dir" checkout --detach "$commit"
test "$(git -C "$source_dir" rev-parse HEAD)" = "$commit"

if [ ! -f "$runtime" ] || [ "$(sha256sum "$runtime" | cut -d' ' -f1)" != "$runtime_sha" ]; then
  curl -fL "$runtime_url" -o "$runtime"
fi
echo "$runtime_sha  $runtime" | sha256sum --check --status

venv="$bridge/.venv-build-linux"
if [ ! -x "$venv/bin/python" ]; then
  python3 -m venv "$venv"
fi
"$venv/bin/python" -m pip install --disable-pip-version-check "lz4==4.4.4" "pyinstaller==6.20.0"

dist="$bridge/dist/linux"
work="$bridge/build/linux"
"$venv/bin/python" -m PyInstaller --noconfirm --clean --onefile \
  --name romforge-ampr-bridge \
  --distpath "$dist" --workpath "$work" --specpath "$work" \
  --paths "$source_dir/tools" \
  --add-data "$runtime:." \
  "$bridge/romforge_ampr_bridge.py"

cp "$dist/romforge-ampr-bridge" "$root/src-tauri/binaries/romforge-ampr-bridge"
cp "$bridge/THIRD_PARTY_NOTICES.txt" "$root/src-tauri/binaries/LICENCIA-AMPR-Bridge.txt"
cp "$source_dir/LICENSE" "$root/src-tauri/binaries/GPL-3.0-AMPR-Emu.txt"
cp "$source_dir/third_party/lz4/LICENSE" "$root/src-tauri/binaries/BSD-2-Clause-LZ4.txt"
chmod +x "$root/src-tauri/binaries/romforge-ampr-bridge"
echo "AMPR Bridge: $root/src-tauri/binaries/romforge-ampr-bridge"

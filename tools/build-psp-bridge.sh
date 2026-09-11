#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
bridge="$root/tools/psp-bridge"
source_dir="$bridge/vendor/LibOrbisPkg"
commit="643477263b2644e0803e0f58b8726ea4e3f3b7d4"

mkdir -p "$(dirname "$source_dir")"
if [ ! -d "$source_dir/.git" ]; then
  git clone --filter=blob:none https://github.com/maxton/LibOrbisPkg.git "$source_dir"
fi
git -C "$source_dir" fetch --depth 1 origin "$commit"
git -C "$source_dir" checkout --detach "$commit"
test "$(git -C "$source_dir" rev-parse HEAD)" = "$commit"

sed -i 's#<TargetFramework>netcoreapp3.0</TargetFramework>#<TargetFramework>net8.0</TargetFramework>#' "$source_dir/LibOrbisPkg.Core/LibOrbisPkg.Core.csproj"
python3 - "$source_dir/LibOrbisPkg/PKG/PkgBuilder.cs" <<'PY'
from pathlib import Path
import sys
path = Path(sys.argv[1])
text = path.read_text()
text = text.replace(
    "e.DataSize = (uint)(pkgSize / 0x10000L) * 4;",
    "e.DataSize = (uint)(pkgSize / 0x10000L) * 4 + 0x10000; // ROMForge: soporte para paquetes grandes",
)
path.write_text(text)
PY

publish="$bridge/publish/linux"
dotnet publish "$bridge/PspBridge.csproj" -c Release -r linux-x64 --self-contained true -o "$publish"
cp "$publish/romforge-psp-bridge" "$root/src-tauri/binaries/romforge-psp-bridge"
cp "$bridge/THIRD_PARTY_NOTICES.txt" "$root/src-tauri/binaries/LICENCIA-PSP-Bridge.txt"
cp "$source_dir/LICENSE.txt" "$root/src-tauri/binaries/LGPL-3.0-LibOrbisPkg.txt"
chmod +x "$root/src-tauri/binaries/romforge-psp-bridge"
echo "PSP Bridge: $root/src-tauri/binaries/romforge-psp-bridge"

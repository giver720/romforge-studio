#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
bridge="$root/tools/prospero-bridge"
vendor="$bridge/vendor"
source="$vendor/LibProsperoPKG-src"
repository="https://github.com/SvenGDK/LibProsperoPKG.git"
commit="748eabf1b7d17819528cabf367d8e27109d8fce3"

mkdir -p "$vendor"
if [ ! -d "$source/.git" ]; then
  git clone --filter=blob:none --no-checkout "$repository" "$source"
fi
git -C "$source" fetch --depth 1 origin "$commit"
git -C "$source" checkout --detach "$commit"
test "$(git -C "$source" rev-parse HEAD)" = "$commit"
cp "$source/LICENSE" "$vendor/LICENSE"
cp "$source/NOTICE" "$vendor/NOTICE"

publish="$bridge/publish/linux-x64"
dotnet publish "$bridge/ProsperoBridge.csproj" -c Release -r linux-x64 --self-contained true -o "$publish"
cp "$publish/romforge-prospero-bridge" "$root/src-tauri/binaries/romforge-prospero-bridge"
cp "$bridge/THIRD_PARTY_NOTICES.txt" "$root/src-tauri/binaries/LICENCIA-Prospero-Bridge.txt"
cp "$vendor/LICENSE" "$root/src-tauri/binaries/GPL-3.0-LibProsperoPKG.txt"
cp "$vendor/NOTICE" "$root/src-tauri/binaries/NOTICE-LibProsperoPKG.txt"
chmod +x "$root/src-tauri/binaries/romforge-prospero-bridge"
echo "Prospero Bridge: $root/src-tauri/binaries/romforge-prospero-bridge"

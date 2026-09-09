#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
bridge="$root/tools/prospero-bridge"
vendor="$bridge/vendor"
archive="$vendor/libprosperopkg-win-x64-v2.5.zip"
expected="b4b3d290d5058a4fd408ca5c6e0207d8f4998eab3849c36d51dc5d97592fc151"
url="https://github.com/SvenGDK/LibProsperoPKG/releases/download/v2.5/libprosperopkg-win-x64.zip"
license_url="https://raw.githubusercontent.com/SvenGDK/LibProsperoPKG/3e159f2ed1d2c2c53c003f79ace378d987773fe3/LICENSE"
notice_url="https://raw.githubusercontent.com/SvenGDK/LibProsperoPKG/3e159f2ed1d2c2c53c003f79ace378d987773fe3/NOTICE"

mkdir -p "$vendor"
if [ ! -f "$archive" ] || [ "$(sha256sum "$archive" | cut -d' ' -f1)" != "$expected" ]; then
  curl -fL "$url" -o "$archive"
fi
echo "$expected  $archive" | sha256sum --check --status
unzip -oq "$archive" -d "$vendor"
curl -fL "$license_url" -o "$vendor/LICENSE"
curl -fL "$notice_url" -o "$vendor/NOTICE"

publish="$bridge/publish/linux-x64"
dotnet publish "$bridge/ProsperoBridge.csproj" -c Release -r linux-x64 --self-contained true -o "$publish"
cp "$publish/romforge-prospero-bridge" "$root/src-tauri/binaries/romforge-prospero-bridge"
cp "$bridge/THIRD_PARTY_NOTICES.txt" "$root/src-tauri/binaries/LICENCIA-Prospero-Bridge.txt"
cp "$vendor/LICENSE" "$root/src-tauri/binaries/GPL-3.0-LibProsperoPKG.txt"
cp "$vendor/NOTICE" "$root/src-tauri/binaries/NOTICE-LibProsperoPKG.txt"
chmod +x "$root/src-tauri/binaries/romforge-prospero-bridge"
echo "Prospero Bridge: $root/src-tauri/binaries/romforge-prospero-bridge"

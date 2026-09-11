#!/usr/bin/env python3
"""ROMForge command-line bridge for drakmor's AMPRPAK4/LZ4 tools.

This program is intentionally distributed as a separate GPL-3.0 executable.
It prepares a compact, deployable /app0 copy without modifying the source dump.
"""

from __future__ import annotations

import argparse
from dataclasses import asdict
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
from typing import Any

import ampr_pack
import build_ampr_index


BRIDGE_VERSION = "1.0.0"
UPSTREAM_VERSION = "0.4.2.1"
UPSTREAM_COMMIT = "cfa85df379f6eeeb165d7badf9b648e266fe77b7"
RUNTIME_NAME = "libSceAmpr.sprx"
RECEIPT_NAME = "ROMFORGE-LZ4.json"


def bundled_file(name: str) -> Path:
    root = Path(getattr(sys, "_MEIPASS", Path(__file__).resolve().parent))
    candidate = root / name
    if not candidate.is_file():
        raise RuntimeError(f"missing bundled file: {name}")
    return candidate


def emit(event: str, **values: Any) -> None:
    print(json.dumps({"event": event, **values}, ensure_ascii=False), flush=True)


def reject_unsafe_tree(root: Path) -> None:
    for current, directories, files in os.walk(root, followlinks=False):
        for name in [*directories, *files]:
            path = Path(current) / name
            if path.is_symlink() or (getattr(path.lstat(), "st_file_attributes", 0) & 0x400):
                raise RuntimeError(f"symlink/reparse points are not supported: {path}")


def ensure_separate_paths(source: Path, output: Path) -> None:
    source_real = source.resolve(strict=True)
    output_real = output.resolve(strict=False)
    try:
        common = Path(os.path.commonpath((source_real, output_real)))
    except ValueError as exc:
        raise RuntimeError("source and output paths cannot be compared safely") from exc
    if common == source_real or common == output_real:
        raise RuntimeError("output must not contain the source or be inside it")


def clean_old_pack_set(root: Path) -> None:
    names = {
        "ampr_emu.index",
        "ampr_assets.index",
        "ampr_assets.index.crc",
        "ampr_assets.index.runtime",
        RECEIPT_NAME,
    }
    for path in root.iterdir():
        lower = path.name.lower()
        if lower in {name.lower() for name in names} or (
            lower.startswith("ampr_assets-") and lower.endswith(".pak")
        ):
            if path.is_dir():
                shutil.rmtree(path)
            else:
                path.unlink()


def profile_text(profile: str) -> str:
    if profile == "fast":
        compression_mode, level, workers = "fast", 1, min(12, os.cpu_count() or 4)
    elif profile == "maximum":
        compression_mode, level, workers = "hc", 12, min(8, os.cpu_count() or 4)
    else:
        compression_mode, level, workers = "hc", 9, min(8, os.cpu_count() or 4)

    return f'''[runtime]
decoded_cache_bytes = "64MiB"
physical_cache_bytes = "32MiB"
workers = 4
latency_reserve_workers = 1

[pack]
index_name = "ampr_assets.index"
pack_pattern = "ampr_assets-{{group}}-lane{{lane:02d}}-vol{{volume:02d}}-{{id:03d}}.pak"
default_action = "compress"
default_block_size = "64KiB"
io_page_size = "64KiB"
payload_alignment = "64KiB"
chunk_alignment = "64B"
workers = {workers}
compression_mode = "{compression_mode}"
compression_level = {level}
acceleration = 1
deduplicate = true
deduplicate_scope = "lane"
min_savings_bytes = 64
min_savings_ratio = 0.01
io_neutral_min_savings_bytes = "8KiB"
io_neutral_min_savings_ratio = 0.125
auto_loose_large_files = true
auto_loose_hot_files = false
auto_loose_min_file_size = "64MiB"
auto_loose_sample_blocks = 32
auto_loose_sample_bytes = "16MiB"
auto_loose_min_savings_ratio = 0.05
auto_loose_max_raw_ratio = 0.90
preserve_mtime = true
validate_index_metadata = true

[groups.default]
pack_count = 4
assignment = "balanced"
max_pack_size = "16GiB"
stripe_large_files = false
io_page_size = "64KiB"

# These paths must remain ordinary files. AMPR's removal verifier enforces the
# same protection independently, so a malformed profile cannot delete them.
[[rule]]
action = "loose"
include = [
  "eboot.bin",
  "sce_sys/**",
  "sce_module/**",
  "system/**",
  "mods/**",
  "save/**",
  "fakelib/**",
  "decrypted/**",
  "**/*.elf",
  "**/*.prx",
  "**/*.self",
  "**/*.sprx",
  "ampr_emu.index",
  "ampr_assets.index",
  "ampr_assets.index.crc",
  "ampr_assets.index.runtime",
  "ampr_assets-*.pak",
  "{RECEIPT_NAME}",
]
'''


def directory_size(root: Path) -> int:
    total = 0
    for path in root.rglob("*"):
        if path.is_file():
            total += path.stat().st_size
    return total


def convert(source_arg: str, output_arg: str, profile: str) -> int:
    source = Path(source_arg).resolve(strict=True)
    output = Path(output_arg).resolve(strict=False)
    if not source.is_dir():
        raise RuntimeError("input is not a directory")
    if not (source / "eboot.bin").is_file() or not (source / "sce_sys" / "param.json").is_file():
        raise RuntimeError("input is not a PS5 dump root (eboot.bin and sce_sys/param.json required)")
    if output.exists():
        raise RuntimeError(f"output already exists: {output}")
    ensure_separate_paths(source, output)
    reject_unsafe_tree(source)

    emit("phase", progress=2, message="Copying the source dump")
    try:
        shutil.copytree(source, output, copy_function=shutil.copy2)
        clean_old_pack_set(output)

        fakelib = output / "fakelib"
        fakelib.mkdir(parents=True, exist_ok=True)
        shutil.copy2(bundled_file(RUNTIME_NAME), fakelib / RUNTIME_NAME)

        emit("phase", progress=12, message="Building AMPRIDX3 index")
        ampr_index = output / "ampr_emu.index"
        status = build_ampr_index.build_index_local(output, ampr_index, False)
        if status != 0 or not ampr_index.is_file():
            raise RuntimeError("AMPRIDX3 index generation failed")

        with tempfile.TemporaryDirectory(prefix="romforge-ampr-profile-") as temporary:
            config_path = Path(temporary) / "romforge-lz4.toml"
            config_path.write_text(profile_text(profile), encoding="utf-8")
            config = ampr_pack.load_config(config_path)

            emit("phase", progress=18, message="Compressing seekable LZ4 blocks")
            index_path, stats, warnings = ampr_pack.build_packs(
                root=output,
                ampr_index=ampr_index,
                output_dir=output,
                config=config,
                include_patterns=[],
                exclude_patterns=[],
                allow_missing=False,
                progress=ampr_pack.ConsoleBuildProgress(),
            )

        emit("phase", progress=82, message="Comparing packed data with source files")
        verified_before = ampr_pack.verify_packs(index_path)
        source_compare = ampr_pack.verify_packs_against_root(index_path, output)
        removal = ampr_pack.remove_packed_sources(index_path, output, remove_empty_dirs=True)

        emit("phase", progress=94, message="Validating the compact deployment set")
        verified_after = ampr_pack.verify_packs(index_path)
        manifest = ampr_pack.load_manifest(index_path)
        receipt = {
            "format": "AMPRPAK4",
            "codec": "LZ4 raw blocks",
            "bridge_version": BRIDGE_VERSION,
            "ampr_emu_version": UPSTREAM_VERSION,
            "ampr_emu_commit": UPSTREAM_COMMIT,
            "profile": profile,
            "build_id": manifest.build_id.hex(),
            "stats": asdict(stats),
            "warnings": warnings,
            "verified_before_removal": verified_before,
            "source_compare": source_compare,
            "removed_from_output": removal,
            "verified_after_removal": verified_after,
            "final_bytes": directory_size(output),
            "source_untouched": True,
            "hardware_test_required": True,
        }
        (output / RECEIPT_NAME).write_text(
            json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True),
            encoding="utf-8",
        )
        emit("complete", progress=100, output=str(output), receipt=receipt)
        return 0
    except Exception:
        shutil.rmtree(output, ignore_errors=True)
        raise


def verify(root_arg: str) -> int:
    root = Path(root_arg).resolve(strict=True)
    index_path = root / "ampr_assets.index"
    if not (root / "ampr_emu.index").is_file():
        raise RuntimeError("ampr_emu.index is missing")
    if not (root / "fakelib" / RUNTIME_NAME).is_file():
        raise RuntimeError(f"fakelib/{RUNTIME_NAME} is missing")
    result = ampr_pack.verify_packs(index_path)
    manifest = ampr_pack.load_manifest(index_path)
    emit(
        "verified",
        format="AMPRPAK4",
        codec="LZ4 raw blocks",
        build_id=manifest.build_id.hex(),
        result=result,
    )
    return 0


def unpack(root_arg: str, output_arg: str) -> int:
    root = Path(root_arg).resolve(strict=True)
    output = Path(output_arg).resolve(strict=False)
    if output.exists():
        raise RuntimeError(f"output already exists: {output}")
    shutil.copytree(root, output)
    result = ampr_pack.extract_packs(
        root / "ampr_assets.index", output, overwrite=True, preserve_mtime=True
    )
    for path in list(output.glob("ampr_assets-*.pak")):
        path.unlink()
    for name in ["ampr_assets.index", "ampr_assets.index.crc", "ampr_assets.index.runtime", RECEIPT_NAME]:
        (output / name).unlink(missing_ok=True)
    emit("unpacked", output=str(output), result=result)
    return 0


def create_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="ROMForge AMPRPAK4/LZ4 bridge")
    parser.add_argument("--version", action="store_true")
    commands = parser.add_subparsers(dest="command")
    conversion = commands.add_parser("convert", help="create a compact AMPRPAK4/LZ4 app0 directory")
    conversion.add_argument("--input", required=True)
    conversion.add_argument("--output", required=True)
    conversion.add_argument("--profile", choices=("fast", "balanced", "maximum"), default="balanced")
    validation = commands.add_parser("verify", help="verify a compact AMPRPAK4/LZ4 directory")
    validation.add_argument("--input", required=True)
    extraction = commands.add_parser("unpack", help="restore packed files into a normal folder")
    extraction.add_argument("--input", required=True)
    extraction.add_argument("--output", required=True)
    return parser


def main() -> int:
    parser = create_parser()
    args = parser.parse_args()
    if args.version:
        print(
            f"ROMForge AMPR Bridge {BRIDGE_VERSION} · ampr_emu {UPSTREAM_VERSION} "
            f"({UPSTREAM_COMMIT[:10]})"
        )
        return 0
    try:
        if args.command == "convert":
            return convert(args.input, args.output, args.profile)
        if args.command == "verify":
            return verify(args.input)
        if args.command == "unpack":
            return unpack(args.input, args.output)
        parser.error("a command is required")
    except (OSError, RuntimeError, ValueError, ampr_pack.PackError) as exc:
        print(f"error: {exc}", file=sys.stderr, flush=True)
        return 2
    return 2


if __name__ == "__main__":
    raise SystemExit(main())

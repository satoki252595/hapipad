#!/usr/bin/env python3
"""Collect Dioxus desktop bundles and rename them for the GitHub Release."""

from __future__ import annotations

import os
import shutil
import sys
import tarfile
import zipfile
from pathlib import Path


def copy_first(dest: Path, candidates: list[Path]) -> Path | None:
    for path in candidates:
        if path.is_file():
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, dest)
            print(f"Copied {path} -> {dest}")
            return dest
    return None


def zip_dir(src: Path, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.exists():
        dest.unlink()
    with zipfile.ZipFile(dest, "w", zipfile.ZIP_DEFLATED) as zf:
        for file in src.rglob("*"):
            if file.is_file():
                zf.write(file, file.relative_to(src.parent))
    print(f"Zipped {src} -> {dest}")


def tar_dir(src: Path, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(dest, "w:gz") as tf:
        tf.add(src, arcname=src.name)
    print(f"Tarball {src} -> {dest}")


def main() -> int:
    target = os.environ.get("TARGET", "")
    search_root = Path("target") / "dx"
    upload = Path("dist") / "upload"
    upload.mkdir(parents=True, exist_ok=True)

    if not search_root.exists():
        print(f"{search_root} does not exist; desktop bundle likely failed")
        return 0

    files = [p for p in search_root.rglob("*") if p.is_file()]
    apps = [p for p in search_root.rglob("*.app") if p.is_dir()]

    if target == "linux":
        copy_first(
            upload / "md-block-editor-linux-x86_64.AppImage",
            [p for p in files if p.suffix == ".AppImage" or p.name.endswith(".AppImage")],
        )
        copy_first(
            upload / "md-block-editor-linux-x86_64.deb",
            [p for p in files if p.suffix == ".deb"],
        )
        if not any(upload.iterdir()):
            bins = [
                p
                for p in files
                if p.name in {"blockpad", "md-block-editor"} and os.access(p, os.X_OK)
            ]
            copy_first(upload / "md-block-editor-linux-x86_64", bins)
    elif target == "windows":
        copy_first(
            upload / "md-block-editor-windows-x64-setup.exe",
            [p for p in files if p.name.endswith("-setup.exe")],
        )
        copy_first(
            upload / "md-block-editor-windows-x64.msi",
            [p for p in files if p.suffix.lower() == ".msi"],
        )
        if not any(p.suffix.lower() == ".exe" for p in upload.glob("*")):
            exes = [
                p
                for p in files
                if p.suffix.lower() == ".exe"
                and p.name.lower() in {"blockpad.exe", "md-block-editor.exe"}
            ]
            copy_first(upload / "md-block-editor-windows-x64.exe", exes)
    elif target == "macos":
        copy_first(
            upload / "md-block-editor-macos-arm64.dmg",
            [p for p in files if p.suffix == ".dmg"],
        )
        if apps:
            zip_path = upload / "md-block-editor-macos-arm64.app.zip"
            try:
                zip_dir(apps[0], zip_path)
            except OSError:
                tar_dir(apps[0], upload / "md-block-editor-macos-arm64.app.tgz")
    else:
        print(f"Unknown TARGET={target}")

    leftover = list(upload.iterdir())
    if leftover:
        print("Staged:")
        for path in leftover:
            print(f"  {path} ({path.stat().st_size} bytes)")
    else:
        print("No desktop artifacts to upload")
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Locate Dioxus web public dirs, zip the root-relative build, and stage Pages."""

from __future__ import annotations

import shutil
import sys
from pathlib import Path


def find_index(root: Path) -> Path:
    matches = sorted(root.rglob("index.html"))
    if not matches:
        raise SystemExit(f"No index.html under {root}")
    # Prefer a public/ folder produced by dx bundle --out-dir.
    for path in matches:
        if path.parent.name == "public":
            return path
    return matches[0]


def stage_site(src: Path, dest: Path) -> None:
    if dest.exists():
        shutil.rmtree(dest)
    shutil.copytree(src, dest)
    shutil.copyfile(dest / "index.html", dest / "404.html")
    (dest / ".nojekyll").write_text("", encoding="utf-8")


def main() -> int:
    repo = Path.cwd()
    zip_root = find_index(repo / "dist" / "web-zip").parent
    pages_root = find_index(repo / "dist" / "web-pages").parent

    upload = repo / "dist" / "upload"
    pages = repo / "dist" / "pages"
    upload.mkdir(parents=True, exist_ok=True)

    archive = shutil.make_archive(
        str(upload / "md-block-editor-web"), "zip", root_dir=zip_root
    )
    print(f"Wrote {archive} from {zip_root}")
    stage_site(pages_root, pages)
    print(f"Staged Pages site at {pages}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

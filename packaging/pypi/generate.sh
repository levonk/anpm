#!/usr/bin/env bash
# PyPI package generator for apmw.
#
# Produces a Python package (pyproject.toml + setup.py shim) that wraps the
# pre-built Rust binary. On `pip install apmw` a postinstall hook downloads
# the platform-appropriate archive from the GitHub release and installs a
# console_script entry point that execs the binary.
#
# Usage:   ./generate.sh [version] [output-dir]
# Example: ./generate.sh 0.1.0 ./dist
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../metadata.sh
source "$SCRIPT_DIR/../metadata.sh"

VERSION="$(strip_v "${1:-$APMW_VERSION}")"
OUTPUT_DIR="${2:-$SCRIPT_DIR/dist}"

mkdir -p "$OUTPUT_DIR/apmw_bin"

# --- pyproject.toml ------------------------------------------------------
cat > "$OUTPUT_DIR/pyproject.toml" <<EOF
[build-system]
requires = ["setuptools>=61.0", "wheel"]
build-backend = "setuptools.build_meta"

[project]
name = "apmw"
version = "${VERSION}"
description = "${APMW_DESCRIPTION}"
readme = "README.md"
license = { text = "${APMW_LICENSE}" }
authors = [{ name = "${APMW_AUTHORS}" }]
requires-python = ">=3.8"
keywords = ["package-manager", "wrapper", "cli", "devbox", "nix"]
classifiers = [
    "Development Status :: 4 - Beta",
    "Environment :: Console",
    "Intended Audience :: Developers",
    "License :: OSI Approved :: MIT License",
    "Operating System :: OS Independent",
    "Programming Language :: Rust",
    "Programming Language :: Python :: 3",
    "Topic :: Software Development",
]
urls = { Homepage = "${APMW_HOMEPAGE}", Repository = "${APMW_REPOSITORY}" }

[project.scripts]
apmw = "apmw_bin.cli:main"

[tool.setuptools]
packages = ["apmw_bin"]
include-package-data = true
EOF

# --- apmw_bin/__init__.py ------------------------------------------------
cat > "$OUTPUT_DIR/apmw_bin/__init__.py" <<'PYINIT'
"""apmw — All Package Manager Wrapper (binary wrapper package)."""
__version__ = "0.1.0"
PYINIT

# --- apmw_bin/cli.py (console_script entry point) ------------------------
cat > "$OUTPUT_DIR/apmw_bin/cli.py" <<'PYCLI'
"""Console-script entry point that execs the pre-built apmw binary."""
from __future__ import annotations

import os
import sys
import platform
import subprocess
from pathlib import Path


def _target_triple() -> str:
    system = platform.system().lower()
    machine = platform.machine().lower()
    if system == "linux" and machine in ("x86_64", "amd64"):
        return "x86_64-unknown-linux-gnu"
    if system == "linux" and machine in ("aarch64", "arm64"):
        return "aarch64-unknown-linux-gnu"
    if system == "darwin" and machine in ("x86_64", "amd64"):
        return "x86_64-apple-darwin"
    if system == "darwin" and machine in ("aarch64", "arm64"):
        return "aarch64-apple-darwin"
    if system == "windows" and machine in ("x86_64", "amd64"):
        return "x86_64-pc-windows-msvc"
    raise RuntimeError(f"Unsupported platform: {system} {machine}")


def _binary_path() -> Path:
    here = Path(__file__).resolve().parent
    name = "apmw.exe" if os.name == "nt" else "apmw"
    return here / name


def main() -> int:
    binary = _binary_path()
    if not binary.exists():
        sys.stderr.write(
            "apmw binary not found. Re-run pip install to trigger the "
            "postinstall download, or install manually from "
            "https://github.com/levonk/apmw/releases\n"
        )
        return 1
    result = subprocess.run([str(binary), *sys.argv[1:]])
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
PYCLI

# --- apmw_bin/download.py (postinstall helper) ---------------------------
cat > "$OUTPUT_DIR/apmw_bin/download.py" <<'PYDL'
"""Postinstall downloader for the pre-built apmw binary."""
from __future__ import annotations

import os
import sys
import stat
import tarfile
import zipfile
import urllib.request
import platform
from pathlib import Path


VERSION = "0.1.0"
REPO = "levonk/apmw"
BASE = f"https://github.com/{REPO}/releases/download/v{VERSION}"


def _target() -> str:
    system = platform.system().lower()
    machine = platform.machine().lower()
    if system == "linux" and machine in ("x86_64", "amd64"):
        return "x86_64-unknown-linux-gnu"
    if system == "linux" and machine in ("aarch64", "arm64"):
        return "aarch64-unknown-linux-gnu"
    if system == "darwin" and machine in ("x86_64", "amd64"):
        return "x86_64-apple-darwin"
    if system == "darwin" and machine in ("aarch64", "arm64"):
        return "aarch64-apple-darwin"
    if system == "windows" and machine in ("x86_64", "amd64"):
        return "x86_64-pc-windows-msvc"
    raise RuntimeError(f"Unsupported platform: {system} {machine}")


def run() -> None:
    target = _target()
    ext = "zip" if "windows" in target else "tar.gz"
    url = f"{BASE}/apmw-{VERSION}-{target}.{ext}"
    here = Path(__file__).resolve().parent
    name = "apmw.exe" if os.name == "nt" else "apmw"
    dest = here / name
    tmp = here / f"apmw-{VERSION}-{target}.{ext}"
    print(f"Downloading {url}")
    urllib.request.urlretrieve(url, tmp)
    if ext == "zip":
        with zipfile.ZipFile(tmp) as zf:
            zf.extractall(here)
    else:
        with tarfile.open(tmp, "r:gz") as tf:
            tf.extractall(here)
    tmp.unlink(missing_ok=True)
    dest.chmod(dest.stat().st_mode | stat.S_IEXEC | stat.S_IXGRP | stat.S_IXOTH)
    print(f"apmw {VERSION} installed to {dest}")


if __name__ == "__main__":
    run()
PYDL

# --- setup.py (shim that runs the downloader post-build) -----------------
cat > "$OUTPUT_DIR/setup.py" <<'PYSETUP'
"""setup.py shim — delegates to pyproject.toml and runs postinstall download."""
from __future__ import annotations

import sys
from setuptools import setup

try:
    from apmw_bin.download import run as _download
except Exception:  # pragma: no cover
    _download = None

setup()

# Run the binary download after install completes.
if _download is not None and "install" in " ".join(sys.argv):
    try:
        _download()
    except Exception as exc:  # pragma: no cover
        sys.stderr.write(f"apmw postinstall download failed: {exc}\n")
PYSETUP

cat > "$OUTPUT_DIR/README.md" <<EOF
# apmw (PyPI wrapper)

${APMW_DESCRIPTION}

This PyPI package wraps the pre-built Rust binary. On \`pip install apmw\`
the postinstall hook downloads the binary for your platform from the
[GitHub release](${APMW_REPOSITORY}/releases).

## Usage

\`\`\`
apmw --help
\`\`\`
EOF

echo "Generated PyPI package in $OUTPUT_DIR"
echo "  pyproject.toml, setup.py, apmw_bin/{__init__,cli,download}.py, README.md"

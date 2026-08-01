#!/usr/bin/env bash
# npm/pnpm package generator for apmw.
#
# Produces an npm package that wraps the pre-built Rust binary. On `npm install`
# a postinstall script downloads the platform-appropriate archive from the
# GitHub release and places the binary on PATH.
#
# Usage:   ./generate.sh [version] [output-dir]
# Example: ./generate.sh 0.1.0 ./dist
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../metadata.sh
source "$SCRIPT_DIR/../metadata.sh"

VERSION="$(strip_v "${1:-$APMW_VERSION}")"
OUTPUT_DIR="${2:-$SCRIPT_DIR/dist}"

mkdir -p "$OUTPUT_DIR"

# --- package.json --------------------------------------------------------
cat > "$OUTPUT_DIR/package.json" <<EOF
{
  "name": "apmw",
  "version": "${VERSION}",
  "description": "${APMW_DESCRIPTION}",
  "license": "${APMW_LICENSE}",
  "repository": {
    "type": "git",
    "url": "${APMW_REPOSITORY}.git"
  },
  "homepage": "${APMW_HOMEPAGE}",
  "keywords": [
    "package-manager",
    "wrapper",
    "cli",
    "devbox",
    "nix",
    "install-on-use"
  ],
  "bin": {
    "apmw": "bin/apmw"
  },
  "files": [
    "bin/",
    "install.js",
    "README.md"
  ],
  "scripts": {
    "postinstall": "node install.js"
  },
  "engines": {
    "node": ">=14"
  }
}
EOF

# --- install.js (postinstall downloader) ---------------------------------
cat > "$OUTPUT_DIR/install.js" <<'INSTALLJS'
#!/usr/bin/env node
// apmw postinstall: downloads the pre-built Rust binary for the current
// platform from the GitHub release and installs it into bin/.
const https = require("https");
const fs = require("fs");
const path = require("path");
const os = require("os");
const { execSync } = require("child_process");

const VERSION = require("./package.json").version;
const REPO = "levonk/apmw";
const BASE_URL = `https://github.com/${REPO}/releases/download/v${VERSION}`;

function targetTriple() {
  const platform = os.platform();
  const arch = os.arch();
  if (platform === "linux" && arch === "x64") return "x86_64-unknown-linux-gnu";
  if (platform === "linux" && arch === "arm64") return "aarch64-unknown-linux-gnu";
  if (platform === "darwin" && arch === "x64") return "x86_64-apple-darwin";
  if (platform === "darwin" && arch === "arm64") return "aarch64-apple-darwin";
  if (platform === "win32" && arch === "x64") return "x86_64-pc-windows-msvc";
  throw new Error(`Unsupported platform: ${platform} ${arch}`);
}

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    const req = (u) =>
      https.get(u, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          res.resume();
          return req(res.headers.location);
        }
        if (res.statusCode !== 200) {
          reject(new Error(`HTTP ${res.statusCode} for ${u}`));
          return;
        }
        res.pipe(file);
        file.on("finish", () => file.close(resolve));
      });
    req(url).on("error", reject);
  });
}

async function main() {
  const target = targetTriple();
  const ext = target.includes("windows") ? "zip" : "tar.gz";
  const url = `${BASE_URL}/apmw-${VERSION}-${target}.${ext}`;
  const binDir = path.join(__dirname, "bin");
  fs.mkdirSync(binDir, { recursive: true });
  const tmp = path.join(os.tmpdir(), `apmw-${VERSION}-${target}.${ext}`);
  console.log(`Downloading ${url}`);
  await download(url, tmp);
  if (ext === "zip") {
    execSync(`powershell -Command "Expand-Archive -Force -Path '${tmp}' -DestinationPath '${binDir}'"`, { stdio: "inherit" });
  } else {
    execSync(`tar -xzf "${tmp}" -C "${binDir}"`, { stdio: "inherit" });
  }
  const bin = path.join(binDir, target.includes("windows") ? "apmw.exe" : "apmw");
  fs.chmodSync(bin, 0o755);
  console.log(`apmw ${VERSION} installed to ${bin}`);
}

main().catch((err) => {
  console.error("apmw postinstall failed:", err.message);
  process.exit(1);
});
INSTALLJS

mkdir -p "$OUTPUT_DIR/bin"
cat > "$OUTPUT_DIR/bin/apmw" <<'WRAPPER'
#!/usr/bin/env node
// Shim that execs the real platform binary placed by postinstall.
const { spawn } = require("child_process");
const path = require("path");
const os = require("os");
const fs = require("fs");

function realBinary() {
  const platform = os.platform();
  const arch = os.arch();
  let target;
  if (platform === "linux" && arch === "x64") target = "x86_64-unknown-linux-gnu";
  else if (platform === "linux" && arch === "arm64") target = "aarch64-unknown-linux-gnu";
  else if (platform === "darwin" && arch === "x64") target = "x86_64-apple-darwin";
  else if (platform === "darwin" && arch === "arm64") target = "aarch64-apple-darwin";
  else if (platform === "win32" && arch === "x64") target = "x86_64-pc-windows-msvc";
  else throw new Error(`Unsupported platform: ${platform} ${arch}`);
  const name = platform === "win32" ? "apmw.exe" : "apmw";
  return path.join(__dirname, name);
}

const bin = realBinary();
if (!fs.existsSync(bin)) {
  console.error("apmw binary not found. Run `npm install` to download it.");
  process.exit(1);
}
const child = spawn(bin, process.argv.slice(2), { stdio: "inherit" });
child.on("exit", (code) => process.exit(code || 0));
WRAPPER
chmod +x "$OUTPUT_DIR/bin/apmw"

cat > "$OUTPUT_DIR/README.md" <<EOF
# apmw (npm wrapper)

${APMW_DESCRIPTION}

This npm package wraps the pre-built Rust binary. On \`npm install\` the
postinstall script downloads the binary for your platform from the
[GitHub release](${APMW_REPOSITORY}/releases).

## Usage

\`\`\`
npx apmw --help
\`\`\`
EOF

echo "Generated npm package in $OUTPUT_DIR"
echo "  package.json, install.js, bin/apmw, README.md"

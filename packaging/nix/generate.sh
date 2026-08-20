#!/usr/bin/env bash
# Nix derivation generator for apmw.
#
# Produces a Nix derivation (default.nix) that downloads the pre-built Rust
# binary for the host platform from the GitHub release and installs it.
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

cat > "$OUTPUT_DIR/default.nix" <<EOF
# apmw Nix derivation — wraps the pre-built Rust binary.
# Build with: nix-build default.nix
# The derivation downloads the platform-appropriate archive from the GitHub
# release and installs the \`apmw\` binary into the Nix store.
{ lib
, stdenv
, fetchurl
, autoPatchelfHook
, installShellFiles
, cacert
}:

let
  version = "${VERSION}";
  repo = "levonk/apmw";
  homepage = "${APMW_HOMEPAGE}";
  description = "${APMW_DESCRIPTION}";
  license = lib.licenses.mit;

  # Per-platform asset metadata.
  assets = {
    x86_64-linux = {
      url = "https://github.com/\${repo}/releases/download/v\${version}/apmw-\${version}-x86_64-unknown-linux-gnu.tar.gz";
      sha256 = lib.fakeSha256;
    };
    aarch64-linux = {
      url = "https://github.com/\${repo}/releases/download/v\${version}/apmw-\${version}-aarch64-unknown-linux-gnu.tar.gz";
      sha256 = lib.fakeSha256;
    };
    x86_64-darwin = {
      url = "https://github.com/\${repo}/releases/download/v\${version}/apmw-\${version}-x86_64-apple-darwin.tar.gz";
      sha256 = lib.fakeSha256;
    };
    aarch64-darwin = {
      url = "https://github.com/\${repo}/releases/download/v\${version}/apmw-\${version}-aarch64-apple-darwin.tar.gz";
      sha256 = lib.fakeSha256;
    };
  };

  asset =
    assets.\${stdenv.hostPlatform.system}
      or (throw "apmw: unsupported system \${stdenv.hostPlatform.system}");

in
stdenv.mkDerivation {
  pname = "apmw";
  inherit version;

  src = fetchurl {
    url = asset.url;
    sha256 = asset.sha256;
  };

  nativeBuildInputs = [ installShellFiles ]
    ++ lib.optionals stdenv.isLinux [ autoPatchelfHook ];
  buildInputs = lib.optionals stdenv.isLinux [ stdenv.cc.cc.lib ];

  sourceRoot = ".";

  dontConfigure = true;
  dontBuild = true;

  installPhase = ''
    runHook preInstall
    install -Dm755 apmw \$out/bin/apmw
    installShellCompletion --cmd apmw \\
      --bash <(\$out/bin/apmw completions bash 2>/dev/null || true) \\
      --zsh <(\$out/bin/apmw completions zsh 2>/dev/null || true) \\
      --fish <(\$out/bin/apmw completions fish 2>/dev/null || true)
    runHook postInstall
  '';

  meta = {
    inherit description homepage license;
    mainProgram = "apmw";
    platforms = builtins.attrNames assets;
    maintainers = [ lib.maintainers.levonk or (lib.maintainers.\${"levonk"} or null) ];
  };
}
EOF

cat > "$OUTPUT_DIR/flake.nix" <<EOF
# apmw Nix flake — exposes the pre-built binary wrapper as a package.
{
  description = "${APMW_DESCRIPTION}";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        apmw = pkgs.callPackage ./default.nix { };
      in
      {
        packages.apmw = apmw;
        packages.default = apmw;
        apps.apmw = {
          type = "app";
          program = "\${apmw}/bin/apmw";
        };
        apps.default = self.apps.\${system}.apmw;
      });
}
EOF

cat > "$OUTPUT_DIR/README.md" <<EOF
# apmw Nix package

${APMW_DESCRIPTION}

## Build (derivation only)

\`\`\`
nix-build default.nix
\`\`\`

## Build (flake)

\`\`\`
nix build
nix run . -- --help
\`\`\`

**Note:** Replace the \`lib.fakeSha256\` placeholders with the actual SHA-256
checksums from the [release assets](${APMW_REPOSITORY}/releases) before
publishing.
EOF

echo "Generated Nix package in $OUTPUT_DIR"
echo "  default.nix, flake.nix, README.md"

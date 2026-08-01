#!/usr/bin/env bash
# Winget manifest generator for apmw.
#
# Produces a Winget package manifest (YAML) that wraps the pre-built Rust
# binary for Windows. The installer downloads the archive from the GitHub
# release and extracts the binary.
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

# Winget manifests are YAML. We emit:
#  1. The package version manifest (apmw.yaml)
#  2. The installer manifest (apmw.installer.yaml)
#  3. The default locale manifest (apmw.locale.en-US.yaml)

# --- Version manifest ----------------------------------------------------
cat > "$OUTPUT_DIR/levonk.apmw.yaml" <<EOF
# apmw Winget package version manifest
PackageIdentifier: levonk.apmw
PackageVersion: ${VERSION}
DefaultLocale: en-US
ManifestType: version
ManifestVersion: 1.5.0
Publishers:
  - levonk
Publisher: levonk
PackageLocale: en-US
PackageName: apmw
ShortDescription: ${APMW_DESCRIPTION}
License: ${APMW_LICENSE}
LicenseUrl: ${APMW_REPOSITORY}/blob/main/LICENSE
PackageUrl: ${APMW_HOMEPAGE}
PublisherUrl: ${APMW_HOMEPAGE}
PublisherSupportUrl: ${APMW_REPOSITORY}/issues
Tags:
  - package-manager
  - wrapper
  - cli
  - devbox
  - nix
Installers:
  - Architecture: x64
    InstallerType: zip
    InstallerUrl: ${APMW_REPOSITORY}/releases/download/v${VERSION}/apmw-${VERSION}-x86_64-pc-windows-msvc.zip
    InstallerSha256: 0000000000000000000000000000000000000000000000000000000000000000
    NestedInstallerType: portable
    NestedInstallerFiles:
      - RelativeFilePath: apmw.exe
        PortableCommandAlias: apmw
ManifestType: version
ManifestVersion: 1.5.0
EOF

# --- Installer manifest --------------------------------------------------
cat > "$OUTPUT_DIR/levonk.apmw.installer.yaml" <<EOF
# apmw Winget installer manifest
PackageIdentifier: levonk.apmw
PackageVersion: ${VERSION}
InstallerType: zip
Installers:
  - Architecture: x64
    InstallerUrl: ${APMW_REPOSITORY}/releases/download/v${VERSION}/apmw-${VERSION}-x86_64-pc-windows-msvc.zip
    InstallerSha256: 0000000000000000000000000000000000000000000000000000000000000000
    NestedInstallerType: portable
    NestedInstallerFiles:
      - RelativeFilePath: apmw.exe
        PortableCommandAlias: apmw
InstallerSwitches:
  Silent: /quiet
  SilentWithProgress: /passive
ManifestType: installer
ManifestVersion: 1.5.0
EOF

# --- Default locale manifest ---------------------------------------------
cat > "$OUTPUT_DIR/levonk.apmw.locale.en-US.yaml" <<EOF
# apmw Winget default locale manifest
PackageIdentifier: levonk.apmw
PackageVersion: ${VERSION}
PackageLocale: en-US
Publisher: levonk
PackageName: apmw
ShortDescription: ${APMW_DESCRIPTION}
License: ${APMW_LICENSE}
LicenseUrl: ${APMW_REPOSITORY}/blob/main/LICENSE
PackageUrl: ${APMW_HOMEPAGE}
PublisherUrl: ${APMW_HOMEPAGE}
PublisherSupportUrl: ${APMW_REPOSITORY}/issues
Tags:
  - package-manager
  - wrapper
  - cli
  - devbox
  - nix
ManifestType: defaultLocale
ManifestVersion: 1.5.0
EOF

cat > "$OUTPUT_DIR/README.md" <<EOF
# apmw Winget manifest

${APMW_DESCRIPTION}

## Install via winget

\`\`\`
winget install levonk.apmw
\`\`\`

**Note:** Replace the \`InstallerSha256\` placeholders with the actual SHA-256
checksums from the [release assets](${APMW_REPOSITORY}/releases) before
submitting to the winget-pkgs repository.
EOF

echo "Generated Winget manifests in $OUTPUT_DIR"
echo "  levonk.apmw.yaml, levonk.apmw.installer.yaml, levonk.apmw.locale.en-US.yaml"

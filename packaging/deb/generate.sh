#!/usr/bin/env bash
# Debian .deb binary package generator for apmw.
#
# Produces a ready-to-build .deb package directory that wraps the pre-built
# Rust binary. Unlike the apt/ generator (which produces a source package),
# this generator produces a binary package layout suitable for direct
# \`dpkg-deb --build\`.
#
# Usage:   ./generate.sh [version] [target] [output-dir]
# Example: ./generate.sh 0.1.0 x86_64-unknown-linux-gnu ./dist
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../metadata.sh
source "$SCRIPT_DIR/../metadata.sh"

VERSION="$(strip_v "${1:-$APMW_VERSION}")"
TARGET="${2:-x86_64-unknown-linux-gnu}"
OUTPUT_DIR="${3:-$SCRIPT_DIR/dist}"

# Map Rust target triple to Debian architecture.
deb_arch() {
    case "$1" in
        x86_64-unknown-linux-gnu)  echo "amd64" ;;
        aarch64-unknown-linux-gnu) echo "arm64" ;;
        *)                         echo "all" ;;
    esac
}

ARCH="$(deb_arch "$TARGET")"
PKG_DIR="$OUTPUT_DIR/apmw_${VERSION}_${ARCH}"

mkdir -p "$PKG_DIR/DEBIAN" "$PKG_DIR/usr/bin"

# --- DEBIAN/control ------------------------------------------------------
cat > "$PKG_DIR/DEBIAN/control" <<EOF
Package: apmw
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${ARCH}
Maintainer: ${APMW_MAINTAINER}
Depends: ca-certificates
Description: ${APMW_DESCRIPTION}
 apmw abstracts every package installer into one intelligent surface. It
 detects the correct package manager, adds tools with install-on-use
 semantics, scans before installing, and keeps an audit log.
Homepage: ${APMW_HOMEPAGE}
EOF

# --- DEBIAN/postinst (download binary on install) ------------------------
cat > "$PKG_DIR/DEBIAN/postinst" <<POSTINST
#!/usr/bin/env bash
set -euo pipefail
# Download the pre-built apmw binary for this platform on install.
VERSION="${VERSION}"
TARGET="${TARGET}"
URL="${APMW_REPOSITORY}/releases/download/v\${VERSION}/apmw-\${VERSION}-\${TARGET}.tar.gz"
TMPDIR="\$(mktemp -d)"
trap 'rm -rf "\$TMPDIR"' EXIT
echo "Downloading apmw \${VERSION} from \${URL}"
curl -fsSL "\${URL}" | tar -xz -C "\$TMPDIR"
install -m755 "\$TMPDIR/apmw" /usr/bin/apmw
echo "apmw \${VERSION} installed to /usr/bin/apmw"
POSTINST
chmod +x "$PKG_DIR/DEBIAN/postinst"

# --- DEBIAN/postrm -------------------------------------------------------
cat > "$PKG_DIR/DEBIAN/postrm" <<'POSTRM'
#!/usr/bin/env bash
set -euo pipefail
if [ "$1" = "remove" ] || [ "$1" = "purge" ]; then
    rm -f /usr/bin/apmw
fi
POSTRM
chmod +x "$PKG_DIR/DEBIAN/postrm"

cat > "$OUTPUT_DIR/README.md" <<EOF
# apmw .deb package

${APMW_DESCRIPTION}

## Build the .deb

\`\`\`
dpkg-deb --build apmw_${VERSION}_${ARCH}
\`\`\`

## Install

\`\`\`
sudo dpkg -i apmw_${VERSION}_${ARCH}.deb
\`\`\`

The \`postinst\` script downloads the pre-built binary from the
[GitHub release](${APMW_REPOSITORY}/releases) on install.
EOF

echo "Generated .deb package layout in $PKG_DIR"
echo "  DEBIAN/{control,postinst,postrm}, usr/bin/"

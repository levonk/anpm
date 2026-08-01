#!/usr/bin/env bash
# Alpine APK package generator for apmw.
#
# Produces an APKBUILD that wraps the pre-built Rust binary. The build()
# function downloads the platform-appropriate archive from the GitHub release
# and installs the binary.
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

cat > "$OUTPUT_DIR/APKBUILD" <<EOF
# Contributor: ${APMW_MAINTAINER}
# Maintainer: ${APMW_MAINTAINER}
# apmw Alpine APK — wraps the pre-built Rust binary.
pkgname=apmw
pkgver=${VERSION}
pkgrel=0
pkgdesc="${APMW_DESCRIPTION}"
url="${APMW_HOMEPAGE}"
arch="x86_64 aarch64"
license="${APMW_LICENSE}"
depends="ca-certificates"
makedepends="curl"
subpackages=""
source=""
builddir="\$srcdir"

case "\$CARCH" in
    x86_64)   _target="x86_64-unknown-linux-gnu" ;;
    aarch64)  _target="aarch64-unknown-linux-gnu" ;;
    *)        die "apmw: unsupported arch \$CARCH" ;;
esac

fetch() {
    mkdir -p "\$srcdir"
    curl -fsSL "${APMW_REPOSITORY}/releases/download/v\${pkgver}/apmw-\${pkgver}-\${_target}.tar.gz" \\
        -o "\$srcdir/apmw-\${pkgver}.tar.gz"
}

prepare() {
    cd "\$srcdir"
    tar -xzf apmw-\${pkgver}.tar.gz
}

build() {
    :
}

package() {
    install -Dm755 "\$srcdir/apmw" "\$pkgdir/usr/bin/apmw"
}

sha512sums=""
EOF

cat > "$OUTPUT_DIR/README.md" <<EOF
# apmw Alpine APK package

${APMW_DESCRIPTION}

## Build

\`\`\`
cd $(basename "$OUTPUT_DIR")
abuild -r
\`\`\`

The \`fetch()\` function downloads the pre-built binary from the
[GitHub release](${APMW_REPOSITORY}/releases) at build time.
EOF

echo "Generated Alpine APKBUILD in $OUTPUT_DIR/APKBUILD"

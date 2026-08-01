#!/usr/bin/env bash
# AUR (Arch User Repository) package generator for apmw.
#
# Produces a PKGBUILD for the `apmw-bin` AUR package. The package name is
# `apmw-bin` (not `apmw`) to avoid collision with the unrelated AUR package
# `apmw` (an apt-to-pacman translator by `mineleng@aur`). See PRD Open
# Question 2.
#
# The PKGBUILD downloads the pre-built Rust binary from the GitHub release.
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

cat > "$OUTPUT_DIR/PKGBUILD" <<EOF
# Maintainer: ${APMW_MAINTAINER}
# Contributor: ${APMW_MAINTAINER}
#
# apmw-bin — pre-built binary package for apmw (All Package Manager Wrapper).
#
# NOTE: This package is named 'apmw-bin' to avoid collision with the
# unrelated AUR package 'apmw' (an apt-to-pacman translator). See PRD Open
# Question 2.

pkgname=apmw-bin
pkgver=${VERSION}
pkgrel=1
pkgdesc="${APMW_DESCRIPTION}"
arch=('x86_64' 'aarch64')
url="${APMW_HOMEPAGE}"
license=('MIT')
depends=('ca-certificates')
provides=('apmw')
conflicts=('apmw')
options=('!strip' '!emptydirs')

source_x86_64=("${APMW_REPOSITORY}/releases/download/v\${pkgver}/apmw-\${pkgver}-x86_64-unknown-linux-gnu.tar.gz")
source_aarch64=("${APMW_REPOSITORY}/releases/download/v\${pkgver}/apmw-\${pkgver}-aarch64-unknown-linux-gnu.tar.gz")

# Replace these checksums with the actual SHA-256 from the release assets.
sha256sums_x86_64=('SKIP')
sha256sums_aarch64=('SKIP')

package() {
    install -Dm755 "\$srcdir/apmw" "\$pkgdir/usr/bin/apmw"
}
EOF

cat > "$OUTPUT_DIR/.SRCINFO" <<EOF
pkgbase = apmw-bin
	pkgdesc = ${APMW_DESCRIPTION}
	pkgver = ${VERSION}
	pkgrel = 1
	url = ${APMW_HOMEPAGE}
	arch = x86_64
	arch = aarch64
	license = MIT
	depends = ca-certificates
	provides = apmw
	conflicts = apmw
	options = !strip
	options = !emptydirs
	source = ${APMW_REPOSITORY}/releases/download/v${VERSION}/apmw-${VERSION}-x86_64-unknown-linux-gnu.tar.gz
	sha256sums = SKIP
	source = ${APMW_REPOSITORY}/releases/download/v${VERSION}/apmw-${VERSION}-aarch64-unknown-linux-gnu.tar.gz
	sha256sums = SKIP

pkgname = apmw-bin
EOF

cat > "$OUTPUT_DIR/README.md" <<EOF
# apmw-bin AUR package

${APMW_DESCRIPTION}

## Why 'apmw-bin'?

This package is named 'apmw-bin' to avoid collision with the unrelated AUR
package 'apmw' (an apt-to-pacman translator by 'mineleng@aur'). See PRD Open
Question 2.

## Install from AUR

\`\`\`
paru -S apmw-bin
# or
yay -S apmw-bin
\`\`\`

## Build manually

\`\`\`
cd $(basename "$OUTPUT_DIR")
makepkg -si
\`\`\`

The PKGBUILD downloads the pre-built binary from the
[GitHub release](${APMW_REPOSITORY}/releases).
EOF

echo "Generated AUR package (apmw-bin) in $OUTPUT_DIR"
echo "  PKGBUILD, .SRCINFO, README.md"

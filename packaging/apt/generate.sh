#!/usr/bin/env bash
# apt source package generator for apmw.
#
# Produces a Debian source package skeleton (debian/control, debian/rules,
# debian/changelog) that wraps the pre-built Rust binary. The resulting
# .deb can be published to an apt repository.
#
# Usage:   ./generate.sh [version] [output-dir]
# Example: ./generate.sh 0.1.0 ./dist
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../metadata.sh
source "$SCRIPT_DIR/../metadata.sh"

VERSION="$(strip_v "${1:-$APMW_VERSION}")"
OUTPUT_DIR="${2:-$SCRIPT_DIR/dist}"

mkdir -p "$OUTPUT_DIR/debian"

# --- debian/control ------------------------------------------------------
cat > "$OUTPUT_DIR/debian/control" <<EOF
Source: apmw
Section: utils
Priority: optional
Maintainer: ${APMW_MAINTAINER}
Build-Depends: debhelper-compat (= 13), curl, ca-certificates
Standards-Version: 4.6.0
Homepage: ${APMW_HOMEPAGE}

Package: apmw
Architecture: amd64
Depends: \${shlibs:Depends}, \${misc:Depends}, ca-certificates
Description: ${APMW_DESCRIPTION}
 apmw abstracts every package installer into one intelligent surface. It
 detects the correct package manager, adds tools with install-on-use
 semantics, scans before installing, and keeps an audit log.
EOF

# --- debian/rules --------------------------------------------------------
cat > "$OUTPUT_DIR/debian/rules" <<'RULES'
#!/usr/bin/make -f
# debian/rules for apmw — downloads the pre-built binary at build time.
include /usr/share/dpkg/architecture.mk

TARGET := $(shell \
	case "$(DEB_HOST_GNU_CPU)" in \
		x86_64)  echo "x86_64-unknown-linux-gnu" ;; \
		aarch64) echo "aarch64-unknown-linux-gnu" ;; \
		*) echo "unsupported" ;; \
	esac)

VERSION := $(shell dpkg-parsechangelog -S Version | sed 's/-.*//')

%:
	dh $@

override_dh_auto_build:
	@echo "Downloading apmw $(VERSION) for $(TARGET)"
	curl -fsSL "https://github.com/levonk/apmw/releases/download/v$(VERSION)/apmw-$(VERSION)-$(TARGET).tar.gz" -o /tmp/apmw.tar.gz
	mkdir -p $(CURDIR)/build
	tar -xzf /tmp/apmw.tar.gz -C $(CURDIR)/build

override_dh_auto_install:
	install -Dm755 $(CURDIR)/build/apmw $(CURDIR)/debian/apmw/usr/bin/apmw

override_dh_auto_test:
	$(CURDIR)/build/apmw --version || true
RULES
chmod +x "$OUTPUT_DIR/debian/rules"

# --- debian/changelog ----------------------------------------------------
cat > "$OUTPUT_DIR/debian/changelog" <<EOF
apmw (${VERSION}-1) unstable; urgency=medium

  * Initial Debian package for apmw.
  * Wraps the pre-built Rust binary from the GitHub release.

 -- ${APMW_MAINTAINER}  $(date -R)
EOF

# --- debian/compat -------------------------------------------------------
echo "13" > "$OUTPUT_DIR/debian/compat"

# --- debian/source/format ------------------------------------------------
mkdir -p "$OUTPUT_DIR/debian/source"
echo "3.0 (native)" > "$OUTPUT_DIR/debian/source/format"

cat > "$OUTPUT_DIR/README.md" <<EOF
# apmw apt source package

${APMW_DESCRIPTION}

## Build the .deb

\`\`\`
dpkg-buildpackage -us -uc -b
\`\`\`

The \`debian/rules\` build target downloads the pre-built binary from the
[GitHub release](${APMW_REPOSITORY}/releases) at build time.
EOF

echo "Generated apt source package in $OUTPUT_DIR/debian/"

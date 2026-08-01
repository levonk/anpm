#!/usr/bin/env bash
# RPM spec generator for apmw.
#
# Produces an RPM .spec file that wraps the pre-built Rust binary. The
# %install section downloads the platform-appropriate archive from the
# GitHub release and installs the binary.
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

cat > "$OUTPUT_DIR/apmw.spec" <<EOF
# apmw RPM spec — wraps the pre-built Rust binary.
# Build with: rpmbuild -ba apmw.spec

Name:           apmw
Version:        ${VERSION}
Release:        1%{?dist}
Summary:        ${APMW_DESCRIPTION}

License:        ${APMW_LICENSE}
URL:            ${APMW_HOMEPAGE}
Source0:        ${APMW_REPOSITORY}/releases/download/v%{version}/apmw-%{version}-x86_64-unknown-linux-gnu.tar.gz
Source1:        ${APMW_REPOSITORY}/releases/download/v%{version}/apmw-%{version}-aarch64-unknown-linux-gnu.tar.gz
BuildRequires:  curl
Requires:       ca-certificates

%ifarch x86_64
%define _target x86_64-unknown-linux-gnu
%define _source %{SOURCE0}
%endif
%ifarch aarch64
%define _target aarch64-unknown-linux-gnu
%define _source %{SOURCE1}
%endif

%description
apmw abstracts every package installer into one intelligent surface. It
detects the correct package manager, adds tools with install-on-use
semantics, scans before installing, and keeps an audit log.

%prep
mkdir -p %{_builddir}/apmw-%{version}
cd %{_builddir}/apmw-%{version}
curl -fsSL "%{source0}" -o apmw-%{version}.tar.gz
tar -xzf apmw-%{version}.tar.gz

%build
:

%install
mkdir -p %{buildroot}/usr/bin
install -m755 %{_builddir}/apmw-%{version}/apmw %{buildroot}/usr/bin/apmw

%files
%license LICENSE
/usr/bin/apmw

%changelog
* $(date +'%a %b %d %Y') ${APMW_MAINTAINER} - ${VERSION}-1
- Initial RPM package for apmw
- Wraps the pre-built Rust binary from the GitHub release
EOF

cat > "$OUTPUT_DIR/README.md" <<EOF
# apmw RPM package

${APMW_DESCRIPTION}

## Build

\`\`\`
rpmbuild -ba apmw.spec
\`\`\`

The \`%prep\` section downloads the pre-built binary from the
[GitHub release](${APMW_REPOSITORY}/releases) at build time.
EOF

echo "Generated RPM spec in $OUTPUT_DIR/apmw.spec"

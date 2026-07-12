#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "  $*"; }
warn()  { echo -e "${YELLOW}  warning:${NC} $*"; }
error() { echo -e "${RED}  error:${NC} $*"; exit 1; }

REPO="pxlmrror/abaran"
INSTALL_DIR="${HOME}/.local/bin"
BIN="${INSTALL_DIR}/abaran"

if [[ "$(uname -s)" != "Linux" ]]; then
    error "abaran only runs on Linux"
fi

ARCH="$(uname -m)"
case "$ARCH" in
    x86_64)   TARGET="x86_64-unknown-linux-gnu" ;;
    aarch64)  TARGET="aarch64-unknown-linux-gnu" ;;
    *)        error "unsupported architecture: ${ARCH}" ;;
esac

info "Fetching latest release..."
RELEASE_JSON="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest")"

TAG="$(echo "$RELEASE_JSON" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": "\(.*\)".*/\1/')"
info "Latest release: ${TAG}"

DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${TAG}/abaran-${TARGET}"

mkdir -p "${INSTALL_DIR}"
info "Downloading abaran..."
curl -fsSL "${DOWNLOAD_URL}" -o "${BIN}"
chmod +x "${BIN}"

if [[ ":${PATH}:" != *":${INSTALL_DIR}:"* ]]; then
    warn "${INSTALL_DIR} is not in PATH"
    warn "Add this to your shell config:"
    echo ""
    echo "    export PATH=\"\${HOME}/.local/bin:\${PATH}\""
    echo ""
fi

info "abaran ${TAG} installed to ${BIN}"

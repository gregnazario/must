#!/usr/bin/env sh
set -eu

REPO="anomalyco/mustfile"
BINARY="must"

println() {
    printf '%s\n' "$1"
}

echoerr() {
    println "$1" >&2
}

http_get() {
    if command -v curl > /dev/null 2>&1; then
        curl -fsSL "$1"
    elif command -v wget > /dev/null 2>&1; then
        wget -qO- "$1"
    else
        echoerr "Error: need curl or wget to download"
        exit 1
    fi
}

detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Linux)  PLATFORM="unknown-linux-gnu" ;;
        Darwin) PLATFORM="apple-darwin" ;;
        MINGW*|MSYS*|CYGWIN*)
            echoerr "Windows detected. Download the .zip from the releases page."
            exit 1
            ;;
        *)      echoerr "Unsupported OS: $OS"; exit 1 ;;
    esac

    case "$ARCH" in
        x86_64|amd64)  ARCH="x86_64" ;;
        aarch64|arm64) ARCH="aarch64" ;;
        *)             echoerr "Unsupported arch: $ARCH"; exit 1 ;;
    esac

    TARGET="${ARCH}-${PLATFORM}"
}

get_latest_version() {
    VERSION=$(http_get "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | head -1 | sed -E 's/.*"([^"]+)".*/\1/')
    if [ -z "$VERSION" ]; then
        echoerr "Failed to determine latest version"
        exit 1
    fi
}

download_and_install() {
    INSTALL_DIR="${MUST_INSTALL_DIR:-${HOME}/.local/bin}"
    TMPDIR="$(mktemp -d)"
    trap 'rm -rf "$TMPDIR"' EXIT

    ARCHIVE="must-${TARGET}.tar.gz"
    URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARCHIVE}"

    println "Downloading ${BINARY} ${VERSION} for ${TARGET}..."
    http_get "$URL" > "${TMPDIR}/${ARCHIVE}"

    mkdir -p "$INSTALL_DIR"
    tar -xzf "${TMPDIR}/${ARCHIVE}" -C "${TMPDIR}"
    mv "${TMPDIR}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
    chmod +x "${INSTALL_DIR}/${BINARY}"

    println ""
    println "Installed ${BINARY} ${VERSION} to ${INSTALL_DIR}/${BINARY}"

    if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
        println ""
        println "Add ${INSTALL_DIR} to your PATH:"
        println "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    fi

    println ""
    "${INSTALL_DIR}/${BINARY}" --version
}

detect_platform
get_latest_version
download_and_install

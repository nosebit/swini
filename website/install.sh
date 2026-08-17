#!/usr/bin/env bash
set -e

# Swini installation script
# https://github.com/nosebit/swini

# ANSI color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}Installing Swini...${NC}"

# 1. Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)
    OS_NAME="unknown-linux-gnu"
    ;;
  Darwin)
    OS_NAME="apple-darwin"
    ;;
  *)
    echo -e "${RED}Unsupported OS: $OS${NC}"
    echo "Swini currently supports Linux and macOS."
    exit 1
    ;;
esac

case "$ARCH" in
  x86_64|amd64)
    ARCH_NAME="x86_64"
    ;;
  arm64|aarch64)
    if [ "$OS" = "Darwin" ]; then
      ARCH_NAME="aarch64"
    else
      echo -e "${RED}Unsupported architecture on Linux: $ARCH${NC}"
      echo "Swini currently supports x86_64 on Linux."
      exit 1
    fi
    ;;
  *)
    echo -e "${RED}Unsupported architecture: $ARCH${NC}"
    exit 1
    ;;
esac

TARGET="${ARCH_NAME}-${OS_NAME}"
echo -e "Detected target: ${YELLOW}${TARGET}${NC}"

# 2. Fetch latest release version from GitHub API
echo -e "Fetching latest release version..."
RELEASE_INFO=$(curl -fsSL https://api.github.com/repos/nosebit/swini/releases/latest)
VERSION=$(echo "$RELEASE_INFO" | grep '"tag_name":' | head -n 1 | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$VERSION" ]; then
  echo -e "${RED}Failed to determine latest release version.${NC}"
  exit 1
fi

echo -e "Latest version: ${GREEN}${VERSION}${NC}"

# 3. Download the binary archive
ARCHIVE_NAME="swini-${TARGET}.tar.gz"
DOWNLOAD_URL="https://github.com/nosebit/swini/releases/download/${VERSION}/${ARCHIVE_NAME}"

TMP_DIR=$(mktemp -d)
cd "$TMP_DIR"

echo -e "Downloading ${ARCHIVE_NAME}..."
if ! curl -fsSLO "$DOWNLOAD_URL"; then
  echo -e "${RED}Failed to download ${ARCHIVE_NAME}${NC}"
  exit 1
fi

# 4. Extract and Install
echo -e "Extracting archive..."
tar -xzf "$ARCHIVE_NAME"

INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

echo -e "Installing to ${INSTALL_DIR}..."
mv swini "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/swini"

# Cleanup
cd - > /dev/null
rm -rf "$TMP_DIR"

echo -e "${GREEN}Successfully installed Swini ${VERSION}!${NC}"

# 5. Check if $INSTALL_DIR is in $PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
  echo -e "\n${YELLOW}WARNING: ${INSTALL_DIR} is not in your PATH.${NC}"
  echo -e "Please add the following line to your shell configuration file (e.g. ~/.bashrc, ~/.zshrc):"
  echo -e "\n  export PATH=\"\$HOME/.local/bin:\$PATH\"\n"
else
  echo -e "\nYou can now run ${GREEN}swini${NC} from your terminal."
fi

#!/usr/bin/env sh
# ==============================================================================
# AnyContext (actx) - Linux Installer Script
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/Levix-Digital/any-context/dev/scripts/install.sh | sh
# ==============================================================================

set -e

REPO="Levix-Digital/any-context"
INSTALL_DIR="$HOME/.local/bin"
EXE_PATH="$INSTALL_DIR/actx"
DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/actx-linux-x86_64"

printf "\n\033[36m🚀 Installing AnyContext (actx)...\033[0m\n"

# 1. Ensure target directory exists
mkdir -p "$INSTALL_DIR"

# 2. Download binary
printf "\033[33m⬇️ Downloading latest actx executable from GitHub...\033[0m\n"
if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$DOWNLOAD_URL" -o "$EXE_PATH"
elif command -v wget >/dev/null 2>&1; then
    wget -qO "$EXE_PATH" "$DOWNLOAD_URL"
else
    printf "\033[31m❌ Error: Neither curl nor wget was found. Please install curl or wget.\033[0m\n"
    exit 1
fi

# 3. Grant execution permissions
chmod +x "$EXE_PATH"
printf "\033[32m✅ Download complete: %s\033[0m\n" "$EXE_PATH"

# 4. PATH Check & Advice
if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
    printf "\033[33m⚠️ Notice: %s is not currently in your PATH.\033[0m\n" "$INSTALL_DIR"
    printf "To add it automatically, run:\n"
    printf "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc\n"
    printf "  source ~/.bashrc\n\n"
fi

printf "\n\033[36m=======================================================\033[0m\n"
printf "\033[32m🎉 AnyContext (actx) installed successfully!\033[0m\n"
printf "👉 Type \033[1mactx\033[0m to launch the assistant.\n"
printf "\033[36m=======================================================\033[0m\n\n"

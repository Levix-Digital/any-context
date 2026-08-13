#!/usr/bin/env sh
# ==============================================================================
# AnyContext (actx) - Multi-platform Installer Script
# Usage:
#   ./install.sh
# ==============================================================================

set -e

REPO="Levix-Digital/any-context"
OS_TYPE="$(uname -s | tr '[:upper:]' '[:lower:]')"

if echo "$OS_TYPE" | grep -qE "mingw|msys|cygwin|windows"; then
    ASSET_NAME="actx-windows-x86_64.exe"
    EXE_NAME="actx.exe"
    INSTALL_DIR="$HOME/AppData/Local/actx/bin"
else
    ASSET_NAME="actx-linux-x86_64"
    EXE_NAME="actx"
    INSTALL_DIR="$HOME/.local/bin"
fi

EXE_PATH="$INSTALL_DIR/$EXE_NAME"
DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/$ASSET_NAME"

printf "\n\033[36m🚀 Installing AnyContext (actx)...\033[0m\n"

# 1. Ensure target directory exists
mkdir -p "$INSTALL_DIR"

# 2. Download binary (Attempt gh CLI first for private repos, fallback to curl/wget)
printf "\033[33m⬇️ Downloading latest %s from GitHub...\033[0m\n" "$ASSET_NAME"

DOWNLOAD_SUCCESS=0

if command -v gh >/dev/null 2>&1; then
    printf "\033[90m⚡ Using GitHub CLI (gh) for authenticated download...\033[0m\n"
    if gh release download --repo "$REPO" --pattern "$ASSET_NAME" --dir "$INSTALL_DIR" --clobber >/dev/null 2>&1; then
        if [ "$ASSET_NAME" != "$EXE_NAME" ]; then
            mv -f "$INSTALL_DIR/$ASSET_NAME" "$EXE_PATH" 2>/dev/null || true
        fi
        DOWNLOAD_SUCCESS=1
    fi
fi

if [ $DOWNLOAD_SUCCESS -eq 0 ]; then
    if command -v curl >/dev/null 2>&1; then
        if curl -fsSL "$DOWNLOAD_URL" -o "$EXE_PATH"; then
            DOWNLOAD_SUCCESS=1
        fi
    elif command -v wget >/dev/null 2>&1; then
        if wget -qO "$EXE_PATH" "$DOWNLOAD_URL"; then
            DOWNLOAD_SUCCESS=1
        fi
    fi
fi

if [ $DOWNLOAD_SUCCESS -eq 0 ]; then
    printf "\033[31m❌ Error: Failed to download release asset '%s'.\033[0m\n" "$ASSET_NAME"
    printf "\033[33m💡 For private repositories, please ensure you are logged in via 'gh auth login' or download the binary directly from GitHub Releases.\033[0m\n\n"
    exit 1
fi

# 3. Grant execution permissions
chmod +x "$EXE_PATH" 2>/dev/null || true
printf "\033[32m✅ Download complete: %s\033[0m\n" "$EXE_PATH"

# 4. PATH Check & Advice
if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
    printf "\033[33m⚠️ Notice: %s is not currently in your PATH.\033[0m\n" "$INSTALL_DIR"
    printf "To add it to your PATH, add this directory to your environment variables.\n\n"
fi

printf "\n\033[36m=======================================================\033[0m\n"
printf "\033[32m🎉 AnyContext (actx) installed successfully!\033[0m\n"
printf "👉 Type \033[1mactx\033[0m to launch the assistant.\n"
printf "\033[36m=======================================================\033[0m\n\n"

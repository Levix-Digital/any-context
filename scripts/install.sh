#!/usr/bin/env sh
# ==============================================================================
# AnyContext (actx) - Multi-platform Installer Script
# Usage:
#   ./install.sh
# ==============================================================================

set -e

REPO="Levix-Digital/any-context-releases"

OS_TYPE="$(uname -s | tr '[:upper:]' '[:lower:]')"

if echo "$OS_TYPE" | grep -qE "mingw|msys|cygwin|windows"; then
    IS_WINDOWS=1
    ASSET_NAME="actx-windows-x86_64.exe"
    EXE_NAME="actx.exe"
    INSTALL_DIR="$HOME/AppData/Local/actx/bin"
else
    IS_WINDOWS=0
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
    if gh release download --repo "$REPO" --pattern "$ASSET_NAME" --dir "$INSTALL_DIR" --clobber 2>/dev/null; then
        if [ "$ASSET_NAME" != "$EXE_NAME" ] && [ -f "$INSTALL_DIR/$ASSET_NAME" ]; then
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
    printf "\033[31m❌ Error: Release asset '%s' is not available yet.\033[0m\n" "$ASSET_NAME"
    printf "\033[33m⏳ GitHub Actions might still be compiling the latest release binary (~2-3 mins).\033[0m\n"
    printf "\033[33m💡 Please wait 1-2 minutes and re-run './install.sh' or log in via 'gh auth login'.\033[0m\n\n"
    exit 1
fi

# 3. Grant execution permissions
chmod +x "$EXE_PATH" 2>/dev/null || true
printf "\033[32m✅ Download complete: %s\033[0m\n" "$EXE_PATH"

# 4. PATH Configuration
if [ "$IS_WINDOWS" -eq 1 ]; then
    WIN_INSTALL_DIR="$(cygpath -w "$INSTALL_DIR" 2>/dev/null || echo "$INSTALL_DIR")"
    powershell.exe -NoProfile -Command "
        \$UserPath = [Environment]::GetEnvironmentVariable('Path', 'User');
        if (\$UserPath -notlike '*$WIN_INSTALL_DIR*') {
            \$NewPath = if ([string]::IsNullOrEmpty(\$UserPath)) { '$WIN_INSTALL_DIR' } else { \"\$UserPath;$WIN_INSTALL_DIR\" };
            [Environment]::SetEnvironmentVariable('Path', \$NewPath, 'User');
            Write-Host '⚙️ Added $WIN_INSTALL_DIR to Windows User PATH environment variable!';
        }
    " 2>/dev/null || true
else
    if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
        SHELL_PROFILE=""
        if [ -f "$HOME/.bashrc" ]; then
            SHELL_PROFILE="$HOME/.bashrc"
        elif [ -f "$HOME/.zshrc" ]; then
            SHELL_PROFILE="$HOME/.zshrc"
        fi

        if [ -n "$SHELL_PROFILE" ]; then
            if ! grep -q "$INSTALL_DIR" "$SHELL_PROFILE"; then
                echo "export PATH=\"$INSTALL_DIR:\$PATH\"" >> "$SHELL_PROFILE"
                printf "\033[32m⚙️ Added %s to %s automatically!\033[0m\n" "$INSTALL_DIR" "$SHELL_PROFILE"
            fi
        fi
    fi
fi

# 5. Check/Ensure Bun is available for OpenTUI desktop interface
if ! command -v bun >/dev/null 2>&1 && [ ! -f "$HOME/.bun/bin/bun" ]; then
    printf "\033[90m💡 Bun runtime not detected. Installing Bun for OpenTUI desktop interface...\033[0m\n"
    curl -fsSL https://bun.sh/install | bash 2>/dev/null || true
fi

printf "\n\033[36m=======================================================\033[0m\n"
printf "\033[32m🎉 AnyContext (actx) installed successfully!\033[0m\n"
printf "👉 Open a new terminal window and type \033[1mactx\033[0m to launch the assistant.\n"
printf "👉 To launch the OpenTUI desktop interface, type \033[1mactx --tui\033[0m.\n"
printf "\033[36m=======================================================\033[0m\n\n"

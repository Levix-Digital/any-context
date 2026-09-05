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
    ARCHIVE_NAME="actx-windows-x86_64.zip"
    FALLBACK_NAME="actx-windows-x86_64.exe"
    CORE_NAME="actx-core.exe"
    EXE_NAME="actx.exe"
    INSTALL_DIR="$HOME/AppData/Local/actx/bin"
else
    IS_WINDOWS=0
    ARCHIVE_NAME="actx-linux-x86_64.tar.gz"
    FALLBACK_NAME="actx-linux-x86_64"
    CORE_NAME="actx-core"
    EXE_NAME="actx"
    INSTALL_DIR="$HOME/.local/bin"
fi

CORE_PATH="$INSTALL_DIR/$CORE_NAME"
EXE_PATH="$INSTALL_DIR/$EXE_NAME"
VERSION_FILE="$INSTALL_DIR/version.txt"

if [ "$IS_WINDOWS" -eq 1 ]; then
    LOG_DIR="$HOME/AppData/Local/AnyContext/logs"
else
    LOG_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/any-context/logs"
fi
mkdir -p "$LOG_DIR"
INSTALL_LOG="$LOG_DIR/install.log"

log_install() {
    ts="$(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date)"
    printf "[%s] [%s] %s\n" "$ts" "${2:-INFO}" "$1" >> "$INSTALL_LOG" 2>/dev/null || true
}

log_install "Installer initiated. OS: $OS_TYPE, InstallDir: $INSTALL_DIR, Repo: $REPO"

printf "\n\033[36m🚀 Installing AnyContext (actx)...\033[0m\n"

# 1. Ensure target directory exists
mkdir -p "$INSTALL_DIR"

# 2. Download Core distribution archive or standalone fallback binary
printf "\033[33m⬇️ Downloading latest AnyContext from GitHub...\033[0m\n"
log_install "Downloading AnyContext ($ARCHIVE_NAME or $FALLBACK_NAME) from GitHub"

DOWNLOAD_SUCCESS=0

# Try downloading distribution archive (.zip / .tar.gz) for sub-second cold boot
if command -v gh >/dev/null 2>&1; then
    printf "\033[90m⚡ Using GitHub CLI (gh) for authenticated download...\033[0m\n"
    if gh release download --repo "$REPO" --pattern "$ARCHIVE_NAME" --dir "$INSTALL_DIR" --clobber 2>/dev/null; then
        if [ -f "$INSTALL_DIR/$ARCHIVE_NAME" ]; then
            if [ "$IS_WINDOWS" -eq 1 ]; then
                if command -v unzip >/dev/null 2>&1; then
                    unzip -o -q "$INSTALL_DIR/$ARCHIVE_NAME" -d "$INSTALL_DIR" 2>/dev/null || true
                elif command -v tar >/dev/null 2>&1; then
                    tar -xf "$INSTALL_DIR/$ARCHIVE_NAME" -C "$INSTALL_DIR" 2>/dev/null || true
                else
                    powershell.exe -NoProfile -Command "Expand-Archive -LiteralPath '$INSTALL_DIR/$ARCHIVE_NAME' -DestinationPath '$INSTALL_DIR' -Force" 2>/dev/null || true
                fi
            else
                tar -xzf "$INSTALL_DIR/$ARCHIVE_NAME" -C "$INSTALL_DIR" 2>/dev/null || true
            fi
            rm -f "$INSTALL_DIR/$ARCHIVE_NAME" 2>/dev/null || true
            DOWNLOAD_SUCCESS=1
        fi
    fi
fi

if [ $DOWNLOAD_SUCCESS -eq 0 ]; then
    ARCHIVE_URL="https://github.com/$REPO/releases/latest/download/$ARCHIVE_NAME"
    TEMP_ARCHIVE="$INSTALL_DIR/$ARCHIVE_NAME"
    if command -v curl >/dev/null 2>&1; then
        if curl -fsSL "$ARCHIVE_URL" -o "$TEMP_ARCHIVE" 2>/dev/null && [ -s "$TEMP_ARCHIVE" ]; then
            if [ "$IS_WINDOWS" -eq 1 ]; then
                if command -v unzip >/dev/null 2>&1; then
                    unzip -o -q "$TEMP_ARCHIVE" -d "$INSTALL_DIR" 2>/dev/null || true
                elif command -v tar >/dev/null 2>&1; then
                    tar -xf "$TEMP_ARCHIVE" -C "$INSTALL_DIR" 2>/dev/null || true
                else
                    powershell.exe -NoProfile -Command "Expand-Archive -LiteralPath '$TEMP_ARCHIVE' -DestinationPath '$INSTALL_DIR' -Force" 2>/dev/null || true
                fi
            else
                tar -xzf "$TEMP_ARCHIVE" -C "$INSTALL_DIR" 2>/dev/null || true
            fi
            rm -f "$TEMP_ARCHIVE" 2>/dev/null || true
            DOWNLOAD_SUCCESS=1
        fi
    elif command -v wget >/dev/null 2>&1; then
        if wget -qO "$TEMP_ARCHIVE" "$ARCHIVE_URL" 2>/dev/null && [ -s "$TEMP_ARCHIVE" ]; then
            if [ "$IS_WINDOWS" -eq 1 ]; then
                if command -v unzip >/dev/null 2>&1; then
                    unzip -o -q "$TEMP_ARCHIVE" -d "$INSTALL_DIR" 2>/dev/null || true
                elif command -v tar >/dev/null 2>&1; then
                    tar -xf "$TEMP_ARCHIVE" -C "$INSTALL_DIR" 2>/dev/null || true
                else
                    powershell.exe -NoProfile -Command "Expand-Archive -LiteralPath '$TEMP_ARCHIVE' -DestinationPath '$INSTALL_DIR' -Force" 2>/dev/null || true
                fi
            else
                tar -xzf "$TEMP_ARCHIVE" -C "$INSTALL_DIR" 2>/dev/null || true
            fi
            rm -f "$TEMP_ARCHIVE" 2>/dev/null || true
            DOWNLOAD_SUCCESS=1
        fi
    fi
fi

# Fallback to single binary if archive was not found
if [ $DOWNLOAD_SUCCESS -eq 0 ]; then
    FALLBACK_URL="https://github.com/$REPO/releases/latest/download/$FALLBACK_NAME"
    if command -v gh >/dev/null 2>&1; then
        if gh release download --repo "$REPO" --pattern "$FALLBACK_NAME" --dir "$INSTALL_DIR" --clobber 2>/dev/null; then
            if [ -f "$INSTALL_DIR/$FALLBACK_NAME" ]; then
                mv -f "$INSTALL_DIR/$FALLBACK_NAME" "$CORE_PATH" 2>/dev/null || true
                DOWNLOAD_SUCCESS=1
            fi
        fi
    fi
    if [ $DOWNLOAD_SUCCESS -eq 0 ]; then
        if command -v curl >/dev/null 2>&1; then
            if curl -fsSL "$FALLBACK_URL" -o "$CORE_PATH" 2>/dev/null; then
                DOWNLOAD_SUCCESS=1
            fi
        elif command -v wget >/dev/null 2>&1; then
            if wget -qO "$CORE_PATH" "$FALLBACK_URL" 2>/dev/null; then
                DOWNLOAD_SUCCESS=1
            fi
        fi
    fi
fi

if [ $DOWNLOAD_SUCCESS -eq 0 ]; then
    printf "\033[31m❌ Error: Release assets are not available yet.\033[0m\n"
    printf "\033[33m⏳ GitHub Actions might still be compiling the latest release binary (~2-3 mins).\033[0m\n"
    printf "\033[33m💡 Please wait 1-2 minutes and re-run './install.sh' or log in via 'gh auth login'.\033[0m\n\n"
    exit 1
fi

chmod +x "$CORE_PATH" 2>/dev/null || true
printf "\033[32m✅ Core engine configured: %s\033[0m\n" "$CORE_PATH"

# 3. Setup Version Cache File
VERSION_TAG="0.28.89"
if command -v gh >/dev/null 2>&1; then
    GH_TAG="$(gh release view --repo "$REPO" --json tagName -q .tagName 2>/dev/null || echo "")"
    if [ -n "$GH_TAG" ]; then
        VERSION_TAG="${GH_TAG#v}"
    fi
fi
printf "%s\n" "$VERSION_TAG" > "$VERSION_FILE"

# 4. Create Ultra-Fast Native Launcher Shim (< 2ms)
if [ "$IS_WINDOWS" -eq 1 ]; then
    SHIM_COMPILED=0
    CSC_PATH="/c/Windows/Microsoft.NET/Framework64/v4.0.30319/csc.exe"
    if [ ! -f "$CSC_PATH" ]; then
        CSC_PATH="/c/Windows/Microsoft.NET/Framework/v4.0.30319/csc.exe"
    fi

    SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
    SHIM_SRC="$SCRIPT_DIR/../launcher/actx_shim.cs"

    if [ -f "$CSC_PATH" ] && [ -f "$SHIM_SRC" ]; then
        if "$CSC_PATH" /nologo /optimize+ /target:exe /out:"$EXE_PATH" "$SHIM_SRC" >/dev/null 2>&1; then
            SHIM_COMPILED=1
            printf "\033[32m⚡ Ultra-fast native Launcher Shim (actx.exe) compiled successfully!\033[0m\n"
        fi
    fi

    if [ $SHIM_COMPILED -eq 0 ]; then
        if [ ! -f "$EXE_PATH" ]; then
            cp -f "$CORE_PATH" "$EXE_PATH" 2>/dev/null || true
        fi
        printf "\033[32m⚡ Launcher (actx.exe) configured: %s\033[0m\n" "$EXE_PATH"
    fi

    # Deploy Git Bash / MSYS2 / Cygwin shell wrapper 'actx'
    BASH_SHIM="$INSTALL_DIR/actx"
    cat << EOF > "$BASH_SHIM"
#!/usr/bin/env sh
BIN_DIR="\$(CDPATH= cd -- "\$(dirname -- "\$0")" && pwd)"
if [ "\$1" = "-v" ] || [ "\$1" = "--version" ]; then
    if [ -f "\$BIN_DIR/version.txt" ]; then
        V="\$(cat "\$BIN_DIR/version.txt" | tr -d '\r\n' | sed -e 's/\xef\xbb\xbf//g' -e 's/^[vV]*//' | sed 's/^/v/')"
        echo "\$V"
    else
        echo "v$VERSION_TAG"
    fi
    exit 0
fi

if [ -f "\$BIN_DIR/actx-core.exe" ]; then
    exec "\$BIN_DIR/actx-core.exe" "\$@"
elif [ -f "\$BIN_DIR/actx.exe" ]; then
    exec "\$BIN_DIR/actx.exe" "\$@"
elif [ -f "\$BIN_DIR/actx-core" ]; then
    exec "\$BIN_DIR/actx-core" "\$@"
fi
EOF
    chmod +x "$BASH_SHIM" "$EXE_PATH" 2>/dev/null || true
    printf "\033[32m⚡ Git Bash wrapper deployed: %s\033[0m\n" "$BASH_SHIM"
else
    cat << EOF > "$EXE_PATH"
#!/usr/bin/env sh
BIN_DIR="\$(CDPATH= cd -- "\$(dirname -- "\$0")" && pwd)"
if [ "\$1" = "-v" ] || [ "\$1" = "--version" ]; then
    if [ -f "\$BIN_DIR/version.txt" ]; then
        V="\$(cat "\$BIN_DIR/version.txt" | tr -d '\r\n' | sed -e 's/\xef\xbb\xbf//g' -e 's/^[vV]*//' | sed 's/^/v/')"
        echo "\$V"
    else
        echo "v$VERSION_TAG"
    fi
    exit 0
fi
exec "\$BIN_DIR/actx-core" "\$@"
EOF
    chmod +x "$EXE_PATH" 2>/dev/null || true
    printf "\033[32m⚡ Ultra-fast native Launcher Shim deployed: %s\033[0m\n" "$EXE_PATH"
fi


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
if [ "$IS_WINDOWS" -eq 0 ]; then
    if ! command -v bun >/dev/null 2>&1 && [ ! -f "$HOME/.bun/bin/bun" ] && [ ! -f "$INSTALL_DIR/bun" ]; then
        printf "\033[90m💡 Bun runtime not detected. Installing Bun for OpenTUI desktop interface...\033[0m\n"
        curl -fsSL https://bun.sh/install | bash || true
    fi
    if [ -f "$HOME/.bun/bin/bun" ] && [ ! -f "$INSTALL_DIR/bun" ]; then
        ln -sf "$HOME/.bun/bin/bun" "$INSTALL_DIR/bun" 2>/dev/null || cp -f "$HOME/.bun/bin/bun" "$INSTALL_DIR/bun" 2>/dev/null || true
    fi
fi

# Ensure both ~/.local/bin and ~/.bun/bin are exported at the front of PATH
if [ "$IS_WINDOWS" -eq 0 ]; then
    SHELL_PROFILE=""
    if [ -f "$HOME/.bashrc" ]; then
        SHELL_PROFILE="$HOME/.bashrc"
    elif [ -f "$HOME/.zshrc" ]; then
        SHELL_PROFILE="$HOME/.zshrc"
    fi
    if [ -n "$SHELL_PROFILE" ]; then
        if ! grep -q "actx / bun path" "$SHELL_PROFILE"; then
            echo '' >> "$SHELL_PROFILE"
            echo '# actx / bun path' >> "$SHELL_PROFILE"
            echo 'export PATH="$HOME/.local/bin:$HOME/.bun/bin:$PATH"' >> "$SHELL_PROFILE"
        fi
    fi
fi

printf "\n\033[36m=======================================================\033[0m\n"
printf "\033[32m🎉 AnyContext (actx) installed successfully!\033[0m\n"
printf "👉 Open a new terminal window and type \033[1mactx\033[0m to launch the assistant.\n"
printf "👉 To launch the OpenTUI desktop interface, type \033[1mactx --tui\033[0m.\n"
printf "\033[36m=======================================================\033[0m\n\n"
log_install "Installation completed successfully. Version: v$VERSION_TAG"


#!/usr/bin/env sh
# ==============================================================================
# AnyContext (actx) - Multi-platform Uninstaller Script
# Usage:
#   ./uninstall.sh
# ==============================================================================

set -e

OS_TYPE="$(uname -s | tr '[:upper:]' '[:lower:]')"

if echo "$OS_TYPE" | grep -qE "mingw|msys|cygwin|windows"; then
    IS_WINDOWS=1
    INSTALL_DIR="$HOME/AppData/Local/actx/bin"
    CANONICAL_DATA_DIR="$HOME/AppData/Local/AnyContext"
    EXE_PATH="$INSTALL_DIR/actx.exe"
else
    IS_WINDOWS=0
    INSTALL_DIR="$HOME/.local/bin"
    CANONICAL_DATA_DIR="$HOME/.local/share/any-context"
    EXE_PATH="$INSTALL_DIR/actx"
fi

printf "\n\033[33m🧹 Uninstalling AnyContext (actx)...\033[0m\n"

# 1. Ask user about Workspaces and Vector History BEFORE removing files
printf "\n\033[33m❓ Do you want to PRESERVE your configured Workspaces and Vector History for future installations? [Y/n]: \033[0m"
read KEEP_WS

if [ -z "$KEEP_WS" ] || echo "$KEEP_WS" | grep -qE "^[Yy]$"; then
    printf "\033[32m📂 Preserving Workspaces, Vector Database & History for future installations...\033[0m\n"
    printf "\033[33m🧹 Resetting Model Settings & API Keys to OpenAI factory defaults...\033[0m\n"
    CANONICAL_SETTINGS="$CANONICAL_DATA_DIR/config/settings.db"
    if [ -f "$CANONICAL_SETTINGS" ]; then
        python3 -c "import sqlite3; con = sqlite3.connect(r'$CANONICAL_SETTINGS'); con.execute(\"UPDATE models SET inference_model = 'gpt-4o-mini', summary_model = 'gpt-4o-mini', model_provider = 'openai', local_base_url = 'https://api.openai.com/v1', embedding_model = 'text-embedding-3-small' WHERE id = 1\"); con.commit(); con.close()" 2>/dev/null || \
        python -c "import sqlite3; con = sqlite3.connect(r'$CANONICAL_SETTINGS'); con.execute(\"UPDATE models SET inference_model = 'gpt-4o-mini', summary_model = 'gpt-4o-mini', model_provider = 'openai', local_base_url = 'https://api.openai.com/v1', embedding_model = 'text-embedding-3-small' WHERE id = 1\"); con.commit(); con.close()" 2>/dev/null || true
    fi
else
    printf "\033[31m🧹 Performing 100%% Clean Uninstall (Wiping all Workspaces, Databases & Configs)...\033[0m\n"
    rm -rf "$CANONICAL_DATA_DIR" 2>/dev/null || true
    rm -rf "$HOME/AppData/Roaming/any-context" 2>/dev/null || true
    rm -rf "$HOME/.config/any-context" 2>/dev/null || true
    rm -rf "$HOME/.local/share/any-context" 2>/dev/null || true
fi

# 2. Always purge legacy orphan settings files
rm -f "$HOME/config/settings.db" 2>/dev/null || true
rm -f "$(pwd)/config/settings.db" 2>/dev/null || true

# 3. Remove standalone binary and actx directory
if [ -f "$EXE_PATH" ]; then
    rm -f "$EXE_PATH"
    printf "\033[32m✅ Removed executable: %s\033[0m\n" "$EXE_PATH"
else
    printf "\033[90mℹ️ Executable not found at %s. Skipping.\033[0m\n" "$EXE_PATH"
fi

if [ "$IS_WINDOWS" -eq 1 ]; then
    PARENT_DIR="$HOME/AppData/Local/actx"
    if [ -d "$PARENT_DIR" ]; then
        rm -rf "$PARENT_DIR"
        printf "\033[32m✅ Removed directory: %s\033[0m\n" "$PARENT_DIR"
    fi

    # Clean Windows User PATH via PowerShell
    WIN_INSTALL_DIR="$(cygpath -w "$INSTALL_DIR" 2>/dev/null || echo "$INSTALL_DIR")"
    powershell.exe -NoProfile -Command "
        \$UserPath = [Environment]::GetEnvironmentVariable('Path', 'User');
        if (\$UserPath -like '*$WIN_INSTALL_DIR*') {
            \$NewPath = (\$UserPath.Split(';') | Where-Object { \$_ -ne '$WIN_INSTALL_DIR' }) -join ';';
            [Environment]::SetEnvironmentVariable('Path', \$NewPath, 'User');
            Write-Host '⚙️ Removed $WIN_INSTALL_DIR from Windows User PATH environment variable!';
        }
    " 2>/dev/null || true
fi

# 4. Detect and clean Python/pip installations
if command -v actx >/dev/null 2>&1; then
    printf "\n\033[33m🔍 Detected 'actx' in Python environment. Running pip uninstall...\033[0m\n"
    python3 -m pip uninstall -y any-context 2>/dev/null || python -m pip uninstall -y any-context 2>/dev/null || true
    printf "\033[32m✅ Executed 'pip uninstall -y any-context'.\033[0m\n"
fi

printf "\n\033[36m=======================================================\033[0m\n"
printf "\033[32m🎉 AnyContext (actx) has been uninstalled successfully.\033[0m\n"
printf "\033[36m=======================================================\033[0m\n\n"

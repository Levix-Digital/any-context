# ==============================================================================
# AnyContext (actx) - Windows PowerShell Uninstaller Script
# Usage:
#   .\uninstall.ps1
# ==============================================================================

$ErrorActionPreference = 'Continue'

$InstallDir = Join-Path $env:LOCALAPPDATA "actx"
$CanonicalDataDir = Join-Path $env:LOCALAPPDATA "AnyContext"
$ExePath = Join-Path $InstallDir "bin\actx.exe"

Write-Host "`n🧹 Uninstalling AnyContext (actx)..." -ForegroundColor Yellow

# 1. Ask user about Workspaces and Vector History BEFORE removing files
Write-Host "`n❓ Do you want to PRESERVE your configured Workspaces and Vector History for future installations? [Y/n]: " -NoNewline -ForegroundColor Yellow
$KeepAns = Read-Host

if ($KeepAns -match "^[Yy]$" -or [string]::IsNullOrEmpty($KeepAns)) {
    Write-Host "📂 Preserving Workspaces, Vector Database & History for future installations..." -ForegroundColor Green
    Write-Host "🧹 Resetting Model Settings & API Keys to OpenAI factory defaults..." -ForegroundColor Yellow
    
    # Reset models in canonical DB if present
    $CanonicalSettingsDb = Join-Path $CanonicalDataDir "config\settings.db"
    if (Test-Path $CanonicalSettingsDb) {
        try {
            python -c "import sqlite3; con = sqlite3.connect(r'$CanonicalSettingsDb'); con.execute(\"UPDATE models SET inference_model = 'gpt-4o-mini', summary_model = 'gpt-4o-mini', model_provider = 'openai', local_base_url = 'https://api.openai.com/v1', embedding_model = 'text-embedding-3-small' WHERE id = 1\"); con.commit(); con.close()" 2>$null
        } catch {}
    }
} else {
    Write-Host "🧹 Performing 100% Clean Uninstall (Wiping all Workspaces, Databases & Configs)..." -ForegroundColor Red
    $DataPaths = @(
        $CanonicalDataDir,
        (Join-Path $env:APPDATA "any-context"),
        (Join-Path $env:USERPROFILE ".config\any-context"),
        (Join-Path $env:USERPROFILE "config\settings.db")
    )
    foreach ($dp in $DataPaths) {
        if (Test-Path -Path $dp) {
            try {
                Remove-Item -Path $dp -Recurse -Force -ErrorAction SilentlyContinue
                Write-Host "🧹 Wiped data/config path: $dp" -ForegroundColor Green
            } catch {}
        }
    }
}

# 2. Always purge legacy orphan settings files outside canonical root
$LegacyOrphans = @(
    (Join-Path $env:USERPROFILE "config\settings.db"),
    (Join-Path $PSScriptRoot "..\config\settings.db"),
    (Join-Path (Get-Location) "config\settings.db")
)
foreach ($orphan in $LegacyOrphans) {
    if (Test-Path -Path $orphan) {
        try {
            Remove-Item -Path $orphan -Force -ErrorAction SilentlyContinue
            Write-Host "🧹 Purged legacy orphan database: $orphan" -ForegroundColor Green
        } catch {}
    }
}

# 3. Remove standalone binary and actx AppData folder
if (Test-Path -Path $InstallDir) {
    try {
        Remove-Item -Path $InstallDir -Recurse -Force
        Write-Host "✅ Removed standalone installation directory: $InstallDir" -ForegroundColor Green
    } catch {
        Write-Host "⚠️ Could not remove directory $InstallDir. Please ensure actx is not running." -ForegroundColor Red
    }
} else {
    Write-Host "ℹ️ Standalone installation directory not found at $InstallDir. Skipping." -ForegroundColor Gray
}

# 4. Detect and clean Python/pip installations
$PythonActxCommands = Get-Command actx -All -ErrorAction SilentlyContinue
if ($PythonActxCommands) {
    Write-Host "`n🔍 Detected 'actx' command in Python environment(s):" -ForegroundColor Yellow
    foreach ($cmd in $PythonActxCommands) {
        Write-Host "   📍 $($cmd.Path)" -ForegroundColor Cyan
    }
    Write-Host "🧹 Attempting to uninstall Python package via pip..." -ForegroundColor Yellow
    try {
        python -m pip uninstall -y any-context 2>$null | Out-Null
        Write-Host "✅ Ran 'pip uninstall -y any-context'." -ForegroundColor Green
    } catch {}
}

# 5. Remove from User PATH
$BinDir = Join-Path $InstallDir "bin"
$UserPath = [Environment]::GetEnvironmentVariable('Path', 'User')

if ($UserPath -like "*$BinDir*") {
    Write-Host "⚙️ Removing $BinDir from User PATH environment variable..." -ForegroundColor Yellow
    $PathList = $UserPath.Split(';') | Where-Object { $_ -and $_ -ne $BinDir }
    $NewPath = $PathList -join ';'
    [Environment]::SetEnvironmentVariable('Path', $NewPath, 'User')
    Write-Host "✅ Removed from User PATH successfully!" -ForegroundColor Green
} else {
    Write-Host "✅ $BinDir was not found in User PATH." -ForegroundColor Gray
}

Write-Host "`n=======================================================" -ForegroundColor Cyan
Write-Host "🎉 AnyContext (actx) has been uninstalled successfully." -ForegroundColor Green
Write-Host "=======================================================\n" -ForegroundColor Cyan

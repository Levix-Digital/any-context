# ==============================================================================
# AnyContext (actx) - Windows PowerShell Uninstaller Script
# Usage:
#   .\uninstall.ps1
# ==============================================================================

$ErrorActionPreference = 'Stop'

$InstallDir = Join-Path $env:LOCALAPPDATA "actx"

Write-Host "`n🧹 Uninstalling AnyContext (actx)..." -ForegroundColor Yellow

# 1. Remove binary and actx AppData folder
if (Test-Path -Path $InstallDir) {
    try {
        Remove-Item -Path $InstallDir -Recurse -Force
        Write-Host "✅ Removed installation directory: $InstallDir" -ForegroundColor Green
    } catch {
        Write-Host "⚠️ Could not remove directory $InstallDir. Please ensure actx is not running." -ForegroundColor Red
    }
} else {
    Write-Host "ℹ️ Installation directory not found at $InstallDir. Skipping." -ForegroundColor Gray
}

# 1b. Ask user if they want to preserve Workspaces & History
$ExePath = Join-Path $InstallDir "bin\actx.exe"
Write-Host "`n❓ Do you want to PRESERVE your configured Workspaces and Vector History for future installations? [Y/n]: " -NoNewline -ForegroundColor Yellow
$KeepAns = Read-Host

if ($KeepAns -match "^[Yy]$" -or [string]::IsNullOrEmpty($KeepAns)) {
    Write-Host "📂 Preserving Workspaces, Vector Database & History for future installations..." -ForegroundColor Green
    Write-Host "🧹 Resetting Model Settings & API Keys to OpenAI factory defaults..." -ForegroundColor Yellow
    if (Test-Path $ExePath) {
        try {
            & $ExePath --reset-models | Out-Null
        } catch {}
    }
} else {
    Write-Host "🧹 Performing 100% Clean Uninstall (Wiping all Workspaces, Databases & Configs)..." -ForegroundColor Red
    $ConfigPaths = @(
        (Join-Path $env:APPDATA "any-context"),
        (Join-Path $env:USERPROFILE ".config\any-context")
    )
    foreach ($CfgPath in $ConfigPaths) {
        if (Test-Path -Path $CfgPath) {
            try {
                Remove-Item -Path $CfgPath -Recurse -Force -ErrorAction SilentlyContinue
                Write-Host "🧹 Wiped configuration directory: $CfgPath" -ForegroundColor Green
            } catch {}
        }
    }
}



# 2. Remove from User PATH
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

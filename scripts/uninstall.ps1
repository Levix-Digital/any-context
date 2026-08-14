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

# 1b. Remove Configuration Databases & Settings (APPDATA & .config)
$ConfigPaths = @(
    (Join-Path $env:APPDATA "any-context"),
    (Join-Path $env:USERPROFILE ".config\any-context")
)
foreach ($CfgPath in $ConfigPaths) {
    if (Test-Path -Path $CfgPath) {
        try {
            Remove-Item -Path $CfgPath -Recurse -Force
            Write-Host "🧹 Cleaned configuration database: $CfgPath" -ForegroundColor Green
        } catch {
            Write-Host "⚠️ Could not remove configuration path $CfgPath." -ForegroundColor Yellow
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

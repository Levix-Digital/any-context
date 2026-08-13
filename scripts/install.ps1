# ==============================================================================
# AnyContext (actx) - Windows PowerShell Installer
# Usage:
#   iwr -useb https://raw.githubusercontent.com/Levix-Digital/any-context/dev/scripts/install.ps1 | iex
# ==============================================================================

$ErrorActionPreference = 'Stop'

$Repo = "Levix-Digital/any-context"
$InstallDir = Join-Path $env:LOCALAPPDATA "actx\bin"
$ExePath = Join-Path $InstallDir "actx.exe"
$DownloadUrl = "https://github.com/$Repo/releases/latest/download/actx-windows-x86_64.exe"

Write-Host "`n🚀 Installing AnyContext (actx)..." -ForegroundColor Cyan

# 1. Create target bin directory if not exists
if (-not (Test-Path -Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

# 2. Download executable
Write-Host "⬇️ Downloading latest actx executable from GitHub..." -ForegroundColor Yellow
try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $ExePath -UseBasicParsing
    Write-Host "✅ Download complete: $ExePath" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to download from Release ($DownloadUrl). Please ensure a Release with actx-windows-x86_64.exe exists on GitHub." -ForegroundColor Red
    exit 1
}

# 3. Add to User PATH if not present
$UserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($UserPath -notlike "*$InstallDir*") {
    Write-Host "⚙️ Adding $InstallDir to User PATH environment variable..." -ForegroundColor Yellow
    $NewPath = if ([string]::IsNullOrEmpty($UserPath)) { $InstallDir } else { "$UserPath;$InstallDir" }
    [Environment]::SetEnvironmentVariable('Path', $NewPath, 'User')
    $env:Path += ";$InstallDir"
    Write-Host "✅ Added to PATH successfully!" -ForegroundColor Green
} else {
    Write-Host "✅ $InstallDir is already in User PATH." -ForegroundColor Gray
}

Write-Host "`n=======================================================" -ForegroundColor Cyan
Write-Host "🎉 AnyContext (actx) installed successfully!" -ForegroundColor Green
Write-Host "👉 Open a new terminal window and type: actx" -ForegroundColor White
Write-Host "=======================================================\n" -ForegroundColor Cyan

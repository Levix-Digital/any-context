# ==============================================================================
# AnyContext (actx) - Windows PowerShell Installer
# Usage:
#   .\scripts\install.ps1
# ==============================================================================

$ErrorActionPreference = 'Stop'

$Repo = "Levix-Digital/any-context-releases"

$InstallDir = Join-Path $env:LOCALAPPDATA "actx\bin"
$ExePath = Join-Path $InstallDir "actx.exe"
$AssetName = "actx-windows-x86_64.exe"
$DownloadUrl = "https://github.com/$Repo/releases/latest/download/$AssetName"

Write-Host "`n🚀 Installing AnyContext (actx)..." -ForegroundColor Cyan

# 1. Create target bin directory if not exists
if (-not (Test-Path -Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

# 2. Download executable (Try gh CLI first for private repos, fallback to Invoke-WebRequest)
Write-Host "⬇️ Downloading latest $AssetName from GitHub..." -ForegroundColor Yellow

$Downloaded = $false

if (Get-Command gh -ErrorAction SilentlyContinue) {
    try {
        Write-Host "⚡ Using GitHub CLI (gh) for authenticated download..." -ForegroundColor Gray
        gh release download --repo $Repo --pattern $AssetName --dir $InstallDir --clobber
        $TempPath = Join-Path $InstallDir $AssetName
        if (Test-Path $TempPath) {
            if ($TempPath -ne $ExePath) {
                Move-Item -Path $TempPath -Destination $ExePath -Force
            }
            $Downloaded = $true
        }
    } catch {
        # Fallback to direct web request
    }
}

if (-not $Downloaded) {
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -Uri $DownloadUrl -OutFile $ExePath -UseBasicParsing
        $Downloaded = $true
    } catch {
        Write-Host "❌ Error: Release asset '$AssetName' is not available yet." -ForegroundColor Red
        Write-Host "⏳ GitHub Actions might still be compiling the latest release binary (~2-3 mins)." -ForegroundColor Yellow
        Write-Host "💡 Please wait 1-2 minutes and re-run '.\install.ps1' or log in via 'gh auth login'." -ForegroundColor Yellow
        exit 1
    }
}

Write-Host "✅ Download complete: $ExePath" -ForegroundColor Green

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

# ==============================================================================
# AnyContext (actx) - Windows PowerShell Installer
# Usage:
#   .\scripts\install.ps1
# ==============================================================================

$ErrorActionPreference = 'Stop'

$Repo = "Levix-Digital/any-context-releases"

$InstallDir = Join-Path $env:LOCALAPPDATA "actx\bin"
$CoreExePath = Join-Path $InstallDir "actx-core.exe"
$ShimExePath = Join-Path $InstallDir "actx.exe"
$VersionFilePath = Join-Path $InstallDir "version.txt"
$AssetName = "actx-windows-x86_64.exe"
$DownloadUrl = "https://github.com/$Repo/releases/latest/download/$AssetName"

Write-Host "`n🚀 Installing AnyContext (actx)..." -ForegroundColor Cyan

# 1. Create target bin directory if not exists
if (-not (Test-Path -Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

# 2. Download Core executable (Try gh CLI first for private repos, fallback to Invoke-WebRequest)
Write-Host "⬇️ Downloading latest $AssetName from GitHub..." -ForegroundColor Yellow

$Downloaded = $false

if (Get-Command gh -ErrorAction SilentlyContinue) {
    try {
        Write-Host "⚡ Using GitHub CLI (gh) for authenticated download..." -ForegroundColor Gray
        gh release download --repo $Repo --pattern $AssetName --dir $InstallDir --clobber
        $TempPath = Join-Path $InstallDir $AssetName
        if (Test-Path $TempPath) {
            Move-Item -Path $TempPath -Destination $CoreExePath -Force
            $Downloaded = $true
        }
    } catch {
        # Fallback to direct web request
    }
}

if (-not $Downloaded) {
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -Uri $DownloadUrl -OutFile $CoreExePath -UseBasicParsing
        $Downloaded = $true
    } catch {
        Write-Host "❌ Error: Release asset '$AssetName' is not available yet." -ForegroundColor Red
        Write-Host "⏳ GitHub Actions might still be compiling the latest release binary (~2-3 mins)." -ForegroundColor Yellow
        Write-Host "💡 Please wait 1-2 minutes and re-run '.\install.ps1' or log in via 'gh auth login'." -ForegroundColor Yellow
        exit 1
    }
}

Write-Host "✅ Engine downloaded: $CoreExePath" -ForegroundColor Green

# 3. Setup Version Cache File
$VersionTag = "0.28.76"
try {
    if (Get-Command gh -ErrorAction SilentlyContinue) {
        $ghTag = (gh release view --repo $Repo --json tagName -q .tagName 2>$null)
        if ($ghTag) { $VersionTag = $ghTag.TrimStart('v') }
    } else {
        $apiTag = (Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers @{"User-Agent"="AnyContext-Installer"} -TimeoutSec 5 -ErrorAction SilentlyContinue).tag_name
        if ($apiTag) { $VersionTag = $apiTag.TrimStart('v') }
    }
} catch {}
Set-Content -Path $VersionFilePath -Value $VersionTag -Encoding UTF8
Write-Host "✅ Version registered: v$VersionTag" -ForegroundColor Gray

# 4. Compile or Deploy Ultra-Fast Native Launcher Shim (actx.exe < 2ms)
Write-Host "⚡ Configuring ultra-fast native Launcher Shim (actx.exe)..." -ForegroundColor Gray
$CscPath = "C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe"
if (-not (Test-Path $CscPath)) {
    $CscPath = "C:\Windows\Microsoft.NET\Framework\v4.0.30319\csc.exe"
}

$ShimCompiled = $false
if (Test-Path $CscPath) {
    $ShimSrc = @"
using System;
using System.IO;
using System.Diagnostics;

namespace AnyContext.Launcher
{
    class Program
    {
        static int Main(string[] args)
        {
            string baseDir = AppDomain.CurrentDomain.BaseDirectory;
            if (args.Length > 0 && (args[0] == "-v" || args[0] == "--version"))
            {
                string vf = Path.Combine(baseDir, "version.txt");
                string ver = "v$VersionTag";
                if (File.Exists(vf)) {
                    try { string c = File.ReadAllText(vf).Trim(); if (!string.IsNullOrEmpty(c)) ver = c.StartsWith("v") ? c : "v" + c; } catch {}
                }
                Console.WriteLine(ver);
                return 0;
            }
            string core = Path.Combine(baseDir, "actx-core.exe");
            if (!File.Exists(core)) core = Path.Combine(baseDir, "actx-core");
            if (!File.Exists(core)) {
                Console.Error.WriteLine("Error: actx-core.exe not found in: " + baseDir);
                return 1;
            }
            string[] escaped = new string[args.Length];
            for (int i = 0; i < args.Length; i++) {
                string a = args[i];
                escaped[i] = (a.Contains(" ") || a.Contains("\"")) ? "\"" + a.Replace("\"", "\\\"") + "\"" : a;
            }
            try {
                ProcessStartInfo psi = new ProcessStartInfo { FileName = core, Arguments = string.Join(" ", escaped), UseShellExecute = false };
                using (Process p = Process.Start(psi)) {
                    if (p == null) return 1;
                    p.WaitForExit();
                    return p.ExitCode;
                }
            } catch (Exception ex) { Console.Error.WriteLine("Error: " + ex.Message); return 1; }
        }
    }
}
"@
    $TempCs = [System.IO.Path]::GetTempFileName() + ".cs"
    Set-Content -Path $TempCs -Value $ShimSrc -Encoding UTF8
    try {
        & $CscPath /nologo /optimize+ /target:exe /out:"$ShimExePath" "$TempCs" | Out-Null
        if (Test-Path $ShimExePath) {
            $ShimCompiled = $true
            Write-Host "✅ Native Launcher Shim compiled successfully!" -ForegroundColor Green
        }
    } catch {}
    Remove-Item -Path $TempCs -Force -ErrorAction SilentlyContinue
}

if (-not $ShimCompiled) {
    Copy-Item -Path $CoreExePath -Destination $ShimExePath -Force
}

# 5. Add to User PATH if not present
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

# 5. Check/Ensure Bun is available for OpenTUI desktop interface
$bunFound = (Get-Command bun -ErrorAction SilentlyContinue) -ne $null -or (Test-Path "$env:USERPROFILE\.bun\bin\bun.exe") -or (Test-Path "$InstallDir\bun.exe")
if (-not $bunFound) {
    Write-Host "💡 Bun runtime not detected. Installing Bun for OpenTUI desktop interface..." -ForegroundColor Gray
    try {
        powershell -c "irm bun.sh/install.ps1 | iex" | Out-Null
    } catch {}
}
if (Test-Path "$env:USERPROFILE\.bun\bin\bun.exe") {
    if (-not (Test-Path "$InstallDir\bun.exe")) {
        Copy-Item "$env:USERPROFILE\.bun\bin\bun.exe" "$InstallDir\bun.exe" -Force -ErrorAction SilentlyContinue
    }
}

Write-Host "`n=======================================================" -ForegroundColor Cyan
Write-Host "🎉 AnyContext (actx) installed successfully!" -ForegroundColor Green
Write-Host "👉 Open a new terminal window and type: actx" -ForegroundColor White
Write-Host "👉 To launch the OpenTUI desktop interface, type: actx --tui" -ForegroundColor White
Write-Host "=======================================================\n" -ForegroundColor Cyan

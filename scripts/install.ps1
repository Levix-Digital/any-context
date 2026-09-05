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

$LogsDir = Join-Path (Join-Path $env:LOCALAPPDATA "AnyContext") "logs"
if (-not (Test-Path -Path $LogsDir)) {
    New-Item -ItemType Directory -Path $LogsDir -Force | Out-Null
}
$InstallLogFile = Join-Path $LogsDir "install.log"

function Log-Install {
    param(
        [string]$Message,
        [string]$Level = "INFO"
    )
    $Timestamp = (Get-Date).ToString("yyyy-MM-ddTHH:mm:ssZ")
    $LogEntry = "[$Timestamp] [$Level] $Message"
    try {
        Add-Content -Path $InstallLogFile -Value $LogEntry -Encoding UTF8 -ErrorAction SilentlyContinue
    } catch {}
}

Log-Install "Installer initiated. Target dir: $InstallDir, Repo: $Repo"

Write-Host "`n>>> Installing AnyContext (actx)..." -ForegroundColor Cyan

# 1. Create target bin directory if not exists
if (-not (Test-Path -Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

function Extract-Package {
    param(
        [string]$ZipPath,
        [string]$Destination
    )
    $tarExe = "C:\Windows\System32\tar.exe"
    if (-not (Test-Path $tarExe)) {
        $found = Get-Command tar.exe -ErrorAction SilentlyContinue
        if ($found) { $tarExe = $found.Source }
    }
    if (Test-Path $tarExe) {
        try {
            & $tarExe -xf "$ZipPath" -C "$Destination"
            if ($LASTEXITCODE -eq 0) { return $true }
        } catch {}
    }
    try {
        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $archive = [System.IO.Compression.ZipFile]::OpenRead($ZipPath)
        foreach ($entry in $archive.Entries) {
            $target = [System.IO.Path]::Combine($Destination, $entry.FullName)
            if ([string]::IsNullOrEmpty($entry.Name)) {
                if (-not [System.IO.Directory]::Exists($target)) {
                    [System.IO.Directory]::CreateDirectory($target) | Out-Null
                }
            } else {
                $dir = [System.IO.Path]::GetDirectoryName($target)
                if (-not [System.IO.Directory]::Exists($dir)) {
                    [System.IO.Directory]::CreateDirectory($dir) | Out-Null
                }
                [System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $target, $true)
            }
        }
        $archive.Dispose()
        return $true
    } catch {}
    try {
        Expand-Archive -LiteralPath $ZipPath -DestinationPath $Destination -Force
        return $true
    } catch {}
    return $false
}

# 2. Download Core executable or distribution archive
$ArchiveName = "actx-windows-x86_64.zip"
$FallbackName = "actx-windows-x86_64.exe"
$Downloaded = $false

Write-Host "[-] Downloading latest AnyContext from GitHub..." -ForegroundColor Yellow
Log-Install "Downloading AnyContext from GitHub"

# Try archive (.zip) first for sub-second cold boot
if (Get-Command gh -ErrorAction SilentlyContinue) {
    try {
        Write-Host "[*] Using GitHub CLI (gh) for authenticated download..." -ForegroundColor Gray
        gh release download --repo $Repo --pattern $ArchiveName --dir $InstallDir --clobber
        $TempZip = Join-Path $InstallDir $ArchiveName
        if (Test-Path $TempZip) {
            Write-Host "[*] Extracting package contents..." -ForegroundColor Gray
            if (Extract-Package -ZipPath $TempZip -Destination $InstallDir) {
                Remove-Item -LiteralPath $TempZip -Force -ErrorAction SilentlyContinue
                $Downloaded = $true
            }
        }
    } catch {}
}

if (-not $Downloaded) {
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        $ZipUrl = "https://github.com/$Repo/releases/latest/download/$ArchiveName"
        $TempZip = Join-Path $InstallDir $ArchiveName
        Invoke-WebRequest -Uri $ZipUrl -OutFile $TempZip -UseBasicParsing
        if ((Test-Path $TempZip) -and (Get-Item $TempZip).Length -gt 0) {
            Write-Host "[*] Extracting package contents..." -ForegroundColor Gray
            if (Extract-Package -ZipPath $TempZip -Destination $InstallDir) {
                Remove-Item -LiteralPath $TempZip -Force -ErrorAction SilentlyContinue
                $Downloaded = $true
            }
        }
    } catch {}
}

# Fallback to single binary if archive was not found
if (-not $Downloaded) {
    if (Get-Command gh -ErrorAction SilentlyContinue) {
        try {
            gh release download --repo $Repo --pattern $FallbackName --dir $InstallDir --clobber
            $TempPath = Join-Path $InstallDir $FallbackName
            if (Test-Path $TempPath) {
                Move-Item -Path $TempPath -Destination $CoreExePath -Force
                $Downloaded = $true
            }
        } catch {}
    }
}

if (-not $Downloaded) {
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        $ExeUrl = "https://github.com/$Repo/releases/latest/download/$FallbackName"
        Invoke-WebRequest -Uri $ExeUrl -OutFile $CoreExePath -UseBasicParsing
        $Downloaded = $true
    } catch {
        Write-Host "[!] Error: Release asset is not available yet." -ForegroundColor Red
        Write-Host "[*] GitHub Actions might still be compiling the latest release binary (~2-3 mins)." -ForegroundColor Yellow
        Write-Host "[*] Please wait 1-2 minutes and re-run '.\install.ps1' or log in via 'gh auth login'." -ForegroundColor Yellow
        exit 1
    }
}

Write-Host "[OK] Engine configured: $CoreExePath" -ForegroundColor Green
Log-Install "Engine configured successfully at $CoreExePath"

# 3. Setup Version Cache File (BOM-free UTF-8)
$VersionTag = "0.28.89"
try {
    if (Get-Command gh -ErrorAction SilentlyContinue) {
        $ghTag = (gh release view --repo $Repo --json tagName -q .tagName 2>$null)
        if ($ghTag) { $VersionTag = $ghTag.TrimStart('v') }
    } else {
        $apiTag = (Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers @{"User-Agent"="AnyContext-Installer"} -TimeoutSec 5 -ErrorAction SilentlyContinue).tag_name
        if ($apiTag) { $VersionTag = $apiTag.TrimStart('v') }
    }
} catch {}
[System.IO.File]::WriteAllText($VersionFilePath, $VersionTag, (New-Object System.Text.UTF8Encoding $False))
Write-Host "[OK] Version registered: v$VersionTag" -ForegroundColor Gray
Log-Install "Version registered: v$VersionTag"


# 4. Compile or Deploy Ultra-Fast Native Launcher Shim (actx.exe < 2ms)
Write-Host "[*] Configuring ultra-fast native Launcher Shim (actx.exe)..." -ForegroundColor Gray
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
                    try {
                        string c = File.ReadAllText(vf).Trim().Trim('\uFEFF');
                        if (!string.IsNullOrEmpty(c)) ver = "v" + c.TrimStart('v', 'V');
                    } catch {}
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
    [System.IO.File]::WriteAllText($TempCs, $ShimSrc, (New-Object System.Text.UTF8Encoding $False))
    try {
        & $CscPath /nologo /optimize+ /target:exe /out:"$ShimExePath" "$TempCs" | Out-Null
        if (Test-Path $ShimExePath) {
            $ShimCompiled = $true
            Write-Host "[OK] Native Launcher Shim compiled successfully!" -ForegroundColor Green
        }
    } catch {}
    Remove-Item -Path $TempCs -Force -ErrorAction SilentlyContinue
}

if (-not $ShimCompiled) {
    if (-not (Test-Path $ShimExePath)) {
        Copy-Item -Path $CoreExePath -Destination $ShimExePath -Force
    }
}

# Deploy Git Bash / MSYS2 / Cygwin shell wrapper 'actx'
$BashShimPath = Join-Path $InstallDir "actx"
$BashShimContent = @"
#!/usr/bin/env sh
BIN_DIR="`$(CDPATH= cd -- "`$(dirname -- "`$0")" && pwd)"
if [ "`$1" = "-v" ] || [ "`$1" = "--version" ]; then
    if [ -f "`$BIN_DIR/version.txt" ]; then
        V="`$(cat "`$BIN_DIR/version.txt" | tr -d '\r\n' | sed -e 's/\xef\xbb\xbf//g' -e 's/^[vV]*//' | sed 's/^/v/')"
        echo "`$V"
    else
        echo "v$VersionTag"
    fi
    exit 0
fi

if [ -f "`$BIN_DIR/actx-core.exe" ]; then
    exec "`$BIN_DIR/actx-core.exe" "`$@"
elif [ -f "`$BIN_DIR/actx.exe" ]; then
    exec "`$BIN_DIR/actx.exe" "`$@"
elif [ -f "`$BIN_DIR/actx-core" ]; then
    exec "`$BIN_DIR/actx-core" "`$@"
fi
"@
[System.IO.File]::WriteAllText($BashShimPath, ($BashShimContent -replace "`r`n", "`n"), (New-Object System.Text.UTF8Encoding $False))
Write-Host "[OK] Git Bash wrapper deployed: $BashShimPath" -ForegroundColor Gray


# 5. Add to User PATH if not present
$UserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($UserPath -notlike "*$InstallDir*") {
    Write-Host "[*] Adding $InstallDir to User PATH environment variable..." -ForegroundColor Yellow
    $NewPath = if ([string]::IsNullOrEmpty($UserPath)) { $InstallDir } else { "$UserPath;$InstallDir" }
    [Environment]::SetEnvironmentVariable('Path', $NewPath, 'User')
    $env:Path += ";$InstallDir"
    Write-Host "[OK] Added to PATH successfully!" -ForegroundColor Green
} else {
    Write-Host "[OK] $InstallDir is already in User PATH." -ForegroundColor Gray
}

# 5. Check/Ensure Bun is available for OpenTUI desktop interface
$bunFound = (Get-Command bun -ErrorAction SilentlyContinue) -ne $null -or (Test-Path "$env:USERPROFILE\.bun\bin\bun.exe") -or (Test-Path "$InstallDir\bun.exe")
if (-not $bunFound) {
    Write-Host "[*] Bun runtime not detected. Installing Bun for OpenTUI desktop interface..." -ForegroundColor Gray
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
Write-Host "AnyContext (actx) installed successfully!" -ForegroundColor Green
Write-Host "Open a new terminal window and type: actx" -ForegroundColor White
Write-Host "To launch the OpenTUI desktop interface, type: actx --tui" -ForegroundColor White
Write-Host "=======================================================\n" -ForegroundColor Cyan
Log-Install "Installation completed successfully. Version: v$VersionTag"


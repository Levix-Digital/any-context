import os
import sys
import json
import subprocess
import urllib.request
from typing import Optional, Tuple
from any_context import __version__ as CURRENT_VERSION

PRIMARY_REPO = "Levix-Digital/any-context-releases"
FALLBACK_REPO = "Levix-Digital/any-context"


def safe_print(msg: str):
    try:
        print(msg)
    except UnicodeEncodeError:
        print(msg.encode("ascii", errors="ignore").decode("ascii"))


def clean_stale_update_files():
    """Silently cleans up any temporary or old backup binaries left from previous updates."""
    try:
        is_windows = sys.platform == "win32" or ("MINGW" in os.environ.get("MSYSTEM", ""))
        if getattr(sys, "frozen", False):
            target_dir = os.path.dirname(os.path.abspath(sys.executable))
        else:
            target_dir = os.path.expanduser("~/AppData/Local/actx/bin" if is_windows else "~/.local/bin")

        if os.path.exists(target_dir):
            for fname in ["actx_old.exe", "actx_new.exe", "actx_old", "actx_new"]:
                p = os.path.join(target_dir, fname)
                if os.path.exists(p):
                    try:
                        os.remove(p)
                    except Exception:
                        pass
    except Exception:
        pass


def parse_version_tuple(version_str: str) -> Tuple[int, ...]:
    """Parses a version string like '0.11.63' or 'v0.11.63' into integer tuple (0, 11, 63)"""
    cleaned = version_str.lstrip("v").strip()
    if " " in cleaned:
        cleaned = cleaned.split()[-1].lstrip("v")
    try:
        parts = []
        for part in cleaned.split("."):
            num = ""
            for c in part:
                if c.isdigit():
                    num += c
                else:
                    break
            if num:
                parts.append(int(num))
        return tuple(parts) if parts else (0, 0, 0)
    except Exception:
        return (0, 0, 0)


def fetch_latest_release_tag() -> Optional[str]:
    """
    Fetches the latest release tag from GitHub.
    Checks Levix-Digital/any-context-releases, and falls back to Levix-Digital/any-context.
    """
    import time
    for repo in [PRIMARY_REPO, FALLBACK_REPO]:
        # 1. Try public GitHub API first
        try:
            url = f"https://api.github.com/repos/{repo}/releases/latest?_t={int(time.time())}"
            headers = {
                "User-Agent": "AnyContext-CLI",
                "Cache-Control": "no-cache, no-store, must-revalidate",
                "Pragma": "no-cache"
            }
            req = urllib.request.Request(url, headers=headers)
            with urllib.request.urlopen(req, timeout=5) as response:
                if response.status == 200:
                    data = json.loads(response.read().decode("utf-8"))
                    tag = data.get("tag_name")
                    if tag:
                        return tag
        except Exception:
            pass

        # 2. Fallback to gh CLI
        try:
            res = subprocess.run(
                ["gh", "release", "view", "--repo", repo, "--json", "tagName"],
                capture_output=True,
                text=True,
                timeout=5
            )
            if res.returncode == 0 and res.stdout.strip():
                data = json.loads(res.stdout)
                if "tagName" in data:
                    return data["tagName"]
        except Exception:
            pass

    return None


def check_for_updates(quiet_if_latest: bool = True) -> Tuple[bool, Optional[str]]:
    """
    Checks if a newer release version exists on GitHub.
    Returns (has_update, latest_version_tag)
    """
    clean_stale_update_files()
    latest_tag = fetch_latest_release_tag()
    if not latest_tag:
        if not quiet_if_latest:
            safe_print("\n⚠️ Could not check for updates (GitHub offline or network unavailable).\n")
        return False, None

    current_tuple = parse_version_tuple(CURRENT_VERSION)
    latest_tuple = parse_version_tuple(latest_tag)

    clean_tag = latest_tag if latest_tag.startswith("v") else f"v{latest_tag}"
    yellow = "\033[93m"
    cyan = "\033[96m"
    bold = "\033[1m"
    reset = "\033[0m"

    if latest_tuple > current_tuple:
        if not quiet_if_latest:
            safe_print(f"\n{yellow}💡 New update available! {bold}v{CURRENT_VERSION}{reset}{yellow} → {bold}{clean_tag}{reset}")
            safe_print(f"{cyan}👉 Run 'actx --update' or type '/update' inside the chat to update automatically.{reset}\n")
        return True, clean_tag
    else:
        if not quiet_if_latest:
            safe_print(f"\n✅ You are already running the latest version of AnyContext (v{CURRENT_VERSION}).\n")
        return False, clean_tag


def print_startup_update_notice():
    """
    Fast, non-blocking check printed right below startup banner if an update is available.
    """
    has_update, latest_tag = check_for_updates(quiet_if_latest=True)
    if has_update and latest_tag:
        clean_tag = latest_tag if latest_tag.startswith("v") else f"v{latest_tag}"
        yellow = "\033[93m"
        cyan = "\033[96m"
        bold = "\033[1m"
        reset = "\033[0m"
        safe_print(f"{yellow}💡 Update available! {bold}v{CURRENT_VERSION}{reset}{yellow} → {bold}{clean_tag}{reset}")
        safe_print(f"{cyan}👉 Run 'actx --update' or type '/update' inside the chat to update automatically.{reset}\n")


def run_self_update():
    """
    Executes automatic binary update by downloading the latest release asset
    and performing safe, atomic replacement (supporting locked executables on Windows).
    """
    safe_print(f"\n🔍 Checking for AnyContext updates...")
    has_update, latest_tag = check_for_updates(quiet_if_latest=False)

    if not has_update or not latest_tag:
        return

    clean_tag = latest_tag if latest_tag.startswith("v") else f"v{latest_tag}"
    safe_print(f"\n🚀 Updating AnyContext from v{CURRENT_VERSION} to {clean_tag}...")

    is_windows = sys.platform == "win32" or ("MINGW" in os.environ.get("MSYSTEM", ""))
    target_asset = "actx-windows-x86_64.exe" if is_windows else "actx-linux-x86_64"

    # Determine target executable location
    if getattr(sys, "frozen", False):
        target_exe = os.path.abspath(sys.executable)
        target_dir = os.path.dirname(target_exe)
    else:
        target_dir = os.path.expanduser("~/AppData/Local/actx/bin" if is_windows else "~/.local/bin")
        target_exe = os.path.join(target_dir, "actx.exe" if is_windows else "actx")

    os.makedirs(target_dir, exist_ok=True)
    temp_download = os.path.join(target_dir, "actx_new.exe" if is_windows else "actx_new")
    old_exe = os.path.join(target_dir, "actx_old.exe" if is_windows else "actx_old")

    if os.path.exists(temp_download):
        try:
            os.remove(temp_download)
        except Exception:
            pass

    downloaded = False

    # 1. Direct HTTP release download with real-time stream progress
    safe_print(f"⬇️ Downloading '{target_asset}' from GitHub Release {clean_tag}...")
    for repo in [PRIMARY_REPO, FALLBACK_REPO]:
        try:
            url = f"https://github.com/{repo}/releases/download/{latest_tag}/{target_asset}"
            req = urllib.request.Request(url, headers={"User-Agent": "AnyContext-CLI"})
            with urllib.request.urlopen(req, timeout=120) as response:
                if response.status == 200:
                    total_size = int(response.headers.get("Content-Length", 0))
                    downloaded_bytes = 0
                    chunk_size = 1024 * 512  # 512 KB chunks

                    with open(temp_download, "wb") as f:
                        while True:
                            chunk = response.read(chunk_size)
                            if not chunk:
                                break
                            f.write(chunk)
                            downloaded_bytes += len(chunk)
                            if total_size > 0:
                                pct = int((downloaded_bytes / total_size) * 100)
                                mb_done = downloaded_bytes / (1024 * 1024)
                                mb_total = total_size / (1024 * 1024)
                                sys.stdout.write(f"\r  [Download] {mb_done:.1f} MB / {mb_total:.1f} MB ({pct}%)")
                                sys.stdout.flush()

                    sys.stdout.write("\n")
                    downloaded = True
                    break
        except Exception:
            pass

    # 2. Fallback to gh CLI download (for private repositories)
    if not downloaded:
        for repo in [PRIMARY_REPO, FALLBACK_REPO]:
            try:
                res = subprocess.run(
                    ["gh", "release", "download", latest_tag, "--repo", repo, "--pattern", target_asset, "--dir", target_dir, "--clobber"],
                    capture_output=True,
                    text=True,
                    timeout=120
                )
                downloaded_file = os.path.join(target_dir, target_asset)
                if res.returncode == 0 and os.path.exists(downloaded_file):
                    if os.path.exists(temp_download):
                        os.remove(temp_download)
                    os.rename(downloaded_file, temp_download)
                    downloaded = True
                    break
            except Exception:
                pass

    if not downloaded or not os.path.exists(temp_download) or os.path.getsize(temp_download) == 0:
        safe_print(f"\n❌ Failed to download update asset '{target_asset}' for release {clean_tag}.")
        safe_print(f"💡 Please check your internet connection or run manual install: https://github.com/{PRIMARY_REPO}/releases\n")
        return

    # Set executable permissions on Unix
    if not is_windows:
        try:
            os.chmod(temp_download, 0o755)
        except Exception:
            pass

    # 3. Perform atomic replacement
    if is_windows:
        # Step A: Try immediate rename of target_exe -> old_exe
        try:
            if os.path.exists(old_exe):
                try:
                    os.remove(old_exe)
                except Exception:
                    pass
            if os.path.exists(target_exe):
                os.rename(target_exe, old_exe)
            os.rename(temp_download, target_exe)
        except Exception:
            pass

        # Step B: Spawn background PowerShell loop with retry to complete swap & cleanup
        swap_script = (
            f"$retries = 0; "
            f"while ($retries -lt 30) {{ "
            f"  try {{ "
            f"    if (Test-Path -LiteralPath '{temp_download}') {{ "
            f"      Move-Item -LiteralPath '{temp_download}' -Destination '{target_exe}' -Force -ErrorAction Stop "
            f"    }} "
            f"    if (Test-Path -LiteralPath '{old_exe}') {{ "
            f"      Remove-Item -LiteralPath '{old_exe}' -Force -ErrorAction SilentlyContinue "
            f"    }} "
            f"    break "
            f"  }} catch {{ "
            f"    Start-Sleep -Milliseconds 500; "
            f"    $retries++ "
            f"  }} "
            f"}}"
        )
        try:
            subprocess.Popen(
                ["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", swap_script],
                creationflags=subprocess.CREATE_NO_WINDOW if hasattr(subprocess, "CREATE_NO_WINDOW") else 0
            )
        except Exception:
            pass

        safe_print(f"\n🎉 AnyContext successfully updated to {clean_tag}!")
        safe_print(f"👉 Restart your terminal or type 'actx' to launch {clean_tag}.\n")
        sys.exit(0)
    else:
        try:
            os.replace(temp_download, target_exe)
            safe_print(f"\n🎉 AnyContext successfully updated to {clean_tag}!")
            safe_print(f"👉 Restart your terminal or type 'actx' to launch {clean_tag}.\n")
            sys.exit(0)
        except Exception as e:
            safe_print(f"⚠️ Saved new binary to: {temp_download}. Please move it to {target_exe} with sudo/chmod.")

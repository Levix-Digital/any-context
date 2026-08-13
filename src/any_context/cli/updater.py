import os
import sys
import json
import subprocess
import urllib.request
from typing import Optional, Tuple
from any_context import __version__ as CURRENT_VERSION

REPO = "Levix-Digital/any-context"

def safe_print(msg: str):
    try:
        print(msg)
    except UnicodeEncodeError:
        print(msg.encode("ascii", errors="ignore").decode("ascii"))

def parse_version_tuple(version_str: str) -> Tuple[int, ...]:
    """Parses a version string like '0.3.4' or 'v0.3.4' into integer tuple (0, 3, 4)"""
    cleaned = version_str.lstrip("v").strip()
    try:
        return tuple(int(part) for part in cleaned.split("."))
    except ValueError:
        return (0, 0, 0)

def fetch_latest_release_tag() -> Optional[str]:
    """
    Fetches the latest release tag from GitHub.
    Uses public GitHub REST API first (fast & zero dependencies), and falls back to gh CLI for private repos.
    """
    # 1. Try public GitHub API first (Fast, lightweight, no sub-process overhead)
    try:
        url = f"https://api.github.com/repos/{REPO}/releases/latest"
        req = urllib.request.Request(url, headers={"User-Agent": "AnyContext-CLI"})
        with urllib.request.urlopen(req, timeout=3) as response:
            if response.status == 200:
                data = json.loads(response.read().decode("utf-8"))
                return data.get("tag_name")
    except Exception:
        pass

    # 2. Fallback to gh CLI (supports private repos with authenticated gh CLI)
    try:
        res = subprocess.run(
            ["gh", "release", "view", "--repo", REPO, "--json", "tagName"],
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
    latest_tag = fetch_latest_release_tag()
    if not latest_tag:
        if not quiet_if_latest:
            safe_print("⚠️ Could not check for updates (GitHub offline or authenticated gh CLI required).")
        return False, None

    current_tuple = parse_version_tuple(CURRENT_VERSION)
    latest_tuple = parse_version_tuple(latest_tag)

    if latest_tuple > current_tuple:
        return True, latest_tag
    else:
        if not quiet_if_latest:
            safe_print(f"✅ You are running the latest version of AnyContext (v{CURRENT_VERSION}).")
        return False, latest_tag

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
    and performing safe replacement (supporting locked executables on Windows).
    """
    safe_print(f"\n🔍 Checking for AnyContext updates...")
    has_update, latest_tag = check_for_updates(quiet_if_latest=False)

    if not has_update:
        return

    clean_tag = latest_tag if latest_tag.startswith("v") else f"v{latest_tag}"
    safe_print(f"\n🚀 Updating AnyContext from v{CURRENT_VERSION} to {clean_tag}...")

    is_windows = sys.platform == "win32" or ("MINGW" in os.environ.get("MSYSTEM", ""))
    target_asset = "actx-windows-x86_64.exe" if is_windows else "actx-linux-x86_64"

    # Default install target directory
    if is_windows:
        target_dir = os.path.expanduser("~/AppData/Local/actx/bin")
    else:
        target_dir = os.path.expanduser("~/.local/bin")

    os.makedirs(target_dir, exist_ok=True)
    target_exe = os.path.join(target_dir, "actx.exe" if is_windows else "actx")
    temp_download = os.path.join(target_dir, "actx_new.exe" if is_windows else "actx_new")

    downloaded = False

    # 1. Try direct HTTP release download first (Fast, zero dependencies)
    safe_print(f"⬇️ Downloading '{target_asset}' from GitHub Release {clean_tag}...")
    try:
        url = f"https://github.com/{REPO}/releases/download/{latest_tag}/{target_asset}"
        req = urllib.request.Request(url, headers={"User-Agent": "AnyContext-CLI"})
        with urllib.request.urlopen(req, timeout=60) as response:
            if response.status == 200:
                with open(temp_download, "wb") as f:
                    f.write(response.read())
                downloaded = True
    except Exception:
        pass

    # 2. Fallback to gh CLI download (for private repositories)
    if not downloaded:
        try:
            res = subprocess.run(
                ["gh", "release", "download", latest_tag, "--repo", REPO, "--pattern", target_asset, "--dir", target_dir, "--clobber"],
                capture_output=True,
                text=True,
                timeout=60
            )
            downloaded_file = os.path.join(target_dir, target_asset)
            if res.returncode == 0 and os.path.exists(downloaded_file):
                if os.path.exists(temp_download):
                    os.remove(temp_download)
                os.rename(downloaded_file, temp_download)
                downloaded = True
        except Exception:
            pass


    if not downloaded:
        safe_print(f"❌ Failed to download update asset '{target_asset}' for release {clean_tag}.")
        safe_print(f"💡 For private repositories, make sure GitHub CLI is authenticated via 'gh auth login'.")
        return

    # Set executable permissions on Unix
    if not is_windows:
        try:
            os.chmod(temp_download, 0o755)
        except Exception:
            pass

    # 3. Perform atomic replacement
    if is_windows:
        # On Windows, locked running executable cannot be overwritten immediately.
        # Spawn background process to wait 1 sec and swap files after CLI exits.
        swap_script = (
            f"Start-Sleep -Seconds 1; "
            f"Move-Item -Path '{temp_download}' -Destination '{target_exe}' -Force; "
            f"Write-Host '🎉 AnyContext updated to {clean_tag}!'"
        )
        try:
            subprocess.Popen(["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", swap_script], creationflags=subprocess.CREATE_NEW_CONSOLE)
            safe_print(f"🎉 Update downloaded! AnyContext will finish replacing files when you close this window.")
            safe_print(f"👉 Restart your terminal or type 'actx' to launch {clean_tag}.")
            sys.exit(0)
        except Exception:
            # Direct rename fallback
            try:
                os.replace(temp_download, target_exe)
                safe_print(f"🎉 AnyContext updated successfully to {clean_tag}!")
            except Exception as e:
                safe_print(f"⚠️ Saved new binary to: {temp_download}. Please replace {target_exe} manually.")
    else:
        try:
            os.replace(temp_download, target_exe)
            safe_print(f"🎉 AnyContext updated successfully to {clean_tag}!")
        except Exception as e:
            safe_print(f"⚠️ Saved new binary to: {temp_download}. Please move it to {target_exe} with sudo/chmod.")

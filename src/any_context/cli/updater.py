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
    Uses gh CLI if available (supports private repos), or falls back to public API.
    """
    # 1. Try gh CLI
    try:
        res = subprocess.run(
            ["gh", "release", "view", "--repo", REPO, "--json", "tagName"],
            capture_output=True,
            text=True,
            timeout=3
        )
        if res.returncode == 0 and res.stdout.strip():
            data = json.loads(res.stdout)
            if "tagName" in data:
                return data["tagName"]
    except Exception:
        pass

    # 2. Fallback to public GitHub API
    try:
        url = f"https://api.github.com/repos/{REPO}/releases/latest"
        req = urllib.request.Request(url, headers={"User-Agent": "AnyContext-CLI"})
        with urllib.request.urlopen(req, timeout=2) as response:
            if response.status == 200:
                data = json.loads(response.read().decode("utf-8"))
                return data.get("tag_name")
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
        yellow = "\033[93m"
        cyan = "\033[96m"
        bold = "\033[1m"
        reset = "\033[0m"
        safe_print(f"{yellow}💡 Update available! {bold}v{CURRENT_VERSION}{reset}{yellow} → {bold}{latest_tag}{reset}")
        safe_print(f"{cyan}👉 Run 'actx --update' or type '/update' inside the chat to update automatically.{reset}\n")

def run_self_update():
    """
    Executes automatic binary update by downloading and running installer script
    """
    safe_print(f"\n🔍 Checking for AnyContext updates...")
    has_update, latest_tag = check_for_updates(quiet_if_latest=False)

    if not has_update:
        return

    safe_print(f"\n🚀 Updating AnyContext from v{CURRENT_VERSION} to {latest_tag}...")

    is_windows = sys.platform == "win32" or ("MINGW" in os.environ.get("MSYSTEM", ""))

    if is_windows:
        # Run install.ps1 or install.sh via subprocess
        try:
            if os.name == "nt":
                cmd = ["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command",
                       f"iwr -useb https://raw.githubusercontent.com/{REPO}/main/scripts/install.ps1 | iex"]
            else:
                cmd = ["bash", "-c", f"curl -fsSL https://raw.githubusercontent.com/{REPO}/main/scripts/install.sh | sh"]

            # If gh is available, use gh release download
            res = subprocess.run(["gh", "release", "download", latest_tag, "--repo", REPO, "--pattern", "actx-windows-x86_64.exe", "--dir", os.path.expanduser("~/AppData/Local/actx/bin"), "--clobber"], capture_output=True, text=True)
            if res.returncode == 0:
                safe_print(f"🎉 AnyContext updated successfully to {latest_tag}!")
                return
        except Exception as e:
            pass

    # Generic installer execution fallback
    safe_print(f"💡 Please run the installer script to complete update to {latest_tag}:")
    if is_windows:
        safe_print("   powershell: .\\install.ps1")
    else:
        safe_print("   bash: ./install.sh")

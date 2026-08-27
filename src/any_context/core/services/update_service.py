"""
Update Service - Core application service for checking and orchestrating AnyContext releases.
Hexagonal architecture: domain capability decoupled from CLI/TUI/API presentation adapters.
"""

import os
import sys
import json
import urllib.request
import subprocess
from typing import Optional, Tuple, List, Dict, Any
from any_context import __version__ as CURRENT_VERSION

PRIMARY_REPO = "Levix-Digital/any-context-releases"
FALLBACK_REPO = "Levix-Digital/any-context"


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
    """Parses a version string like '0.28.4' or 'v0.28.4' into integer tuple (0, 28, 4)"""
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


def normalize_version_tag(version_str: str) -> str:
    """Normalizes version strings like '@0.28.4', '0.28.4', 'v0.28.4', '@latest' -> 'v0.28.4' or 'latest'."""
    cleaned = version_str.strip()
    if cleaned.startswith("@"):
        cleaned = cleaned[1:].strip()
    if cleaned.lower() in ["latest", "current", "head"]:
        return "latest"
    if not cleaned.startswith("v"):
        cleaned = f"v{cleaned}"
    return cleaned


class UpdateService:
    """Core domain service for querying release status and updating AnyContext."""

    def __init__(self, primary_repo: str = PRIMARY_REPO, fallback_repo: str = FALLBACK_REPO):
        self.primary_repo = primary_repo
        self.fallback_repo = fallback_repo

    def get_current_version(self) -> str:
        """Returns the current local version string."""
        return CURRENT_VERSION

    def fetch_latest_release_tag(self) -> Optional[str]:
        """Fetches the latest release tag from GitHub releases."""
        import time
        for repo in [self.primary_repo, self.fallback_repo]:
            try:
                url = f"https://api.github.com/repos/{repo}/releases/latest?_t={int(time.time())}"
                headers = {
                    "User-Agent": "AnyContext-UpdateService",
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

    def fetch_available_releases(self, limit: int = 15) -> List[Dict[str, Any]]:
        """Fetches the list of recent release tags and metadata from GitHub."""
        import time
        releases = []
        for repo in [self.primary_repo, self.fallback_repo]:
            try:
                url = f"https://api.github.com/repos/{repo}/releases?per_page={limit}&_t={int(time.time())}"
                headers = {
                    "User-Agent": "AnyContext-UpdateService",
                    "Cache-Control": "no-cache, no-store, must-revalidate",
                    "Pragma": "no-cache"
                }
                req = urllib.request.Request(url, headers=headers)
                with urllib.request.urlopen(req, timeout=6) as response:
                    if response.status == 200:
                        data = json.loads(response.read().decode("utf-8"))
                        if isinstance(data, list) and data:
                            for item in data:
                                tag = item.get("tag_name", "")
                                if tag:
                                    releases.append({
                                        "tag": tag if tag.startswith("v") else f"v{tag}",
                                        "name": item.get("name") or tag,
                                        "published_at": (item.get("published_at") or "")[:10],
                                        "prerelease": item.get("prerelease", False),
                                        "draft": item.get("draft", False),
                                    })
                            if releases:
                                return releases
            except Exception:
                pass

        return releases

    def check_for_updates(self) -> Tuple[bool, Optional[str]]:
        """
        Checks if a newer release exists on GitHub.
        Returns: (has_update: bool, latest_version_tag: Optional[str])
        """
        clean_stale_update_files()
        latest_tag = self.fetch_latest_release_tag()
        if not latest_tag:
            return False, None

        current_tuple = parse_version_tuple(CURRENT_VERSION)
        latest_tuple = parse_version_tuple(latest_tag)
        clean_tag = latest_tag if latest_tag.startswith("v") else f"v{latest_tag}"

        if latest_tuple > current_tuple:
            return True, clean_tag
        return False, clean_tag

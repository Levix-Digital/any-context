"""
Update Service - Core application service for checking and orchestrating AnyContext releases.
Hexagonal architecture: domain capability decoupled from CLI/TUI/API presentation adapters.
"""

import os
import sys
import json
import urllib.request
import subprocess
from typing import Optional, Tuple, List, Dict, Any, Set
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


def get_current_session_pids() -> Set[int]:
    """
    Returns the comprehensive set of Process IDs belonging to the CURRENT AnyContext session:
    - Current process PID
    - Parent PID (os.getppid())
    - Any PID defined in ACTX_LAUNCHER_PID, ACTX_ROOT_PID, ACTX_TUI_PID
    - All ancestors walking up the process tree until reaching the interactive terminal shell
    - All descendants spawned by any session ancestor
    
    Guarantees that find_active_instances() and close_active_instances() NEVER identify or kill
    the active terminal foreground process (Launcher Shim actx.exe, actx-core.exe, bun, or RPC bridge),
    preventing terminal prompt leak / screen corruption during updates.
    """
    session_pids: Set[int] = {os.getpid()}

    for var in ["ACTX_LAUNCHER_PID", "ACTX_ROOT_PID", "ACTX_TUI_PID"]:
        val = os.environ.get(var)
        if val and val.isdigit():
            session_pids.add(int(val))

    if hasattr(os, "getppid"):
        try:
            session_pids.add(os.getppid())
        except Exception:
            pass

    is_windows = sys.platform == "win32" or ("MINGW" in os.environ.get("MSYSTEM", ""))

    if is_windows:
        import ctypes
        from ctypes import wintypes

        class PROCESSENTRY32(ctypes.Structure):
            _fields_ = [
                ("dwSize", wintypes.DWORD),
                ("cntUsage", wintypes.DWORD),
                ("th32ProcessID", wintypes.DWORD),
                ("th32DefaultHeapID", ctypes.POINTER(ctypes.c_ulong)),
                ("th32ModuleID", wintypes.DWORD),
                ("cntThreads", wintypes.DWORD),
                ("th32ParentProcessID", wintypes.DWORD),
                ("pcPriClassBase", ctypes.c_long),
                ("dwFlags", wintypes.DWORD),
                ("szExeFile", ctypes.c_char * 260)
            ]

        try:
            kernel32 = ctypes.windll.kernel32
            hSnapshot = kernel32.CreateToolhelp32Snapshot(0x00000002, 0)
            if hSnapshot and hSnapshot != -1:
                pe = PROCESSENTRY32()
                pe.dwSize = ctypes.sizeof(PROCESSENTRY32)

                parent_of = {}
                children_of = {}
                exe_of = {}

                if kernel32.Process32First(hSnapshot, ctypes.byref(pe)):
                    while True:
                        pid = int(pe.th32ProcessID)
                        ppid = int(pe.th32ParentProcessID)
                        exe = pe.szExeFile.decode("utf-8", errors="ignore").lower()
                        parent_of[pid] = ppid
                        children_of.setdefault(ppid, []).append(pid)
                        exe_of[pid] = exe
                        if not kernel32.Process32Next(hSnapshot, ctypes.byref(pe)):
                            break

                kernel32.CloseHandle(hSnapshot)

                TERMINAL_SHELLS = {
                    "explorer.exe", "cmd.exe", "powershell.exe", "pwsh.exe",
                    "bash.exe", "zsh.exe", "mintty.exe", "windowsterminal.exe",
                    "conhost.exe", "openconsole.exe", "wsl.exe", "services.exe",
                    "system", "svchost.exe"
                }

                # 1. Walk upwards: follow ancestors of session_pids as long as they are not terminal shells
                to_trace = list(session_pids)
                actx_family_roots = set(session_pids)

                while to_trace:
                    p = to_trace.pop()
                    parent = parent_of.get(p)
                    if not parent or parent <= 4:
                        continue
                    parent_exe = exe_of.get(parent, "")
                    if parent_exe in TERMINAL_SHELLS:
                        continue
                    if parent not in actx_family_roots:
                        actx_family_roots.add(parent)
                        to_trace.append(parent)

                session_pids.update(actx_family_roots)

                # 2. Walk downwards from actx family roots
                to_descend = list(actx_family_roots)
                descendants = set(actx_family_roots)
                while to_descend:
                    curr = to_descend.pop()
                    for child in children_of.get(curr, []):
                        child_exe = exe_of.get(child, "")
                        if child_exe not in TERMINAL_SHELLS and child not in descendants:
                            descendants.add(child)
                            to_descend.append(child)

                session_pids.update(descendants)
        except Exception:
            pass
    else:
        # Unix
        try:
            curr = os.getpid()
            while curr > 1:
                with open(f"/proc/{curr}/stat", "r") as f:
                    ppid = int(f.read().split()[3])
                if ppid <= 1 or ppid == curr or ppid in session_pids:
                    break
                session_pids.add(ppid)
                curr = ppid
        except Exception:
            pass

    return session_pids


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

    @staticmethod
    def find_active_instances() -> List[Dict[str, Any]]:
        """
        Detects all running AnyContext processes across the system, strictly excluding the current session hierarchy.
        """
        instances = []
        ignored_pids = get_current_session_pids()
        is_windows = sys.platform == "win32" or ("MINGW" in os.environ.get("MSYSTEM", ""))

        if is_windows:
            try:
                res = subprocess.run(
                    ["tasklist", "/FO", "CSV", "/NH"],
                    capture_output=True,
                    text=True,
                    timeout=5
                )
                if res.returncode == 0 and res.stdout:
                    import csv
                    reader = csv.reader(res.stdout.strip().splitlines())
                    for row in reader:
                        if len(row) >= 2:
                            img_name = row[0].strip().strip('"')
                            pid_str = row[1].strip().strip('"')

                            if pid_str.isdigit():
                                pid = int(pid_str)
                                if pid in ignored_pids:
                                    continue

                                img_lower = img_name.lower()
                                if img_lower in ["actx.exe", "actx-core.exe", "anycontext.exe", "any-context.exe", "ac.exe"]:
                                    instances.append({
                                        "pid": pid,
                                        "name": img_name,
                                        "title": f"AnyContext Session (PID: {pid})",
                                        "type": "cli"
                                    })
            except Exception:
                pass
        else:
            try:
                res = subprocess.run(
                    ["ps", "-eo", "pid,comm,args"],
                    capture_output=True,
                    text=True,
                    timeout=4
                )
                if res.returncode == 0 and res.stdout:
                    for line in res.stdout.strip().splitlines()[1:]:
                        parts = line.strip().split(None, 2)
                        if len(parts) >= 2 and parts[0].isdigit():
                            pid = int(parts[0])
                            if pid in ignored_pids:
                                continue
                            comm = parts[1].lower()
                            args = parts[2] if len(parts) > 2 else ""
                            args_lower = args.lower()

                            if comm in ["actx", "actx-core", "anycontext", "ac"] or ("python" in comm and "any_context" in args_lower):
                                if any(t in args_lower for t in ["test_", "pytest", "run_all"]):
                                    continue
                                proc_type = "mcp" if "--mcp" in args_lower else ("server" if ("--serve" in args_lower or "serve" in args_lower) else "cli")
                                instances.append({
                                    "pid": pid,
                                    "name": parts[1],
                                    "title": args if args else parts[1],
                                    "type": proc_type
                                })
            except Exception:
                pass

        return instances

    @staticmethod
    def close_active_instances(instances: List[Dict[str, Any]]) -> int:
        """
        Gracefully closes or terminates running AnyContext instances.
        Strictly protects the current session hierarchy against self-termination.
        """
        closed_count = 0
        is_windows = sys.platform == "win32" or ("MINGW" in os.environ.get("MSYSTEM", ""))
        immune_pids = get_current_session_pids()

        for inst in instances:
            pid = inst.get("pid")
            if not pid or pid in immune_pids:
                continue
            try:
                if is_windows:
                    res = subprocess.run(
                        ["taskkill", "/PID", str(pid), "/F"],
                        capture_output=True,
                        text=True,
                        timeout=3
                    )
                    if res.returncode == 0:
                        closed_count += 1
                else:
                    import signal
                    os.kill(pid, signal.SIGTERM)
                    closed_count += 1
            except Exception:
                pass

        return closed_count

    def execute_binary_update(
        self,
        target_tag: Optional[str] = None,
        auto_close_instances: bool = False,
        force_background: bool = True,
        auto_restart: bool = False,
        is_tui: bool = False,
    ) -> Tuple[bool, str, Dict[str, Any]]:
        """
        Downloads the target/latest release asset and orchestrates atomic replacement.
        Returns: (success: bool, message: str, state_updates: Dict[str, Any])
        """
        clean_stale_update_files()

        # 1. Resolve Target Version Tag
        if not target_tag:
            latest = self.fetch_latest_release_tag()
            if not latest:
                return False, "❌ Could not fetch latest release from GitHub. Please check your internet connection.", {}
            target_tag = latest

        clean_tag = target_tag if target_tag.startswith("v") else f"v{target_tag}"

        # 2. Handle active instances
        active_instances = self.find_active_instances()
        closed_count = 0
        if active_instances and auto_close_instances:
            closed_count = self.close_active_instances(active_instances)

        # 3. Determine paths and assets
        is_windows = sys.platform == "win32" or ("MINGW" in os.environ.get("MSYSTEM", ""))
        target_asset = "actx-windows-x86_64.exe" if is_windows else "actx-linux-x86_64"

        if "ACTX_UPDATE_DIR" in os.environ and os.environ["ACTX_UPDATE_DIR"].strip():
            target_dir = os.path.abspath(os.environ["ACTX_UPDATE_DIR"].strip())
        elif getattr(sys, "frozen", False):
            target_exe_cur = os.path.abspath(sys.executable)
            target_dir = os.path.dirname(target_exe_cur)
        else:
            import shutil
            found_which = shutil.which("actx.exe" if is_windows else "actx")
            if found_which:
                target_dir = os.path.dirname(os.path.abspath(found_which))
            else:
                target_dir = os.path.expanduser("~/AppData/Local/actx/bin" if is_windows else "~/.local/bin")

        os.makedirs(target_dir, exist_ok=True)
        # Dual-binary architecture: actx-core.exe (heavy PyInstaller engine) vs actx.exe (native launcher shim)
        core_exe = os.path.join(target_dir, "actx-core.exe" if is_windows else "actx-core")
        shim_exe = os.path.join(target_dir, "actx.exe" if is_windows else "actx")
        if os.path.exists(core_exe):
            target_exe = core_exe
        else:
            target_exe = shim_exe

        temp_download = os.path.join(target_dir, "actx_new.exe" if is_windows else "actx_new")
        old_exe = os.path.join(target_dir, "actx_old.exe" if is_windows else "actx_old")

        if os.path.exists(temp_download):
            try:
                os.remove(temp_download)
            except Exception:
                pass

        # 4. Download asset from GitHub
        downloaded = False
        for repo in [self.primary_repo, self.fallback_repo]:
            try:
                url = f"https://github.com/{repo}/releases/download/{clean_tag}/{target_asset}"
                req = urllib.request.Request(url, headers={"User-Agent": "AnyContext-UpdateService"})
                with urllib.request.urlopen(req, timeout=120) as response:
                    if response.status == 200:
                        with open(temp_download, "wb") as f:
                            while True:
                                chunk = response.read(1024 * 512)
                                if not chunk:
                                    break
                                f.write(chunk)
                        downloaded = True
                        break
            except Exception:
                pass

        if not downloaded:
            for repo in [self.primary_repo, self.fallback_repo]:
                try:
                    res = subprocess.run(
                        ["gh", "release", "download", clean_tag, "--repo", repo, "--pattern", target_asset, "--dir", target_dir, "--clobber"],
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
            return False, f"❌ Failed to download update asset '{target_asset}' for release {clean_tag}.", {}

        if not is_windows:
            try:
                os.chmod(temp_download, 0o755)
            except Exception:
                pass

        # 5. Atomic swap and version registration
        version_file = os.path.join(target_dir, "version.txt")
        try:
            with open(version_file, "w", encoding="utf-8") as vf:
                vf.write(f"{clean_tag}\n")
        except Exception:
            pass

        if is_windows:
            # Self-healing: if actx.exe (shim) is missing or was accidentally overwritten with heavy binary (> 1MB), rebuild it
            if not os.path.exists(shim_exe) or os.path.getsize(shim_exe) > 1024 * 1024:
                try:
                    from launcher.build_shim import build_windows_shim
                    build_windows_shim(shim_exe)
                except Exception:
                    pass

            # Ensure Git Bash / MSYS2 wrapper 'actx' is deployed
            bash_shim = os.path.join(target_dir, "actx")
            bash_content = (
                "#!/usr/bin/env sh\n"
                "BIN_DIR=\"$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd)\"\n"
                "if [ \"$1\" = \"-v\" ] || [ \"$1\" = \"--version\" ]; then\n"
                "    if [ -f \"$BIN_DIR/version.txt\" ]; then\n"
                "        V=\"$(cat \"$BIN_DIR/version.txt\" | tr -d '\\r\\n')\"\n"
                "        case \"$V\" in\n"
                "            v*) echo \"$V\" ;;\n"
                "            *) echo \"v$V\" ;;\n"
                "        esac\n"
                "    else\n"
                f"        echo \"{clean_tag}\"\n"
                "    fi\n"
                "    exit 0\n"
                "fi\n"
                "\n"
                "if [ -f \"$BIN_DIR/actx-core.exe\" ]; then\n"
                "    exec \"$BIN_DIR/actx-core.exe\" \"$@\"\n"
                "elif [ -f \"$BIN_DIR/actx.exe\" ]; then\n"
                "    exec \"$BIN_DIR/actx.exe\" \"$@\"\n"
                "elif [ -f \"$BIN_DIR/actx-core\" ]; then\n"
                "    exec \"$BIN_DIR/actx-core\" \"$@\"\n"
                "fi\n"
            )
            try:
                with open(bash_shim, "w", encoding="utf-8", newline="\n") as bf:
                    bf.write(bash_content)
            except Exception:
                pass


            swap_script = (
                f"$retries = 0; "
                f"while ($retries -lt 40) {{ "
                f"  try {{ "
                f"    if (Test-Path -LiteralPath '{target_exe}') {{ "
                f"      Move-Item -LiteralPath '{target_exe}' -Destination '{old_exe}' -Force -ErrorAction SilentlyContinue "
                f"    }} "
                f"    if (Test-Path -LiteralPath '{temp_download}') {{ "
                f"      Move-Item -LiteralPath '{temp_download}' -Destination '{target_exe}' -Force -ErrorAction Stop "
                f"    }} "
                f"    Set-Content -LiteralPath '{version_file}' -Value '{clean_tag}' -Encoding UTF8 -Force -ErrorAction SilentlyContinue; "
                f"    if (Test-Path -LiteralPath '{old_exe}') {{ "
                f"      Remove-Item -LiteralPath '{old_exe}' -Force -ErrorAction SilentlyContinue "
                f"    }} "
                f"    break "
                f"  }} catch {{ "
                f"    Start-Sleep -Milliseconds 400; "
                f"    $retries++ "
                f"  }} "
                f"}}; "
            )

            try:
                subprocess.Popen(
                    ["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", swap_script],
                    creationflags=subprocess.CREATE_NO_WINDOW if hasattr(subprocess, "CREATE_NO_WINDOW") else 0
                )
            except Exception:
                pass

            msg = f"🎉 Successfully updated AnyContext to {clean_tag}!\n👉 The new version will take effect the next time you launch 'actx' or 'actx --tui'."
            return True, msg, {"action": "none", "version": clean_tag}

        else:
            # Unix / macOS
            try:
                os.replace(temp_download, target_exe)
            except Exception as e:
                return False, f"❌ Failed to replace binary: {e}", {}

            msg = f"🎉 Successfully updated AnyContext to {clean_tag}!\n👉 The new version will take effect the next time you launch 'actx' or 'actx --tui'."
            return True, msg, {"action": "none", "version": clean_tag}

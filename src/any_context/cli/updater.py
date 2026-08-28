import os
import sys
import time
import json
import subprocess
import urllib.request
from typing import Optional, Tuple, List, Dict, Any
from any_context import __version__ as CURRENT_VERSION
from any_context.core.services.update_service import (
    UpdateService,
    PRIMARY_REPO,
    FALLBACK_REPO,
    clean_stale_update_files,
    parse_version_tuple,
    normalize_version_tag,
)


def safe_print(msg: str):
    try:
        print(msg)
    except UnicodeEncodeError:
        print(msg.encode("ascii", errors="ignore").decode("ascii"))


def normalize_version_tag(version_str: str) -> str:
    """
    Normalizes version strings like '@0.15.2', '0.15.2', 'v0.15.2', '@latest' -> 'v0.15.2' or 'latest'.
    """
    cleaned = version_str.strip()
    if cleaned.startswith("@"):
        cleaned = cleaned[1:].strip()
    if cleaned.lower() in ["latest", "current", "head"]:
        return "latest"
    if not cleaned.startswith("v"):
        cleaned = f"v{cleaned}"
    return cleaned


def fetch_available_releases(limit: int = 15) -> List[Dict[str, Any]]:
    """
    Fetches the list of recent available release tags and metadata from GitHub.
    """
    import time
    releases = []
    for repo in [PRIMARY_REPO, FALLBACK_REPO]:
        try:
            url = f"https://api.github.com/repos/{repo}/releases?per_page={limit}&_t={int(time.time())}"
            headers = {
                "User-Agent": "AnyContext-CLI",
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
                                    "published_at": item.get("published_at", "")[:10],
                                    "prerelease": item.get("prerelease", False),
                                    "body": (item.get("body") or "").strip()
                                })
                        if releases:
                            return releases
        except Exception:
            pass

        # Fallback to gh CLI
        try:
            res = subprocess.run(
                ["gh", "release", "list", "--repo", repo, "--limit", str(limit), "--json", "tagName,name,createdAt,isPrerelease"],
                capture_output=True,
                text=True,
                timeout=6
            )
            if res.returncode == 0 and res.stdout.strip():
                data = json.loads(res.stdout)
                if isinstance(data, list) and data:
                    for item in data:
                        tag = item.get("tagName", "")
                        if tag:
                            releases.append({
                                "tag": tag if tag.startswith("v") else f"v{tag}",
                                "name": item.get("name") or tag,
                                "published_at": item.get("createdAt", "")[:10],
                                "prerelease": item.get("isPrerelease", False),
                                "body": ""
                            })
                    if releases:
                        return releases
        except Exception:
            pass

    return releases


def display_available_releases(interactive_select: bool = False) -> Optional[str]:
    """
    Displays a formatted list of available versions from GitHub and optionally allows selecting one.
    """
    safe_print("\n🔍 Fetching available AnyContext releases from GitHub...")
    releases = fetch_available_releases(limit=12)
    if not releases:
        safe_print("⚠️ Could not retrieve release list from GitHub (offline or rate limited).\n")
        return None

    safe_print("\n=======================================================")
    safe_print("📦 Available AnyContext Releases")
    safe_print("=======================================================")
    safe_print(f"Current Installed Version: \033[1m\033[92mv{CURRENT_VERSION}\033[0m\n")

    choices = []
    for r in releases:
        tag = r["tag"]
        is_cur = tag.lstrip("v") == CURRENT_VERSION.lstrip("v")
        tag_disp = f"\033[92m{tag} [Active / Current]\033[0m" if is_cur else f"\033[93m{tag}\033[0m"
        date_str = f"({r['published_at']})" if r.get("published_at") else ""
        title_str = f"- {r['name']}" if r.get("name") and r['name'] != tag else ""
        safe_print(f"  • {tag_disp} {date_str} {title_str}")
        
        status_label = " [Installed]" if is_cur else ""
        choices.append(f"{tag} {date_str} {title_str}{status_label}")

    safe_print("=======================================================\n")
    safe_print("💡 Usage:")
    safe_print("  • Update to latest   : actx --update          OR /update")
    safe_print("  • Update to specific : actx --update@0.15.2    OR /update@0.15.2")
    safe_print("  • Interactive update : actx --update --list   OR /update --list\n")

    if interactive_select:
        try:
            import questionary
            choices.append("🔙 Cancel")
            pick = questionary.select(
                "Select a version to install / rollback to:",
                choices=choices
            ).ask()
            if not pick or pick.startswith("🔙"):
                return None
            picked_tag = pick.split()[0].strip()
            return picked_tag
        except Exception:
            return None

    return None


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


def find_active_instances() -> List[Dict[str, Any]]:
    """
    Scans for other running AnyContext processes (CLI sessions, MCP servers, REST servers)
    excluding the current process PID and parent PID.
    Returns a list of dictionaries: [{'pid': int, 'name': str, 'title': str, 'type': str}]
    """
    instances = []
    current_pid = os.getpid()
    ignored_pids = {current_pid}
    if hasattr(os, "getppid"):
        try:
            ignored_pids.add(os.getppid())
        except Exception:
            pass

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
                            if img_lower in ["actx.exe", "anycontext.exe", "any-context.exe", "ac.exe"]:
                                instances.append({
                                    "pid": pid,
                                    "name": img_name,
                                    "title": f"AnyContext Session (PID: {pid})",
                                    "type": "cli"
                                })
        except Exception:
            pass
    else:
        # Unix (Linux / macOS)
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

                        if comm in ["actx", "anycontext", "ac"] or ("python" in comm and "any_context" in args_lower):
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


def close_active_instances(instances: List[Dict[str, Any]]) -> int:
    """
    Gracefully closes or terminates the specified running AnyContext instances.
    Returns number of successfully terminated instances.
    """
    closed_count = 0
    is_windows = sys.platform == "win32" or ("MINGW" in os.environ.get("MSYSTEM", ""))

    for inst in instances:
        pid = inst.get("pid")
        if not pid:
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


def prompt_multi_instance_decision(active_instances: List[Dict[str, Any]]) -> str:
    """
    Prompts user for how to handle active instances during update.
    Returns: 'background' | 'close' | 'cancel'
    """
    try:
        import questionary
        choices = [
            questionary.Choice(
                title="⚡ Update in background (Recommended - Active sessions will continue working undisturbed)",
                value="background"
            ),
            questionary.Choice(
                title="⏹️ Close other instances and update now (Terminates running background processes)",
                value="close"
            ),
            questionary.Choice(
                title="🔙 Cancel update",
                value="cancel"
            ),
        ]
        choice = questionary.select(
            "How would you like to handle active AnyContext sessions?",
            choices=choices,
            default=choices[0]
        ).ask()
        if not choice:
            return "cancel"
        return choice
    except Exception:
        try:
            sys.stdout.write("Choose [1] Update in background (Default), [2] Close instances, [3] Cancel: ")
            sys.stdout.flush()
            ans = input().strip()
            if ans == "2":
                return "close"
            elif ans == "3":
                return "cancel"
            return "background"
        except Exception:
            return "background"


def run_self_update(
    target_version: Optional[str] = None,
    auto_close_instances: bool = False,
    force_background: bool = False,
    force: bool = False
):
    """
    Executes automatic binary update or rollback by downloading the target/latest release asset
    and performing safe, atomic replacement (supporting locked executables on Windows
    and graceful handling of multiple active instances).
    """
    clean_stale_update_files()
    
    if target_version:
        norm = normalize_version_tag(target_version)
        if norm.lower() in ["latest", "current", "head"]:
            safe_print(f"\n🔍 Checking for latest AnyContext release...")
            latest_tag = fetch_latest_release_tag()
            if not latest_tag:
                safe_print("\n⚠️ Could not fetch latest release from GitHub.\n")
                return
            target_tag = latest_tag if latest_tag.startswith("v") else f"v{latest_tag}"
        else:
            target_tag = norm
    else:
        safe_print(f"\n🔍 Checking for AnyContext updates...")
        has_update, latest_tag = check_for_updates(quiet_if_latest=False)
        if not latest_tag:
            return
        if not has_update and not force:
            return
        target_tag = latest_tag if latest_tag.startswith("v") else f"v{latest_tag}"

    clean_tag = target_tag if target_tag.startswith("v") else f"v{target_tag}"
    cur_tuple = parse_version_tuple(CURRENT_VERSION)
    target_tuple = parse_version_tuple(clean_tag)

    if cur_tuple == target_tuple and not force:
        safe_print(f"\nℹ️ You are already running AnyContext \033[1m\033[92mv{CURRENT_VERSION}\033[0m.")
        try:
            import questionary
            reinstall = questionary.confirm("Do you want to force reinstall / repair this version?").ask()
            if not reinstall:
                return
        except Exception:
            return

    if target_tuple < cur_tuple:
        safe_print(f"\n🔄 Rolling back / Downgrading AnyContext from \033[93mv{CURRENT_VERSION}\033[0m to \033[1m\033[96m{clean_tag}\033[0m...")
    elif target_tuple > cur_tuple:
        safe_print(f"\n🚀 Updating AnyContext from \033[93mv{CURRENT_VERSION}\033[0m to \033[1m\033[92m{clean_tag}\033[0m...")
    else:
        safe_print(f"\n⚡ Reinstalling AnyContext \033[1m\033[92m{clean_tag}\033[0m...")

    # Multi-instance detection and graceful handling
    active_instances = find_active_instances()
    decision = "background"

    if active_instances:
        yellow = "\033[93m"
        bold = "\033[1m"
        reset = "\033[0m"
        safe_print(f"\n{yellow}ℹ️ Detected {len(active_instances)} other active AnyContext session(s):{reset}")
        for inst in active_instances:
            type_label = inst.get("type", "cli").upper()
            safe_print(f"  • PID {bold}{inst['pid']}{reset}: {inst.get('name', 'actx')} \033[90m[{type_label}]\033[0m \033[90m({inst.get('title', '')})\033[0m")
        print()

        if auto_close_instances:
            decision = "close"
        elif force_background:
            decision = "background"
        else:
            decision = prompt_multi_instance_decision(active_instances)

        if decision == "cancel":
            safe_print("\n⚠️ Update cancelled by user.\n")
            return
        elif decision == "close":
            safe_print("⏹️ Closing active AnyContext sessions...")
            closed_cnt = close_active_instances(active_instances)
            safe_print(f"✅ Closed {closed_cnt} active session(s).\n")
        else:
            safe_print("⚡ Proceeding with background update (active sessions will remain undisturbed)...\n")

    is_windows = sys.platform == "win32" or ("MINGW" in os.environ.get("MSYSTEM", ""))
    target_asset = "actx-windows-x86_64.exe" if is_windows else "actx-linux-x86_64"

    # Determine target executable location
    if "ACTX_UPDATE_DIR" in os.environ and os.environ["ACTX_UPDATE_DIR"].strip():
        target_dir = os.path.abspath(os.environ["ACTX_UPDATE_DIR"].strip())
        target_exe = os.path.join(target_dir, "actx.exe" if is_windows else "actx")
    elif getattr(sys, "frozen", False):
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
            url = f"https://github.com/{repo}/releases/download/{clean_tag}/{target_asset}"
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
    import shutil
    replaced = False

    if is_windows:
        # Step A: Perform immediate atomic rename of running target_exe -> old_exe and move temp_download -> target_exe
        try:
            if os.path.exists(old_exe):
                try:
                    os.remove(old_exe)
                except Exception:
                    pass
            if os.path.exists(target_exe):
                os.rename(target_exe, old_exe)
            shutil.move(temp_download, target_exe)
            replaced = True

            # Also sync Python313/Scripts/actx.exe if present on this system
            alt_script_exe = os.path.join(
                os.environ.get("LOCALAPPDATA", ""),
                "Programs",
                "Python",
                "Python313",
                "Scripts",
                "actx.exe"
            )
            if os.path.exists(alt_script_exe) and os.path.abspath(alt_script_exe) != os.path.abspath(target_exe):
                try:
                    shutil.copy2(target_exe, alt_script_exe)
                except Exception:
                    pass
        except Exception:
            pass

        if not replaced:
            # Step B: Fallback to background PowerShell loop if file lock prevented immediate rename
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
                f"    Start-Sleep -Milliseconds 400; "
                f"    $retries++ "
                f"  }} "
                f"}};"
            )
            try:
                subprocess.Popen(
                    ["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", swap_script],
                    creationflags=subprocess.CREATE_NO_WINDOW if hasattr(subprocess, "CREATE_NO_WINDOW") else 0
                )
            except Exception:
                pass

        safe_print(f"\n🎉 AnyContext successfully updated to {clean_tag}!")
        safe_print("🚀 Restarting AnyContext automatically in the current terminal...\n")
        time.sleep(1)

        # In-place restart preserving CLI arguments and active terminal session
        restart_args = [target_exe]
        for arg in sys.argv[1:]:
            if not arg.startswith("--update") and not arg.startswith("-u") and not arg.startswith("/update"):
                restart_args.append(arg)

        try:
            os.execv(target_exe, restart_args)
        except Exception:
            try:
                subprocess.call(restart_args)
                sys.exit(0)
            except Exception:
                sys.exit(0)
    else:
        try:
            os.replace(temp_download, target_exe)
            safe_print(f"\n🎉 AnyContext successfully updated to {clean_tag}!")
            safe_print("🚀 Restarting AnyContext automatically in the current terminal...\n")
            time.sleep(1)

            restart_args = [target_exe]
            for arg in sys.argv[1:]:
                if not arg.startswith("--update") and not arg.startswith("-u") and not arg.startswith("/update"):
                    restart_args.append(arg)

            try:
                os.execv(target_exe, restart_args)
            except Exception:
                try:
                    subprocess.call(restart_args)
                    sys.exit(0)
                except Exception:
                    sys.exit(0)
        except Exception as e:
            safe_print(f"⚠️ Saved new binary to: {temp_download}. Please move it to {target_exe} with sudo/chmod.")

import os
import sys
import shutil
import subprocess
from any_context.observability import obs


def launch_opentui(workspace: str = "Default") -> bool:
    """Attempts to launch the OpenTUI frontend using bun if installed."""
    obs.info("TUI:LAUNCH", "Initiating OpenTUI launch", {"workspace": workspace})

    is_windows = sys.platform.startswith("win")

    bun_bin = None
    if is_windows:
        candidates = [
            shutil.which("bun"),
            shutil.which("bun.exe"),
            os.path.expanduser("~/.bun/bin/bun.exe"),
            os.path.join(os.environ.get("USERPROFILE", ""), ".bun", "bin", "bun.exe"),
            os.path.join(os.environ.get("LOCALAPPDATA", ""), "actx", "bin", "bun.exe"),
        ]
    else:
        # On Linux/WSL, prioritize native Linux Bun and strictly exclude Windows .exe binaries
        candidates = [
            os.path.expanduser("~/.bun/bin/bun"),
            "/usr/local/bin/bun",
            "/usr/bin/bun",
            os.path.expanduser("~/.local/bin/bun"),
        ]
        which_bun = shutil.which("bun")
        if which_bun and not which_bun.lower().endswith(".exe") and not which_bun.startswith("/mnt/c"):
            candidates.append(which_bun)

    for c in candidates:
        if c and os.path.exists(c) and (is_windows or os.access(c, os.X_OK)):
            bun_bin = c
            break

    if not bun_bin:
        obs.warn("TUI:BUN_MISSING", "Bun runtime not detected in candidates", {"candidates": [c for c in candidates if c]})
        print("\n❌ OpenTUI Error: Bun runtime was not found.")
        print("💡 OpenTUI requires Bun (fast JavaScript runtime). Install it via:")
        print('   • Windows: powershell -c "irm bun.sh/install.ps1 | iex"')
        print('   • Linux / macOS: curl -fsSL https://bun.sh/install | bash\n')
        return False

    tui_index = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "tui", "index.tsx"))
    if not os.path.exists(tui_index):
        obs.error("TUI:ENTRYPOINT_MISSING", f"Frontend entrypoint not found at: {tui_index}")
        print(f"\n❌ OpenTUI Error: Frontend entrypoint not found at: {tui_index}\n")
        return False

    try:
        # Sanitize ALL PyInstaller bootloader internal variables to prevent child process security validation failure
        def _is_pyi_var(k: str) -> bool:
            lk = k.lower()
            return (
                lk.startswith("_mei")
                or lk.startswith("_pyi")
                or lk.startswith("pyi")
                or "meipass" in lk
                or "pyinstaller" in lk
            )

        env = {k: v for k, v in os.environ.items() if not _is_pyi_var(k)}

        # Prepend Linux/Windows bun directory to PATH in child process
        bun_dir = os.path.dirname(bun_bin)
        if "PATH" in env:
            paths = env["PATH"].split(os.pathsep)
            clean_paths = [p for p in paths if not _is_pyi_var(p)]
            if bun_dir not in clean_paths:
                clean_paths.insert(0, bun_dir)
            env["PATH"] = os.pathsep.join(clean_paths)
        else:
            env["PATH"] = bun_dir

        from any_context.config.db_store import ConfigDBStore
        env["ACTX_SETTINGS_DB"] = ConfigDBStore().db_path
        env["ACTX_CALLER_CWD"] = os.getcwd()
        env["ACTX_EXECUTABLE"] = sys.executable or "actx"
        env["ACTX_PYTHON_PATH"] = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
        env["ACTX_FRONTEND"] = "tui"
        env["ACTX_ROOT_PID"] = os.environ.get("ACTX_ROOT_PID", str(os.getpid()))
        env["ACTX_LAUNCHER_PID"] = os.environ.get("ACTX_LAUNCHER_PID", str(os.getpid()))

        obs.info("TUI:EXEC", "Spawning OpenTUI process", {
            "bun_bin": bun_bin,
            "tui_index": tui_index,
            "executable": env["ACTX_EXECUTABLE"],
            "cwd": os.path.dirname(tui_index),
        })

        res = subprocess.run([bun_bin, "run", tui_index, workspace], cwd=os.path.dirname(tui_index), env=env)
        obs.info("TUI:EXIT", f"OpenTUI process exited with code {res.returncode}", {"returncode": res.returncode})

        # Check if an update exit was completed cleanly
        import tempfile
        root_pid = os.environ.get("ACTX_ROOT_PID", str(os.getpid()))
        notice_file = os.path.join(tempfile.gettempdir(), f"actx_update_notice_{root_pid}.txt")
        if os.path.exists(notice_file):
            try:
                with open(notice_file, "r", encoding="utf-8") as nf:
                    updated_ver = nf.read().strip()
                os.remove(notice_file)
                print(f"\n🎉 AnyContext foi atualizado com sucesso para {updated_ver}!")
                print("👉 Execute 'actx' para iniciar a nova versão.\n")
            except Exception:
                pass

        return res.returncode == 0
    except Exception as e:
        obs.error("TUI:CRASH", f"Exception running OpenTUI: {e}", exc=e)
        print(f"\n❌ OpenTUI Error: {e}\n")
        return False

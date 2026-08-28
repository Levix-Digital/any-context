"""
Diagnostics & Health Inspector for AnyContext.
Gathers runtime environment facts, file integrity, Bun runtime presence,
database metrics, and recent error traces for 'actx --diagnostics' and 'actx --logs'.
"""

import os
import sys
import shutil
import platform
import subprocess
from typing import List, Dict, Any, Optional
from any_context import __version__
from any_context.config.paths import get_app_data_root
from any_context.config.db_store import ConfigDBStore
from any_context.observability.schemas import DiagnosticReport, LogEvent, LogLevel
from any_context.observability.storage import ObservabilityStorage


def collect_diagnostic_report() -> DiagnosticReport:
    """Collects complete health and runtime metrics of the local AnyContext installation."""
    db_store = ConfigDBStore()
    db_path = db_store.db_path
    db_exists = os.path.exists(db_path)
    db_size = os.path.getsize(db_path) if db_exists else 0

    onboarding_done = False
    active_model = "unknown"
    active_provider = "unknown"
    try:
        onboarding_done = bool(db_store.get_onboarding_completed())
        settings = db_store.get_app_settings()
        if settings and settings.models:
            active_model = settings.models.inference_model or "gpt-4o-mini"
            active_provider = settings.models.model_provider or "openai"
    except Exception:
        pass

    # Inspect Bun
    bun_bin = None
    bun_ver = None
    is_windows = sys.platform.startswith("win")
    if is_windows:
        candidates = [
            shutil.which("bun"),
            shutil.which("bun.exe"),
            os.path.expanduser("~/.bun/bin/bun.exe"),
            os.path.join(os.environ.get("USERPROFILE", ""), ".bun", "bin", "bun.exe"),
            os.path.join(os.environ.get("LOCALAPPDATA", ""), "actx", "bin", "bun.exe"),
        ]
    else:
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

    if bun_bin:
        try:
            res = subprocess.run([bun_bin, "--version"], capture_output=True, text=True, timeout=3.0)
            if res.returncode == 0:
                bun_ver = res.stdout.strip()
        except Exception:
            pass

    storage = ObservabilityStorage(db_path=db_path)
    recent_errors = storage.get_recent_logs(limit=10, level="ERROR")

    return DiagnosticReport(
        os_name=platform.system(),
        os_platform=platform.platform(),
        python_version=platform.python_version(),
        actx_version=__version__,
        executable_path=sys.executable or "actx",
        is_frozen=getattr(sys, "frozen", False),
        bun_installed=bool(bun_bin),
        bun_version=bun_ver,
        bun_path=bun_bin,
        database_path=db_path,
        database_exists=db_exists,
        database_size_bytes=db_size,
        onboarding_completed=onboarding_done,
        active_model=active_model,
        active_provider=active_provider,
        recent_errors=recent_errors,
    )


def format_diagnostic_report(report: DiagnosticReport) -> str:
    """Formats a diagnostic report into a clean, human-readable terminal display."""
    lines = [
        "\n\033[36m=======================================================\033[0m",
        "\033[1m📊 AnyContext Diagnostics & Health Checkup\033[0m",
        "\033[36m=======================================================\033[0m",
        f"  • \033[1mVersion:\033[0m actx v{report.actx_version} (levix.digital)",
        f"  • \033[1mOperating System:\033[0m {report.os_name} ({report.os_platform})",
        f"  • \033[1mPython Runtime:\033[0m {report.python_version} (Frozen/Standalone: {report.is_frozen})",
        f"  • \033[1mExecutable Binary:\033[0m {report.executable_path}",
        "",
        "  \033[1m⚡ OpenTUI Desktop Runtime (Bun):\033[0m",
        f"    - Installed: {'\033[32m✅ Yes\033[0m' if report.bun_installed else '\033[31m❌ No (required for actx --tui)\033[0m'}",
        f"    - Version: {report.bun_version or 'N/A'}",
        f"    - Binary Path: {report.bun_path or 'Not detected'}",
        "",
        "  \033[1m💾 SQLite Configuration & Settings DB:\033[0m",
        f"    - Database Path: {report.database_path}",
        f"    - Exists on Disk: {'\033[32m✅ Yes\033[0m' if report.database_exists else '\033[31m❌ No\033[0m'}",
        f"    - Size: {report.database_size_bytes:,} bytes",
        f"    - Onboarding Completed: {'\033[32m✅ Yes\033[0m' if report.onboarding_completed else '\033[93m⏳ Pending (first-time setup)\033[0m'}",
        f"    - Active Provider / Model: {report.active_provider.upper()} / {report.active_model}",
        "\033[36m=======================================================\033[0m\n",
    ]

    if report.recent_errors:
        lines.append("\033[91m⚠️ Recent Error Logs:\033[0m")
        for err in report.recent_errors:
            lines.append(f"  [{err.timestamp}] [{err.component}] {err.message}")
            if err.traceback:
                lines.append(f"    \033[90m{err.traceback.strip().splitlines()[-1]}\033[0m")
        lines.append("")

    return "\n".join(lines)


def format_recent_logs(logs: List[LogEvent], limit: int = 50) -> str:
    """Formats recent system logs with ANSI coloring."""
    if not logs:
        return "\n\033[90mℹ️ No system logs recorded yet.\033[0m\n"

    lines = [
        "\n\033[36m=======================================================\033[0m",
        f"\033[1m📜 AnyContext System Logs (Showing last {len(logs)} entries)\033[0m",
        "\033[36m=======================================================\033[0m",
    ]

    for log in logs:
        lvl_color = "\033[32m"
        if log.level == LogLevel.WARN:
            lvl_color = "\033[93m"
        elif log.level in [LogLevel.ERROR, LogLevel.CRITICAL]:
            lvl_color = "\033[91m"
        elif log.level == LogLevel.DEBUG:
            lvl_color = "\033[90m"

        meta_str = f" {log.metadata}" if log.metadata else ""
        lines.append(
            f"  \033[90m{log.timestamp}\033[0m [{lvl_color}{log.level.value:<5}\033[0m] "
            f"\033[36m[{log.component}]\033[0m {log.message}{meta_str}"
        )
        if log.traceback:
            lines.append(f"    \033[91m{log.traceback.strip()}\033[0m")

    lines.append("\033[36m=======================================================\033[0m\n")
    return "\n".join(lines)

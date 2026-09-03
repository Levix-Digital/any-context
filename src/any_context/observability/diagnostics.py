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
    latency_summary = storage.get_latency_summary(limit=200)

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
        latency_summary=latency_summary,
        recent_errors=recent_errors,
    )


def format_diagnostic_report(report: DiagnosticReport) -> str:
    """Formats a diagnostic report into clean, structured GitHub Flavored Markdown."""
    bun_status = "✅ Yes" if report.bun_installed else "❌ No (required for actx --tui)"
    db_status = "✅ Yes" if report.database_exists else "❌ No"
    onboarding_status = "✅ Yes" if report.onboarding_completed else "⏳ Pending (first-time setup)"
    provider_str = report.active_provider.upper()

    lines = [
        "### 📊 AnyContext Diagnostics & Health Checkup\n",
        f"• **Version:** actx v{report.actx_version} (levix.digital)",
        f"• **Operating System:** {report.os_name} ({report.os_platform})",
        f"• **Python Runtime:** {report.python_version} (Frozen/Standalone: {report.is_frozen})",
        f"• **Executable Binary:** `{report.executable_path}`",
        "",
        "#### ⚡ OpenTUI Desktop Runtime (Bun)",
        f"- **Installed:** {bun_status}",
        f"- **Version:** {report.bun_version or 'N/A'}",
        f"- **Binary Path:** `{report.bun_path or 'Not detected'}`",
        "",
        "#### 💾 SQLite Configuration & Settings DB",
        f"- **Database Path:** `{report.database_path}`",
        f"- **Exists on Disk:** {db_status}",
        f"- **Size:** {report.database_size_bytes:,} bytes",
        f"- **Onboarding Completed:** {onboarding_status}",
        f"- **Active Provider / Model:** {provider_str} / `{report.active_model}`",
    ]

    if report.latency_summary:
        lines.append("")
        lines.append("#### ⏱️ Performance & Latency Metrics (Recent Spans)")
        for item in report.latency_summary:
            avg_ms = item["avg_ms"]
            lines.append(
                f"- `{item['name']}` (x{item['count']}): **avg {avg_ms:.1f}ms** "
                f"[min: {item['min_ms']:.1f}ms, max: {item['max_ms']:.1f}ms]"
            )

    if report.recent_errors:
        lines.append("")
        lines.append("#### ⚠️ Recent Error Logs")
        for err in report.recent_errors:
            lines.append(f"- `[{err.timestamp}]` `[{err.component}]` **{err.message}**")
            if err.traceback:
                tb_last = err.traceback.strip().splitlines()[-1]
                lines.append(f"  > *Traceback:* `{tb_last}`")

    return "\n".join(lines)


def format_recent_logs(logs: List[LogEvent], limit: int = 50) -> str:
    """Formats recent system logs into clean, structured Markdown with a fenced code block."""
    if not logs:
        return "ℹ️ *No system logs recorded yet.*"

    count = len(logs)
    lines = [
        f"### 📜 AnyContext System Logs (Showing last {count} entries)\n",
        "```text",
    ]

    for log in logs:
        ts = str(log.timestamp).replace("T", " ").split(".")[0]
        lvl = f"[{log.level.value:<5}]"
        comp = f"[{log.component}]"
        meta_str = f" {log.metadata}" if log.metadata else ""
        lines.append(f"{ts} {lvl} {comp} {log.message}{meta_str}")
        if log.traceback:
            for tb_line in log.traceback.strip().splitlines():
                lines.append(f"    {tb_line}")

    lines.append("```")
    return "\n".join(lines)


def format_recent_spans(spans: List[Any], limit: int = 50) -> str:
    """Formats recent execution trace spans with latency metrics into clean Markdown."""
    if not spans:
        return "ℹ️ *No latency spans recorded yet.*"

    count = len(spans)
    lines = [
        f"### ⏱️ AnyContext Latency & Performance Spans (Last {count})\n",
        "```text",
    ]

    for s in spans:
        dur = s.duration_ms or 0.0
        status_tag = "✔" if getattr(s, "status", "ok") == "ok" else "✖"
        st = str(getattr(s, "start_time", "")).replace("T", " ").split(".")[0]
        name = getattr(s, "name", "unknown")
        meta = f" [{s.metadata}]" if getattr(s, "metadata", None) else ""
        lines.append(f"{status_tag} {st:<19}  {name:<32} {dur:>8.2f} ms{meta}")

    lines.append("```")
    return "\n".join(lines)


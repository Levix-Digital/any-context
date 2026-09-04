"""
AnyContext OS-Native Directory & Storage Path Provider.
Guarantees standard application data isolation across Windows (%LOCALAPPDATA%),
macOS (~/Library/Application Support), and Linux (~/.local/share).
"""
import os
import sys
from typing import Optional


def get_app_data_root() -> str:
    r"""
    Returns the root directory for AnyContext application data and databases.
    - Windows: %LOCALAPPDATA%\AnyContext (or %APPDATA%\AnyContext)
    - macOS: ~/Library/Application Support/AnyContext
    - Linux / Other: ~/.local/share/any-context (or ~/.any-context)
    """
    env_override = os.getenv("ACTX_DATA_DIR")
    if env_override and env_override.strip():
        target = os.path.abspath(env_override.strip())
        os.makedirs(target, exist_ok=True)
        return target

    if sys.platform == "win32":
        base = os.environ.get("LOCALAPPDATA") or os.environ.get("APPDATA") or os.path.expanduser("~")
        target = os.path.join(base, "AnyContext")
    elif sys.platform == "darwin":
        target = os.path.expanduser("~/Library/Application Support/AnyContext")
    else:
        xdg_data = os.environ.get("XDG_DATA_HOME") or os.path.expanduser("~/.local/share")
        target = os.path.join(xdg_data, "any-context")

    os.makedirs(target, exist_ok=True)
    return os.path.abspath(target)


def get_default_config_db_path() -> str:
    """Returns the canonical path for settings.db in the OS config directory."""
    env_db = os.getenv("ACTX_SETTINGS_DB")
    if env_db and env_db.strip():
        p = os.path.abspath(env_db.strip())
        os.makedirs(os.path.dirname(p), exist_ok=True)
        return p

    config_dir = os.path.join(get_app_data_root(), "config")
    os.makedirs(config_dir, exist_ok=True)
    return os.path.abspath(os.path.join(config_dir, "settings.db"))


def get_default_vector_db_path() -> str:
    """Returns the canonical path for context vector storage (LanceDB)."""
    env_ctx = os.getenv("ACTX_CONTEXT_DB")
    if env_ctx and env_ctx.strip():
        p = os.path.abspath(env_ctx.strip())
        os.makedirs(p, exist_ok=True)
        return p

    data_dir = os.path.join(get_app_data_root(), "data")
    target = os.path.join(data_dir, "context_db")
    os.makedirs(target, exist_ok=True)
    return os.path.abspath(target)


def get_default_session_db_path() -> str:
    """Returns the canonical path for session memory storage (LanceDB)."""
    env_mem = os.getenv("ACTX_MEMORY_DB")
    if env_mem and env_mem.strip():
        p = os.path.abspath(env_mem.strip())
        os.makedirs(p, exist_ok=True)
        return p

    data_dir = os.path.join(get_app_data_root(), "data")
    target = os.path.join(data_dir, "memory")
    os.makedirs(target, exist_ok=True)
    return os.path.abspath(target)


def get_logs_dir() -> str:
    """Returns the canonical directory for application, update, install, and migration logs."""
    env_logs = os.getenv("ACTX_LOGS_DIR")
    if env_logs and env_logs.strip():
        p = os.path.abspath(env_logs.strip())
        os.makedirs(p, exist_ok=True)
        return p

    logs_dir = os.path.join(get_app_data_root(), "logs")
    os.makedirs(logs_dir, exist_ok=True)
    return os.path.abspath(logs_dir)


def get_install_log_path() -> str:
    """Returns the canonical path for installer logs (install.log)."""
    return os.path.join(get_logs_dir(), "install.log")


def get_update_log_path() -> str:
    """Returns the canonical path for update logs (update.log)."""
    return os.path.join(get_logs_dir(), "update.log")


def get_migration_log_path() -> str:
    """Returns the canonical path for database migration logs (migration.log)."""
    return os.path.join(get_logs_dir(), "migration.log")


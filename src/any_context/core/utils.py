import os
import sys
import dotenv
from any_context.config.app_settings import AppSettings

def load_env():
    candidates = [
        os.path.join(os.getcwd(), ".env"),
        os.path.expanduser(os.path.join("~", ".config", "any-context", ".env")),
    ]
    if sys.platform == "win32" and "APPDATA" in os.environ:
        candidates.append(os.path.join(os.environ["APPDATA"], "any-context", ".env"))

    for c in candidates:
        if os.path.exists(c):
            dotenv.load_dotenv(c)
            return
    dotenv.load_dotenv()

def get_api_key(provider: str = "openai") -> str:
    """
    Resolves API Key in order:
    1. Environment variables (OPENAI_API_KEY / LOCAL_API_KEY)
    2. SQLite ConfigDBStore table (api_keys)
    3. Fallback dummy 'lm-studio' (for local offline models)
    """
    load_env()
    key = os.getenv("OPENAI_API_KEY") or os.getenv("LOCAL_API_KEY")
    if key and key.strip():
        return key.strip()

    try:
        from any_context.config.db_store import ConfigDBStore
        store_key = ConfigDBStore().get_api_key(provider)
        if store_key and store_key.strip():
            return store_key.strip()
    except Exception:
        pass

    return "lm-studio"

def find_agent_prompt_file(filename: str = "AGENT.md") -> str:
    candidates = [
        os.path.join(os.getcwd(), "config", filename),
        os.path.join(os.getcwd(), filename),
        os.path.expanduser(os.path.join("~", ".config", "any-context", filename)),
    ]

    if sys.platform == "win32" and "APPDATA" in os.environ:
        candidates.append(os.path.join(os.environ["APPDATA"], "any-context", filename))

    if hasattr(sys, "_MEIPASS"):
        candidates.append(os.path.join(sys._MEIPASS, "config", filename))
        candidates.append(os.path.join(sys._MEIPASS, filename))

    package_dir = os.path.dirname(os.path.abspath(__file__))
    candidates.append(os.path.join(package_dir, "..", "config", filename))
    candidates.append(os.path.join(package_dir, "..", "..", "..", "config", filename))

    for candidate in candidates:
        if candidate and os.path.exists(candidate):
            return os.path.abspath(candidate)
    return None

def get_system_prompt(path: str = None, active_workspace: str = None):
    target_path = path if (path and os.path.exists(path)) else find_agent_prompt_file("AGENT.md")
    prompt = ""
    if target_path and os.path.exists(target_path):
        try:
            with open(target_path, "r", encoding="utf-8") as f:
                prompt = f.read()
        except Exception as e:
            print(f"⚠️ Warning: Could not read AGENT.md from {target_path}: {e}")

    if not prompt:
        prompt = "You are AnyContext, an AI assistant with access to user workspace documents."

    try:
        settings = AppSettings.load()
        if settings and settings.workspaces:
            workspaces_str = ", ".join([f"'{ws.name}'" for ws in settings.workspaces])
            prompt += f"\n\n### 6. Workspaces\n- **Available Workspaces:** {workspaces_str}\n"

        if active_workspace:
            prompt += f"\n\n### 🎯 ACTIVE WORKSPACE & TOOL CALLING CONTEXT\n"
            prompt += f"- You are currently chatting inside active workspace: **'{active_workspace}'**.\n"
            prompt += f"- **WORKSPACE FILTER RULE:** When calling `search_db` to search documents, you MUST pass `workspace='{active_workspace}'` unless the user explicitly requests searching globally.\n"
            prompt += f"- **SINGLE SEARCH EXECUTION:** Call `search_db` AT MOST ONCE per question. Do NOT repeat or loop calls to `search_db`. Analyze the retrieved document snippets immediately and write a complete, beautifully structured answer.\n"
        else:
            prompt += "- When searching the knowledge base (search_session_memory=False), specify the `workspace` argument in `search_db` if a specific workspace topic is mentioned.\n"
    except Exception as e:
        print(f"⚠️ Warning: Could not load workspaces into system prompt: {e}")

    return prompt


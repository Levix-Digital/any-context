import os
import sys
from any_context.config.app_settings import AppSettings

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

def get_system_prompt(path: str = None):
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
            prompt += "- When searching the knowledge base (search_session_memory=False), you should specify the `workspace` argument in the `search_db` tool if the user indicates a specific topic or workspace.\n"
            prompt += "- If the user's request is ambiguous, you may ask them which workspace they want to search, or leave the `workspace` argument empty to search globally."
    except Exception as e:
        print(f"⚠️ Warning: Could not load workspaces into system prompt: {e}")

    return prompt

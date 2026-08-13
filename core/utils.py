from config.app_settings import AppSettings

def get_system_prompt(path = "config/AGENT.md"):
    try:
        with open(path, "r", encoding="utf-8") as f:
            prompt = f.read()
            
            # Inject available workspaces dynamically
            try:
                settings = AppSettings.load()
                if settings.workspaces:
                    workspaces_str = ", ".join([f"'{ws.name}'" for ws in settings.workspaces])
                    prompt += f"\n\n### 6. Workspaces\n- **Available Workspaces:** {workspaces_str}\n"
                    prompt += "- When searching the knowledge base (search_session_memory=False), you should specify the `workspace` argument in the `search_db` tool if the user indicates a specific topic or workspace.\n"
                    prompt += "- If the user's request is ambiguous, you may ask them which workspace they want to search, or leave the `workspace` argument empty to search globally."
            except Exception as e:
                print(f"⚠️ Warning: Could not load workspaces into system prompt: {e}")
                
            return prompt
    except FileNotFoundError:
        print(f"❌ Error: The file {path} was not found.")
        return None
import os
import sys
import dotenv
from typing import Optional
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

def get_api_key(provider: str = "openai") -> Optional[str]:
    """
    Resolves API Key in order:
    1. Environment variables for specified provider
    2. SQLite ConfigDBStore table (api_keys)
    3. Fallback for local offline models ('lm-studio')
    """
    load_env()
    p = (provider or "openai").lower().strip()

    env_map = {
        "openai": ["OPENAI_API_KEY"],
        "anthropic": ["ANTHROPIC_API_KEY"],
        "gemini": ["GEMINI_API_KEY", "GOOGLE_API_KEY", "GOOGLE_GENAI_API_KEY"],
        "google": ["GEMINI_API_KEY", "GOOGLE_API_KEY", "GOOGLE_GENAI_API_KEY"],
        "google_genai": ["GEMINI_API_KEY", "GOOGLE_API_KEY", "GOOGLE_GENAI_API_KEY"],
        "deepseek": ["DEEPSEEK_API_KEY"],
        "groq": ["GROQ_API_KEY"],
        "xai": ["XAI_API_KEY"],
        "openrouter": ["OPENROUTER_API_KEY"],
        "azure": ["AZURE_OPENAI_API_KEY"],
        "azure_openai": ["AZURE_OPENAI_API_KEY"],
    }

    # 1. Check environment variables
    if p in env_map:
        for var_name in env_map[p]:
            val = os.getenv(var_name)
            if val and val.strip():
                return val.strip()
    else:
        val = os.getenv(f"{p.upper()}_API_KEY")
        if val and val.strip():
            return val.strip()

    # 2. Check SQLite Store
    try:
        from any_context.config.db_store import ConfigDBStore
        store = ConfigDBStore()
        store_key = store.get_api_key(p)
        if store_key and store_key.strip():
            cleaned_key = store_key.strip()
            if p == "openai" and cleaned_key == "lm-studio":
                return None
            return cleaned_key
        # Check alias if google/gemini
        if p in ["gemini", "google_genai", "google"]:
            for alt in ["gemini", "google", "google_genai"]:
                alt_key = store.get_api_key(alt)
                if alt_key and alt_key.strip():
                    return alt_key.strip()
    except Exception:
        pass

    # 3. Local offline fallback
    if p in ["local", "lm-studio", "ollama"]:
        return "lm-studio"

    return None


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

def get_system_prompt(path: str = None, active_workspace: str = None, grounding_mode: str = None, web_search_enabled: bool = None):
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

    effective_mode = (grounding_mode or "strict").lower().strip()
    if effective_mode not in ["hybrid", "strict", "proactive"]:
        effective_mode = "strict"

    try:
        from any_context.config.db_store import ConfigDBStore
        store = ConfigDBStore()
        settings = store.get_app_settings()
        if settings and settings.workspaces:
            workspaces_str = ", ".join([f"'{ws.name}'" for ws in settings.workspaces])
            prompt += f"\n\n### 6. Workspaces\n- **Available Workspaces:** {workspaces_str}\n"

        if not grounding_mode and active_workspace:
            effective_mode = store.get_grounding_mode(workspace_name=active_workspace)
        elif not grounding_mode and settings and settings.context:
            effective_mode = getattr(settings.context, "grounding_mode", "strict").lower().strip()

        if web_search_enabled is None:
            if active_workspace:
                web_search_enabled = store.get_web_search_status(workspace_name=active_workspace)
            elif settings and settings.context:
                web_search_enabled = bool(getattr(settings.context, "web_search_enabled", False))
            else:
                web_search_enabled = False

        if active_workspace:
            prompt += f"\n\n### 🎯 ACTIVE WORKSPACE & TOOL CALLING CONTEXT\n"
            prompt += f"- You are currently chatting inside active workspace: **'{active_workspace}'**.\n"
            prompt += f"- **WORKSPACE FILTER RULE:** When calling `search_db` to search documents, you MUST pass `workspace='{active_workspace}'` unless the user explicitly requests searching globally.\n"
            prompt += f"- **SINGLE SEARCH EXECUTION:** Call `search_db` AT MOST ONCE per question. Do NOT repeat or loop calls to `search_db`. Analyze the retrieved document snippets immediately and write a complete, beautifully structured answer.\n"
        else:
            prompt += "- When searching the knowledge base (search_session_memory=False), specify the `workspace` argument in `search_db` if a specific workspace topic is mentioned.\n"

        # Inject Web Search Engine Directives
        if web_search_enabled:
            prompt += f"\n\n### 🌐 LIVE WEB SEARCH ENGINE: ACTIVE (ENABLED FOR WORKSPACE '{active_workspace or 'Current'}')\n"
            prompt += "- You have access to the `live_web_search` tool to fetch real-time public internet data.\n"
            prompt += "- **PORTAL PRIORITIZATION:** If the active workspace has registered web sources, always attempt queries focused on those domains first before falling back to general internet search.\n"
            prompt += "- **SOURCE DISCRIMINATION RULE:** You MUST strictly and visually discriminate the origin of each piece of information in your final response.\n"

            if effective_mode == "strict":
                prompt += (
                    "- **STRICT PROTOCOL FOR WEB SEARCH (USER PERMISSION MANDATORY):**\n"
                    "  1. Search `search_db` FIRST. Rely 100% on the local workspace documents.\n"
                    "  2. You are STRICTLY FORBIDDEN from calling `live_web_search` autonomously on the initial question without prior explicit user confirmation.\n"
                    "  3. If information is missing from the local workspace files (e.g. weather forecast, current news, topics not indexed in files):\n"
                    "     - DO NOT call `live_web_search` immediately.\n"
                    "     - DO NOT guess or hallucinate answers.\n"
                    "     - You MUST inform the user and explicitly ASK:\n"
                    "       *\"⚠️ Essa informação não foi encontrada nos documentos locais do workspace. Deseja que eu faça uma busca na internet sobre '[tópico]'?\"*\n"
                    "  4. When the user confirms (e.g. 'sim', 'yes', 'pode buscar', 'ok', 'faça isso') OR if their prompt explicitly instructs to search online:\n"
                    "     - You MUST call `live_web_search` immediately.\n"
                    "     - **QUERY RECONSTRUCTION RULE:** In the `query` argument of `live_web_search`, you MUST pass the FULL TARGET TOPIC/QUESTION discussed (e.g. `live_web_search(query='previsão do tempo para Calgary amanhã')`), NEVER the confirmation keyword ('sim').\n"
                    "  5. Present all web findings under:\n"
                    "     `### 🌐 Resultados da Web Externa (Fonte: <URL>)`\n"
                )
            elif effective_mode == "proactive":
                prompt += (
                    "- **PROACTIVE PROTOCOL FOR WEB SEARCH:**\n"
                    "  1. Combine `search_db` (local workspace documents) and `live_web_search` (web search) proactively to provide a state-of-the-art, comprehensive synthesis.\n"
                    "  2. Clearly tag and differentiate every statement by its exact origin: `[Documento: <arquivo>]` vs `[Web: <URL>]`.\n"
                )
            else: # hybrid
                prompt += (
                    "- **HYBRID DUAL-LAYER PROTOCOL FOR WEB SEARCH:**\n"
                    "  1. Query `search_db` first for local workspace facts.\n"
                    "  2. If the workspace context is incomplete, outdated, or benefits from online lookup, call `live_web_search` to gather external information.\n"
                    "  3. Present the response with clear separated sections:\n"
                    "     `### 📂 Informações do Workspace` (baseado nos arquivos locais com citações)\n"
                    "     `### 🌐 Informações Complementares da Web` (com links e fontes externas https://...)\n"
                )
        else:
            prompt += (
                "\n\n### 🔒 LIVE WEB SEARCH: DISABLED (OFFLINE-FIRST LOCAL ISOLATION)\n"
                "- Web search is DISABLED for this workspace. Answer exclusively using local workspace documents and parametric reasoning.\n"
            )

        # Inject Grounding Mode Directives
        if effective_mode == "strict":
            prompt += (
                "\n### 🛡️ ACTIVE GROUNDING MODE: STRICT (AUDIT & LEGAL - 100% FACTUAL & MANDATORY CITATIONS)\n"
                "- **ZERO SPECULATION / ZERO HALLUCINATION:** Answer EXCLUSIVELY and ONLY using verified facts present in the retrieved workspace chunks.\n"
                "- **MANDATORY SOURCE CITATIONS:** You MUST explicitly cite the exact file names, page numbers, or URLs where every piece of information was found.\n"
                "- **MANDATORY CITATION FOOTER:** At the end of every answer that uses workspace documents, you MUST append:\n"
                "  `---\n  📄 **Fontes Consultadas no Workspace:**\n  - [Nome do Arquivo / URL]`\n"
                "- **FACTUAL ABSENCE PROTOCOL:** If the user asks for information, status, rules, or details NOT present in the retrieved documents, you MUST explicitly state that this information is not found in the indexed workspace files.\n"
                "- **FORBIDDEN ACTION:** Do NOT use pre-training weights or parametric memory to invent, assume, extrapolate, or provide unverified external factual lists.\n"
            )
        elif effective_mode == "proactive":
            prompt += (
                "\n### 🚀 ACTIVE GROUNDING MODE: PROACTIVE (RESEARCH & STRATEGY)\n"
                "- **COMPREHENSIVE SYNTHESIS:** Provide a rich, detailed answer using the retrieved workspace context as the core factual foundation.\n"
                "- **FORWARD-LOOKING INSIGHTS:** In addition to answering the user's question, proactively identify potential risks, related considerations, adjacent questions to explore, and actionable next steps.\n"
                "- **WEB SOURCE RECOMMENDATIONS:** If relevant, recommend authoritative public websites or documentation URLs the user could index into this workspace using the command '/web add <url>'.\n"
            )
        else: # Default: hybrid
            prompt += (
                "\n### ⚖️ ACTIVE GROUNDING MODE: HYBRID (BALANCED - DUAL-LAYER GROUNDING)\n"
                "- **DUAL-LAYER STRUCTURE:**\n"
                "  1. **Layer 1 (Workspace Facts):** Answer first using all verified facts found in the retrieved workspace documents and cite the sources.\n"
                "  2. **Layer 2 (External Suggestions / General Knowledge):** If any part of the user's question is not covered by the workspace documents, you MAY provide general background, reasoning, or suggestions, BUT you MUST clearly segregate and label it under a distinct heading:\n"
                "     '### 💡 Sugestões / Conhecimento Geral do Modelo (Não Verificado nos Documentos)'\n"
                "  3. **VERIFICATION WARNING:** Advise the user to verify external suggestions on official sources, or suggest indexing additional URLs using '/web add <url>'.\n"
            )

    except Exception as e:
        print(f"⚠️ Warning: Could not configure system prompt directives: {e}")

    return prompt


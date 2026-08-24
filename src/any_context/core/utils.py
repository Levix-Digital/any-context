import os
import sys
import dotenv
from typing import Optional
from any_context.config.app_settings import AppSettings


def load_env():
    candidates = []
    if sys.platform == "win32":
        if "APPDATA" in os.environ:
            candidates.append(os.path.join(os.environ["APPDATA"], "any-context", ".env"))
            candidates.append(os.path.join(os.environ["APPDATA"], "actx", ".env"))
        if "LOCALAPPDATA" in os.environ:
            candidates.append(os.path.join(os.environ["LOCALAPPDATA"], "any-context", ".env"))
            candidates.append(os.path.join(os.environ["LOCALAPPDATA"], "actx", ".env"))
            candidates.append(os.path.join(os.environ["LOCALAPPDATA"], "actx", "bin", ".env"))

    candidates.extend([
        os.path.expanduser(os.path.join("~", ".config", "any-context", ".env")),
        os.path.join(os.path.dirname(sys.executable), "..", ".env"),
        os.path.join(os.path.dirname(sys.executable), ".env"),
        os.path.join(os.getcwd(), ".env"),
    ])

    for c in candidates:
        if os.path.exists(c):
            try:
                dotenv.load_dotenv(c, override=False)
            except Exception:
                pass

    dotenv.load_dotenv()
    _map_langsmith_env()

def _map_langsmith_env():
    """Maps LANGSMITH environment variables to LANGCHAIN variables for 100% tracing compatibility."""
    if os.getenv("LANGSMITH_TRACING") and not os.getenv("LANGCHAIN_TRACING_V2"):
        os.environ["LANGCHAIN_TRACING_V2"] = os.getenv("LANGSMITH_TRACING")
    if os.getenv("LANGSMITH_API_KEY") and not os.getenv("LANGCHAIN_API_KEY"):
        os.environ["LANGCHAIN_API_KEY"] = os.getenv("LANGSMITH_API_KEY")
    if os.getenv("LANGSMITH_PROJECT") and not os.getenv("LANGCHAIN_PROJECT"):
        os.environ["LANGCHAIN_PROJECT"] = os.getenv("LANGSMITH_PROJECT")
    if os.getenv("LANGSMITH_ENDPOINT") and not os.getenv("LANGCHAIN_ENDPOINT"):
        os.environ["LANGCHAIN_ENDPOINT"] = os.getenv("LANGSMITH_ENDPOINT")

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
            prompt += "- **SMART DOMAIN ROUTING & FALLBACK:** If the user asks about a specific portal or organization registered in this workspace (especially for SPAs with dynamic content), pass `target_domain` to `live_web_search`. For general queries (weather, news, facts), omit `target_domain` to search the open global web.\n"
            prompt += "- **MANDATORY WEB CITATION FOOTER RULE:** Whenever you fetch or use information from `live_web_search`, you MUST include a dedicated sources footer block at the very end of your response with the exact clickable Markdown URLs consulted:\n"
            prompt += "  ---\n"
            prompt += "  🌐 **Fontes Consultadas na Web:**\n"
            prompt += "  - [Título da Página / Loja](https://...)\n"
            prompt += "- **SOURCE DISCRIMINATION RULE:** You MUST strictly and visually discriminate the origin of each piece of information in your final response.\n"

            if effective_mode == "strict":
                prompt += (
                    "- **🛡️ STRICT PROTOCOL FOR WEB SEARCH & PERMISSION GATE (MANDATORY RULE):**\n"
                    "  1. You are operating in STRICT AUDIT & LEGAL GROUNDING MODE.\n"
                    "  2. Search `search_db` FIRST. Rely 100% on the local workspace documents with ZERO parametric hallucination.\n"
                    "  3. **ABSOLUTE PROHIBITION ON AUTONOMOUS WEB SEARCH:** You are STRICTLY FORBIDDEN from calling `live_web_search` on any initial question unless the user explicitly used search commands (e.g. 'pesquise na internet', 'busque online') or explicitly confirmed permission.\n"
                    "  4. If information is missing from the local workspace files:\n"
                    "     - DO NOT call `live_web_search`.\n"
                    "     - DO NOT invent or guess.\n"
                    "     - You MUST STOP, inform the user, and explicitly ASK:\n"
                    "       *\"⚠️ Essa informação não consta nos documentos deste workspace. Deseja que eu faça uma busca na internet sobre '[tópico]'?\"*\n"
                    "  5. ONLY when the user replies confirming (e.g. 'sim', 'yes', 'pode buscar', 'ok', 'faça isso') is `live_web_search` permitted to be invoked.\n"
                    "  6. When confirmed, pass the FULL TOPIC to `query` (never the literal 'sim') and present web findings under:\n"
                    "     `### 🌐 Resultados da Web Externa (Fonte: <URL>)`\n"
                )
            elif effective_mode == "proactive":
                prompt += (
                    "- **PROACTIVE PROTOCOL FOR WEB SEARCH (AUTONOMOUS & COMPREHENSIVE):**\n"
                    "  1. Proactively combine `search_db` (local workspace documents) and `live_web_search` (web search) without asking for user permission.\n"
                    "  2. Give equal priority to workspace web portals and open web to deliver a comprehensive, forward-looking strategic synthesis.\n"
                    "  3. Clearly tag and differentiate every statement by its exact origin: `[Documento: <arquivo>]` vs `[Web: <URL>]` vs `[Análise do Modelo]`.\n"
                )
            else: # hybrid
                prompt += (
                    "- **HYBRID DUAL-LAYER PROTOCOL FOR WEB SEARCH (AUTONOMOUS EXECUTION):**\n"
                    "  1. Query `search_db` first for local workspace facts.\n"
                    "  2. If the workspace context is incomplete or the question benefits from online facts, call `live_web_search` AUTONOMOUSLY without asking for user permission.\n"
                    "  3. Prioritize registered workspace portals first with automatic open web fallback, focusing strictly on delivering a specific, targeted answer.\n"
                    "  4. Present the response with clear separated sections:\n"
                    "     `### 📂 Informações do Workspace` (baseado nos arquivos locais com citações)\n"
                    "     `### 🌐 Informações Complementares da Web` (com links e fontes externas https://...)\n"
                )
        else:
            prompt += (
                "\n\n### 🔒 LIVE WEB SEARCH: DISABLED (OFFLINE-FIRST LOCAL ISOLATION)\n"
                "- Web search is DISABLED for this workspace. Answer exclusively using local workspace documents.\n"
            )

        # Inject Grounding Mode Directives
        if effective_mode == "strict":
            prompt += (
                "\n### 🛡️ ACTIVE GROUNDING MODE: STRICT (AUDIT & LEGAL - 100% FACTUAL & ZERO PARAMETRIC ANSWERS)\n"
                "- **ZERO SPECULATION / ZERO HALLUCINATION / ZERO PARAMETRIC MEMORY:** You are STRICTLY FORBIDDEN from using your pre-trained internal memory or parametric weights to answer, invent, assume, or provide unverified facts (e.g. citing laws from other countries, unindexed regulations, or outside facts). If a fact is not in the workspace documents, you MUST declare its absence.\n"
                "- **FACTUAL ABSENCE PROTOCOL:** If the information is not found in the workspace files, you MUST state:\n"
                "  '⚠️ Essa informação não consta nos documentos deste workspace.'\n"
                "- **MANDATORY SOURCE CITATIONS:** You MUST explicitly cite the exact file names, page numbers, or URLs where every piece of information was found.\n"
                "- **MANDATORY CITATION FOOTER:** At the end of every answer that uses workspace documents, you MUST append:\n"
                "  `---\n  📄 **Fontes Consultadas no Workspace:**\n  - [Nome do Arquivo / URL]`\n"
            )
        elif effective_mode == "proactive":
            prompt += (
                "\n### 🚀 ACTIVE GROUNDING MODE: PROACTIVE (RESEARCH & STRATEGY - FREE INTEGRATION)\n"
                "- **FREE INTEGRATION:** You are free to seamlessly combine workspace documents, real-time web intelligence, and pre-trained parametric memory to offer comprehensive advice, anticipate risks, and recommend actionable solutions.\n"
                "- **FORWARD-LOOKING INSIGHTS:** In addition to answering the user's question, proactively identify potential risks, related considerations, adjacent questions to explore, and actionable next steps.\n"
                "- **STRICT SOURCE TAGGING:** Tag facts with their source (`[Documento: ...]`, `[Web: ...]`, `[Recomendação]`).\n"
                "- **WEB SOURCE RECOMMENDATIONS:** Recommend authoritative public websites the user can index into this workspace using '/web add <url>'.\n"
            )
        else: # Default: hybrid
            prompt += (
                "\n### ⚖️ ACTIVE GROUNDING MODE: HYBRID (BALANCED - WORKSPACE FIRST + LABELED MODEL KNOWLEDGE)\n"
                "- **WORKSPACE PRIORITY:** Always query and present local workspace facts first.\n"
                "- **DUAL-LAYER STRUCTURE:**\n"
                "  `### 📂 Informações do Workspace` (fatos extraídos dos documentos)\n"
                "  `### 💡 Sugestões / Conhecimento Geral do Modelo` (conhecimento paramétrico devidamente rotulado)\n"
                "- **PARAMETRIC MEMORY TRANSPARENCY RULE:** If you supplement the answer with your pre-trained model knowledge, you MUST explicitly disclose its origin to the user using phrases such as:\n"
                "  *\"De acordo com meus conhecimentos gerais...\"* or *\"Com base no conhecimento geral do modelo (não verificado nos documentos)...\"*\n"
            )

    except Exception as e:
        print(f"⚠️ Warning: Could not configure system prompt directives: {e}")

    return prompt


def format_turn_grounding_header(
    active_workspace: Optional[str] = None,
    grounding_mode: Optional[str] = None,
    web_search_enabled: bool = False
) -> str:
    """
    Convenience helper to format an ultra-compact, token-efficient prompt header
    for the active conversation turn using the Strategy Pattern.
    """
    from any_context.core.grounding_strategies import get_grounding_strategy
    strategy = get_grounding_strategy(grounding_mode)
    return strategy.format_turn_header(workspace_name=active_workspace, web_search_enabled=web_search_enabled)



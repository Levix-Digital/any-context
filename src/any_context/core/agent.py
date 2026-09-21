import os
import re
import uuid
import sqlite3
import threading
from typing import Optional, List, Dict, Any, Set

MAX_ACTIVE_SESSION_TURNS = 15
_active_summarizing_threads: Set[str] = set()
_summarizing_lock = threading.Lock()


from langchain.chat_models import init_chat_model
from langchain.agents import create_agent
from langgraph.checkpoint.sqlite import SqliteSaver
from langgraph.checkpoint.memory import MemorySaver

from any_context.tools.search_tools import search_db, add_web_source, list_web_sources, remove_web_source
from any_context.tools.web_search_tool import live_web_search
from any_context.ingestion.local_folder_ingestor import index_folder
from any_context.core.utils import get_system_prompt, get_api_key
from any_context.config.app_settings import AppSettings
from any_context.config.db_store import ConfigDBStore


def _filter_citation_footer(text: str, deleted_basenames: Optional[set] = None, is_immediate_prior: bool = False) -> str:
    """
    Intelligently filters citation footers from historical assistant messages:
    1. For the immediate prior assistant message, preserves the active citation footer intact
       (purging only records matching deleted_basenames) so follow-up provenance questions work.
    2. For older historical messages (>1 turn ago), condenses the footer to a compact footnote
       preserving file names while saving token budget.
    3. If all cited files were deleted from disk, strips the footer entirely.
    """
    if not isinstance(text, str) or not text:
        return text

    del_set = deleted_basenames or set()
    pattern = r"((?:\n+---\s*)?\n*(?:###\s*)?(?:[📄🌐☁️]\s*)?\*?\*?(?:Fontes Consultadas|Sources Consulted)[\s\S]*$)"
    match = re.search(pattern, text, flags=re.IGNORECASE)
    if not match:
        return text

    footer_text = match.group(1)
    body_text = text[:match.start()].rstrip()

    lines = footer_text.splitlines()
    header_line = "📄 Fontes Consultadas:"
    source_lines = []
    for line in lines:
        clean_line = line.strip()
        if not clean_line or clean_line.startswith("---") or clean_line.startswith("###"):
            continue
        if re.search(r"(?:Fontes Consultadas|Sources Consulted)", clean_line, flags=re.IGNORECASE):
            header_line = clean_line
            continue
        is_deleted = any(del_b.lower() in clean_line.lower() for del_b in del_set)
        if not is_deleted:
            source_lines.append(clean_line)

    if not source_lines:
        return body_text

    if is_immediate_prior:
        clean_footer = "\n" + header_line + "\n" + "\n".join(source_lines)
        return f"{body_text}\n\n{clean_footer.strip()}"
    else:
        basenames = []
        for sl in source_lines:
            sl_clean = re.sub(r"^[-\s*•]+", "", sl)
            sl_clean = re.sub(r"\s*\(.*?\)", "", sl_clean).strip()
            if sl_clean:
                basenames.append(sl_clean)
        if basenames:
            return f"{body_text}\n\n📄 Fontes Consultadas: {', '.join(basenames)}"
        return body_text


def _strip_historical_citation_footers(text: str) -> str:
    """Strips raw workspace/web citation footers completely from historical assistant messages."""
    if not isinstance(text, str) or not text:
        return text

    pattern = r"(?:\n+---\s*)?\n*(?:###\s*)?(?:[📄🌐☁️]\s*)?\*?\*?(?:Fontes Consultadas|Sources Consulted)[\s\S]*$"
    cleaned = re.sub(pattern, "", text, flags=re.IGNORECASE)
    return cleaned.rstrip()


def _prune_historical_tool_messages(messages):
    """
    Prunes heavy raw chunk dumps from prior turns' ToolMessages,
    retaining only compact English markers while preserving all HumanMessage and AIMessage
    conversational dialog and the current turn's fresh ToolMessages.
    """
    if not isinstance(messages, list) or len(messages) <= 2:
        return messages

    # 1. Identify the demarcation of the active/current turn (latest HumanMessage)
    last_human_idx = -1
    for idx in range(len(messages) - 1, -1, -1):
        m = messages[idx]
        m_type = getattr(m, "type", "")
        if m_type == "human" or m.__class__.__name__ == "HumanMessage":
            last_human_idx = idx
            break

    # 2. Strictly prune ONLY ToolMessages that belong to PRIOR turns (idx < last_human_idx).
    # ToolMessages belonging to the current turn (idx > last_human_idx) MUST REMAIN 100% INTACT
    # with their retrieved document chunks so that the model can answer the user's question!
    if last_human_idx != -1:
        for idx in range(last_human_idx):
            msg = messages[idx]
            m_type = getattr(msg, "type", "")
            if m_type in ["tool", "ToolMessage"] or hasattr(msg, "tool_call_id"):
                c_str = str(getattr(msg, "content", ""))
                if len(c_str) > 300:
                    setattr(msg, "content", "[Prior workspace context retrieved and synthesized in conversation history]")
    return messages


def sanitize_conversation_messages(messages):
    """
    Ensures that every assistant message with tool_calls is followed by matching ToolMessages.
    If an interrupted turn left orphan tool_calls without ToolMessages, injects synthetic
    ToolMessages so that OpenAI / Anthropic / Gemini API contracts are strictly satisfied.
    """
    if not isinstance(messages, list):
        return messages

    sanitized = []
    i = 0
    while i < len(messages):
        msg = messages[i]
        sanitized.append(msg)

        tool_calls = getattr(msg, "tool_calls", None)
        m_type = getattr(msg, "type", msg.__class__.__name__)

        # If this is an assistant/AI message with tool calls
        if (m_type in ["ai", "AIMessage", "assistant"] or hasattr(msg, "tool_calls")) and tool_calls:
            required_call_ids = []
            for tc in tool_calls:
                cid = tc.get("id") if isinstance(tc, dict) else getattr(tc, "id", None)
                if cid:
                    required_call_ids.append(cid)

            # Scan immediately following messages for tool messages responding to these IDs
            j = i + 1
            answered_call_ids = set()
            while j < len(messages):
                next_msg = messages[j]
                next_type = getattr(next_msg, "type", next_msg.__class__.__name__)
                next_tool_id = getattr(next_msg, "tool_call_id", None)

                if next_type in ["tool", "ToolMessage"] or next_tool_id:
                    if next_tool_id:
                        answered_call_ids.add(next_tool_id)
                    sanitized.append(next_msg)
                    j += 1
                else:
                    # Encountered human or next AI message before completing tool calls
                    break

            # For any missing tool_call_id, inject a synthetic ToolMessage to satisfy LLM API schema
            for req_id in required_call_ids:
                if req_id not in answered_call_ids:
                    from langchain_core.messages import ToolMessage
                    synthetic_tool = ToolMessage(
                        content="[Context retrieval was interrupted or cancelled in prior turn]",
                        tool_call_id=req_id,
                        name="search_db"
                    )
                    sanitized.append(synthetic_tool)

            i = j
        else:
            i += 1

    return sanitized


def _prune_messages_for_llm(
    messages,
    max_current_turn_chars=40000,
    max_history_messages: int = 10,
    active_workspace: str = None,
    grounding_mode: str = None,
    web_search_enabled: bool = False
):
    """
    Prunes raw chunk dumps from prior turns' ToolMessages at LLM call-time,
    enforces a sliding window on prior turns to prevent context pollution and attention echo chambers,
    intelligently consolidates multiple ToolMessages within the current turn under a safe budget,
    and dynamically injects the active GroundingStrategy header on the active HumanMessage.
    """
    if not messages or not isinstance(messages, list):
        return messages

    # 1. Find the index of the latest HumanMessage (demarcating current turn)
    last_human_idx = -1
    for i in range(len(messages) - 1, -1, -1):
        m = messages[i]
        if getattr(m, "type", "") == "human" or m.__class__.__name__ == "HumanMessage":
            last_human_idx = i
            break

    # 2. Epistemic Hygiene: Filter historical turns that ended in FACTUAL_ABSENCE
    # This completely eliminates Self-Consistency Bias / Attention Echo Chambers
    # where the LLM's attention is trapped into confirming past negative statements.
    if last_human_idx != -1 and last_human_idx > 0:
        from any_context.core.epistemic import EpistemicState, is_pure_negative_disclaimer

        filtered_historical = []
        i = 0
        while i < last_human_idx:
            msg = messages[i]
            m_type = getattr(msg, "type", "")
            is_ai = (m_type in ["ai", "AIMessage", "assistant"] or msg.__class__.__name__ in ["AIMessage", "AIMessageChunk"])
            
            if is_ai:
                kwargs = getattr(msg, "additional_kwargs", {}) or {}
                state = kwargs.get("epistemic_state")
                c_str = str(getattr(msg, "content", "") or "")
                
                is_absence = False
                if state in [EpistemicState.FACTUAL_ABSENCE.value, EpistemicState.FACTUAL_ABSENCE]:
                    is_absence = True
                elif state is None and is_pure_negative_disclaimer(c_str):
                    is_absence = True
                    
                if is_absence:
                    # Pure absence disclaimer in historical turn: purge from active inference payload
                    i += 1
                    continue
            
            filtered_historical.append(msg)
            i += 1

        # Deduplicate consecutive identical HumanMessages in historical context
        deduped_historical = []
        for m in filtered_historical:
            if deduped_historical:
                prev = deduped_historical[-1]
                is_prev_human = getattr(prev, "type", "") in ["human", "user"] or prev.__class__.__name__ == "HumanMessage"
                is_curr_human = getattr(m, "type", "") in ["human", "user"] or m.__class__.__name__ == "HumanMessage"
                if is_prev_human and is_curr_human and str(getattr(prev, "content", "")).strip() == str(getattr(m, "content", "")).strip():
                    continue
            deduped_historical.append(m)

        # If the trailing historical message is an unanswered HumanMessage identical to the active turn, drop it
        if deduped_historical and last_human_idx < len(messages):
            active_human = messages[last_human_idx]
            last_hist = deduped_historical[-1]
            is_last_human = getattr(last_hist, "type", "") in ["human", "user"] or last_hist.__class__.__name__ == "HumanMessage"
            if is_last_human and str(getattr(last_hist, "content", "")).strip() == str(getattr(active_human, "content", "")).strip():
                deduped_historical.pop()

        messages = deduped_historical + messages[last_human_idx:]
        last_human_idx = len(deduped_historical)

    # 3. Sliding window for historical messages before the active turn
    # This prevents the attention attractor / echo chamber where dozens of past turns
    # degrade transformer attention and cause hallucinated negative responses.
    if last_human_idx != -1 and max_history_messages and max_history_messages > 0:
        prior_messages_count = last_human_idx
        if prior_messages_count > max_history_messages:
            prior_human_indices = [
                i for i in range(last_human_idx)
                if getattr(messages[i], "type", "") in ["human", "user"] or messages[i].__class__.__name__ == "HumanMessage"
            ]
            target_start = max(0, last_human_idx - max_history_messages)
            eligible_starts = [idx for idx in prior_human_indices if idx >= target_start]
            if eligible_starts:
                window_start_idx = eligible_starts[0]
            elif prior_human_indices:
                window_start_idx = prior_human_indices[-1]
            else:
                window_start_idx = target_start

            if window_start_idx > 0:
                messages = messages[window_start_idx:]
                last_human_idx -= window_start_idx
    elif last_human_idx == -1 and max_history_messages and max_history_messages > 0:
        if len(messages) > max_history_messages:
            messages = messages[-max_history_messages:]

    # 3. Collect current turn ToolMessages
    current_turn_tool_indices = []
    if last_human_idx != -1:
        for idx in range(last_human_idx + 1, len(messages)):
            m = messages[idx]
            m_type = getattr(m, "type", "")
            if m_type in ["tool", "ToolMessage"] or hasattr(m, "tool_call_id"):
                current_turn_tool_indices.append(idx)
    else:
        for idx, m in enumerate(messages):
            m_type = getattr(m, "type", "")
            if m_type in ["tool", "ToolMessage"] or hasattr(m, "tool_call_id"):
                current_turn_tool_indices.append(idx)

    # Budget per tool call in current turn
    num_current_tools = len(current_turn_tool_indices)
    per_tool_char_budget = max(4000, max_current_turn_chars // max(1, num_current_tools))

    # Retrieve workspace sync ledger deleted sources to filter expurgated files from citations
    deleted_basenames = set()
    try:
        store = ConfigDBStore()
        ledger = store.get_sync_ledger(active_workspace or "Default")
        if ledger and ledger.get("deleted_sources"):
            deleted_basenames = {os.path.basename(p).lower() for p in ledger["deleted_sources"]}
    except Exception:
        pass

    # Find the index of the immediately preceding assistant message (turn N-1)
    last_ai_idx_before_human = -1
    if last_human_idx != -1:
        for i in range(last_human_idx - 1, -1, -1):
            m = messages[i]
            m_t = getattr(m, "type", "")
            if m_t in ["ai", "AIMessage", "assistant"] or m.__class__.__name__ in ["AIMessage", "AIMessageChunk"]:
                last_ai_idx_before_human = i
                break

    # Prepare turn grounding header if mode is supplied
    turn_header = None
    if grounding_mode:
        from any_context.core.grounding_strategies import get_grounding_strategy
        strategy = get_grounding_strategy(grounding_mode)
        turn_header = strategy.format_turn_header(workspace_name=active_workspace, web_search_enabled=web_search_enabled)

    pruned = []
    for idx, msg in enumerate(messages):
        m_type = getattr(msg, "type", "")
        is_human = (m_type == "human" or msg.__class__.__name__ == "HumanMessage")
        is_tool = (m_type in ["tool", "ToolMessage"] or hasattr(msg, "tool_call_id"))
        is_ai = (m_type in ["ai", "AIMessage", "assistant"] or msg.__class__.__name__ in ["AIMessage", "AIMessageChunk"])

        if is_human and idx == last_human_idx and turn_header:
            # Active turn HumanMessage: inject concise dynamic grounding strategy header
            cloned = msg.model_copy() if hasattr(msg, "model_copy") else msg
            raw_content = getattr(cloned, "content", "")
            if isinstance(raw_content, str):
                if "[GROUNDING:" not in raw_content:
                    cloned.content = f"{turn_header}\n\n{raw_content}"
            elif isinstance(raw_content, list):
                text_injected = False
                new_parts = []
                for part in raw_content:
                    if isinstance(part, dict) and part.get("type") == "text" and not text_injected:
                        orig = part.get("text", "")
                        if "[GROUNDING:" not in orig:
                            new_parts.append({**part, "text": f"{turn_header}\n\n{orig}"})
                        else:
                            new_parts.append(part)
                        text_injected = True
                    elif isinstance(part, str) and not text_injected:
                        if "[GROUNDING:" not in part:
                            new_parts.append(f"{turn_header}\n\n{part}")
                        else:
                            new_parts.append(part)
                        text_injected = True
                    else:
                        new_parts.append(part)
                if not text_injected:
                    new_parts.insert(0, {"type": "text", "text": turn_header})
                cloned.content = new_parts
            pruned.append(cloned)
        elif is_tool and idx < last_human_idx:
            # Historical tool from a prior turn: compact to English marker
            cloned = msg.model_copy() if hasattr(msg, "model_copy") else msg
            cloned.content = "[Prior workspace context retrieved and synthesized in conversation history]"
            pruned.append(cloned)
        elif is_ai and idx < last_human_idx:
            # Historical assistant message: filter citation footers, preserving active sources for immediate prior turn
            cloned = msg.model_copy() if hasattr(msg, "model_copy") else msg
            raw_content = getattr(cloned, "content", "")
            is_immediate = (idx == last_ai_idx_before_human)
            if isinstance(raw_content, str):
                cloned.content = _filter_citation_footer(raw_content, deleted_basenames=deleted_basenames, is_immediate_prior=is_immediate)
            elif isinstance(raw_content, list):
                new_parts = []
                for part in raw_content:
                    if isinstance(part, dict) and part.get("type") == "text":
                        new_parts.append({**part, "text": _filter_citation_footer(part.get("text", ""), deleted_basenames=deleted_basenames, is_immediate_prior=is_immediate)})
                    elif isinstance(part, str):
                        new_parts.append(_filter_citation_footer(part, deleted_basenames=deleted_basenames, is_immediate_prior=is_immediate))
                    else:
                        new_parts.append(part)
                cloned.content = new_parts
            pruned.append(cloned)
        elif is_tool and idx in current_turn_tool_indices:
            # Current turn tool: preserve topics within proportional budget!
            content_str = str(getattr(msg, "content", "") or "")
            if len(content_str) > per_tool_char_budget:
                cloned = msg.model_copy() if hasattr(msg, "model_copy") else msg
                cloned.content = content_str[:per_tool_char_budget] + "\n[...additional topic snippets condensed for turn budget...]"
                pruned.append(cloned)
            else:
                pruned.append(msg)
        else:
            pruned.append(msg)

    return sanitize_conversation_messages(pruned)


def _is_web_search_authorized_by_prompt(prompt: str) -> bool:
    """Checks whether the user's prompt is an explicit web search confirmation or request."""
    if not prompt:
        return False
    # If prompt contains injected turn header, extract the actual human message content below it
    p = prompt.strip()
    if "[GROUNDING:" in p:
        parts = p.split("\n\n", 1)
        if len(parts) > 1:
            p = parts[1].strip()
        else:
            lines = [l for l in p.splitlines() if not l.startswith("[") and not l.startswith("-")]
            p = " ".join(lines).strip()

    p = p.lower()
    confirmations = {
        "sim", "yes", "y", "s", "pode", "pode buscar", "pode pesquisar",
        "faça isso", "faca isso", "prossiga", "ok", "buscar", "pesquisar",
        "pesquise", "busca", "faça", "faca", "claro", "com certeza", "autorizo"
    }
    if p in confirmations or any(p.startswith(c) for c in ["sim,", "sim ", "yes,", "yes ", "pode ", "claro,"]):
        return True
    search_keywords = [
        "pesquise na web", "pesquise na internet", "busque na web", "busque na internet",
        "procure na web", "procure na internet", "search online", "search the web", "search internet",
        "google", "na internet", "no google", "pesquisa online"
    ]
    return any(k in p for k in search_keywords)


class PruningBoundModel:
    def __init__(
        self,
        bound_model,
        bound_no_web=None,
        active_workspace: str = None,
        grounding_mode: str = "hybrid",
        web_search_enabled: bool = False
    ):
        self._bound_model = bound_model
        self._bound_no_web = bound_no_web
        self._active_workspace = active_workspace
        self._grounding_mode = (grounding_mode or "hybrid").lower().strip()
        self._web_search_enabled = bool(web_search_enabled)

    def __getattr__(self, name):
        return getattr(self._bound_model, name)

    def _prune(self, input_val):
        if isinstance(input_val, list):
            return _prune_messages_for_llm(
                input_val,
                active_workspace=self._active_workspace,
                grounding_mode=self._grounding_mode,
                web_search_enabled=self._web_search_enabled
            )
        return input_val

    def _select_bound(self, messages):
        if self._bound_no_web is None or self._grounding_mode != "strict" or not isinstance(messages, list):
            return self._bound_model

        latest_human_text = ""
        for i in range(len(messages) - 1, -1, -1):
            m = messages[i]
            if getattr(m, "type", "") == "human" or m.__class__.__name__ == "HumanMessage":
                content = getattr(m, "content", "")
                if isinstance(content, str):
                    latest_human_text = content
                elif isinstance(content, list):
                    latest_human_text = " ".join([str(p) for p in content])
                break

        if _is_web_search_authorized_by_prompt(latest_human_text):
            return self._bound_model
        return self._bound_no_web

    def invoke(self, input_val, config=None, **kwargs):
        from any_context.tools.search_tools import set_active_workspace_context, reset_active_workspace_context
        tok = set_active_workspace_context(self._active_workspace)
        try:
            input_val = self._prune(input_val)
            active_bound = self._select_bound(input_val)
            res = active_bound.invoke(input_val, config=config, **kwargs)
            if hasattr(res, "additional_kwargs") and not getattr(res, "tool_calls", None):
                from any_context.core.epistemic import classify_epistemic_state
                state = classify_epistemic_state(getattr(res, "content", ""))
                res.additional_kwargs["epistemic_state"] = state.value
            return res
        finally:
            reset_active_workspace_context(tok)

    def stream(self, input_val, config=None, **kwargs):
        from any_context.tools.search_tools import set_active_workspace_context, reset_active_workspace_context
        tok = set_active_workspace_context(self._active_workspace)
        try:
            input_val = self._prune(input_val)
            active_bound = self._select_bound(input_val)
            for chunk in active_bound.stream(input_val, config=config, **kwargs):
                yield chunk
        finally:
            reset_active_workspace_context(tok)

    async def ainvoke(self, input_val, config=None, **kwargs):
        from any_context.tools.search_tools import set_active_workspace_context, reset_active_workspace_context
        tok = set_active_workspace_context(self._active_workspace)
        try:
            input_val = self._prune(input_val)
            active_bound = self._select_bound(input_val)
            res = await active_bound.ainvoke(input_val, config=config, **kwargs)
            if hasattr(res, "additional_kwargs") and not getattr(res, "tool_calls", None):
                from any_context.core.epistemic import classify_epistemic_state
                state = classify_epistemic_state(getattr(res, "content", ""))
                res.additional_kwargs["epistemic_state"] = state.value
            return res
        finally:
            reset_active_workspace_context(tok)

    async def astream(self, input_val, config=None, **kwargs):
        from any_context.tools.search_tools import set_active_workspace_context, reset_active_workspace_context
        tok = set_active_workspace_context(self._active_workspace)
        try:
            input_val = self._prune(input_val)
            active_bound = self._select_bound(input_val)
            async for chunk in active_bound.astream(input_val, config=config, **kwargs):
                yield chunk
        finally:
            reset_active_workspace_context(tok)

    def generate_prompt(self, prompts, **kwargs):
        pruned_prompts = []
        for p in prompts:
            if hasattr(p, "to_messages"):
                msgs = self._prune(p.to_messages())
                pruned_prompts.append(msgs)
            else:
                pruned_prompts.append(p)
        return self._bound_model.generate(pruned_prompts, **kwargs)


class PruningChatModelWrapper:
    def __init__(
        self,
        raw_model,
        active_workspace: str = None,
        grounding_mode: str = "hybrid",
        web_search_enabled: bool = False
    ):
        self._raw_model = raw_model
        self._active_workspace = active_workspace
        self._grounding_mode = (grounding_mode or "hybrid").lower().strip()
        self._web_search_enabled = bool(web_search_enabled)

    def __getattr__(self, name):
        return getattr(self._raw_model, name)

    def _prune(self, input_val):
        if isinstance(input_val, list):
            return _prune_messages_for_llm(
                input_val,
                active_workspace=self._active_workspace,
                grounding_mode=self._grounding_mode,
                web_search_enabled=self._web_search_enabled
            )
        return input_val

    def bind_tools(self, tools, **kwargs):
        has_web = any(getattr(t, "name", "") == "live_web_search" for t in tools)
        if has_web and self._grounding_mode == "strict":
            bound_all = self._raw_model.bind_tools(tools, **kwargs)
            tools_no_web = [t for t in tools if getattr(t, "name", "") != "live_web_search"]
            bound_no_web = self._raw_model.bind_tools(tools_no_web, **kwargs)
            return PruningBoundModel(
                bound_all,
                bound_no_web=bound_no_web,
                active_workspace=self._active_workspace,
                grounding_mode="strict",
                web_search_enabled=self._web_search_enabled
            )

        bound = self._raw_model.bind_tools(tools, **kwargs)
        return PruningBoundModel(
            bound,
            active_workspace=self._active_workspace,
            grounding_mode=self._grounding_mode,
            web_search_enabled=self._web_search_enabled
        )

    def invoke(self, input_val, config=None, **kwargs):
        input_val = self._prune(input_val)
        return self._raw_model.invoke(input_val, config=config, **kwargs)

    def stream(self, input_val, config=None, **kwargs):
        input_val = self._prune(input_val)
        return self._raw_model.stream(input_val, config=config, **kwargs)


class ResilientSqliteSaver(SqliteSaver):
    """
    Auto-healing SqliteSaver that safely recovers from corrupted zlib streams,
    incomplete checkpoint bytes, or database locking errors without crashing the agent,
    maintains a rolling active session window of 15 turns in SQLite checkpoints,
    and dispatches older turns silently to LanceDB hierarchical memory.
    """
    @classmethod
    def _dispatch_background_summarization(cls, messages: list, workspace: Optional[str] = None, thread_id: Optional[str] = None):
        if not thread_id or not messages:
            return

        with _summarizing_lock:
            if thread_id in _active_summarizing_threads:
                return
            _active_summarizing_threads.add(thread_id)

        def _worker():
            try:
                from any_context.memory.manager import MemoryManager
                mgr = MemoryManager()
                mgr.process_session_messages(messages, workspace=workspace, thread_id=thread_id)
            except Exception:
                pass
            finally:
                with _summarizing_lock:
                    _active_summarizing_threads.discard(thread_id)

        t = threading.Thread(target=_worker, daemon=True, name=f"mem-summary-{str(thread_id)[:16]}")
        t.start()

    def delete_thread(self, thread_id: str):
        """Safely purges all checkpoint rows for a specific thread_id."""
        try:
            if hasattr(self, "conn") and self.conn:
                cursor = self.conn.cursor()
                cursor.execute("DELETE FROM checkpoints WHERE thread_id = ?", (thread_id,))
                try:
                    cursor.execute("DELETE FROM checkpoint_blobs WHERE thread_id = ?", (thread_id,))
                except Exception:
                    pass
                try:
                    cursor.execute("DELETE FROM checkpoint_writes WHERE thread_id = ?", (thread_id,))
                except Exception:
                    pass
                self.conn.commit()
        except Exception:
            pass

    def get_tuple(self, config):
        try:
            tup = super().get_tuple(config)
            if tup and hasattr(tup, "checkpoint") and isinstance(tup.checkpoint, dict):
                msgs = tup.checkpoint.get("channel_values", {}).get("messages")
                if msgs and isinstance(msgs, list):
                    # 1. Rolling session cascade: maintain strictly MAX_ACTIVE_SESSION_TURNS in checkpoints.db
                    human_indices = []
                    for idx, m in enumerate(msgs):
                        m_type = getattr(m, "type", "")
                        if m_type in ["human", "user"] or m.__class__.__name__ == "HumanMessage" or (isinstance(m, dict) and m.get("role") in ["user", "human"]):
                            human_indices.append(idx)

                    if len(human_indices) > MAX_ACTIVE_SESSION_TURNS:
                        cutoff_idx = human_indices[-MAX_ACTIVE_SESSION_TURNS]
                        older_msgs = msgs[:cutoff_idx]
                        msgs = msgs[cutoff_idx:]

                        conf = config.get("configurable", {}) if config else {}
                        thread_id = conf.get("thread_id")
                        workspace = conf.get("active_workspace") or conf.get("workspace")
                        if not workspace and thread_id and thread_id.startswith("rpc_session_"):
                            workspace = thread_id[len("rpc_session_"):]

                        # Dispatch older turns to LanceDB Level 2/3 silently in daemon background thread
                        if older_msgs and thread_id:
                            self._dispatch_background_summarization(older_msgs, workspace=workspace, thread_id=thread_id)

                    # Ensure epistemic_state is stamped on loaded AI messages
                    from any_context.core.epistemic import classify_epistemic_state
                    for m in msgs:
                        m_type = getattr(m, "type", "")
                        if m_type in ["ai", "AIMessage", "assistant"] or m.__class__.__name__ in ["AIMessage", "AIMessageChunk"]:
                            if hasattr(m, "additional_kwargs") and not getattr(m, "tool_calls", None):
                                if not m.additional_kwargs.get("epistemic_state"):
                                    m.additional_kwargs["epistemic_state"] = classify_epistemic_state(getattr(m, "content", "")).value

                    _prune_historical_tool_messages(msgs)
                    tup.checkpoint["channel_values"]["messages"] = sanitize_conversation_messages(msgs)
            return tup
        except Exception:
            try:
                thread_id = config.get("configurable", {}).get("thread_id") if config else None
                if thread_id:
                    self.delete_thread(thread_id)
            except Exception:
                pass
            return None

    def list(self, config=None, *, filter=None, before=None, limit=None):
        try:
            for item in super().list(config, filter=filter, before=before, limit=limit):
                yield item
        except Exception:
            return

    def get_delta_channel_history(self, *, config, channels):
        try:
            return super().get_delta_channel_history(config=config, channels=channels)
        except Exception:
            return {ch: {"writes": []} for ch in channels}


_global_checkpointer = None

def get_safe_checkpointer():
    """
    Returns a resilient checkpoint saver with automatic corruption detection,
    database healing, and fallback to MemorySaver if SQLite checkpoint decompressing fails.
    """
    global _global_checkpointer
    if _global_checkpointer is not None:
        return _global_checkpointer

    try:
        user_home = os.path.expanduser("~/.anycontext/memory")
        os.makedirs(user_home, exist_ok=True)
        db_path = os.path.join(user_home, "checkpoints.db")
        conn = sqlite3.connect(db_path, check_same_thread=False)
        saver_inst = ResilientSqliteSaver(conn=conn)
        saver_inst.setup()
        _global_checkpointer = saver_inst
        return _global_checkpointer
    except Exception:
        try:
            local_mem = os.path.abspath("./memory")
            os.makedirs(local_mem, exist_ok=True)
            db_path = os.path.join(local_mem, "checkpoints.db")
            conn = sqlite3.connect(db_path, check_same_thread=False)
            saver_inst = ResilientSqliteSaver(conn=conn)
            saver_inst.setup()
            _global_checkpointer = saver_inst
            return _global_checkpointer
        except Exception:
            _global_checkpointer = MemorySaver()
            return _global_checkpointer

# Backward compatible module export
saver = get_safe_checkpointer()

def create_anycontext_agent(
    active_workspace: str = None, 
    checkpointer=None,
    model_override: str = None,
    provider_override: str = None,
    grounding_mode: str = None,
    web_search_enabled: bool = None,
    **kwargs
):
    """
    Dynamically creates an AnyContext AI Agent with temperature=0.0 for deterministic RAG synthesis,
    active workspace context awareness, on-the-fly model switching, grounding mode directives, and fresh configuration.
    """
    from any_context.core.models_catalog import infer_provider_for_model, normalize_model_id

    # Support flexible alias arguments across all adapters (CLI, TUI, RPC, REST, MCP)
    active_workspace = active_workspace or kwargs.get("workspace_name") or kwargs.get("workspace")
    if active_workspace:
        from any_context.tools.search_tools import set_active_workspace_context
        set_active_workspace_context(active_workspace)
        try:
            from any_context.config.db_store import ConfigDBStore
            ConfigDBStore().set_active_workspace(active_workspace)
        except Exception:
            pass

    model_override = model_override or kwargs.get("model_name") or kwargs.get("model")
    provider_override = provider_override or kwargs.get("provider") or kwargs.get("model_provider")

    settings = AppSettings.load()
    default_provider = settings.models.model_provider if settings else "openai"
    default_model = normalize_model_id(settings.models.inference_model if (settings and settings.models and settings.models.inference_model) else "gpt-4o-mini")
    base_url = settings.models.local_base_url if settings else "http://localhost:1234/v1"

    inference_model = normalize_model_id(model_override or default_model)
    model_provider = provider_override or infer_provider_for_model(inference_model, fallback_provider=default_provider)

    api_key = get_api_key(provider=model_provider)
    if not api_key:
        api_key = "sk-placeholder" if model_provider == "openai" else "lm-studio"

    init_kwargs = {
        "model": inference_model,
        "model_provider": model_provider,
        "api_key": api_key,
        "max_retries": 5
    }

    # Standard models use temperature=0.0 for deterministic RAG; reasoning models (o1/o3/gpt-5/sonnet-5) reject custom temperature
    is_reasoning = any(r in inference_model.lower() for r in ["o1-", "o1", "o3-", "o3", "reasoner", "gpt-5", "claude-sonnet-5", "claude-opus-5"])
    if not is_reasoning and model_provider in ["openai", "anthropic", "google_genai", "groq", "mistral", "local"]:
        init_kwargs["temperature"] = 0.0

    # Route provider base URLs when switching on the fly
    if model_provider in ["local", "lm-studio", "ollama"]:
        init_kwargs["base_url"] = base_url or "http://localhost:1234/v1"
    elif model_provider == "deepseek":
        init_kwargs["base_url"] = "https://api.deepseek.com/v1"
        init_kwargs["model_provider"] = "openai" # langchain deepseek uses openai-compatible client
    elif model_provider == "groq":
        init_kwargs["base_url"] = "https://api.groq.com/openai/v1"
        init_kwargs["model_provider"] = "openai"
    elif model_provider == "xai":
        init_kwargs["base_url"] = "https://api.x.ai/v1"
        init_kwargs["model_provider"] = "openai"
    elif model_provider == "openrouter":
        init_kwargs["base_url"] = "https://openrouter.ai/api/v1"
        init_kwargs["model_provider"] = "openai"
    elif model_provider == "mistral":
        init_kwargs["base_url"] = "https://api.mistral.ai/v1"
        init_kwargs["model_provider"] = "openai"
    elif model_provider in ["google_genai", "gemini"]:
        init_kwargs["model_provider"] = "google_genai"

    # Resolve grounding mode and web search status if not explicitly passed
    if grounding_mode is None:
        try:
            store = ConfigDBStore()
            grounding_mode = store.get_grounding_mode(workspace_name=active_workspace)
        except Exception:
            grounding_mode = "strict"

    if web_search_enabled is None:
        try:
            store = ConfigDBStore()
            web_search_enabled = store.get_web_search_status(workspace_name=active_workspace)
        except Exception:
            web_search_enabled = False

    raw_model = init_chat_model(**init_kwargs)
    model = PruningChatModelWrapper(
        raw_model,
        active_workspace=active_workspace,
        grounding_mode=grounding_mode,
        web_search_enabled=web_search_enabled
    )

    system_prompt = get_system_prompt(
        active_workspace=active_workspace,
        grounding_mode=grounding_mode,
        web_search_enabled=web_search_enabled
    )

    tools = [search_db, add_web_source, list_web_sources, remove_web_source, index_folder]
    if web_search_enabled:
        tools.append(live_web_search)

    chk = checkpointer if checkpointer is not None else get_safe_checkpointer()
    try:
        return create_agent(
            model=model,
            system_prompt=system_prompt,
            tools=tools,
            checkpointer=chk
        )
    except Exception:
        # If checkpointer threw zlib or database error, fall back cleanly to MemorySaver
        from langgraph.checkpoint.memory import MemorySaver
        return create_agent(
            model=model,
            system_prompt=system_prompt,
            tools=tools,
            checkpointer=MemorySaver()
        )

class LazyAgentProxy:
    def __init__(self, checkpointer=saver):
        self.checkpointer = checkpointer

    def stream(self, input_data, stream_mode="messages", config=None):
        active_ws = config.get("configurable", {}).get("active_workspace") if config else None
        model_override = config.get("configurable", {}).get("model") or config.get("configurable", {}).get("model_override") if config else None
        grounding_override = config.get("configurable", {}).get("grounding_mode") or config.get("configurable", {}).get("mode") if config else None
        web_search_override = config.get("configurable", {}).get("web_search_enabled") if config else None
        agent_inst = create_anycontext_agent(
            active_workspace=active_ws, 
            checkpointer=self.checkpointer,
            model_override=model_override,
            grounding_mode=grounding_override,
            web_search_enabled=web_search_override
        )
        return agent_inst.stream(input_data, stream_mode=stream_mode, config=config)

    def invoke(self, input_data, config=None):
        active_ws = config.get("configurable", {}).get("active_workspace") if config else None
        model_override = config.get("configurable", {}).get("model") or config.get("configurable", {}).get("model_override") if config else None
        grounding_override = config.get("configurable", {}).get("grounding_mode") or config.get("configurable", {}).get("mode") if config else None
        web_search_override = config.get("configurable", {}).get("web_search_enabled") if config else None
        agent_inst = create_anycontext_agent(
            active_workspace=active_ws, 
            checkpointer=self.checkpointer,
            model_override=model_override,
            grounding_mode=grounding_override,
            web_search_enabled=web_search_override
        )
        return agent_inst.invoke(input_data, config=config)

# Lazy global exports (instantiated at runtime when called)
agent = LazyAgentProxy(checkpointer=None)
cli_agent = LazyAgentProxy(checkpointer=saver)

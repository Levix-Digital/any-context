import os
import sys
import uuid
from typing import Optional, List, Dict, Any
from pydantic import BaseModel, Field

from fastapi import FastAPI, HTTPException, BackgroundTasks
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import RedirectResponse

from any_context import __version__
from any_context.config.db_store import ConfigDBStore
from any_context.core.agent import cli_agent
from any_context.tools.search_tools import search_db
from any_context.ingestion.local_folder_ingestor import index_folder
from any_context.memory import MemoryManager

# --- Pydantic Schemas ---

class HealthResponse(BaseModel):
    status: str = "ok"
    version: str = __version__
    company: str = "Levix Digital"
    description: str = "AnyContext Universal AI Context Server"

class WorkspaceDTO(BaseModel):
    name: str
    paths: List[str]

class WorkspacesResponse(BaseModel):
    total: int
    workspaces: List[WorkspaceDTO]

class ChatRequest(BaseModel):
    message: str = Field(..., description="User query or instruction for the AI agent")
    workspace: Optional[str] = Field(None, description="Target workspace name (optional)")
    thread_id: Optional[str] = Field(None, description="Session thread ID for conversation context continuity")

class ChatResponse(BaseModel):
    thread_id: str
    workspace: Optional[str]
    reply: str

class SearchRequest(BaseModel):
    query: str = Field(..., description="Search query string")
    workspace: Optional[str] = Field(None, description="Workspace to filter document search")

class SearchResponse(BaseModel):
    query: str
    workspace: Optional[str]
    results: str

class IndexRequest(BaseModel):
    workspace: Optional[str] = Field(None, description="Workspace name to re-index. If omitted, indexes all workspaces.")

class IndexResponse(BaseModel):
    status: str
    message: str
    workspace: Optional[str]

class MemoryResetRequest(BaseModel):
    workspace: Optional[str] = Field(None, description="Workspace name to reset long-term memory. If omitted, resets all memory.")

class MemoryResetResponse(BaseModel):
    status: str
    deleted_entries: int
    workspace: Optional[str]

# --- FastAPI App Factory ---

def create_app() -> FastAPI:
    app = FastAPI(
        title="AnyContext Universal AI Server",
        description="REST API Server exposing RAG vector search, isolated workspaces, and 3-level long-term memory for external applications.",
        version=__version__,
        docs_url="/docs",
        redoc_url="/redoc"
    )

    # Enable CORS for external web dashboards, VS Code extensions, and local apps
    app.add_middleware(
        CORSMiddleware,
        allow_origins=["*"],
        allow_credentials=True,
        allow_methods=["*"],
        allow_headers=["*"],
    )

    @app.get("/", include_in_schema=False)
    def root():
        return RedirectResponse(url="/docs")

    @app.get("/v1/health", response_model=HealthResponse, tags=["System"])
    def get_health():
        return HealthResponse()

    @app.get("/v1/workspaces", response_model=WorkspacesResponse, tags=["Workspaces"])
    def list_workspaces():
        store = ConfigDBStore()
        settings = store.get_app_settings()
        if not settings or not settings.workspaces:
            return WorkspacesResponse(total=0, workspaces=[])
        
        dto_list = [WorkspaceDTO(name=ws.name, paths=ws.paths) for ws in settings.workspaces]
        return WorkspacesResponse(total=len(dto_list), workspaces=dto_list)

    @app.post("/v1/chat", response_model=ChatResponse, tags=["AI Agent"])
    def chat_with_agent(req: ChatRequest):
        if not req.message.strip():
            raise HTTPException(status_code=400, detail="Message cannot be empty.")

        thread_id = req.thread_id or f"api_chat_{uuid.uuid4()}"
        config = {
            "configurable": {
                "thread_id": thread_id,
                "active_workspace": req.workspace
            }
        }

        try:
            full_response = ""
            for token, metadata in cli_agent.stream(
                {"messages": [req.message]},
                stream_mode="messages",
                config=config
            ):
                if hasattr(token, "type") and token.type in ["ai", "AIMessageChunk", "AIMessage"]:
                    if isinstance(token.content, str) and token.content:
                        full_response += token.content

            return ChatResponse(
                thread_id=thread_id,
                workspace=req.workspace,
                reply=full_response.strip()
            )
        except Exception as e:
            raise HTTPException(status_code=500, detail=f"Agent execution error: {str(e)}")

    @app.post("/v1/search", response_model=SearchResponse, tags=["Knowledge Base"])
    def search_knowledge_base(req: SearchRequest):
        if not req.query.strip():
            raise HTTPException(status_code=400, detail="Query cannot be empty.")

        try:
            results = search_db.invoke({
                "query": req.query,
                "workspace": req.workspace,
                "search_session_memory": False
            })
            return SearchResponse(
                query=req.query,
                workspace=req.workspace,
                results=str(results)
            )
        except Exception as e:
            raise HTTPException(status_code=500, detail=f"Vector search error: {str(e)}")

    @app.post("/v1/index", response_model=IndexResponse, tags=["Knowledge Base"])
    def trigger_indexing(req: IndexRequest, background_tasks: BackgroundTasks):
        try:
            background_tasks.add_task(index_folder.invoke, {"workspace_name": req.workspace})
            msg = f"Re-indexing started in background for workspace '{req.workspace}'." if req.workspace else "Re-indexing started in background for all workspaces."
            return IndexResponse(status="accepted", message=msg, workspace=req.workspace)
        except Exception as e:
            raise HTTPException(status_code=500, detail=f"Indexing trigger error: {str(e)}")

    @app.post("/v1/reset-memory", response_model=MemoryResetResponse, tags=["Memory"])
    def reset_long_term_memory(req: MemoryResetRequest):
        try:
            memory_mgr = MemoryManager()
            deleted = memory_mgr.reset_memory(workspace=req.workspace)
            return MemoryResetResponse(
                status="success",
                deleted_entries=deleted,
                workspace=req.workspace
            )
        except Exception as e:
            raise HTTPException(status_code=500, detail=f"Memory reset error: {str(e)}")

    return app

def start_api_server(host: str = "127.0.0.1", port: int = 8000):
    """
    Launches the Uvicorn ASGI Web Server for AnyContext REST API
    """
    import uvicorn
    print("\n=======================================================")
    print(f"🚀 AnyContext REST API Server v{__version__} - Levix Digital")
    print(f"🌐 Server running at: http://{host}:{port}")
    print(f"📚 Interactive Swagger Docs: http://{host}:{port}/docs")
    print("=======================================================\n")
    uvicorn.run(create_app(), host=host, port=port, log_level="info")

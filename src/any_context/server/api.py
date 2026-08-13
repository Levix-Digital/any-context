import os
import sys
import uuid
from typing import Optional, List, Dict, Any
from pydantic import BaseModel, Field

from fastapi import FastAPI, HTTPException, BackgroundTasks, Depends, Header
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import RedirectResponse
from fastapi.security import HTTPBearer, HTTPAuthorizationCredentials

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
    security_auth: str = "RBAC Token & User Authentication Enabled"

class AuthStatusResponse(BaseModel):
    admin_configured: bool
    security_enforced: bool
    mode: str

class AdminSetupRequest(BaseModel):
    name: str = Field(..., description="Administrator full name (e.g. 'Dr. Silva')")
    email: str = Field(..., description="Administrator email address")
    password: str = Field(..., description="Administrator password")

class LoginRequest(BaseModel):
    email: str = Field(..., description="User email address")
    password: str = Field(..., description="User password")

class UserDTO(BaseModel):
    user_id: str
    email: str
    name: str
    role: str
    allowed_workspaces: List[str]
    created_at: Optional[str] = None
    token_id: Optional[str] = None

class UserCreateRequest(BaseModel):
    name: str = Field(..., description="User full name (e.g. 'Dra. Amanda')")
    email: str = Field(..., description="User email address")
    password: str = Field(..., description="User password")
    role: str = Field("analyst", description="Role: 'admin', 'analyst', or 'viewer'")
    allowed_workspaces: List[str] = Field(default_factory=lambda: ["Default"], description="Allowed workspace names")

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

class TokenCreateRequest(BaseModel):
    name: str = Field(..., description="Token descriptive name (e.g. 'HR Bot', 'Dev Team')")
    role: str = Field("viewer", description="Role level: 'admin', 'analyst', or 'viewer'")
    allowed_workspaces: List[str] = Field(default_factory=lambda: ["*"], description="List of allowed workspace names or ['*'] for all")

class TokenResponse(BaseModel):
    token_id: str
    user_id: Optional[str] = "system"
    name: str
    role: str
    allowed_workspaces: List[str]
    created_at: str

# --- Security Dependency ---

security = HTTPBearer(auto_error=False)

def verify_token_access(
    credentials: Optional[HTTPAuthorizationCredentials] = Depends(security),
    required_role: Optional[str] = None,
    required_workspace: Optional[str] = None
):
    """
    Verifies HTTP Bearer token against stored SQLite access tokens.
    In REST API Server Mode, if no Admin account is configured yet, access to server endpoints is BLOCKED
    until the initial Administrator is created via POST /v1/auth/setup-admin.
    """
    store = ConfigDBStore()
    admin_cfg = store.is_admin_configured()
    active_tokens = store.get_access_tokens()

    # Block server endpoints if no Admin is configured yet
    if not admin_cfg and not active_tokens:
        raise HTTPException(
            status_code=401,
            detail="Server Security Setup Required: An Administrator account must be initialized via 'POST /v1/auth/setup-admin' or CLI '/config' before accessing server endpoints."
        )

    if not credentials or not credentials.credentials:
        raise HTTPException(
            status_code=401,
            detail="Authentication required. Provide 'Authorization: Bearer <actx_sec_...>' header."
        )

    token_str = credentials.credentials
    valid = store.validate_token_permissions(
        token_id=token_str,
        required_role=required_role,
        required_workspace=required_workspace
    )

    if not valid:
        raise HTTPException(
            status_code=403,
            detail="Access Denied: Invalid security token, insufficient role level, or workspace scope restricted."
        )

    return token_str


# --- FastAPI App Factory ---

def create_app() -> FastAPI:
    description_md = """
### 🏢 AnyContext Universal AI Context Server

Welcome to the **AnyContext REST API**. This server exposes RAG vector search, isolated workspaces, 3-level long-term memory, and **RBAC User Authentication & Access Control**.

#### 🔐 Authentication & Role-Based Access Control (RBAC)
- **Open Local Mode (Default)**: Zero friction for personal single-user mode. No password required.
- **Enterprise / Multi-User Mode**: Setup Admin at `/v1/auth/setup-admin`. Log in at `/v1/auth/login` to obtain Bearer Tokens (`actx_sec_...`).
- **Workspace Scopes**: Restrict users/tokens to specific workspace scopes (`"Finance"`, `"HR"`, etc.) or roles (`"admin"`, `"analyst"`, `"viewer"`).
"""

    app = FastAPI(
        title="AnyContext Universal AI Server",
        description=description_md,
        version=__version__,
        docs_url="/docs",
        redoc_url="/redoc"
    )

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

    # --- Authentication Endpoints ---

    @app.get("/v1/auth/status", response_model=AuthStatusResponse, tags=["Authentication & Security"])
    def get_auth_status():
        store = ConfigDBStore()
        admin_cfg = store.is_admin_configured()
        has_tokens = len(store.get_access_tokens()) > 0
        enforced = admin_cfg or has_tokens
        mode = "Enterprise / Multi-User Protected Mode" if enforced else "Open Local Mode (Personal / Friction-Free)"
        return AuthStatusResponse(
            admin_configured=admin_cfg,
            security_enforced=enforced,
            mode=mode
        )

    @app.post("/v1/auth/setup-admin", response_model=UserDTO, tags=["Authentication & Security"])
    def setup_admin_user(req: AdminSetupRequest):
        """Initial Admin setup wizard for first-time server security deployment."""
        store = ConfigDBStore()
        if store.is_admin_configured():
            raise HTTPException(status_code=400, detail="Admin user is already configured.")

        try:
            admin_info = store.setup_admin_user(name=req.name, email=req.email, password=req.password)
            return UserDTO(
                user_id=admin_info["user_id"],
                email=admin_info["email"],
                name=admin_info["name"],
                role=admin_info["role"],
                allowed_workspaces=admin_info["allowed_workspaces"],
                token_id=admin_info["token"]["token_id"]
            )
        except Exception as e:
            raise HTTPException(status_code=500, detail=f"Admin setup failed: {str(e)}")

    @app.post("/v1/auth/login", response_model=UserDTO, tags=["Authentication & Security"])
    def login_user(req: LoginRequest):
        """Authenticates user credentials and returns active Bearer Token."""
        store = ConfigDBStore()
        user_info = store.authenticate_user(email=req.email, password=req.password)
        if not user_info:
            raise HTTPException(status_code=401, detail="Invalid email or password.")

        return UserDTO(
            user_id=user_info["user_id"],
            email=user_info["email"],
            name=user_info["name"],
            role=user_info["role"],
            allowed_workspaces=user_info["allowed_workspaces"],
            token_id=user_info["token_id"]
        )

    # --- User Management Endpoints ---

    @app.get("/v1/users", response_model=List[UserDTO], tags=["User Management"])
    def list_users(credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_role="admin")
        store = ConfigDBStore()
        users = store.list_users()
        return [UserDTO(**u) for u in users]

    @app.post("/v1/users", response_model=UserDTO, tags=["User Management"])
    def create_team_user(req: UserCreateRequest, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_role="admin")
        store = ConfigDBStore()
        try:
            new_u = store.create_user(
                name=req.name,
                email=req.email,
                password=req.password,
                role=req.role,
                allowed_workspaces=req.allowed_workspaces
            )
            return UserDTO(**new_u)
        except Exception as e:
            raise HTTPException(status_code=400, detail=f"User creation failed: {str(e)}")

    @app.delete("/v1/users/{user_id}", tags=["User Management"])
    def delete_user(user_id: str, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_role="admin")
        store = ConfigDBStore()
        deleted = store.delete_user(user_id)
        if not deleted:
            raise HTTPException(status_code=404, detail="User not found.")
        return {"status": "success", "message": f"User '{user_id}' has been removed."}

    # --- Access Token Management Endpoints ---

    @app.get("/v1/tokens", response_model=List[TokenResponse], tags=["Access Tokens & Security"])
    def list_access_tokens(credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_role="admin")
        store = ConfigDBStore()
        tokens = store.get_access_tokens()
        return [TokenResponse(**t) for t in tokens]

    @app.post("/v1/tokens", response_model=TokenResponse, tags=["Access Tokens & Security"])
    def create_access_token(req: TokenCreateRequest, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_role="admin")
        store = ConfigDBStore()
        new_token = store.create_access_token(
            name=req.name,
            role=req.role,
            allowed_workspaces=req.allowed_workspaces
        )
        return TokenResponse(**new_token)

    @app.delete("/v1/tokens/{token_id}", tags=["Access Tokens & Security"])
    def delete_access_token(token_id: str, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_role="admin")
        store = ConfigDBStore()
        deleted = store.delete_access_token(token_id)
        if not deleted:
            raise HTTPException(status_code=404, detail="Security token not found.")
        return {"status": "success", "message": f"Security token '{token_id}' has been revoked."}

    # --- Core Application Endpoints ---

    @app.get("/v1/workspaces", response_model=WorkspacesResponse, tags=["Workspaces"])
    def list_workspaces(credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials)
        store = ConfigDBStore()
        settings = store.get_app_settings()
        if not settings or not settings.workspaces:
            return WorkspacesResponse(total=0, workspaces=[])
        
        dto_list = [WorkspaceDTO(name=ws.name, paths=ws.paths) for ws in settings.workspaces]
        return WorkspacesResponse(total=len(dto_list), workspaces=dto_list)

    @app.post("/v1/chat", response_model=ChatResponse, tags=["AI Agent"])
    def chat_with_agent(req: ChatRequest, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_workspace=req.workspace)

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
    def search_knowledge_base(req: SearchRequest, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_workspace=req.workspace)

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
    def trigger_indexing(req: IndexRequest, background_tasks: BackgroundTasks, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_role="analyst", required_workspace=req.workspace)
        try:
            background_tasks.add_task(index_folder.invoke, {"workspace_name": req.workspace})
            msg = f"Re-indexing started in background for workspace '{req.workspace}'." if req.workspace else "Re-indexing started in background for all workspaces."
            return IndexResponse(status="accepted", message=msg, workspace=req.workspace)
        except Exception as e:
            raise HTTPException(status_code=500, detail=f"Indexing trigger error: {str(e)}")

    @app.post("/v1/reset-memory", response_model=MemoryResetResponse, tags=["Memory"])
    def reset_long_term_memory(req: MemoryResetRequest, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_role="analyst", required_workspace=req.workspace)
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

    @app.post("/v1/factory-reset", tags=["System"])
    def perform_factory_reset(credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_role="admin")
        try:
            store = ConfigDBStore()
            store.factory_reset()
            return {"status": "success", "message": "AnyContext has been reset to factory defaults."}
        except Exception as e:
            raise HTTPException(status_code=500, detail=f"Factory reset error: {str(e)}")

    @app.get("/v1/docs/readme", tags=["System"])
    def get_system_readme():
        """Returns the full official AnyContext application documentation (README.md) as raw Markdown."""
        readme_candidates = [
            os.path.join(os.getcwd(), "README.md"),
            os.path.join(os.path.dirname(__file__), "..", "config", "README.md"),
            os.path.join(os.path.dirname(__file__), "..", "..", "..", "README.md")
        ]
        for cand in readme_candidates:
            if os.path.exists(cand):
                try:
                    with open(cand, "r", encoding="utf-8") as f:
                        return {"version": __version__, "content": f.read()}
                except Exception:
                    pass
        raise HTTPException(status_code=404, detail="System documentation file (README.md) not found.")

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
    if host == "0.0.0.0":
        print(f"🔒 VPC Enterprise Mode Enabled: Listening on all internal network interfaces.")
    print("=======================================================\n")
    uvicorn.run(create_app(), host=host, port=port, log_level="info")

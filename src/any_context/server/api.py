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
from any_context.core.agent import create_anycontext_agent, saver

from any_context.tools.search_tools import search_db
from any_context.ingestion.local_folder_ingestor import index_folder
from any_context.memory import MemoryManager
from any_context.workspace_sharing import WorkspaceSharingStore, WorkspaceSharingManager
from any_context.billing import BillingManager, get_all_plans, get_plan_by_id



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

class TransferSourceRequest(BaseModel):
    source_workspace: str = Field(..., description="Origin workspace name")
    target_workspace: str = Field(..., description="Destination workspace name")
    source_type: str = Field("folder", description="Type of source: 'folder' or 'web'")
    source_path_or_url: str = Field(..., description="Absolute folder path (e.g. 'C:\\Docs') or website URL (e.g. 'https://canada.ca')")

class TransferSourceResponse(BaseModel):
    status: str = "success"
    source_workspace: str
    target_workspace: str
    source_type: str
    source_path_or_url: str
    transferred_chunks: int = 0
    transferred_pages: int = 0
    api_embedding_cost: str = "$0.00"
    message: str

class ChatRequest(BaseModel):
    message: str = Field(..., description="User query or instruction for the AI agent")
    workspace: Optional[str] = Field(None, description="Target workspace name (optional)")
    thread_id: Optional[str] = Field(None, description="Session thread ID for conversation context continuity")
    model: Optional[str] = Field(None, description="Optional inference model override on-the-fly (e.g. 'gpt-4o', 'claude-3-5-sonnet-20241022', 'deepseek-chat')")

class ChatResponse(BaseModel):
    thread_id: str
    workspace: Optional[str]
    model_used: str
    reply: str

class ModelDTO(BaseModel):
    id: str
    name: str
    provider: str

class AvailableModelsResponse(BaseModel):
    active_default: str
    available_models: List[ModelDTO]

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

class ShareInviteCreateRequest(BaseModel):
    workspace_name: str = Field(..., description="Workspace name to share")
    access_level: str = Field("viewer", description="Access level: 'viewer' (chat/search) or 'editor' (chat/search + add folders)")
    max_uses: int = Field(1, description="Max usage count (1 for single use, 0 for unlimited)")

class ShareInviteAcceptRequest(BaseModel):
    invite_code: str = Field(..., description="Workspace invite code (e.g. 'SHARE-WKS-1234')")
    user_email: str = Field(..., description="User email accepting the invite")

class AddFolderRequest(BaseModel):
    folder_path: str = Field(..., description="Absolute path of the local folder to add")
    user_email: str = Field(..., description="Email of the user adding the folder")

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

    @app.post("/v1/workspaces", response_model=WorkspaceDTO, tags=["Workspaces"])
    def create_workspace_endpoint(name: str, paths: Optional[List[str]] = None, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Creates a new workspace with optional local folder paths (empty by default)."""
        verify_token_access(credentials=credentials, required_role="analyst")
        clean_name = name.strip()
        if not clean_name:
            raise HTTPException(status_code=400, detail="Workspace name cannot be empty.")
        store = ConfigDBStore()
        clean_paths = paths or []
        store.add_workspace(clean_name, clean_paths)
        return WorkspaceDTO(name=clean_name, paths=clean_paths)

    @app.post("/v1/workspaces/transfer", response_model=TransferSourceResponse, tags=["Workspaces"])
    def transfer_workspace_source_endpoint(req: TransferSourceRequest, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Transfers a local folder or web portal and its vector embeddings from source_workspace to target_workspace in sub-50ms with zero API cost ($0.00)."""
        verify_token_access(credentials=credentials, required_role="analyst")
        src_ws = req.source_workspace.strip()
        tgt_ws = req.target_workspace.strip()
        src_type = req.source_type.strip().lower()
        src_item = req.source_path_or_url.strip()

        if not src_ws or not tgt_ws or not src_item:
            raise HTTPException(status_code=400, detail="source_workspace, target_workspace, and source_path_or_url are required.")

        if src_ws == tgt_ws:
            raise HTTPException(status_code=400, detail="Source and target workspaces cannot be the same.")

        store = ConfigDBStore()
        from any_context.ingestion.web_scheduler import WebSchedulerStore
        web_store = WebSchedulerStore()

        if src_type in ["web", "url", "site", "portal"] or src_item.startswith("http://") or src_item.startswith("https://"):
            res = web_store.transfer_web_source(source_ws=src_ws, target_ws=tgt_ws, url_or_root=src_item)
            if not res.get("success"):
                raise HTTPException(status_code=400, detail=res.get("error", "Web source transfer failed."))
            return TransferSourceResponse(
                status="success",
                source_workspace=src_ws,
                target_workspace=tgt_ws,
                source_type="web",
                source_path_or_url=res.get("url", src_item),
                transferred_chunks=res.get("transferred_chunks", 0),
                transferred_pages=res.get("transferred_pages", 0),
                api_embedding_cost="$0.00",
                message=f"Web source '{src_item}' successfully transferred from '{src_ws}' to '{tgt_ws}' in < 50ms with zero API cost ($0.00)."
            )
        else:
            res = store.transfer_local_folder_source(source_ws=src_ws, target_ws=tgt_ws, folder_path=src_item)
            if not res.get("success"):
                raise HTTPException(status_code=400, detail=res.get("error", "Local folder transfer failed."))
            return TransferSourceResponse(
                status="success",
                source_workspace=src_ws,
                target_workspace=tgt_ws,
                source_type="folder",
                source_path_or_url=res.get("folder_path", src_item),
                transferred_chunks=res.get("transferred_chunks", 0),
                transferred_pages=0,
                api_embedding_cost="$0.00",
                message=f"Local folder '{src_item}' successfully transferred from '{src_ws}' to '{tgt_ws}' in < 50ms with zero API cost ($0.00)."
            )

    @app.get("/v1/models", response_model=AvailableModelsResponse, tags=["AI Models"])
    def list_available_models_endpoint():
        """Lists available inference models based on configured and validated API keys."""
        from any_context.core.models_catalog import get_available_models
        store = ConfigDBStore()
        settings = store.get_app_settings()
        active_default = settings.models.inference_model if (settings and settings.models) else "gpt-4o-mini"
        models = get_available_models()
        return AvailableModelsResponse(
            active_default=active_default,
            available_models=[ModelDTO(**m) for m in models]
        )

    @app.post("/v1/chat", response_model=ChatResponse, tags=["AI Agent"])
    def chat_with_agent(req: ChatRequest, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_workspace=req.workspace)

        if not req.message.strip():
            raise HTTPException(status_code=400, detail="Message cannot be empty.")

        effective_model = req.model
        if req.model:
            from any_context.core.models_catalog import validate_model_key_availability
            is_valid, prov, err_msg = validate_model_key_availability(req.model)
            if not is_valid:
                raise HTTPException(status_code=400, detail=err_msg)
        else:
            store = ConfigDBStore()
            settings = store.get_app_settings()
            effective_model = settings.models.inference_model if (settings and settings.models) else "gpt-4o-mini"

        thread_id = req.thread_id or f"api_chat_{uuid.uuid4()}"
        config = {
            "configurable": {
                "thread_id": thread_id,
                "active_workspace": req.workspace
            }
        }

        try:
            full_response = ""
            agent_instance = create_anycontext_agent(
                active_workspace=req.workspace, 
                checkpointer=saver,
                model_override=effective_model
            )
            for token, metadata in agent_instance.stream(
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
                model_used=effective_model,
                reply=full_response.strip()
            )
        except Exception as e:
            from any_context.core.models_catalog import format_inference_error
            err_info = format_inference_error(e, effective_model)
            raise HTTPException(
                status_code=400 if "not_found" in str(e).lower() or "404" in str(e) else 500,
                detail={
                    "error": err_info["title"],
                    "model": effective_model,
                    "cause": err_info["cause"],
                    "suggested_action": err_info["action"]
                }
            )

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
    # --- Workspace Collaboration & Sharing Endpoints ---

    @app.post("/v1/workspaces/share/invite", tags=["Workspace Sharing"])
    def create_workspace_share_invite(req: ShareInviteCreateRequest, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        token = verify_token_access(credentials=credentials, required_workspace=req.workspace_name)
        try:
            store = WorkspaceSharingStore()
            invite = store.create_share_invite(
                workspace_name=req.workspace_name,
                access_level=req.access_level,
                created_by_email=token or "admin@local",
                max_uses=req.max_uses
            )
            return {
                "status": "success",
                "invite_code": invite.invite_code,
                "workspace_name": invite.workspace_name,
                "access_level": invite.access_level,
                "max_uses": invite.max_uses
            }
        except Exception as e:
            raise HTTPException(status_code=500, detail=f"Error creating workspace invite: {str(e)}")

    @app.post("/v1/workspaces/share/accept", tags=["Workspace Sharing"])
    def accept_workspace_share_invite(req: ShareInviteAcceptRequest, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials)
        try:
            store = WorkspaceSharingStore()
            perm = store.accept_share_invite(invite_code=req.invite_code, user_email=req.user_email)
            return {
                "status": "success",
                "message": f"Successfully joined workspace '{perm.workspace_name}' as '{perm.access_level.upper()}'!",
                "workspace_name": perm.workspace_name,
                "access_level": perm.access_level
            }
        except ValueError as ve:
            raise HTTPException(status_code=400, detail=str(ve))
        except Exception as e:
            raise HTTPException(status_code=500, detail=f"Error accepting workspace invite: {str(e)}")

    @app.get("/v1/workspaces/{workspace_name}/collaborators", tags=["Workspace Sharing"])
    def list_workspace_collaborators(workspace_name: str, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_workspace=workspace_name)
        try:
            store = WorkspaceSharingStore()
            collaborators = store.list_workspace_collaborators(workspace_name)
            return {"workspace_name": workspace_name, "collaborators": [c.dict() for c in collaborators]}
        except Exception as e:
            raise HTTPException(status_code=500, detail=f"Error listing collaborators: {str(e)}")

    @app.get("/v1/workspaces/{workspace_name}/folders", tags=["Workspace Sharing"])
    def list_workspace_folders_transparent(workspace_name: str, user_email: Optional[str] = "admin@local", credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_workspace=workspace_name)
        try:
            mgr = WorkspaceSharingManager()
            transparent_folders = mgr.get_transparent_folders_view(workspace_name=workspace_name, current_user_email=user_email or "admin@local")
            return {"workspace_name": workspace_name, "folders": transparent_folders}
        except Exception as e:
            raise HTTPException(status_code=500, detail=f"Error retrieving workspace folders: {str(e)}")

    @app.post("/v1/workspaces/{workspace_name}/folders", tags=["Workspace Sharing"])
    def add_folder_to_workspace(workspace_name: str, req: AddFolderRequest, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_workspace=workspace_name)
        try:
            mgr = WorkspaceSharingManager()
            if not mgr.can_add_folder(user_email=req.user_email, workspace_name=workspace_name):
                raise HTTPException(status_code=403, detail="Access Denied: Read-only 'Viewer' role cannot add folders to this workspace.")

            entry = mgr.store.add_workspace_folder(
                workspace_name=workspace_name,
                folder_path=req.folder_path,
                added_by_email=req.user_email
            )
            return {"status": "success", "folder": entry.dict()}
        except HTTPException:
            raise
        except Exception as e:
            raise HTTPException(status_code=500, detail=f"Error adding folder: {str(e)}")

    @app.delete("/v1/workspaces/{workspace_name}/folders/{folder_id}", tags=["Workspace Sharing"])
    def delete_workspace_folder(workspace_name: str, folder_id: str, user_email: Optional[str] = "admin@local", credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_workspace=workspace_name)
        try:
            store = WorkspaceSharingStore()
            deleted = store.delete_workspace_folder(folder_id=folder_id, user_email=user_email or "admin@local")
            if not deleted:
                raise HTTPException(status_code=403, detail="Access Denied: You can only remove folders that you added to this workspace.")
            return {"status": "success", "message": f"Folder {folder_id} removed from workspace {workspace_name}."}
        except HTTPException:
            raise
        except Exception as e:
            raise HTTPException(status_code=500, detail=f"Error deleting folder: {str(e)}")

    # --- Billing & Subscription Endpoints ---

    @app.get("/v1/billing/plans", tags=["Billing & Subscriptions"])
    def get_subscription_plans():
        """Returns the complete list of AnyContext subscription tiers, pricing, and capability matrix."""
        mgr = BillingManager()
        plans = get_all_plans()
        return {
            "plans": [p.dict() for p in plans],
            "pricing_table_markdown": mgr.format_pricing_table_markdown()
        }

    @app.get("/v1/billing/status", tags=["Billing & Subscriptions"])
    def get_subscription_status():
        """Returns the active subscription tier status and feature capabilities."""
        mgr = BillingManager()
        return mgr.get_status().dict()

    @app.post("/v1/billing/license", tags=["Billing & Subscriptions"])
    def set_subscription_license(tier_id: str, license_key: Optional[str] = None, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Activates or updates the active subscription plan tier (Admin only)."""
        verify_token_access(credentials=credentials)
        mgr = BillingManager()
        status = mgr.store.set_active_tier(tier_id=tier_id, license_key=license_key)
        return {"status": "success", "subscription": status.dict()}

    # --- Web Scraping & Polling Endpoints ---

    @app.get("/v1/workspaces/{workspace_name}/web-urls", tags=["Web Scraping & Ingestion"])
    def list_workspace_web_urls(workspace_name: str, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Lists all web URLs registered for background scraping & polling in a workspace."""
        verify_token_access(credentials=credentials, required_workspace=workspace_name)
        from any_context.ingestion.web_scheduler import WebSchedulerStore
        store = WebSchedulerStore()
        urls = store.get_workspace_web_urls(workspace_name)
        return {"workspace_name": workspace_name, "web_urls": urls}

    @app.post("/v1/workspaces/{workspace_name}/web-urls", tags=["Web Scraping & Ingestion"])
    def add_workspace_web_url(workspace_name: str, url: str, background_tasks: BackgroundTasks, polling_interval_hours: int = 24, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Adds a web URL to a workspace and triggers background web scraping & ChromaDB indexing."""
        verify_token_access(credentials=credentials, required_workspace=workspace_name)
        from any_context.billing import BillingManager
        b_mgr = BillingManager()
        if not b_mgr.can_ingest_source("web"):
            raise HTTPException(status_code=403, detail="Access Denied: Web Scraping requires 'Pro', 'Team', or 'Enterprise' plan tier.")

        from any_context.ingestion.web_scheduler import WebSchedulerStore, index_web_url_to_chromadb
        store = WebSchedulerStore()
        entry = store.add_web_url(workspace_name=workspace_name, url=url, polling_interval_hours=polling_interval_hours)
        background_tasks.add_task(index_web_url_to_chromadb, workspace_name, url, entry["id"])
        return {"status": "success", "message": f"Web URL '{url}' registered. Background scraping initiated.", "entry": entry}

    @app.delete("/v1/workspaces/{workspace_name}/web-urls/{url_id}", tags=["Web Scraping & Ingestion"])
    def delete_workspace_web_url(workspace_name: str, url_id: str, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Deletes a web URL from a workspace and purges its indexed vectors from ChromaDB."""
        verify_token_access(credentials=credentials, required_workspace=workspace_name)
        from any_context.ingestion.web_scheduler import WebSchedulerStore, remove_web_url_from_chromadb
        store = WebSchedulerStore()
        entry = store.get_web_url_by_id(url_id)
        if not entry or entry.get("workspace_name") != workspace_name:
            raise HTTPException(status_code=404, detail="Web URL not found in workspace.")
        
        store.delete_web_url(url_id, workspace_name=workspace_name)
        remove_web_url_from_chromadb(workspace_name=workspace_name, url=entry["url"])
        return {"status": "success", "message": f"Web URL '{entry['url']}' removed and vectors purged."}

    @app.post("/v1/workspaces/{workspace_name}/web-urls/sync", tags=["Web Scraping & Ingestion"])
    def sync_workspace_web_urls_endpoint(workspace_name: str, background_tasks: BackgroundTasks, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Triggers synchronization / re-scraping for all registered web URLs in a workspace."""
        verify_token_access(credentials=credentials, required_workspace=workspace_name)
        from any_context.billing import BillingManager
        b_mgr = BillingManager()
        if not b_mgr.can_ingest_source("web"):
            raise HTTPException(status_code=403, detail="Access Denied: Web Scraping requires 'Pro', 'Team', or 'Enterprise' plan tier.")

        from any_context.ingestion.web_scheduler import sync_workspace_web_urls
        background_tasks.add_task(sync_workspace_web_urls, workspace_name)
        return {"status": "success", "message": f"Web URLs synchronization initiated for workspace '{workspace_name}'."}

    # --- OCR Image Ingestion Endpoints ---

    @app.post("/v1/ingest/ocr", tags=["OCR Image Ingestion"])
    def parse_image_ocr(workspace_name: str, image_path: str, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Parses an image or scanned document file using OCR and indexes extracted text to ChromaDB."""
        verify_token_access(credentials=credentials, required_workspace=workspace_name)
        from any_context.billing import BillingManager
        b_mgr = BillingManager()
        if not b_mgr.can_use_ocr():
            raise HTTPException(status_code=403, detail="Access Denied: Image & Scanned PDF OCR requires 'Starter', 'Pro', 'Team', or 'Enterprise' plan tier.")

        from any_context.ingestion.image_ocr_ingestor import extract_text_from_image, index_image_file_to_chromadb
        try:
            indexed = index_image_file_to_chromadb(workspace_name=workspace_name, image_path=image_path)
            data = extract_text_from_image(image_path)
            return {"status": "success", "indexed": indexed, "ocr_data": data}
        except Exception as e:
            raise HTTPException(status_code=500, detail=f"Error parsing image OCR: {str(e)}")

    return app






def start_api_server(host: str = "127.0.0.1", port: int = 8000):
    """
    Launches the Uvicorn ASGI Web Server for AnyContext REST API
    """
    from any_context.billing import BillingManager
    b_mgr = BillingManager()
    status = b_mgr.get_status()

    if not status.capabilities.supports_server_mode:
        print("\n=======================================================")
        print("🔒 AnyContext REST API Server Mode (REST / Multi-Tenant)")
        print("=======================================================")
        print("⚠️ Server Mode requires a 'Pro', 'Team', or 'Enterprise' License Key.")
        print(f"   Current Active Plan: \033[93m{status.active_tier_name}\033[0m")
        print("\n👉 To activate Server Mode, add your license key to your .env file:")
        print("   \033[96mANYCONTEXT_LICENSE_KEY=actx_ent_your_license_here\033[0m")
        print("\n👉 Or run '\033[93m/billing\033[0m' inside the chat to view plans & upgrade.")
        print("=======================================================\n")
        return

    import uvicorn
    print("\n=======================================================")
    print(f"🚀 AnyContext REST API Server v{__version__} - Levix Digital")
    print(f"🔑 Active License Tier: \033[92m{status.active_tier_name}\033[0m")
    print(f"🌐 Server running at: http://{host}:{port}")
    print(f"📚 Interactive Swagger Docs: http://{host}:{port}/docs")
    if host == "0.0.0.0":
        print(f"🔒 VPC Enterprise Mode Enabled: Listening on all internal network interfaces.")
    print("=======================================================\n")
    uvicorn.run(create_app(), host=host, port=port, log_level="info")

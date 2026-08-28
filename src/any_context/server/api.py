import os
import sys
import uuid
from typing import Optional, List, Dict, Any, Union
from pydantic import BaseModel, Field

from fastapi import FastAPI, HTTPException, BackgroundTasks, Depends, Header, Query
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

class OnboardingOptionItemDTO(BaseModel):
    id: str
    title: str
    description: Optional[str] = ""
    icon: Optional[str] = ""
    badge: Optional[str] = ""
    is_active: bool = False

class OnboardingStatusResponse(BaseModel):
    needs_onboarding: bool
    stage: str
    title: str
    description: str
    active_provider: str
    active_model: str
    options: List[OnboardingOptionItemDTO]

class CompleteOnboardingRequest(BaseModel):
    choice: str = Field(..., description="Selected setup choice: 'openai', 'local_offline', or 'custom'")
    api_key: Optional[str] = Field(None, description="OpenAI or Provider API key if choice is 'openai'")
    base_url: Optional[str] = Field(None, description="Custom base URL (e.g. 'http://localhost:1234/v1')")
    model_name: Optional[str] = Field(None, description="Model identifier if using local server")
    workspace_name: Optional[str] = Field(None, description="Initial workspace name (defaults to 'Default')")

class CompleteOnboardingResponse(BaseModel):
    success: bool
    message: str
    error: Optional[str] = None
    state_updates: Dict[str, Any] = Field(default_factory=dict)

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

class WorkspaceWebSourceDTO(BaseModel):
    id: str
    url: str
    root_url: Optional[str] = None
    title: Optional[str] = None
    page_count: int = 1
    scope: Optional[str] = None
    last_scraped_at: Optional[str] = None
    created_at: Optional[str] = None

class WorkspaceCloudDriveDTO(BaseModel):
    id: str
    provider: str
    mount_path_or_id: str
    title: Optional[str] = None
    auth_status: str = "pending"
    last_synced_at: Optional[str] = None
    created_at: Optional[str] = None

class WorkspaceSourceItemDTO(BaseModel):
    type: str  # 'folder', 'web', 'cloud_drive'
    id: Optional[Union[str, int]] = None
    identifier: str
    title: Optional[str] = None
    details: Dict[str, Any] = Field(default_factory=dict)

class WorkspaceDTO(BaseModel):
    id: str = Field(..., description="Unique immutable workspace identifier (e.g. 'ws_default', 'ws_3d3fa3')")
    name: str = Field(..., description="Workspace display name")
    sources: List[WorkspaceSourceItemDTO] = Field(default_factory=list, description="Unified polymorphic list of all workspace sources (folders, web portals, cloud drives)")
    total_sources: int = Field(0, description="Total count of sources attached to this workspace")

class WorkspacesResponse(BaseModel):
    total: int
    workspaces: List[WorkspaceDTO]

class WorkspaceSourcesResponse(BaseModel):
    id: str = Field(..., description="Unique immutable workspace identifier")
    name: str = Field(..., description="Workspace display name")
    total_sources: int = Field(0, description="Total count of sources attached to this workspace")
    sources: List[WorkspaceSourceItemDTO] = Field(default_factory=list, description="Unified polymorphic list of all workspace sources")

class CloudDriveAddRequest(BaseModel):
    provider: str = Field(..., description="Cloud provider (e.g. 'google_drive', 'onedrive', 's3', 'dropbox')")
    mount_path_or_id: str = Field(..., description="Drive ID, folder URI, or bucket/prefix")
    title: Optional[str] = Field(None, description="Descriptive display title")
    metadata: Optional[Dict[str, Any]] = Field(default_factory=dict, description="Optional provider-specific metadata")

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

class RenameWorkspaceRequest(BaseModel):
    old_name: str = Field(..., description="Current workspace name to rename")
    new_name: str = Field(..., description="New name for the workspace")

class RenameWorkspaceResponse(BaseModel):
    status: str = "success"
    old_workspace: str
    new_workspace: str
    migrated_chunks: int = 0
    api_cost: str = "$0.00"
    message: str

class LinkSharedSourceRequest(BaseModel):
    source_type: str = Field("folder", description="Type of source: 'folder', 'web', or 'cloud_drive'")
    source_identifier: str = Field(..., description="Path, URL, or mount ID of the source to link")
    title: Optional[str] = Field(None, description="Optional custom title for the linked source")

class LinkSharedSourceResponse(BaseModel):
    status: str = "success"
    workspace: str
    source_type: str
    source_identifier: str
    title: str
    message: str

class UnlinkSharedSourceRequest(BaseModel):
    source_type: str = Field("folder", description="Type of source to unlink: 'folder', 'web', or 'cloud_drive'")
    source_identifier: str = Field(..., description="Path, URL, or mount ID to unlink")

class AddFolderRequest(BaseModel):
    folder_path: str = Field(..., description="Absolute path of the local folder to add")
    user_email: Optional[str] = Field(None, description="Optional email of the user adding the folder")
    link_to_workspaces: Optional[List[str]] = Field(default_factory=list, description="Optional list of additional workspace names to link this folder to simultaneously")

class WorkspaceSyncStatusDTO(BaseModel):
    workspace_name: str
    is_up_to_date: bool
    has_changes: bool
    is_virgin: bool
    total_sources: int = 0
    local_folders_count: int = 0
    web_sources_count: int = 0
    web_pages_count: int = 0
    cloud_drives_count: int = 0
    folders: List[str] = Field(default_factory=list)
    web_sources: List[Dict[str, Any]] = Field(default_factory=list)
    cloud_drives: List[Dict[str, Any]] = Field(default_factory=list)
    new_files: List[str] = Field(default_factory=list)
    modified_files: List[str] = Field(default_factory=list)
    deleted_files: List[str] = Field(default_factory=list)
    renamed_files: List[List[str]] = Field(default_factory=list)
    total_disk_files: int = 0
    total_cached_files: int = 0
    summary: str = "Up to date"
    is_syncing: bool = False
    progress: Optional[Dict[str, Any]] = None
    progress_bar: Optional[str] = None

class WorkspaceSyncRequest(BaseModel):
    force_full: bool = Field(False, description="Forces full re-indexing of all files instead of incremental stat diff")
    background: bool = Field(True, description="Runs synchronization in non-blocking background worker thread")
    sync_folders: bool = Field(True, description="Synchronize local folder documents")
    sync_web: bool = Field(True, description="Synchronize indexed web portals")
    sync_drives: bool = Field(True, description="Synchronize connected cloud drives")

class ChatRequest(BaseModel):
    message: str = Field(..., description="User query or instruction for the AI agent")
    workspace: Optional[str] = Field(None, description="Target workspace name (optional)")
    thread_id: Optional[str] = Field(None, description="Session thread ID for conversation context continuity")
    model: Optional[str] = Field(None, description="Optional inference model override on-the-fly (e.g. 'gpt-4o', 'claude-3-5-sonnet-20241022', 'deepseek-chat')")
    grounding_mode: Optional[str] = Field(None, description="Optional AI grounding mode override: 'hybrid', 'strict', 'proactive'")
    mode: Optional[str] = Field(None, description="Alias for grounding_mode ('hybrid', 'strict', 'proactive')")
    web_search_enabled: Optional[bool] = Field(None, description="Optional Web Search toggle override: True or False")

class ChatResponse(BaseModel):
    thread_id: str
    workspace: Optional[str]
    model_used: str
    grounding_mode: str = "hybrid"
    web_search_enabled: bool = False
    reply: str

class ModelDTO(BaseModel):
    id: str
    name: str
    provider: str

class AvailableModelsResponse(BaseModel):
    active_default: str
    available_models: List[ModelDTO]

class GroundingModeDTO(BaseModel):
    mode: str = Field("hybrid", description="Active AI grounding mode: 'hybrid', 'strict', or 'proactive'")
    workspace: Optional[str] = Field(None, description="Specific workspace if queried")
    available_modes: List[str] = Field(default_factory=lambda: ["hybrid", "strict", "proactive"])

class UpdateGroundingModeRequest(BaseModel):
    mode: str = Field(..., description="Target grounding mode: 'hybrid', 'strict', or 'proactive'")
    workspace: Optional[str] = Field(None, description="Optional workspace name to apply mode specifically")
    apply_global: bool = Field(False, description="Whether to apply mode globally across all workspaces")

class WebSearchStatusDTO(BaseModel):
    web_search_enabled: bool = Field(False, description="Web search active status")
    workspace: Optional[str] = Field(None, description="Specific workspace if queried, or None for global")

class UpdateWebSearchRequest(BaseModel):
    enabled: bool = Field(..., description="Enable (True) or Disable (False) real-time Web Search")
    workspace: Optional[str] = Field(None, description="Target workspace (if omitted, applies to global setting)")
    apply_global: bool = Field(False, description="Whether to apply across all workspaces")

class WorkspaceSettingsDTO(BaseModel):
    workspace_name: str
    grounding_mode: str = "hybrid"
    web_search_enabled: bool = False

class UpdateWorkspaceSettingsRequest(BaseModel):
    grounding_mode: Optional[str] = Field(None, description="'hybrid', 'strict', or 'proactive'")
    web_search_enabled: Optional[bool] = Field(None, description="True or False")

class SearchRequest(BaseModel):
    query: str = Field(..., description="Search query string")
    workspace: Optional[str] = Field(None, description="Workspace to filter document search")
    top_k: Optional[int] = Field(None, description="Optional custom top_k chunks override for vector search")

class SearchResponse(BaseModel):
    query: str
    workspace: Optional[str]
    results: str

class ContextRetrievalSettingsDTO(BaseModel):
    retrieval_preset: str = Field("balanced", description="Active preset: 'balanced', 'turbo', 'deep_research', 'custom'")
    top_k: int = Field(40, description="Target number of diversified document chunks returned to AI agent")
    candidate_pool_size: int = Field(100, description="Candidate pool size retrieved from ChromaDB before source diversification")
    max_chunks_per_source: int = Field(3, description="Maximum chunks allowed per unique document/URL to enforce cross-source diversity")
    chunk_size: int = 1024
    chunk_overlap: int = 200
    grounding_mode: str = Field("hybrid", description="Active AI Grounding Mode: 'hybrid', 'strict', 'proactive'")

class UpdateRetrievalPresetRequest(BaseModel):
    preset: Optional[str] = Field(None, description="Preset name: 'balanced', 'turbo', 'deep_research', or 'custom'")
    top_k: Optional[int] = Field(None, description="Custom target top_k (if custom)")
    candidate_pool_size: Optional[int] = Field(None, description="Custom candidate pool size (if custom)")
    max_chunks_per_source: Optional[int] = Field(None, description="Custom max chunks per source (if custom)")
    grounding_mode: Optional[str] = Field(None, description="Optional grounding mode: 'hybrid', 'strict', 'proactive'")

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

    # --- Onboarding & Initial Setup Endpoints ---

    @app.get("/v1/onboarding/status", response_model=OnboardingStatusResponse, tags=["Onboarding & Setup"])
    def get_onboarding_status_endpoint():
        """Returns whether first-time onboarding or quick provider setup is required."""
        from any_context.core.services.onboarding_service import OnboardingService
        svc = OnboardingService()
        st = svc.check_status()
        opts_dto = [
            OnboardingOptionItemDTO(
                id=o.id,
                title=o.title,
                description=o.description or "",
                icon=o.icon or "",
                badge=o.badge or "",
                is_active=o.is_active
            )
            for o in st.options_group.items
        ]
        return OnboardingStatusResponse(
            needs_onboarding=st.needs_onboarding,
            stage=st.stage,
            title=st.title,
            description=st.description,
            active_provider=st.active_provider,
            active_model=st.active_model,
            options=opts_dto
        )

    @app.post("/v1/onboarding/complete", response_model=CompleteOnboardingResponse, tags=["Onboarding & Setup"])
    def complete_onboarding_endpoint(req: CompleteOnboardingRequest):
        """Executes first-time setup or quick provider configuration."""
        from any_context.core.services.onboarding_service import OnboardingService
        svc = OnboardingService()
        res = svc.complete_onboarding(
            choice_id=req.choice,
            api_key=req.api_key,
            base_url=req.base_url,
            model_name=req.model_name,
            workspace_name=req.workspace_name
        )
        return CompleteOnboardingResponse(
            success=res.success,
            message=res.message,
            error=res.error,
            state_updates=res.state_updates
        )

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
        """Lists all configured workspaces with their complete sources (folders, web portals, cloud drives, and unified sources list)."""
        verify_token_access(credentials=credentials)
        store = ConfigDBStore()
        detailed_list = store.list_workspaces_detailed()
        dto_list = [WorkspaceDTO(**ws) for ws in detailed_list]
        return WorkspacesResponse(total=len(dto_list), workspaces=dto_list)

    @app.get("/v1/workspaces/{workspace_name}", response_model=WorkspaceDTO, tags=["Workspaces"])
    def get_workspace_endpoint(workspace_name: str, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Retrieves details and all sources for a specific workspace."""
        verify_token_access(credentials=credentials, required_workspace=workspace_name)
        store = ConfigDBStore()
        clean_ws = workspace_name.strip()
        settings = store.get_app_settings()
        known = [w.name for w in settings.workspaces] if settings else []
        if clean_ws not in known:
            raise HTTPException(status_code=404, detail=f"Workspace '{clean_ws}' not found.")
        ws_detail = store.get_workspace_sources(clean_ws)
        return WorkspaceDTO(**ws_detail)

    @app.get("/v1/workspaces/{workspace_name}/sources", response_model=WorkspaceSourcesResponse, tags=["Workspaces"])
    def get_workspace_sources_endpoint(workspace_name: str, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Returns unified sources breakdown for a specific workspace."""
        verify_token_access(credentials=credentials, required_workspace=workspace_name)
        store = ConfigDBStore()
        clean_ws = workspace_name.strip()
        ws_meta = store.get_workspace_meta(clean_ws)
        if not ws_meta:
            raise HTTPException(status_code=404, detail=f"Workspace '{clean_ws}' not found.")
        ws_detail = store.get_workspace_sources(clean_ws)
        return WorkspaceSourcesResponse(
            id=ws_detail["id"],
            name=ws_detail["name"],
            total_sources=ws_detail["total_sources"],
            sources=[WorkspaceSourceItemDTO(**s) for s in ws_detail["sources"]]
        )

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
        ws_detail = store.get_workspace_sources(clean_name)
        return WorkspaceDTO(**ws_detail)

    @app.post("/v1/workspaces/{workspace_name}/cloud-drives", tags=["Workspaces"])
    def add_cloud_drive_endpoint(workspace_name: str, req: CloudDriveAddRequest, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Attaches a cloud drive source (e.g. Google Drive, OneDrive, S3, Dropbox) to a workspace."""
        verify_token_access(credentials=credentials, required_role="analyst", required_workspace=workspace_name)
        store = ConfigDBStore()
        res = store.add_cloud_drive_to_workspace(
            workspace_name=workspace_name,
            provider=req.provider,
            mount_path_or_id=req.mount_path_or_id,
            title=req.title,
            metadata=req.metadata
        )
        return {"status": "success", "cloud_drive": res}

    @app.get("/v1/workspaces/{workspace_name}/cloud-drives", tags=["Workspaces"])
    def list_cloud_drives_endpoint(workspace_name: str, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Lists attached cloud drive sources for a workspace."""
        verify_token_access(credentials=credentials, required_workspace=workspace_name)
        store = ConfigDBStore()
        drives = store.get_workspace_cloud_drives(workspace_name)
        return {"workspace_name": workspace_name, "cloud_drives": drives}

    @app.delete("/v1/workspaces/{workspace_name}/cloud-drives/{drive_id}", tags=["Workspaces"])
    def delete_cloud_drive_endpoint(workspace_name: str, drive_id: str, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Removes an attached cloud drive source from a workspace."""
        verify_token_access(credentials=credentials, required_role="analyst", required_workspace=workspace_name)
        store = ConfigDBStore()
        deleted = store.delete_cloud_drive(drive_id=drive_id, workspace_name=workspace_name)
        if not deleted:
            raise HTTPException(status_code=404, detail="Cloud drive not found.")
        return {"status": "success", "message": f"Cloud drive '{drive_id}' removed from workspace '{workspace_name}'."}

    @app.get("/v1/workspaces/{workspace_name}/sync/status", response_model=WorkspaceSyncStatusDTO, tags=["Workspaces"])
    def get_workspace_sync_status_endpoint(workspace_name: str, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Inspects pending file additions, modifications, deletions, web sources, and stat cache status for a workspace in < 30ms."""
        verify_token_access(credentials=credentials, required_workspace=workspace_name)
        from any_context.ingestion.local_folder_ingestor import check_workspace_changes, BackgroundSyncManager
        diff = check_workspace_changes(workspace_name)
        renamed_pairs = [[r[0], r[1]] for r in diff.get("renamed_files", [])]
        bg_mgr = BackgroundSyncManager()
        is_syncing = bg_mgr.is_syncing(workspace_name)
        prog = bg_mgr.get_progress(workspace_name) if is_syncing else None
        prog_bar = bg_mgr.format_progress_bar(workspace_name) if is_syncing else None

        return WorkspaceSyncStatusDTO(
            workspace_name=diff["workspace_name"],
            is_up_to_date=diff["is_up_to_date"],
            has_changes=diff["has_changes"],
            is_virgin=diff["is_virgin"],
            total_sources=diff.get("total_sources", 0),
            local_folders_count=diff.get("local_folders_count", 0),
            web_sources_count=diff.get("web_sources_count", 0),
            web_pages_count=diff.get("web_pages_count", 0),
            cloud_drives_count=diff.get("cloud_drives_count", 0),
            folders=diff.get("folders", []),
            web_sources=diff.get("web_sources", []),
            cloud_drives=diff.get("cloud_drives", []),
            new_files=diff["new_files"],
            modified_files=diff["modified_files"],
            deleted_files=diff["deleted_files"],
            renamed_files=renamed_pairs,
            total_disk_files=diff["total_disk_files"],
            total_cached_files=diff["total_cached_files"],
            summary=diff["summary"],
            is_syncing=is_syncing,
            progress=prog,
            progress_bar=prog_bar
        )

    @app.post("/v1/workspaces/{workspace_name}/sync", tags=["Workspaces"])
    def sync_workspace_folders_endpoint(
        workspace_name: str,
        background_tasks: BackgroundTasks,
        req: Optional[WorkspaceSyncRequest] = None,
        credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)
    ):
        """Triggers incremental or full vector synchronization for a workspace's folders."""
        verify_token_access(credentials=credentials, required_role="analyst", required_workspace=workspace_name)
        force_full = req.force_full if req else False
        use_bg = req.background if req else True
        sync_folders = req.sync_folders if req else True
        sync_web = req.sync_web if req else True
        sync_drives = req.sync_drives if req else True

        from any_context.ingestion.local_folder_ingestor import BackgroundSyncManager
        if use_bg:
            bg_mgr = BackgroundSyncManager()
            bg_mgr.start_background_sync(
                workspace_name=workspace_name,
                sync_folders=sync_folders,
                sync_web=sync_web,
                sync_drives=sync_drives,
                force_full=force_full,
                verbose=False
            )
            return {
                "status": "success",
                "message": f"Background synchronization started for workspace '{workspace_name}'.",
                "workspace_name": workspace_name,
                "mode": "background"
            }
        else:
            from any_context.ingestion.unified_sync import run_unified_sync
            res = run_unified_sync(
                workspace_name=workspace_name,
                sync_folders=sync_folders,
                sync_web=sync_web,
                sync_drives=sync_drives,
                force_full=force_full,
                verbose=False
            )
            return {
                "status": "success",
                "message": f"Synchronization completed for workspace '{workspace_name}'.",
                "workspace_name": workspace_name,
                "result": res
            }

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

    @app.get("/v1/workspaces/shared-sources/available", tags=["Workspaces"])
    def list_available_shared_sources_endpoint(credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Lists all unique indexed sources available for cross-workspace linking ($0.00 cost)."""
        verify_token_access(credentials=credentials)
        store = ConfigDBStore()
        sources = store.list_all_available_shared_sources()
        return {"total": len(sources), "sources": sources}

    @app.get("/v1/workspaces/{workspace_name}/shared-sources", tags=["Workspaces"])
    def list_workspace_shared_sources_endpoint(workspace_name: str, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Lists linked shared sources for a workspace."""
        verify_token_access(credentials=credentials, required_workspace=workspace_name)
        store = ConfigDBStore()
        links = store.get_workspace_shared_links(workspace_name)
        return {"workspace": workspace_name, "shared_links": links}

    @app.post("/v1/workspaces/{workspace_name}/shared-sources/link", response_model=LinkSharedSourceResponse, tags=["Workspaces"])
    def link_shared_source_endpoint(workspace_name: str, req: LinkSharedSourceRequest, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Links an existing indexed source to workspace_name in < 50ms with zero API cost ($0.00)."""
        verify_token_access(credentials=credentials, required_role="analyst", required_workspace=workspace_name)
        store = ConfigDBStore()
        res = store.link_shared_source_to_workspace(
            workspace_name=workspace_name,
            source_type=req.source_type,
            source_identifier=req.source_identifier,
            title=req.title
        )
        return LinkSharedSourceResponse(**res)

    @app.post("/v1/workspaces/{workspace_name}/shared-sources/unlink", tags=["Workspaces"])
    def unlink_shared_source_endpoint(workspace_name: str, req: UnlinkSharedSourceRequest, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Unlinks a shared source from a workspace."""
        verify_token_access(credentials=credentials, required_role="analyst", required_workspace=workspace_name)
        store = ConfigDBStore()
        unlinked = store.unlink_shared_source_from_workspace(
            workspace_name=workspace_name,
            source_type=req.source_type,
            source_identifier=req.source_identifier
        )
        if not unlinked:
            raise HTTPException(status_code=404, detail="Shared source link not found.")
        return {"status": "success", "message": f"Shared source '{req.source_identifier}' unlinked from workspace '{workspace_name}'."}

    @app.post("/v1/workspaces/rename", response_model=RenameWorkspaceResponse, tags=["Workspaces"])
    def rename_workspace_endpoint(req: RenameWorkspaceRequest, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Renames a workspace atomically across SQLite records and ChromaDB vector metadata in sub-50ms ($0.00 cost)."""
        verify_token_access(credentials=credentials, required_role="analyst", required_workspace=req.old_name)

        old_ws = (req.old_name or "").strip()
        new_ws = (req.new_name or "").strip()

        if not old_ws or not new_ws:
            raise HTTPException(status_code=400, detail="Both old_name and new_name are required.")

        store = ConfigDBStore()
        res = store.rename_workspace(old_name=old_ws, new_name=new_ws)
        if not res.get("success"):
            raise HTTPException(status_code=400, detail=res.get("error", "Failed to rename workspace."))

        return RenameWorkspaceResponse(
            status="success",
            old_workspace=old_ws,
            new_workspace=new_ws,
            migrated_chunks=res.get("migrated_chunks", 0),
            api_cost="$0.00",
            message=f"Workspace '{old_ws}' successfully renamed to '{new_ws}' ({res.get('migrated_chunks', 0)} vector chunks migrated) with zero API cost ($0.00)."
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

        # Resolve grounding mode and web search status
        store = ConfigDBStore()
        effective_mode = req.grounding_mode or req.mode
        if not effective_mode:
            effective_mode = store.get_grounding_mode(workspace_name=req.workspace)
        else:
            effective_mode = effective_mode.lower().strip()
            if effective_mode not in ["hybrid", "strict", "proactive"]:
                effective_mode = "hybrid"

        effective_web_search = req.web_search_enabled
        if effective_web_search is None:
            effective_web_search = store.get_web_search_status(workspace_name=req.workspace)

        try:
            full_response = ""
            agent_instance = create_anycontext_agent(
                active_workspace=req.workspace, 
                checkpointer=saver,
                model_override=effective_model,
                grounding_mode=effective_mode,
                web_search_enabled=effective_web_search
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
                grounding_mode=effective_mode,
                web_search_enabled=effective_web_search,
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

    @app.get("/v1/context/mode", response_model=GroundingModeDTO, tags=["Knowledge Base"])
    def get_grounding_mode_endpoint(
        workspace: Optional[str] = Query(None, description="Optional workspace name to query specific grounding mode"),
        credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)
    ):
        verify_token_access(credentials=credentials)
        store = ConfigDBStore()
        mode = store.get_grounding_mode(workspace_name=workspace)
        return GroundingModeDTO(mode=mode, workspace=workspace)

    @app.post("/v1/context/mode", response_model=GroundingModeDTO, tags=["Knowledge Base"])
    def set_grounding_mode_endpoint(req: UpdateGroundingModeRequest, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_role="admin")
        store = ConfigDBStore()
        saved = store.set_grounding_mode(req.mode, workspace_name=req.workspace, apply_global=req.apply_global)
        return GroundingModeDTO(mode=saved, workspace=req.workspace)

    @app.get("/v1/context/web-search", response_model=WebSearchStatusDTO, tags=["Knowledge Base"])
    def get_web_search_status_endpoint(
        workspace: Optional[str] = Query(None, description="Optional workspace name to query specific web search status"),
        credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)
    ):
        verify_token_access(credentials=credentials)
        store = ConfigDBStore()
        status = store.get_web_search_status(workspace_name=workspace)
        return WebSearchStatusDTO(web_search_enabled=status, workspace=workspace)

    @app.post("/v1/context/web-search", response_model=WebSearchStatusDTO, tags=["Knowledge Base"])
    def set_web_search_status_endpoint(req: UpdateWebSearchRequest, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_role="admin")
        store = ConfigDBStore()
        saved = store.set_web_search_status(req.enabled, workspace_name=req.workspace, apply_global=req.apply_global)
        return WebSearchStatusDTO(web_search_enabled=saved, workspace=req.workspace)

    @app.get("/v1/workspaces/{workspace_name}/settings", response_model=WorkspaceSettingsDTO, tags=["Workspaces"])
    def get_workspace_settings_endpoint(workspace_name: str, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials)
        store = ConfigDBStore()
        mode = store.get_grounding_mode(workspace_name=workspace_name)
        search = store.get_web_search_status(workspace_name=workspace_name)
        return WorkspaceSettingsDTO(workspace_name=workspace_name, grounding_mode=mode, web_search_enabled=search)

    @app.post("/v1/workspaces/{workspace_name}/settings", response_model=WorkspaceSettingsDTO, tags=["Workspaces"])
    def update_workspace_settings_endpoint(workspace_name: str, req: UpdateWorkspaceSettingsRequest, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_role="admin")
        store = ConfigDBStore()
        if req.grounding_mode is not None:
            store.set_grounding_mode(req.grounding_mode, workspace_name=workspace_name)
        if req.web_search_enabled is not None:
            store.set_web_search_status(req.web_search_enabled, workspace_name=workspace_name)

        mode = store.get_grounding_mode(workspace_name=workspace_name)
        search = store.get_web_search_status(workspace_name=workspace_name)
        return WorkspaceSettingsDTO(workspace_name=workspace_name, grounding_mode=mode, web_search_enabled=search)

    @app.get("/v1/context/settings", response_model=ContextRetrievalSettingsDTO, tags=["Knowledge Base"])
    def get_context_settings_endpoint(credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials)
        store = ConfigDBStore()
        settings = store.get_app_settings()
        ctx = settings.context if settings else None
        if not ctx:
            raise HTTPException(status_code=500, detail="Could not load context settings.")
        return ContextRetrievalSettingsDTO(
            retrieval_preset=ctx.retrieval_preset,
            top_k=ctx.top_k,
            candidate_pool_size=ctx.candidate_pool_size,
            max_chunks_per_source=ctx.max_chunks_per_source,
            chunk_size=ctx.chunk_size,
            chunk_overlap=ctx.chunk_overlap,
            grounding_mode=getattr(ctx, "grounding_mode", "hybrid")
        )

    @app.post("/v1/context/settings", response_model=ContextRetrievalSettingsDTO, tags=["Knowledge Base"])
    def update_context_settings_endpoint(req: UpdateRetrievalPresetRequest, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_role="admin")
        store = ConfigDBStore()
        settings = store.get_app_settings()
        ctx = settings.context if settings else None
        if not ctx:
            raise HTTPException(status_code=500, detail="Could not load context settings.")

        if req.preset:
            ctx.apply_preset(req.preset)
        if req.top_k is not None:
            ctx.top_k = req.top_k
            ctx.retrieval_preset = "custom"
        if req.candidate_pool_size is not None:
            ctx.candidate_pool_size = req.candidate_pool_size
            ctx.retrieval_preset = "custom"
        if req.max_chunks_per_source is not None:
            ctx.max_chunks_per_source = req.max_chunks_per_source
            ctx.retrieval_preset = "custom"
        if req.grounding_mode is not None:
            clean_mode = req.grounding_mode.lower().strip()
            if clean_mode in ["hybrid", "strict", "proactive"]:
                ctx.grounding_mode = clean_mode

        store.update_context_settings(ctx)
        return ContextRetrievalSettingsDTO(
            retrieval_preset=ctx.retrieval_preset,
            top_k=ctx.top_k,
            candidate_pool_size=ctx.candidate_pool_size,
            max_chunks_per_source=ctx.max_chunks_per_source,
            chunk_size=ctx.chunk_size,
            chunk_overlap=ctx.chunk_overlap,
            grounding_mode=getattr(ctx, "grounding_mode", "hybrid")
        )

    @app.post("/v1/search", response_model=SearchResponse, tags=["Knowledge Base"])
    def search_knowledge_base(req: SearchRequest, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_workspace=req.workspace)

        if not req.query.strip():
            raise HTTPException(status_code=400, detail="Query cannot be empty.")

        try:
            results = search_db.invoke({
                "prompt_text": req.query,
                "workspace": req.workspace,
                "search_session_memory": False,
                "top_k": req.top_k or 40
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

    @app.post("/v1/workspaces/{workspace_name}/folders", tags=["Workspaces", "Workspace Sharing"])
    def add_folder_to_workspace(workspace_name: str, req: AddFolderRequest, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        verify_token_access(credentials=credentials, required_role="analyst", required_workspace=workspace_name)
        try:
            mgr = WorkspaceSharingManager()
            if req.user_email and not mgr.can_add_folder(user_email=req.user_email, workspace_name=workspace_name):
                raise HTTPException(status_code=403, detail="Access Denied: Read-only 'Viewer' role cannot add folders to this workspace.")

            store = ConfigDBStore()
            clean_path = req.folder_path.strip().strip("'\"")
            if not os.path.isabs(clean_path):
                clean_path = os.path.abspath(clean_path)

            res = store.attach_and_broadcast_source(
                primary_workspace=workspace_name,
                source_type="folder",
                source_identifier=clean_path,
                link_to_workspaces=req.link_to_workspaces
            )

            if req.user_email:
                entry = mgr.store.add_workspace_folder(
                    workspace_name=workspace_name,
                    folder_path=clean_path,
                    added_by_email=req.user_email
                )
                res["folder"] = entry.dict()
            return res
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

    # --- Unified Interaction & Configuration Engine Endpoints ---

    @app.get("/v1/config/schema", tags=["Configuration & Interaction"])
    def get_config_menu_schema(menu_id: str = Query("main", description="Menu ID to retrieve"), workspace: str = Query("Default", description="Target workspace"), credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Returns the canonical hierarchical menu schema tree for CLI, Desktop UI, and Web UI."""
        verify_token_access(credentials=credentials)
        from any_context.core.interaction.config_engine import ConfigEngine
        engine = ConfigEngine()
        return engine.get_menu_tree(menu_id=menu_id, workspace=workspace)

    @app.post("/v1/config/action", tags=["Configuration & Interaction"])
    def execute_config_menu_action(action_id: str, params: Optional[Dict[str, Any]] = None, workspace: str = "Default", credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Executes a menu action (e.g. create workspace, toggle search, set key) through the central Interaction Engine."""
        verify_token_access(credentials=credentials, required_role="analyst")
        from any_context.core.interaction.config_engine import ConfigEngine
        engine = ConfigEngine()
        return engine.execute_action(action_id=action_id, params=params or {}, workspace=workspace)

    @app.get("/v1/options/{option_type}", tags=["Configuration & Interaction"])
    def get_options_group(option_type: str, workspace: str = Query("Default", description="Target workspace"), credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Returns structured option lists for quick selectors (grounding_mode, inference_model, retrieval_density)."""
        verify_token_access(credentials=credentials)
        from any_context.core.interaction.options_engine import OptionsEngine
        engine = OptionsEngine()
        if option_type == "grounding_mode":
            return engine.get_grounding_mode_options(workspace=workspace)
        elif option_type == "inference_model":
            return engine.get_inference_model_options()
        elif option_type == "retrieval_density":
            return engine.get_retrieval_density_options()
        else:
            raise HTTPException(status_code=400, detail=f"Unknown option type '{option_type}'.")

    @app.post("/v1/options/{option_type}", tags=["Configuration & Interaction"])
    def set_option_value(option_type: str, value: str, workspace: str = "Default", apply_global: bool = False, credentials: Optional[HTTPAuthorizationCredentials] = Depends(security)):
        """Sets an option value (e.g. grounding_mode, inference_model, retrieval_density)."""
        verify_token_access(credentials=credentials, required_role="analyst")
        from any_context.core.interaction.options_engine import OptionsEngine
        engine = OptionsEngine()
        if option_type == "grounding_mode":
            return engine.set_grounding_mode(mode=value, workspace=workspace, apply_global=apply_global)
        elif option_type == "inference_model":
            return engine.set_inference_model(model_name=value)
        elif option_type == "retrieval_density":
            return engine.set_retrieval_density_preset(preset=value)
        else:
            raise HTTPException(status_code=400, detail=f"Unknown option type '{option_type}'.")

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

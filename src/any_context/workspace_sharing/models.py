from typing import Optional, List
from pydantic import BaseModel, Field
from datetime import datetime

class WorkspaceFolderEntry(BaseModel):
    folder_id: str
    workspace_name: str
    folder_path: str
    added_by_email: str
    created_at: str

class WorkspacePermission(BaseModel):
    permission_id: str
    workspace_name: str
    user_email: str
    access_level: str = Field(..., description="'viewer', 'editor', or 'owner'")
    granted_by_email: str
    created_at: str

class WorkspaceShareInvite(BaseModel):
    invite_id: str
    invite_code: str
    workspace_name: str
    access_level: str = Field("viewer", description="'viewer' or 'editor'")
    created_by_email: str
    max_uses: int = Field(1, description="1 for single use, 0 for unlimited, N for group")
    current_uses: int = 0
    is_active: bool = True
    created_at: str

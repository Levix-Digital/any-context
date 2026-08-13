from typing import Optional, List, Dict
from any_context.workspace_sharing.store import WorkspaceSharingStore
from any_context.workspace_sharing.models import WorkspaceFolderEntry, WorkspacePermission

class WorkspaceSharingManager:
    def __init__(self, store: Optional[WorkspaceSharingStore] = None):
        self.store = store or WorkspaceSharingStore()

    def check_access(self, user_email: str, workspace_name: str) -> Optional[str]:
        """Returns effective access level: 'owner', 'editor', 'viewer', or None."""
        return self.store.get_user_workspace_permission(user_email, workspace_name)

    def can_chat(self, user_email: str, workspace_name: str) -> bool:
        role = self.check_access(user_email, workspace_name)
        return role in ["owner", "editor", "viewer"]

    def can_add_folder(self, user_email: str, workspace_name: str) -> bool:
        role = self.check_access(user_email, workspace_name)
        return role in ["owner", "editor"]

    def can_manage_folder(self, user_email: str, folder_id: str) -> bool:
        """Returns True if the folder was added by user_email or user is admin/owner."""
        if not user_email or user_email in ["admin", "local_owner"]:
            return True
        return self.store.delete_workspace_folder(folder_id, user_email)

    def get_transparent_folders_view(self, workspace_name: str, current_user_email: str) -> List[Dict]:
        """
        Returns all folders for the workspace formatted with transparent ownership tags:
        e.g. '[👑 Your Folder]' vs '[🔒 Read-Only (Added by Amanda)]'
        """
        folders = self.store.get_workspace_folders(workspace_name)
        results = []
        for f in folders:
            is_own = (f.added_by_email == current_user_email or current_user_email in ["admin", "local_owner"])
            tag = "👑 Your Folder - Editable" if is_own else f"🔒 Read-Only (Added by: {f.added_by_email})"
            results.append({
                "folder_id": f.folder_id,
                "workspace_name": f.workspace_name,
                "folder_path": f.folder_path,
                "added_by_email": f.added_by_email,
                "is_owner": is_own,
                "tag": tag,
                "created_at": f.created_at
            })
        return results

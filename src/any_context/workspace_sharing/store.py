import os
import sqlite3
import uuid
import json
from datetime import datetime
from typing import List, Optional, Dict
from any_context.config.app_settings import AppSettings
from any_context.config.db_store import ConfigDBStore
from any_context.workspace_sharing.models import (
    WorkspaceFolderEntry,
    WorkspacePermission,
    WorkspaceShareInvite
)

def safe_print(msg: str):
    try:
        print(msg)
    except UnicodeEncodeError:
        print(msg.encode("ascii", errors="ignore").decode("ascii"))

class WorkspaceSharingStore:
    def __init__(self, db_path: Optional[str] = None):
        self.db_path = db_path or ConfigDBStore.find_db_file("settings.db")
        self._init_db()


    def _get_connection(self):
        conn = sqlite3.connect(self.db_path)
        conn.row_factory = sqlite3.Row
        return conn

    def _init_db(self):
        with self._get_connection() as conn:
            cursor = conn.cursor()
            # 1. Workspace Folders (Track Folder Ownership)
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS workspace_folders (
                    folder_id TEXT PRIMARY KEY,
                    workspace_name TEXT NOT NULL,
                    folder_path TEXT NOT NULL,
                    added_by_email TEXT NOT NULL,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    UNIQUE(workspace_name, folder_path, added_by_email)
                );
            """)

            # 2. Workspace User Permissions (User Access Per Workspace)
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS workspace_user_permissions (
                    permission_id TEXT PRIMARY KEY,
                    workspace_name TEXT NOT NULL,
                    user_email TEXT NOT NULL,
                    access_level TEXT NOT NULL,
                    granted_by_email TEXT NOT NULL,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    UNIQUE(workspace_name, user_email)
                );
            """)


            # 3. Workspace Share Invites (Invite Links / Codes)
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS workspace_share_invites (
                    invite_id TEXT PRIMARY KEY,
                    invite_code TEXT UNIQUE NOT NULL,
                    workspace_name TEXT NOT NULL,
                    access_level TEXT NOT NULL,
                    created_by_email TEXT NOT NULL,
                    max_uses INTEGER DEFAULT 1,
                    current_uses INTEGER DEFAULT 0,
                    is_active INTEGER DEFAULT 1,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                );
            """)
            conn.commit()

    # --- Folder Management & Ownership ---

    def add_workspace_folder(self, workspace_name: str, folder_path: str, added_by_email: str) -> WorkspaceFolderEntry:
        folder_id = f"fld_{uuid.uuid4().hex[:10]}"
        abs_path = os.path.abspath(folder_path)
        now_str = datetime.utcnow().isoformat()
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT INTO workspace_folders (folder_id, workspace_name, folder_path, added_by_email, created_at)
                VALUES (?, ?, ?, ?, ?)
                ON CONFLICT(workspace_name, folder_path, added_by_email) DO UPDATE SET created_at = excluded.created_at
            """, (folder_id, workspace_name, abs_path, added_by_email, now_str))
            conn.commit()

        return WorkspaceFolderEntry(
            folder_id=folder_id,
            workspace_name=workspace_name,
            folder_path=abs_path,
            added_by_email=added_by_email,
            created_at=now_str
        )

    def get_workspace_folders(self, workspace_name: str) -> List[WorkspaceFolderEntry]:
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                SELECT folder_id, workspace_name, folder_path, added_by_email, created_at
                FROM workspace_folders
                WHERE workspace_name = ?
                ORDER BY created_at ASC
            """, (workspace_name,))
            rows = cursor.fetchall()
            return [
                WorkspaceFolderEntry(
                    folder_id=r["folder_id"],
                    workspace_name=r["workspace_name"],
                    folder_path=r["folder_path"],
                    added_by_email=r["added_by_email"],
                    created_at=str(r["created_at"])
                ) for r in rows
            ]

    def delete_workspace_folder(self, folder_id: str, user_email: str) -> bool:
        """Only the user who added the folder (or an admin) can delete it."""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                DELETE FROM workspace_folders
                WHERE folder_id = ? AND (added_by_email = ? OR added_by_email = 'admin' OR ? = 'admin@system.local')
            """, (folder_id, user_email, user_email))
            deleted = cursor.rowcount > 0
            conn.commit()
            return deleted

    # --- Workspace Share Invites ---

    def create_share_invite(self, workspace_name: str, access_level: str, created_by_email: str, max_uses: int = 1) -> WorkspaceShareInvite:
        invite_id = f"sinv_{uuid.uuid4().hex[:10]}"
        invite_code = f"SHARE-{workspace_name[:4].upper()}-{uuid.uuid4().hex[:6].upper()}"
        now_str = datetime.utcnow().isoformat()
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT INTO workspace_share_invites (invite_id, invite_code, workspace_name, access_level, created_by_email, max_uses, current_uses, is_active, created_at)
                VALUES (?, ?, ?, ?, ?, ?, 0, 1, ?)
            """, (invite_id, invite_code, workspace_name, access_level, created_by_email, max_uses, now_str))
            conn.commit()

        return WorkspaceShareInvite(
            invite_id=invite_id,
            invite_code=invite_code,
            workspace_name=workspace_name,
            access_level=access_level,
            created_by_email=created_by_email,
            max_uses=max_uses,
            current_uses=0,
            is_active=True,
            created_at=now_str
        )

    def get_share_invite(self, invite_code: str) -> Optional[WorkspaceShareInvite]:
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                SELECT invite_id, invite_code, workspace_name, access_level, created_by_email, max_uses, current_uses, is_active, created_at
                FROM workspace_share_invites
                WHERE invite_code = ? AND is_active = 1
            """, (invite_code.strip(),))
            r = cursor.fetchone()
            if not r:
                return None
            return WorkspaceShareInvite(
                invite_id=r["invite_id"],
                invite_code=r["invite_code"],
                workspace_name=r["workspace_name"],
                access_level=r["access_level"],
                created_by_email=r["created_by_email"],
                max_uses=r["max_uses"],
                current_uses=r["current_uses"],
                is_active=bool(r["is_active"]),
                created_at=str(r["created_at"])
            )

    def accept_share_invite(self, invite_code: str, user_email: str) -> WorkspacePermission:
        invite = self.get_share_invite(invite_code)
        if not invite:
            raise ValueError("Invalid or expired workspace invite code.")

        if invite.max_uses > 0 and invite.current_uses >= invite.max_uses:
            raise ValueError("Workspace invite code usage limit has been reached.")

        perm_id = f"perm_{uuid.uuid4().hex[:10]}"
        now_str = datetime.utcnow().isoformat()

        with self._get_connection() as conn:
            cursor = conn.cursor()
            # Grant permission
            cursor.execute("""
                INSERT INTO workspace_user_permissions (permission_id, workspace_name, user_email, access_level, granted_by_email, created_at)
                VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT(workspace_name, user_email) DO UPDATE SET access_level = excluded.access_level
            """, (perm_id, invite.workspace_name, user_email, invite.access_level, invite.created_by_email, now_str))

            # Increment usage count
            new_uses = invite.current_uses + 1
            is_active = 1 if (invite.max_uses == 0 or new_uses < invite.max_uses) else 0
            cursor.execute("""
                UPDATE workspace_share_invites
                SET current_uses = ?, is_active = ?
                WHERE invite_id = ?
            """, (new_uses, is_active, invite.invite_id))

            conn.commit()

        return WorkspacePermission(
            permission_id=perm_id,
            workspace_name=invite.workspace_name,
            user_email=user_email,
            access_level=invite.access_level,
            granted_by_email=invite.created_by_email,
            created_at=now_str
        )

    # --- Workspace Permissions & Collaborators ---

    def grant_direct_permission(self, workspace_name: str, user_email: str, access_level: str, granted_by_email: str) -> WorkspacePermission:
        perm_id = f"perm_{uuid.uuid4().hex[:10]}"
        now_str = datetime.utcnow().isoformat()
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT INTO workspace_user_permissions (permission_id, workspace_name, user_email, access_level, granted_by_email, created_at)
                VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT(workspace_name, user_email) DO UPDATE SET access_level = excluded.access_level
            """, (perm_id, workspace_name, user_email, access_level, granted_by_email, now_str))
            conn.commit()

        return WorkspacePermission(
            permission_id=perm_id,
            workspace_name=workspace_name,
            user_email=user_email,
            access_level=access_level,
            granted_by_email=granted_by_email,
            created_at=now_str
        )

    def get_user_workspace_permission(self, user_email: str, workspace_name: str) -> Optional[str]:
        """Returns 'owner', 'editor', 'viewer', or None."""
        if not user_email or user_email in ["admin", "local_owner"]:
            return "owner"

        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                SELECT access_level FROM workspace_user_permissions
                WHERE workspace_name = ? AND user_email = ?
            """, (workspace_name, user_email))
            r = cursor.fetchone()
            if r:
                return r["access_level"]
            return None

    def list_workspace_collaborators(self, workspace_name: str) -> List[WorkspacePermission]:
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                SELECT permission_id, workspace_name, user_email, access_level, granted_by_email, created_at
                FROM workspace_user_permissions
                WHERE workspace_name = ?
                ORDER BY created_at ASC
            """, (workspace_name,))
            rows = cursor.fetchall()
            return [
                WorkspacePermission(
                    permission_id=r["permission_id"],
                    workspace_name=r["workspace_name"],
                    user_email=r["user_email"],
                    access_level=r["access_level"],
                    granted_by_email=r["granted_by_email"],
                    created_at=str(r["created_at"])
                ) for r in rows
            ]

    def revoke_workspace_permission(self, workspace_name: str, user_email: str) -> bool:
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                DELETE FROM workspace_user_permissions
                WHERE workspace_name = ? AND user_email = ?
            """, (workspace_name, user_email))
            deleted = cursor.rowcount > 0
            conn.commit()
            return deleted

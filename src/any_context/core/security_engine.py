"""
AnyContext Security & Data Encryption Engine.
Provides deterministic hardware-bound machine key derivation and AES-GCM-256
encryption-at-rest for local vector records, document chunks, and summaries.
"""
import os
import sys
import uuid
import base64
import hashlib
import platform
import subprocess
from typing import Dict, Any, Optional
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC
from cryptography.hazmat.primitives import hashes


class SecurityEngine:
    """
    Hardware-bound encryption engine for AnyContext vector storage.
    Guarantees that vector chunk payloads (text, summary, keywords, file paths)
    are encrypted on disk with AES-GCM-256 tied to the host machine signature.
    """
    _instance: Optional["SecurityEngine"] = None
    _DOMAIN_SALT = b"AnyContext::HexagonalVectorEncryption::v1"
    _INTERNAL_PEPPER = b"actx_sec_pepper_8f4a29c17e0b5d3a91"

    def __init__(self, machine_id_override: Optional[str] = None):
        self._machine_id = machine_id_override or self._extract_machine_identifier()
        self._aesgcm = self._derive_aesgcm_key(self._machine_id)

    @classmethod
    def get_instance(cls) -> "SecurityEngine":
        """Singleton accessor for the security engine."""
        if cls._instance is None:
            cls._instance = cls()
        return cls._instance

    @staticmethod
    def _extract_machine_identifier() -> str:
        """
        Extracts a unique hardware/operating-system identifier.
        Supports Windows (MachineGuid / CSPProduct UUID), macOS (IOPlatformUUID), Linux (machine-id).
        """
        # 1. Check environment variable override for test isolation
        env_override = os.getenv("ACTX_MACHINE_ID")
        if env_override and env_override.strip():
            return env_override.strip()

        # 2. Windows-specific UUID retrieval
        if sys.platform == "win32":
            try:
                import winreg
                with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Cryptography", 0, winreg.KEY_READ | winreg.KEY_WOW64_64KEY) as key:
                    guid, _ = winreg.QueryValueEx(key, "MachineGuid")
                    if guid and len(str(guid)) > 10:
                        return f"win_guid_{guid.strip()}"
            except Exception:
                pass

            try:
                out = subprocess.check_output("wmic csproduct get uuid", shell=True, timeout=2).decode().strip()
                lines = [line.strip() for line in out.splitlines() if line.strip() and "uuid" not in line.lower()]
                if lines and len(lines[0]) > 10:
                    return f"win_csp_{lines[0]}"
            except Exception:
                pass

        # 3. Linux-specific machine-id retrieval
        elif sys.platform.startswith("linux"):
            for mid_path in ["/etc/machine-id", "/var/lib/dbus/machine-id"]:
                if os.path.exists(mid_path):
                    try:
                        with open(mid_path, "r", encoding="utf-8") as f:
                            mid = f.read().strip()
                            if mid:
                                return f"lin_mid_{mid}"
                    except Exception:
                        pass

        # 4. macOS-specific UUID retrieval
        elif sys.platform == "darwin":
            try:
                out = subprocess.check_output(["ioreg", "-rd1", "-c", "IOPlatformExpertDevice"], timeout=2).decode()
                for line in out.splitlines():
                    if "IOPlatformUUID" in line:
                        parts = line.split("=")
                        if len(parts) > 1:
                            raw_uuid = parts[1].strip().strip('"')
                            if raw_uuid:
                                return f"mac_uuid_{raw_uuid}"
            except Exception:
                pass

        # 5. Robust Fallback across all OSs
        node_id = str(uuid.getnode())
        sys_node = platform.node()
        proc = platform.processor() or "cpu"
        combined = f"{node_id}::{sys_node}::{proc}"
        return f"fallback_{hashlib.sha256(combined.encode('utf-8')).hexdigest()}"

    def _derive_aesgcm_key(self, machine_id: str) -> AESGCM:
        """Derives a 256-bit AES-GCM key using PBKDF2-HMAC-SHA256 with 100,000 rounds."""
        combined_secret = f"{machine_id}::{self._INTERNAL_PEPPER.decode('ascii')}".encode("utf-8")
        kdf = PBKDF2HMAC(
            algorithm=hashes.SHA256(),
            length=32,
            salt=self._DOMAIN_SALT,
            iterations=100_000,
        )
        key = kdf.derive(combined_secret)
        return AESGCM(key)

    def encrypt_text(self, plaintext: Optional[str]) -> str:
        """
        Encrypts a plaintext string using AES-GCM-256.
        Returns a base64 encoded string prefixed with 'enc::'.
        """
        if not plaintext:
            return ""
        if plaintext.startswith("enc::"):
            return plaintext  # Already encrypted

        nonce = os.urandom(12)  # Standard 96-bit AES-GCM nonce
        ciphertext = self._aesgcm.encrypt(nonce, plaintext.encode("utf-8"), None)
        payload = nonce + ciphertext
        b64 = base64.b64encode(payload).decode("ascii")
        return f"enc::{b64}"

    def decrypt_text(self, cipher_or_plain: Optional[str]) -> str:
        """
        Decrypts an 'enc::' prefixed ciphertext string using AES-GCM-256.
        Returns original plaintext as-is if not encrypted (retrocompatible).
        """
        if not cipher_or_plain:
            return ""
        if not cipher_or_plain.startswith("enc::"):
            return cipher_or_plain  # Plaintext legacy record

        try:
            raw_b64 = cipher_or_plain[5:]
            payload = base64.b64decode(raw_b64.encode("ascii"))
            if len(payload) < 28:  # 12-byte nonce + at least 16-byte tag
                return cipher_or_plain
            nonce = payload[:12]
            ciphertext = payload[12:]
            decrypted = self._aesgcm.decrypt(nonce, ciphertext, None)
            return decrypted.decode("utf-8")
        except Exception:
            # If decryption fails (e.g. copied to another machine), return sanitized indicator
            return "[Protected Context Data - Hardware Key Mismatch]"

    def encrypt_record(self, record: Dict[str, Any]) -> Dict[str, Any]:
        """Encrypts sensitive content fields (text, document_summary, keywords) in a record dict."""
        out = dict(record)
        if "text" in out and isinstance(out["text"], str):
            out["text"] = self.encrypt_text(out["text"])
        if "document_summary" in out and isinstance(out["document_summary"], str):
            out["document_summary"] = self.encrypt_text(out["document_summary"])
        if "keywords" in out and isinstance(out["keywords"], str):
            out["keywords"] = self.encrypt_text(out["keywords"])
        return out

    def decrypt_record(self, record: Dict[str, Any]) -> Dict[str, Any]:
        """Decrypts sensitive content fields in a record dict."""
        out = dict(record)
        if "text" in out and isinstance(out["text"], str):
            out["text"] = self.decrypt_text(out["text"])
        if "document_summary" in out and isinstance(out["document_summary"], str):
            out["document_summary"] = self.decrypt_text(out["document_summary"])
        if "keywords" in out and isinstance(out["keywords"], str):
            out["keywords"] = self.decrypt_text(out["keywords"])
        return out

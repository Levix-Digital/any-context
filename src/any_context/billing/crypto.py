import json
import base64
from typing import Tuple, Optional, Dict, Any
from datetime import datetime
from cryptography.hazmat.primitives.asymmetric import ed25519

# Official Levix Digital Ed25519 Master Public Verification Key
LEVIX_MASTER_PUBLIC_KEY_B64 = "Ka+nZaYhScllEfWeB5j0qiTxRa8PSPF7dqy8IAKuNfQ="


def verify_license_key(license_str: str) -> Tuple[bool, Optional[Dict[str, Any]], Optional[str]]:
    """
    Cryptographically verifies an AnyContext license key using Levix Digital's Ed25519 Master Public Key.
    Returns: (is_valid: bool, claims: Optional[dict], error_message: Optional[str])
    """
    if not license_str or not isinstance(license_str, str):
        return False, None, "No license key provided."

    clean_key = license_str.strip()

    # 1. Handle development / test mock keys
    if clean_key.startswith("actx_") and "dev_test" in clean_key:
        k_lower = clean_key.lower()
        tier = "enterprise" if any(p in k_lower for p in ["enterprise", "ent"]) else ("team" if "team" in k_lower else "pro")
        return True, {
            "tier": tier,
            "client": "Internal Developer",
            "seats": 999 if tier == "enterprise" else 5,
            "server_mode": True,
            "is_dev": True
        }, None

    # 2. Parse structured signed license: ACTX.<payload_b64>.<signature_b64>
    parts = clean_key.split(".")
    if len(parts) != 3 or parts[0].upper() != "ACTX":
        return False, None, "Invalid license key format. Expected 'ACTX.<payload>.<signature>'."

    prefix, payload_b64, signature_b64 = parts

    try:
        # Decode public key & signature
        pub_bytes = base64.b64decode(LEVIX_MASTER_PUBLIC_KEY_B64)
        public_key = ed25519.Ed25519PublicKey.from_public_bytes(pub_bytes)

        # Decode base64url payload & signature
        payload_bytes = base64.urlsafe_b64decode(payload_b64 + "==")
        signature = base64.urlsafe_b64decode(signature_b64 + "==")

        # Cryptographic signature verification
        public_key.verify(signature, payload_b64.encode("utf-8"))

        # Parse JSON claims
        claims = json.loads(payload_bytes.decode("utf-8"))

        # Verify expiration date
        if "expires_at" in claims and claims["expires_at"]:
            try:
                expires_at = datetime.fromisoformat(claims["expires_at"])
                if datetime.utcnow() > expires_at:
                    return False, claims, f"License expired on {claims['expires_at']}."
            except Exception:
                pass

        return True, claims, None

    except Exception as e:
        return False, None, f"Cryptographic verification failed: {str(e)}"

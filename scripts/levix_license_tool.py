#!/usr/bin/env python3
"""
=============================================================================
Levix Digital - AnyContext License Generator Tool
=============================================================================
This tool is used by Levix Digital (and automated Stripe / Payment Webhooks)
to cryptographically sign and issue AnyContext Pro, Team, and Enterprise licenses.

NEVER expose the Master Private Key in client-side or open-source bundles.
=============================================================================
"""

import os
import sys
import json
import base64
import argparse
from datetime import datetime, timedelta
from typing import Optional, Dict, Any
from cryptography.hazmat.primitives.asymmetric import ed25519

# Levix Digital Master Signing Private Key (Keep Secret / Set via LEVIX_PRIVATE_KEY env var in production)
DEFAULT_LEVIX_PRIVATE_KEY_B64 = os.environ.get(
    "LEVIX_PRIVATE_KEY",
    "tQWDz4Snk3BWt4STOADQZYXAYwo/j9NCypcJ/AMWuw0="
)


def issue_license(
    client_name: str,
    tier: str = "enterprise",
    expires_at: Optional[str] = None,
    days_valid: int = 365,
    seats: int = 1,
    client_email: Optional[str] = None,
    private_key_b64: Optional[str] = None
) -> str:
    """
    Cryptographically signs and generates an AnyContext license key string.
    Format: ACTX.<payload_b64>.<signature_b64>
    """
    key_b64 = private_key_b64 or DEFAULT_LEVIX_PRIVATE_KEY_B64
    priv_bytes = base64.b64decode(key_b64)
    private_key = ed25519.Ed25519PrivateKey.from_private_bytes(priv_bytes)

    if not expires_at:
        exp_dt = datetime.utcnow() + timedelta(days=days_valid)
        expires_at = exp_dt.date().isoformat()

    tier_clean = tier.lower().strip()
    if tier_clean not in ["pro", "team", "enterprise", "starter"]:
        tier_clean = "enterprise"

    payload = {
        "client": client_name,
        "email": client_email or "",
        "tier": tier_clean,
        "seats": seats if tier_clean != "enterprise" else 999,
        "server_mode": tier_clean in ["pro", "team", "enterprise"],
        "issued_at": datetime.utcnow().date().isoformat(),
        "expires_at": expires_at
    }

    # Encode payload to Base64URL
    payload_bytes = json.dumps(payload, separators=(',', ':')).encode("utf-8")
    payload_b64 = base64.urlsafe_b64encode(payload_bytes).decode("utf-8").rstrip("=")

    # Generate Ed25519 cryptographic signature over payload_b64
    signature = private_key.sign(payload_b64.encode("utf-8"))
    signature_b64 = base64.urlsafe_b64encode(signature).decode("utf-8").rstrip("=")

    # Return full token
    return f"ACTX.{payload_b64}.{signature_b64}"


def main():
    parser = argparse.ArgumentParser(description="Levix Digital - AnyContext License Generator")
    subparsers = parser.add_subparsers(dest="command")

    # Command: issue
    issue_parser = subparsers.add_parser("issue", help="Issue a new signed license key")
    issue_parser.add_argument("-c", "--client", required=True, help="Client or Company Name (e.g. 'Acme Law Firm')")
    issue_parser.add_argument("-e", "--email", default=None, help="Client Email Address")
    issue_parser.add_argument("-t", "--tier", choices=["pro", "team", "enterprise"], default="enterprise", help="Subscription Plan Tier")
    issue_parser.add_argument("-d", "--days", type=int, default=365, help="Days of validity (default: 365)")
    issue_parser.add_argument("-s", "--seats", type=int, default=5, help="Number of seats (default: 5)")
    issue_parser.add_argument("--expires", default=None, help="Explicit expiration date (YYYY-MM-DD)")

    args = parser.parse_args()

    if args.command == "issue":
        key = issue_license(
            client_name=args.client,
            client_email=args.email,
            tier=args.tier,
            days_valid=args.days,
            expires_at=args.expires,
            seats=args.seats
        )
        print("\n=======================================================")
        print("🎉 AnyContext Cryptographic License Key Generated!")
        print("=======================================================")
        print(f"👤 Client  : {args.client}")
        print(f"📦 Plan    : {args.tier.upper()}")
        print(f"⏳ Days    : {args.days} days")
        print("\n🔑 License Key (send to client or paste in .env):")
        print(f"\033[92m{key}\033[0m")
        print("\n👉 Usage in .env:")
        print(f"   ANYCONTEXT_LICENSE_KEY={key}")
        print("=======================================================\n")
    else:
        parser.print_help()


if __name__ == "__main__":
    main()

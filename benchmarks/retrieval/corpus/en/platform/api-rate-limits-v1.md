# Public API Rate Limits (v1, deprecated)

> Fictitious document of Kestrel Analytics. API v1 is deprecated and stops answering on 2026-09-30.

Each API key may send up to 600 requests per minute, with bursts of at most 50 requests per second. Requests over the limit are rejected with HTTP 503 and no retry hint, so clients had to back off on their own.

Bulk export endpoints in v1 count as 10 requests each.

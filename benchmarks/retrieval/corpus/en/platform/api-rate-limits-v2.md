# Public API Rate Limits (v2)

> Fictitious document of Kestrel Analytics.

## Limits

In API v2 each access token may send up to 1,200 requests per minute, with bursts of up to 100 requests per second. Limits apply per token, not per account.

## When the limit is reached

Requests over the limit get HTTP 429 Too Many Requests with a `Retry-After` header that gives the number of seconds to wait. Clients must honor it; tokens that keep retrying earlier are blocked for 15 minutes.

## Bulk export

Bulk export endpoints count as a single request but are limited to 5 concurrent exports per account.

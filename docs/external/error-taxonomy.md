# External adapter error taxonomy

Status: draft

Stable categories
- `api`
- `auth`
- `network`
- `rate_limit`
- `validation`
- `config`
- `io`
- `parse`
- `unknown`

Routing rules
- Route on category only.
- Never route on message text.
- Preserve provider-specific detail in diagnostics, but do not make workflow logic depend on it.

Category meanings
- `api`
  - upstream rejected request or returned domain error
  - usually not retryable unchanged
- `auth`
  - credentials invalid/missing/insufficient
  - not retryable without operator fix
- `network`
  - DNS/TLS/connectivity/timeout/reset
  - retryable with bounded backoff
- `rate_limit`
  - upstream throttled request
  - retryable after adaptation/backoff
- `validation`
  - bad inputs before or at provider boundary
  - not retryable unchanged
- `config`
  - bad local configuration / missing endpoint / malformed config
  - not retryable unchanged
- `io`
  - file/path/permission/local state issues
  - not retryable unless environment changes
- `parse`
  - provider returned malformed or unexpected payload
  - usually not retryable unchanged
- `unknown`
  - fallback bucket; should be minimized

Retry guidance
- retryable by default:
  - `network`
  - `rate_limit`
- non-retryable by default:
  - `api`
  - `auth`
  - `validation`
  - `config`
  - `io`
  - `parse`

Suggested persisted fields
```json
{
  "provider": "example",
  "operation": "ohlc.fetch",
  "category": "network",
  "retryable": true,
  "message": "connection timeout",
  "diagnostics": {
    "stderr_excerpt": "...",
    "exit_code": 28
  }
}
```

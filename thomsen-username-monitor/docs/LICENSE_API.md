# License API contract (to be implemented in the existing Thomsen backend)

The client (`thomsen_monitor/license.py`) calls one endpoint. Product slug: `thomsen-username-monitor`.

`POST /v1/licenses/validate`  body: `{"key": str, "product": str, "device_id": str}`

- 200 valid:   `{"valid": true, "status": "active", "expires_at": <unix seconds | null>}`
- 200 invalid: `{"valid": false, "reason": "license revoked" | "license expired" | ...}`
- 404/4xx:     `{"error": "unknown license"}`

Server-side requirements (not yet implemented - the existing backend was not accessible):
- Keys: >=128 bits from a CSPRNG; store only a hash; bind to Discord user id + product.
- Bind/limit `device_id` per license; rate-limit this endpoint.
- Bot `/license generate|view|retrieve` must check entitlement (role/subscription/admin grant),
  enforce one active license per user+product, and write an audit row.
- Admin panel: list/search by Discord user, revoke, extend, activity log, product eligibility rules.
- Keep bot token, DB credentials and any signing key server-side only.

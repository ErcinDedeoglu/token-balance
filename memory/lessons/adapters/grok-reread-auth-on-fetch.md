# grok-reread-auth-on-fetch

- **Domain:** adapters
- **Date:** 2026-09-19
- **Pattern-Key:** grok-reread-auth-on-fetch
- **Confidence:** high
- **Evidence:** `cargo test -p token-balance --locked -- grok`; live `~/.grok/auth.json` JWT `exp` ~6h; `tb` pid 52234 started at login; urllib/reqwest GET billing HTTP 200 with that JWT; `tui.rs` `reload_live` only on `r`
- **Status:** active

## Situation

Grok card showed `HTTP 401 Unauthorized` while `~/.grok/auth.json` held a valid JWT (curl/urllib/reqwest billing GET 200). `GrokAdapter::from_account` cached `key` once. Timer refresh does not call `reload_live`.

## Decision

Re-read the credentials pointer on every `fetch`. Map HTTP 401 to `grok login`. Rejected: treating 401 as rustls (already native-tls HTTP/1); writing `auth.json`; OIDC refresh.

## Reasoning

Grok CLI OIDC access tokens expire in ~6 hours and rewrite `auth.json`. Manual `r` rebuilds adapters; the 60s timer does not. Caching the JWT at construct leaves 401 until restart or `r`.

## Outcome

`grok_rereads_auth_json_after_login` and `grok_401_points_at_login` pass. Same JWT still HTTP 200 on `cli-chat-proxy.grok.com/v1/billing?format=credits`.

## Lesson

Do not cache Grok CLI JWT at adapter construct. Read `auth.json` on each fetch. Do not treat billing 401 as expired until curl/urllib with that JWT is tried.

# grok-oidc-refresh-same-as-cli

- **Domain:** adapters
- **Date:** 2026-09-21
- **Pattern-Key:** grok-oidc-refresh-same-as-cli
- **Confidence:** high
- **Evidence:** grok 1.0.40 `xai-grok-login` oidc_refresher; `POST https://auth.x.ai/oauth2/token`; `grok_oidc.rs`
- **Status:** active

## Situation

Access JWT in `~/.grok/auth.json` expires in ~6h. Opening Grok CLI still works because it silently refreshes. `tb` used the expired `key` and painted `error`.

## Decision

Same grant as the CLI: `grant_type=refresh_token` + `client_id` + `refresh_token` to `{oidc_issuer}/oauth2/token`. Persist rotated `key` / `refresh_token` / `expires_at` into `auth.json` (mode 0600). Rejected: in-memory-only (IdP rotates RT).

## Reasoning

CLI strings and a live refresh returned HTTP 200, `expires_in` 21600, and a **new** `refresh_token`. Not writing it back races Grok.

## Outcome

Expired access is refreshed before billing; 401 retries once. Unit tests cover parse, map, persist.

## Lesson

Do not treat Grok 401 as `grok login` until OIDC refresh with the stored `refresh_token` has been tried. Always write the rotated refresh token back.

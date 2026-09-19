# muse-429-is-exhausted-everyday

- **Domain:** adapters
- **Date:** 2026-09-19
- **Pattern-Key:** muse-429-is-exhausted-everyday
- **Confidence:** high
- **Evidence:** `docs/design/providers.md` Muse row; openusage 429 `error.resets_at`; `cargo test -p token-balance --locked -- muse`
- **Status:** active

## Situation

The Muse card showed `HTTP 429 Too Many Requests`. `http_post_text` dropped the body, so exhausted Everyday looked like a transport error.

## Decision

Keep the 429 body. `error.resets_at` / quota-exhausted maps to 5h 0% remaining. Rejected: live POST to probe (burns Everyday); treating 429 as a dead token.

## Reasoning

Design: remaining is SSE usage **or** 429 + `resets_at`. Meta body is `{"error":{"code":"rate_limit_exceeded","resets_at":…}}`.

## Outcome

`muse_429_with_resets_at_is_exhausted_everyday_not_dead_token` passes. No live Muse POST in this change.

## Lesson

Do not paint Muse HTTP 429 + `resets_at` as a dead token. It is exhausted Everyday remaining.

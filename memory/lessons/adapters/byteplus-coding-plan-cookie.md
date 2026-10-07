# byteplus-coding-plan-cookie

- **Domain:** adapters
- **Date:** 2026-10-07
- **Pattern-Key:** byteplus-coding-plan-cookie
- **Confidence:** high
- **Evidence:** chrome-mcp capture of the console page `console.byteplus.com/ark/region:ap-southeast-1/subscription/coding-plan`; `POST .../api/top/ark/ap-southeast-1/2024-01-01/GetCodingPlanUsage` body `{}`; headerless call returned `ResponseMetadata.Error.Code "InvalidCSRFToken"`; a page fetch with the `csrfToken` cookie echoed as `X-Csrf-Token` returned full `Result.QuotaUsage`; `cargo test -p token-balance -- byteplus` 4 passed
- **Status:** active

## Situation

BytePlus ModelArk offers a Coding Plan (Lite/Pro: ~1,200/6,000 requests per 5h, daily/weekly/monthly caps). The data-plane API key `https://ark.ap-southeast.bytepluses.com/api/v3` has **no** remaining/usage endpoint — unknown GET paths return an empty `200`, and the console FAQ says remaining is on the console page. The signed OpenAPI "Query inference usage" endpoints need IAM Access Key/Secret Key, not the API key.

## Decision

Read remaining from the console JSON the page itself calls: `POST https://console.byteplus.com/api/top/ark/ap-southeast-1/2024-01-01/GetCodingPlanUsage` with body `{}`, `Cookie: <console cookie>`, and `X-Csrf-Token: <csrfToken cookie>`. Map `Result.QuotaUsage[]` levels `session`/`weekly`/`monthly` (`Percent` is remaining, `ResetTimestamp` epoch secs) to 5h/wk/`Other("mo")`. Tier from `ListSubscribeTrade` `Result.InfoList[0].BizInfo`. Rejected: the `/api/v3` API key (no endpoint), signed AK/SK OpenAPI (not the bearer key).

## Reasoning

The console is a normal cookie-authenticated SPA; the page fetches `GetCodingPlanUsage` on load and every ~10–20s. CSRF is double-submit: the header must equal the `csrfToken` cookie, and a request without it returns `200` with `InvalidCSRFToken` (an error *in the body*, not an HTTP status). `connect.sid` is HttpOnly+Secure, so the user must copy the whole `Cookie` request header from DevTools, not `document.cookie`. The `x-web-id` header is not required.

## Outcome

`crates/token-balance/src/adapters/byteplus.rs` maps the three windows and plan tier; `map_byteplus`, `byteplus_maps_session_weekly_monthly`, `byteplus_recorded_fetch`, `byteplus_csrf_from_cookie` pass. `byteplus.json` `{ "cookie": "…" }` is the credential (console cookie adapter, like `exa.json`).

## Lesson

BytePlus Coding Plan remaining is only in the console `GetCodingPlanUsage` JSON, authenticated by the console cookie plus an `X-Csrf-Token` header equal to the `csrfToken` cookie; the `/api/v3` API key cannot read it.

<!-- agent-kit:context:begin -->
Branch: main
Revision: 1ae85381e9fe643e00310ea439e4860edb98a791
Scope: checkout
Verification: observed — chrome-mcp console capture, headerless call `InvalidCSRFToken`, csrf-header fetch returned `Result.QuotaUsage`; cargo test -p token-balance -- byteplus 4 passed; full cargo test 175 passed / 1 pre-existing kiro :9090 failure
Integration: unknown
Target: unknown
Integration-evidence: none
Deployment: unknown
Owner: repository
<!-- agent-kit:context:end -->

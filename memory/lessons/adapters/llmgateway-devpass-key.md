# llmgateway-devpass-key

- **Domain:** adapters
- **Date:** 2026-10-07
- **Pattern-Key:** llmgateway-devpass-key
- **Confidence:** high
- **Evidence:** live `GET https://api.llmgateway.io/v1/key` (DevPass lite key) returned `data.devPlanCreditsRemaining` "86.97" / `devPlanCreditsLimit` "87", `devPlanPremiumWeeklyLimit` "10.44", `devPlanPremiumWeekResetsAt` null; `GET /v1/credits` returned `{"balance":"0","total_used":"…"}`; docs.llmgateway.io/developers/devpass-usage; recorded test `llmgateway_devpass_maps_monthly_and_weekly`
- **Status:** active

## Situation

Adding LLM Gateway remaining. The vendor has a dashboard but no obvious remaining API, and the configured key is a DevPass plan key rather than pay-as-you-go.

## Decision

Fetch `GET https://api.llmgateway.io/v1/key` with the plain gateway API key (`Authorization: Bearer`) — no dashboard or master-key session. Map `devPlanCreditsRemaining`/`devPlanCreditsLimit` to a monthly `Other("mo")` window and `(devPlanPremiumWeeklyLimit − devPlanPremiumCreditsUsed)` to a `Weekly` window reset by `devPlanPremiumWeekResetsAt`. A `devPlan:"none"` key (PAYG/BYOK) has no plan window. Rejected: `/v1/credits` `balance` as plan remaining (it is a prepaid wallet) and a dashboard session.

## Reasoning

The docs mark `/v1/key` as the endpoint "so clients that only hold an API key surface remaining quota without a dashboard session". Every value is a decimal string, which `json_f64` already parses. The monthly billing-cycle reset is not present in the response, so that window carries no `resets_at`. `/v1/key` is 1200/min, so 60s polling is safe (unlike Muse's request-metered endpoint).

## Outcome

`llmgateway_devpass_maps_monthly_and_weekly` (86.97/87 → ~99.97% mo, 10.44/10.44 → 100% wk), `llmgateway_weekly_reset_is_parsed`, and `llmgateway_payg_key_is_unsupported` pass.

## Lesson

For LLM Gateway remaining, use `GET /v1/key` with the plain gateway API key; the fields are decimal strings, and a `devPlan:"none"` key has no plan window.

<!-- agent-kit:context:begin -->
Branch: main
Revision: 080ead9c4c2f81de68e8beed67b2f9491ff3ce8f
Scope: checkout
Verification: observed — live /v1/key and /v1/credits returned HTTP 200; cargo test -p token-balance -- llmgateway 5 passed
Integration: unknown
Target: unknown
Integration-evidence: none
Deployment: unknown
Owner: repository
<!-- agent-kit:context:end -->

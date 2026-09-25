# grok-omitted-percent-is-fresh-week

- **Domain:** adapters
- **Date:** 2026-09-25
- **Pattern-Key:** grok-omitted-percent-is-fresh-week
- **Confidence:** high
- **Evidence:** live `GET /v1/billing?format=credits` after `2026-09-25T19:00:05Z`; grok 1.0.40 log `billing: fetched credits config` with `creditUsagePercent: null`
- **Status:** active

## Situation

Grok row painted `error` eighteen minutes after the weekly reset. The JWT was still valid. Billing HTTP 200 had a weekly `currentPeriod` and no `creditUsagePercent`.

## Decision

Map a weekly period with an omitted percent as 0% used. Rejected: treating it as `Error`, and mapping `/v1/billing` without `format=credits` (`used` is monthly spend).

## Reasoning

Grok CLI 1.0.40 fetched the same body and logged success, not a parse failure. Prepaid-only JSON still has no weekly period, so that stays `Error`.

## Outcome

`grok_fresh_weekly_period_without_percent_is_full` expects 100% remaining. `grok_prepaid_only_is_error` still fails closed.

## Lesson

Do not treat a missing Grok `creditUsagePercent` as a schema break when `currentPeriod.type` is `USAGE_PERIOD_TYPE_WEEKLY`. That is a fresh week at 0% used.

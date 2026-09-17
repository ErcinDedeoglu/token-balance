---
id: GOAL-EXA-1
title: Exa prepaid wallet contract (no spend-as-remaining)
status: done
created: 2026-09-17
source: discussion
---

# GOAL-EXA-1: Exa prepaid wallet contract (no spend-as-remaining)

## Intent

A listed Exa account is a prepaid USD wallet on the remaining-quota board and cannot be painted from Team Management usage spend or as 5h/weekly plan remaining.

## Problem

The user asked to add Exa AI after research. Official Exa docs (2026-09-17) describe pay-as-you-go credits whose remaining balance is on the Billing dashboard, while `GET https://admin-api.exa.ai/team-management/api-keys/{id}/usage` returns `total_cost_usd` (spend). There is no documented remaining-balance GET. Shipping `/usage` as leftover credits would repeat the Copilot/Admin-Usage footgun.

## Outcome

`docs/design/providers.md` has an Exa row. Mapper tests use recorded JSON: spend-only shapes and `NO_MORE_CREDITS` do not become 5h/weekly windows or remaining percent; missing credentials are `NotConfigured` not numeric `0%`. Live remaining fetch is out of this goal until a remaining source is chosen.

## Scope

### In

- Exa row in `docs/design/providers.md` (PrepaidWallet, dashboard remaining, `/usage` is spend)
- Recorded-JSON mapper tests for spend-only and 402 `NO_MORE_CREDITS` bodies
- `NotConfigured` when no `EXA_API_KEY` / pointer secret
- Grep that fetch does not call `/team-management/api-keys/` `usage` as remaining

### Out

- Do not implement a live remaining-balance HTTP fetch in this goal
- Do not treat `GET .../api-keys/{id}/usage` `total_cost_usd` as remaining
- Do not scrape `dashboard.exa.ai` cookies or session
- Do not invent 5h or weekly plan windows for Exa
- Do not poll `POST https://api.exa.ai/search` to discover remaining (burns credits)
- Do not ship x402, MPP, or Nevermined pay-per-request as this card
- Do not use per-key `budgetCents` as team wallet remaining

## Context

- Constraints: Ledger-1 remaining-quota board; `Never fake numeric 0%` for `NotConfigured` / `Unsupported` (`crates/token-balance/AGENTS.md`); live membership is `accounts.toml` only; undocumented endpoints need a recorded redacted fixture (`docs/design/providers.md`)
- Files / systems: `docs/design/providers.md`; `crates/token-balance/src/adapters/`; `EXA_API_KEY`; https://exa.ai/docs/reference/billing ; https://exa.ai/docs/reference/team-management/get-api-key-usage ; https://exa.ai/docs/team-management-spec.yaml
- Commands: `cargo test -p token-balance --locked`; `cargo build -p token-balance --locked`; Lint `n/a`; Typecheck `n/a`; `.githooks/check-quality.sh`; `tooling/agent-kit/check.sh`

## Decisions

| Decision | Provenance |
| --- | --- |
| Add Exa after research, not before | user: "I want to add exa ai also. but first research" |
| Exa is pay-as-you-go prepaid credits, not a coding-plan window | this discussion; https://exa.ai/docs/reference/billing |
| Remaining is dashboard-visible; `/usage` is spend (`total_cost_usd`) | this discussion; https://exa.ai/docs/reference/team-management/get-api-key-usage |
| Map as `LedgerKind::PrepaidWallet` if remaining JSON exists later (fal/DeepSeek family) | this discussion; `docs/design/providers.md` fal/DeepSeek rows |
| Do not paint `/usage` as remaining | this discussion (same class as Copilot billing spend) |

## Assumptions

- Public Exa docs as of 2026-09-17 still have no remaining-balance GET
- Team Management API stays opt-in (service key; contact support)
- Free-tier $10/month is a wallet top-up, not a session window

## Open questions

n/a

## Work

- [x] **T1** Write the Exa row in `docs/design/providers.md` with PrepaidWallet, dashboard remaining, and `/usage` labeled spend → AC-1
- [x] **T2** Add a recorded spend-only JSON fixture (`total_cost_usd`, no remaining field) and a mapper test that yields `ProviderStatus::Error` not remaining percent → AC-2
- [x] **T3** Add a recorded 402 `NO_MORE_CREDITS` fixture and a mapper test with no FiveHour or Weekly windows → AC-3
- [x] **T4** Keep Exa fetch off `GET .../team-management/api-keys/{id}/usage` as remaining → AC-4
- [x] **T5** Map missing Exa secret to `NotConfigured` without numeric `0%` → AC-5

## Acceptance criteria

### Must

- [x] **AC-1** Given `docs/design/providers.md`, when the Exa row is read, then it names `LedgerKind::PrepaidWallet`, remaining on the Billing dashboard, and `GET https://admin-api.exa.ai/team-management/api-keys/{id}/usage` as spend (`total_cost_usd`) not remaining.
      Verify: `rg -n "exa|PrepaidWallet|total_cost_usd|dashboard.exa.ai/billing" docs/design/providers.md`
- [x] **AC-2** Given recorded JSON whose billing object is `total_cost_usd` with no remaining-balance field, when the Exa mapper runs, then the status is `ProviderStatus::Error` and not a remaining percent.
      Verify: `cargo test -p token-balance --locked -- exa`
- [x] **AC-3** Given recorded JSON or HTTP 402 body with tag `NO_MORE_CREDITS`, when the Exa mapper runs, then the status has no FiveHour window and no Weekly window.
      Verify: `cargo test -p token-balance --locked -- exa`
- [x] **AC-4** Given Exa adapter source, when searched for the usage path, then fetch does not use `/team-management/api-keys/` plus `/usage` as the remaining source.
      Verify: `rg -n "team-management/api-keys" crates/token-balance/src/adapters/`
- [x] **AC-5** Given an Exa account pointer with no secret, when the adapter builds or fetches, then the status is `NotConfigured` and the board does not show numeric `0%` for that card.
      Verify: `cargo test -p token-balance --locked -- exa`

### Should

- [x] **AC-S1** Given `accounts.toml` comments or the Exa row, when a user adds a row, then vendor id `exa` and `api_key_env` / `credentials` pointer are documented (no inline key field).
      Verify: `rg -n "exa" docs/design/providers.md crates/token-balance/src/accounts.rs`

## Definition of done

- [x] Every Must AC is `[x]` with evidence under Evidence
- [x] Work items that those AC require are `[x]`
- [x] Applicable gates pass: `cargo test -p token-balance --locked` / `n/a` lint / `n/a` typecheck; `.githooks/check-quality.sh`; `tooling/agent-kit/check.sh`
- [x] No secrets in the file or the change
- [x] Leftovers filed as a later `GOAL-EXA-N` or listed in Out

## Evidence

| AC | Result | Proof |
| --- | --- | --- |
| AC-1 | pass | `docs/design/providers.md` Exa row: PrepaidWallet, dashboard.exa.ai/billing, total_cost_usd spend |
| AC-2 | pass | `cargo test -p token-balance --locked -- exa`: `spend_only_total_cost_usd_is_error_not_remaining_percent` and `recorded_spend_fetch_is_error` |
| AC-3 | pass | same filter: `no_more_credits_has_no_fivehour_or_weekly_window` and `recorded_no_more_credits_fetch_has_no_plan_windows` |
| AC-4 | pass | `rg team-management/api-keys crates/token-balance/src/adapters/` no matches; live fetch is Unsupported, no HTTP |
| AC-5 | pass | `missing_key_is_not_configured` → NotConfigured hint contains EXA_API_KEY, not 0% |

## Risks

- An undocumented dashboard XHR might still return remaining; capturing it is `GOAL-EXA-2`, not this file
- Team Management `/usage` looks like a billing API and will tempt a spend-as-remaining implementation

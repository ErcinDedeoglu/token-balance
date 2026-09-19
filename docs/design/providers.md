# Provider port

## Trait (frozen — object-safe)

RPITIT `fn fetch(&self) -> impl Future<…>` is **not** dyn-compatible and cannot be `JoinSet::spawn`ed without `'static`. Lock boxed type-erasure:

```rust
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

pub type FetchFuture = Pin<Box<dyn Future<Output = ProviderStatus> + Send + 'static>>;

pub enum RefreshPolicy {
    Default,                 // 60s timer + r
    OnDemand,                // r only (Muse)
    Interval(Duration),      // future
}

pub trait Provider: Send + Sync {
    fn id(&self) -> &str;            // account id, e.g. "claude-work"
    fn display_name(&self) -> &str;  // unique label on the card title
    fn vendor(&self) -> &str;        // "claude" — glyph key
    fn glyph_ascii(&self) -> &'static str; // vendor letter; 1-column; required
    fn ledger(&self) -> LedgerKind { LedgerKind::PlanRemaining }
    fn refresh_policy(&self) -> RefreshPolicy { RefreshPolicy::Default }
    fn docs_url(&self) -> Option<&'static str> { None }
    fn fetch_timeout(&self) -> Duration { Duration::from_secs(10) }
    /// Clone everything the future needs so it is `'static`.
    fn fetch(&self) -> FetchFuture;
}
```

Registry: `Vec<Arc<dyn Provider>>`. Spawn:

```rust
let p = Arc::clone(&provider);
join.spawn(async move {
    match tokio::time::timeout(p.fetch_timeout(), p.fetch()).await {
        Ok(status) => status,
        Err(_elapsed) => ProviderStatus::Error { message: "timed out".into(), stale: None },
    }
});
```

`FixtureProvider` implements this. Live adapters **must not** change the trait in a way that forces a TUI rewrite. Adding methods is allowed with defaults.

Live membership is **config-listed accounts**, not one adapter per vendor. `{HOME}/.config/token-balance/accounts.toml` `[[account]]` rows (vendor, id, label, `api_key_env` or `credentials` path). No auto-include of official CLI logins. Same vendor may appear twice (shared glyph, distinct labels). Codex: at most one row. Inline key fields fail load.

## Adapter contract (later PRs)

Auth rule: **reuse tokens the official CLIs already stored**, via pointers the user listed. Never password login. Never cookie stealers. Never log secrets. Read credential files **read-only**.

Undocumented endpoints live **only** behind adapters. Parse with `serde` + `Option` fields. Unknown shape → `ProviderStatus::Error`, never `unwrap`.

| Provider | Remaining source | Auth | Map into domain | Footguns |
| --- | --- | --- | --- | --- |
| **Kimi Coding** | `GET https://api.kimi.com/coding/v1/usages` | Coding API key / `~/.kimi-code/credentials/kimi-code.json` (or `KIMI_API_KEY` / `KIMI_CODE_API_KEY`). Key shape `sk-kimi-…`, **not** open-platform `sk-…`. | `usage` → weekly; `limits[]` where `duration=300` + `TIME_UNIT_MINUTE` → session FiveHour. **PR5 records a redacted JSON fixture** and picks `from_used_percent` vs `from_remaining_percent` from that sample. Do not guess both. | Do **not** mix PAYG `https://api.moonshot.ai/v1/users/me/balance` into this card (ledger 2). |
| **z.ai GLM Coding** | `GET https://api.z.ai/api/monitor/usage/quota/limit` | `ZAI_API_KEY` or a key file pointer | `limits[].type`: `TOKENS_LIMIT` **or** `CREDIT_LIMIT` (live Max uses credit). `unit`/`number`: `(3,5)` → 5h; `(6,7)` or `(6,1)` → weekly. `percentage` is **used**. `from_used_percent`. `nextResetTime` epoch ms. | `ZHIPUAI_API_KEY` / `open.bigmodel.cn` is a **different China account**. Do not silently fall across regions. Coding list-price `$` is **not on the card**. Skip `TIME_LIMIT`. |
| **xAI API prepaid** | official prepaid balance | `XAI_API_KEY` or management key | `LedgerKind::PrepaidWallet` only | **Not SuperGrok.** Not a v1 card. Never paint as amber plan remaining. |
| **Grok SuperGrok** | unofficial: `~/.grok/auth.json` then `cli-chat-proxy.grok.com/v1/billing` and/or grok.com gRPC-web `GetGrokCreditsConfig` | Grok CLI login only | weekly % + extra credits | No public remaining API. Settings → Usage is the official UI. Schema change → `Error`, not panic. |
| **Codex Plus/Pro** | `codex app-server` JSON-RPC `account/rateLimits/read` | existing `~/.codex` ChatGPT login | Classify by **`windowDurationMins`**, not by primary/secondary name. `duration_mins <= 360` → session-class (15m, 60m, 300m). Longer / ~weekly → `Weekly`. Field is **`usedPercent`**. `from_used_percent`. `resetsAt` unix **seconds**. Prefer `rateLimitsByLimitId["codex"]` when present. Some accounts only have weekly. | Official example uses `windowDurationMins: 15` and `secondary: null`. **Never map `primary` → FiveHour blindly.** Do not scrape `/status`. `rateLimitResetCredits` → extra teal, not a fake window. |
| **Claude Pro/Max** | Official: `GET https://api.anthropic.com/api/oauth/usage`. If `credentials` is `http(s)://…`, GET that JSON instead (personal proxy; not the default). | Official: Bearer from `~/.claude/.credentials.json`; `anthropic-beta: oauth-2025-04-20`. Unofficial: usage URL in `accounts.toml` only. | `five_hour`/`seven_day` utilization used %. Nested `usage` unwrap. If those are null and `extra_usage` is enabled, monthly `Other("mo")` from `used_credits`/`monthly_limit`. **No invented 5h.** | Official channel stays OAuth. Do not ship a localhost default. Admin Usage & Cost is spend, not remaining. |
| **DeepSeek** | `GET https://api.deepseek.com/user/balance` | `DEEPSEEK_API_KEY` or a key file pointer | Prepaid **wallet**, not a plan. `balance_infos[].total_balance` (prefer USD). `LedgerKind::PrepaidWallet`. Teal `$` remaining. **No 5h/weekly %.** | Not a subscription. Do not fake remaining percent. Top-up does not expire; granted balance may. |
| **Kiro (AWS)** | Official: POST `https://q.<region>.amazonaws.com/` `x-amz-target: AmazonCodeWhispererService.GetUsageLimits` (fallback `management.<region>.kiro.dev`). If `credentials` is `http(s)://…`, GET that URL instead (personal proxy; not the default). | Official: kiro-cli sqlite or kiro-proxy `auth.json`. Unofficial: usage URL in `accounts.toml` only. | `usageBreakdownList` used/limit (`currentUsageWithPrecision` / `usageLimitWithPrecision` or short names). Monthly pool as Weekly. `subscriptionInfo.subscriptionTitle`. `nextDateReset` on root or bucket. | Official channel stays AWS. Do not ship a localhost default. Stale sqlite → `kiro-cli login`. Do not persist refreshed tokens. |
| **Muse (Meta Muse Code)** | **no poll API**. Only SSE `response.subscription_usage` on `POST https://api.meta.ai/v1/responses`, or 429 + `error.resets_at` when exhausted (map to 5h 0% remaining, not a dead token) | `~/.config/muse/auth.json` / keychain / `META_API_KEY` (PAYG **leaves** subscription) | If ever enabled: parse usage from a **user-initiated** request, cache it. | Burns a **request** per poll. Everyday meters requests (10–50 / 5h). **v1 = Unsupported. Later = OnDemand, last adapter PR.** Never auto-60s. |
| **Muse web (dashboard)** | Side card, not `muse.rs`. `POST https://dev.meta.ai/api/graphql/` `LLMDCUsageQuery` (`doc_id` 28117303444603430). Maps `data.team.subscription_quota_usage` window/weekly weighted used/limit. If `credentials` is `http(s)://…`, GET that JSON instead. | User-owned `muse-web.json`: `team_id`, `cookie`, `fb_dtsg` (optional `lsd`, `doc_id`). Never auto-scrape a browser. | `RefreshPolicy::Interval(180s)`. Plan text from `tier` with `Muse Code ` stripped. Rate-limit errors auto-retry after 180s. | Leave the Model API Muse adapter untouched. Do not 60s-poll GraphQL (Meta `1675004`). Do not commit Facebook cookies. |
| **fal** | Official `GET https://api.fal.ai/v1/account/billing?expand=credits` | `FAL_KEY` or a key file pointer. Header `Authorization: Key …` (admin key). | Prepaid **wallet**. `credits.current_balance` + `currency`. `LedgerKind::PrepaidWallet`. Teal `$` remaining. **No 5h/weekly %.** Username → plan text. | Not a subscription. Do not fake remaining percent. `/models/usage` is spend history, not remaining. Purchased credits expire ~365 days. |
| **GitHub Copilot** | Unofficial (VS Code): `GET https://api.github.com/copilot_internal/user`. Official REST `…/billing/premium_request/usage` is **spend**, not remaining. | `credentials = "gh"` runs `gh auth token`. Or `GITHUB_TOKEN` / a token file. One GitHub identity per row. | `quota_snapshots.premium_interactions.percent_remaining` → monthly `Other("mo")`. Skip unlimited chat/completions. `from_remaining_percent`. Reset `quota_reset_date`. | Do not scrape billing spend. Do not invent a 5h bar. Weekly token usage limits are separate from premium remaining. |
| **Exa** | Unofficial `GET https://dashboard.exa.ai/api/get-credits`. `orbCreditsInCents / 100` → USD. Official `/usage` is spend. `EXA_API_KEY` cannot read remaining. Vercel Security Checkpoint 429s stock reqwest/curl; fetch with wreq Chrome emulation + HTTP/1.1. | User-owned `exa.json` `{ "cookie": "…" }`. Never commit cookies. Never drive a browser from `tb`. | `LedgerKind::PrepaidWallet`. Teal `$`. **No 5h/weekly %.** | Do not paint `/usage` as remaining. Do not poll `/search`. Do not open Chrome. |
| **Firecrawl** | Official `GET https://api.firecrawl.dev/v2/team/credit-usage`. `data.remainingCredits` (v1 `remaining_credits`). `planCredits` is plan allotment, not spend. Historical `/team/credit-usage/historical` is **used** credits, not remaining. | `FIRECRAWL_API_KEY` or a key file pointer (`vendor = "firecrawl"`). `Authorization: Bearer`. | `LedgerKind::PrepaidWallet`. `CreditUnit::Credits`. Teal remaining count. **No 5h/weekly %.** Do not invent USD from credits. | Do not poll `/scrape` (burns credits). Do not paint historical `totalCredits` as remaining. 402 is empty allotment, not a session window. |

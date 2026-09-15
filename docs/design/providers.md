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
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn glyph_ascii(&self) -> &'static str; // unique 1-column; required
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

v1: `FixtureProvider` implements this. Live adapters **must not** change the trait in a way that forces a TUI rewrite. Adding methods is allowed with defaults.

## Adapter contract (later PRs)

Auth rule: **reuse tokens the official CLIs already stored**. Never password login. Never cookie stealers. Never log secrets. Read credential files **read-only**.

Undocumented endpoints live **only** behind adapters. Parse with `serde` + `Option` fields. Unknown shape → `ProviderStatus::Error`, never `unwrap`.

| Provider | Remaining source | Auth | Map into domain | Footguns |
| --- | --- | --- | --- | --- |
| **Kimi Coding** | `GET https://api.kimi.com/coding/v1/usages` | Coding API key / `~/.kimi-code/credentials/kimi-code.json` (or `KIMI_API_KEY` / `KIMI_CODE_API_KEY`). Key shape `sk-kimi-…`, **not** open-platform `sk-…`. | `usage` → weekly; `limits[]` where `duration=300` + `TIME_UNIT_MINUTE` → session FiveHour. **PR5 records a redacted JSON fixture** and picks `from_used_percent` vs `from_remaining_percent` from that sample. Do not guess both. | Do **not** mix PAYG `https://api.moonshot.ai/v1/users/me/balance` into this card (ledger 2). |
| **z.ai GLM Coding** | `GET https://api.z.ai/api/monitor/usage/quota/limit` | `ZAI_API_KEY` | `limits[].type`: `TIME_LIMIT` / `TOKENS_LIMIT`; `unit`/`number` distinguish 5h vs 7d. `percentage` is **used**. `from_used_percent`. `nextResetTime` epoch ms. | `ZHIPUAI_API_KEY` / `open.bigmodel.cn` is a **different China account**. Do not silently fall across regions. Coding list-price `$` is **not on the card**. |
| **xAI API prepaid** | official prepaid balance | `XAI_API_KEY` or management key | `LedgerKind::PrepaidWallet` only | **Not SuperGrok.** Not a v1 card. Never paint as amber plan remaining. |
| **Grok SuperGrok** | unofficial: `~/.grok/auth.json` then `cli-chat-proxy.grok.com/v1/billing` and/or grok.com gRPC-web `GetGrokCreditsConfig` | Grok CLI login only | weekly % + extra credits | No public remaining API. Settings → Usage is the official UI. Schema change → `Error`, not panic. |
| **Codex Plus/Pro** | `codex app-server` JSON-RPC `account/rateLimits/read` | existing `~/.codex` ChatGPT login | Classify by **`windowDurationMins`**, not by primary/secondary name. `duration_mins <= 360` → session-class (15m, 60m, 300m). Longer / ~weekly → `Weekly`. Field is **`usedPercent`**. `from_used_percent`. `resetsAt` unix **seconds**. Prefer `rateLimitsByLimitId["codex"]` when present. Some accounts only have weekly. | Official example uses `windowDurationMins: 15` and `secondary: null`. **Never map `primary` → FiveHour blindly.** Do not scrape `/status`. `rateLimitResetCredits` → extra teal, not a fake window. |
| **Claude Pro/Max** | `GET https://api.anthropic.com/api/oauth/usage` | Bearer from `~/.claude/.credentials.json`; header `anthropic-beta: oauth-2025-04-20` | `five_hour.utilization` / `seven_day.utilization` are **used %**. `resets_at`. `extra_usage` → teal tertiary. | Same numbers as `/usage`/`/limits`. **Admin Usage & Cost API is historical org spend — not remaining.** Read-only credentials. |
| **Muse (Meta Muse Code)** | **no poll API**. Only SSE `response.subscription_usage` on `POST https://api.meta.ai/v1/responses`, or 429 + `resets_at` when exhausted | `~/.config/muse/auth.json` / keychain / `META_API_KEY` (PAYG **leaves** subscription) | If ever enabled: parse usage from a **user-initiated** request, cache it. | Burns a **request** per poll. Everyday meters requests (10–50 / 5h). **v1 = Unsupported. Later = OnDemand, last adapter PR.** Never auto-60s. |

# Domain model

No database. In-memory + serde-ready structs (so `--json` is a flag, not a rewrite).

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LedgerKind {
    PlanRemaining,
    PrepaidWallet,
    LocalConsumed, // never rendered as hero in v1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowLabel {
    FiveHour, // nominal ~5h; caption still uses duration_mins when set
    Weekly,
    Other(String), // "15m", "60m", …
}

pub const SESSION_MAX_MINS: u32 = 360;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaWindow {
    pub label: WindowLabel,
    /// Canonical provider figure. Codex `usedPercent` lands here.
    pub used_percent: f32,
    /// Always `100.0 - used_percent` after clamp. Card face reads this.
    pub remaining_percent: f32,
    pub resets_at: Option<DateTime<Utc>>,
    pub duration_mins: Option<u32>,
    pub limit: Option<f64>,
    pub remaining: Option<f64>,
}

impl QuotaWindow {
    pub fn from_used_percent(
        label: WindowLabel,
        used_percent: f32,
        resets_at: Option<DateTime<Utc>>,
        duration_mins: Option<u32>,
    ) -> Self {
        let used = used_percent.clamp(0.0, 100.0);
        Self {
            label,
            used_percent: used,
            remaining_percent: (100.0 - used).clamp(0.0, 100.0),
            resets_at,
            duration_mins,
            limit: None,
            remaining: None,
        }
    }

    /// Never implement remaining as `100.0 - remaining` in an adapter.
    pub fn from_remaining_percent(
        label: WindowLabel,
        remaining_percent: f32,
        resets_at: Option<DateTime<Utc>>,
        duration_mins: Option<u32>,
    ) -> Self {
        let remaining = remaining_percent.clamp(0.0, 100.0);
        Self::from_used_percent(label, 100.0 - remaining, resets_at, duration_mins)
    }

    pub fn is_session(&self) -> bool {
        match self.duration_mins {
            Some(m) => m <= SESSION_MAX_MINS,
            None => matches!(self.label, WindowLabel::FiveHour),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode { Risk, Name }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CreditUnit { Usd, Credits, Unknown }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtraCredits {
    pub label: String, // "extra usage" | "reset credits" | "prepaid"
    pub remaining: f64,
    pub unit: CreditUnit,
    pub limit: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderStatus {
    Available {
        plan: Option<String>,
        windows: Vec<QuotaWindow>,
        extra: Option<ExtraCredits>,
    },
    NotConfigured { hint: String },
    Unsupported { reason: String },
    Error {
        message: String,
        /// Last **Available** snapshot only. Never box an Error (no nesting).
        stale: Option<Box<ProviderSnapshot>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSnapshot {
    pub id: String,
    pub display_name: String,
    pub glyph: String,
    pub ledger: LedgerKind,
    pub fetched_at: DateTime<Utc>,
    pub status: ProviderStatus,
    pub docs_url: Option<String>,
}

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
    fn is_frozen(&self) -> bool { false }
}

pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> { Utc::now() }
}

pub struct FrozenClock(pub DateTime<Utc>);
impl Clock for FrozenClock {
    fn now(&self) -> DateTime<Utc> { self.0 }
    fn is_frozen(&self) -> bool { true }
}

/// Offset stored in fixtures; materialized at fetch. Clone for `'static` fetch futures.
#[derive(Clone)]
pub struct WindowSpec {
    pub label: WindowLabel,
    pub used_percent: f32,
    pub resets_in: Duration,
    pub duration_mins: Option<u32>,
}
```

Both constructors share one clamp rule (`used`/`remaining` always sum to 100 after clamp).

**Codex footgun:** adapters call `from_used_percent` or `from_remaining_percent`. Renderers **never** compute remaining inline from a raw API field. Tests: `used=25 → remaining=75`; `used=0 → 100`; `used=100 → 0`; clamp `used=140 → 100/0`; `from_remaining_percent(40) → used=60 remaining=40`.

## `apply_fetch`

Live adapters typically return `Error { stale: None }`. Blind replace drops bars.

```rust
/// `stale` is always the last Available snapshot, never an Error.
pub fn last_available(snap: &ProviderSnapshot) -> Option<ProviderSnapshot> {
    match &snap.status {
        ProviderStatus::Available { .. } => Some(snap.clone()),
        ProviderStatus::Error { stale: Some(inner), .. } => last_available(inner),
        _ => None,
    }
}

pub fn apply_fetch(prev: Option<&ProviderSnapshot>, new_status: ProviderStatus) -> ProviderStatus {
    match new_status {
        ProviderStatus::Error { message, stale } => {
            let available = stale
                .as_deref()
                .and_then(last_available)
                .or_else(|| prev.and_then(last_available));
            ProviderStatus::Error {
                message,
                stale: available.map(Box::new),
            }
        }
        other => other,
    }
}
```

Tests: Available(18%) → `Error { stale: None }` still exposes 18% + footer `stale`. Second Error does **not** nest. `NotConfigured` / `Unsupported` clear bars.

**Paint rule:** hero bars when `ledger == PlanRemaining` **and** `effective_available(status)` is `Some`. Footer follows the outer status. Do **not** return `&ProviderSnapshot` from the `Available` arm.

```rust
#[derive(Clone, Copy)]
pub struct AvailableRef<'a> {
    pub plan: &'a Option<String>,
    pub windows: &'a [QuotaWindow],
    pub extra: &'a Option<ExtraCredits>,
}

pub fn effective_available(status: &ProviderStatus) -> Option<AvailableRef<'_>> {
    match status {
        ProviderStatus::Available { plan, windows, extra } => {
            Some(AvailableRef { plan, windows, extra })
        }
        ProviderStatus::Error { stale: Some(inner), .. } => match &inner.status {
            ProviderStatus::Available { plan, windows, extra } => {
                Some(AvailableRef { plan, windows, extra })
            }
            _ => None,
        },
        _ => None,
    }
}
```

Helpers in `domain.rs` (pure): `min_remaining`, `hero_window`, `secondary_window`, `sort_snapshots`, `apply_fetch`, `last_available`, `effective_available`.

Migration: none. When `--json` lands, add `schema_version: u32 = 1` at that PR — not now.

## v1 fixture set

`--fixture mixed` is the screenshot set. Fixtures store **offsets**, not absolute `resets_at`. `FixtureProvider` holds `Arc<dyn Clock>` and materializes `resets_at = clock.now() + offset` inside `fetch`. Tests inject `FrozenClock(2026-09-15T14:32:00Z)`.

| id | display | status | plan | session remaining | weekly remaining | extra | notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `codex` | Codex | Available | Plus | 18% (danger), 5h, in 1h12m | 63%, in 4d2h | none | sorts first |
| `grok` | Grok | Available | SuperGrok | *absent* | 27% (warn), in 3d4h | 400 cr teal | no fake 5h |
| `claude` | Claude | Available | Max 20x | 72%, 5h, in 2h05m | 41% (warn), in 5d11h | $12.40 teal | header can show 41% |
| `kimi` | Kimi | Available | Moderato | 55%, 5h, in 3h40m | 88%, in 6d1h | none | |
| `zai` | z.ai | NotConfigured | — | — | — | — | `export ZAI_API_KEY` |
| `muse` | Muse | Unsupported | Everyday | — | — | — | dashed unknown bar, no 0% |

`--fixture unsigned`: all six `NotConfigured` with per-provider login hints.

`--fixture danger`: every Available window `< 20%`.

`--fixture error` (seed before any paint, then one refresh):

1. `App::new` inserts mixed **Available** snapshots **without painting**.
2. Run **one** refresh. Codex `fetch` returns `Error { message: "401", stale: None }`. `apply_fetch` attaches the Available.
3. **First frame** is post-`apply_fetch` (bars + `stale` footer).

Other fixture sets start with empty `snapshots` and fill on the first refresh.

`FixtureProvider::fetch`: PR2 `Box::pin(std::future::ready(...))` (delay zero). PR4+: 150 ms sleep only when not `#[cfg(test)]` and not `clock.is_frozen()`.

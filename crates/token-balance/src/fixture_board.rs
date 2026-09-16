use crate::domain::{Clock, CreditUnit, ExtraCredits, WindowLabel, WindowSpec};
use crate::fixtures::{fp, FixtureProvider, Spec};
use chrono::Duration;
use std::sync::Arc;

fn spec_session_weekly(
    session_used: f32,
    session_in: Duration,
    weekly_used: f32,
    weekly_in: Duration,
) -> Vec<WindowSpec> {
    vec![
        WindowSpec {
            label: WindowLabel::FiveHour,
            used_percent: session_used,
            resets_in: session_in,
            duration_mins: Some(300),
        },
        WindowSpec {
            label: WindowLabel::Weekly,
            used_percent: weekly_used,
            resets_in: weekly_in,
            duration_mins: Some(10080),
        },
    ]
}

pub(crate) fn mixed_specs(clock: &Arc<dyn Clock>, error_codex: bool) -> Vec<FixtureProvider> {
    let usd = ExtraCredits {
        label: "extra usage".into(),
        remaining: 12.40,
        unit: CreditUnit::Usd,
        limit: Some(25.0),
    };
    let cr = ExtraCredits {
        label: "extra usage".into(),
        remaining: 400.0,
        unit: CreditUnit::Credits,
        limit: None,
    };
    let codex_spec = if error_codex {
        Spec::Fail {
            message: "401".into(),
        }
    } else {
        Spec::Available {
            plan: Some("Plus".into()),
            windows: spec_session_weekly(82.0, Duration::minutes(72), 37.0, Duration::hours(98)),
            extra: None,
        }
    };
    vec![
        fp(
            "codex",
            "Codex",
            "codex",
            Some("https://developers.openai.com/codex/"),
            clock,
            codex_spec,
        ),
        fp(
            "grok",
            "Grok",
            "grok",
            Some("https://grok.com"),
            clock,
            Spec::Available {
                plan: Some("SuperGrok".into()),
                windows: vec![WindowSpec {
                    label: WindowLabel::Weekly,
                    used_percent: 73.0,
                    resets_in: Duration::hours(76),
                    duration_mins: Some(10080),
                }],
                extra: Some(cr),
            },
        ),
        fp(
            "claude",
            "Claude",
            "claude",
            Some("https://docs.anthropic.com/"),
            clock,
            Spec::Available {
                plan: Some("Max 20x".into()),
                windows: spec_session_weekly(
                    28.0,
                    Duration::minutes(125),
                    59.0,
                    Duration::hours(131),
                ),
                extra: Some(usd),
            },
        ),
        fp(
            "kimi",
            "Kimi",
            "kimi",
            Some("https://www.kimi.com/en/help/kimi-code/benefits"),
            clock,
            Spec::Available {
                plan: Some("Moderato".into()),
                windows: spec_session_weekly(
                    45.0,
                    Duration::minutes(220),
                    12.0,
                    Duration::hours(145),
                ),
                extra: None,
            },
        ),
        fp(
            "zai",
            "z.ai",
            "zai",
            Some("https://docs.z.ai/devpack/usage-policy"),
            clock,
            Spec::NotConfigured {
                hint: "export ZAI_API_KEY".into(),
            },
        ),
        fp(
            "muse",
            "Muse",
            "muse",
            Some("https://ai.developer.meta.com/docs/muse-code/subscriptions/"),
            clock,
            Spec::Unsupported {
                reason: "no remaining API; polling burns Everyday requests".into(),
            },
        ),
    ]
}

pub(crate) fn unsigned_specs(clock: &Arc<dyn Clock>) -> Vec<FixtureProvider> {
    [
        ("codex", "Codex", "codex login"),
        ("grok", "Grok", "grok login"),
        ("claude", "Claude", "run claude"),
        ("kimi", "Kimi", "export KIMI_API_KEY"),
        ("zai", "z.ai", "export ZAI_API_KEY"),
        ("muse", "Muse", "muse login"),
    ]
    .into_iter()
    .map(|(id, name, hint)| {
        fp(
            id,
            name,
            id,
            None,
            clock,
            Spec::NotConfigured { hint: hint.into() },
        )
    })
    .collect()
}

pub(crate) fn danger_specs(clock: &Arc<dyn Clock>) -> Vec<FixtureProvider> {
    mixed_specs(clock, false)
        .into_iter()
        .map(|mut p| {
            if let Spec::Available {
                windows,
                extra,
                plan,
            } = &p.spec
            {
                let windows: Vec<WindowSpec> = windows
                    .iter()
                    .map(|w| WindowSpec {
                        used_percent: 90.0,
                        ..w.clone()
                    })
                    .collect();
                p.spec = Spec::Available {
                    plan: plan.clone(),
                    windows,
                    extra: extra.clone(),
                };
            }
            p
        })
        .collect()
}

pub(crate) fn multi_specs(clock: &Arc<dyn Clock>) -> Vec<FixtureProvider> {
    let claude = |id: &str, label: &str, used: f32| {
        fp(
            id,
            label,
            "claude",
            Some("https://docs.anthropic.com/"),
            clock,
            Spec::Available {
                plan: Some("Max 20x".into()),
                windows: spec_session_weekly(
                    used,
                    Duration::minutes(125),
                    59.0,
                    Duration::hours(131),
                ),
                extra: None,
            },
        )
    };
    let kimi = |id: &str, label: &str, used: f32| {
        fp(
            id,
            label,
            "kimi",
            Some("https://www.kimi.com/en/help/kimi-code/benefits"),
            clock,
            Spec::Available {
                plan: Some("Moderato".into()),
                windows: spec_session_weekly(
                    used,
                    Duration::minutes(220),
                    12.0,
                    Duration::hours(145),
                ),
                extra: None,
            },
        )
    };
    vec![
        claude("claude-work", "work", 28.0),
        claude("claude-home", "home", 40.0),
        kimi("kimi-team", "kimi team", 45.0),
        kimi("kimi-solo", "kimi solo", 50.0),
        fp(
            "codex",
            "codex",
            "codex",
            Some("https://developers.openai.com/codex/"),
            clock,
            Spec::Available {
                plan: Some("Plus".into()),
                windows: spec_session_weekly(82.0, Duration::minutes(72), 37.0, Duration::hours(98)),
                extra: None,
            },
        ),
        fp(
            "grok",
            "grok",
            "grok",
            Some("https://grok.com"),
            clock,
            Spec::Available {
                plan: Some("SuperGrok".into()),
                windows: vec![WindowSpec {
                    label: WindowLabel::Weekly,
                    used_percent: 73.0,
                    resets_in: Duration::hours(76),
                    duration_mins: Some(10080),
                }],
                extra: None,
            },
        ),
        fp(
            "zai",
            "z.ai",
            "zai",
            Some("https://docs.z.ai/devpack/usage-policy"),
            clock,
            Spec::NotConfigured {
                hint: "export ZAI_API_KEY".into(),
            },
        ),
        fp(
            "muse",
            "muse",
            "muse",
            Some("https://ai.developer.meta.com/docs/muse-code/subscriptions/"),
            clock,
            Spec::Unsupported {
                reason: "no remaining API; polling burns Everyday requests".into(),
            },
        ),
    ]
}

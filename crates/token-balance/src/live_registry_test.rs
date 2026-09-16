use super::*;
use crate::credentials::Credentials;
use crate::domain::{Clock, ProviderStatus};
use crate::fixtures::{FixtureSet, frozen_demo_clock};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static SEQ: AtomicU64 = AtomicU64::new(1);

fn acct_home() -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("tb-live-{}-{n}", std::process::id()));
    std::fs::create_dir_all(p.join(".config/token-balance")).unwrap();
    p
}

fn write_toml(home: &std::path::Path, body: &str) {
    std::fs::write(home.join(".config/token-balance/accounts.toml"), body).unwrap();
}

#[tokio::test]
async fn live_registry_missing_file_is_empty() {
    let list = live_registry(&Credentials::empty(), false).unwrap();
    assert!(list.is_empty(), "unlisted vendors must not auto-include");
}

#[tokio::test]
async fn live_registry_lists_only_file_rows() {
    let home = acct_home();
    write_toml(
        &home,
        r#"
[[account]]
vendor = "claude"
id = "claude-work"
label = "work"
credentials = ".missing-claude.json"

[[account]]
vendor = "kimi"
id = "kimi-team"
label = "team"
api_key_env = "KIMI_CODE_API_KEY"
"#,
    );
    let list = live_registry(&Credentials::isolated(home, BTreeMap::new()), false).unwrap();
    let ids: Vec<_> = list.iter().map(|p| p.id().to_string()).collect();
    assert_eq!(ids, vec!["claude-work", "kimi-team"]);
    assert!(list.iter().all(|p| p.vendor() != "muse"));
}

#[tokio::test]
async fn live_registry_two_claude_share_glyph() {
    let home = acct_home();
    write_toml(
        &home,
        r#"
[[account]]
vendor = "claude"
id = "claude-work"
label = "work"
credentials = ".a.json"
[[account]]
vendor = "claude"
id = "claude-home"
label = "home"
credentials = ".b.json"
"#,
    );
    let list = live_registry(&Credentials::isolated(home, BTreeMap::new()), false).unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].glyph_ascii(), "L");
    assert_eq!(list[1].glyph_ascii(), "L");
    assert_eq!(list[0].display_name(), "work");
    assert_eq!(list[1].display_name(), "home");
}

#[tokio::test]
async fn live_registry_two_codex_is_error() {
    let home = acct_home();
    write_toml(
        &home,
        r#"
[[account]]
vendor = "codex"
id = "codex-a"
label = "plus"
credentials = ".codex/auth.json"
[[account]]
vendor = "codex"
id = "codex-b"
label = "pro"
credentials = ".other/auth.json"
"#,
    );
    match live_registry(&Credentials::isolated(home, BTreeMap::new()), false) {
        Err(err) => assert!(err.contains("Codex"), "{err}"),
        Ok(_) => panic!("expected Codex singleton load error"),
    }
}

#[tokio::test]
async fn live_registry_inline_key_is_error() {
    let home = acct_home();
    write_toml(
        &home,
        r#"
[[account]]
vendor = "kimi"
id = "kimi-1"
label = "kimi"
api_key = "not-a-real-secret"
"#,
    );
    match live_registry(&Credentials::isolated(home, BTreeMap::new()), false) {
        Err(err) => assert!(err.contains("inline"), "{err}"),
        Ok(_) => panic!("expected inline key load error"),
    }
}

#[tokio::test]
async fn live_registry_missing_pointer_isolates_not_configured() {
    let home = acct_home();
    std::fs::create_dir_all(home.join(".kimi")).unwrap();
    std::fs::write(home.join(".kimi/key.json"), r#"{"api_key":"redacted"}"#).unwrap();
    write_toml(
        &home,
        r#"
[[account]]
vendor = "zai"
id = "zai-pack"
label = "z.ai"
api_key_env = "ZAI_API_KEY"

[[account]]
vendor = "kimi"
id = "kimi-team"
label = "team"
credentials = ".kimi/key.json"
"#,
    );
    let list = live_registry(&Credentials::isolated(home, BTreeMap::new()), false).unwrap();
    assert_eq!(list.len(), 2);
    let zai = list.iter().find(|p| p.vendor() == "zai").unwrap();
    match zai.fetch().await {
        ProviderStatus::NotConfigured { .. } => {}
        other => panic!("{other:?}"),
    }
    assert!(list.iter().any(|p| p.id() == "kimi-team"));
}

#[tokio::test]
async fn open_registry_fixture_ignores_accounts_file() {
    let home = acct_home();
    write_toml(
        &home,
        r#"
[[account]]
vendor = "codex"
id = "codex-a"
label = "plus"
credentials = ".a"
[[account]]
vendor = "codex"
id = "codex-b"
label = "pro"
credentials = ".b"
"#,
    );
    let clock: std::sync::Arc<dyn Clock> = std::sync::Arc::new(frozen_demo_clock());
    let list = open_registry(
        Some(FixtureSet::Mixed),
        &Credentials::isolated(home, BTreeMap::new()),
        false,
        clock,
    )
    .unwrap();
    assert_eq!(list.len(), 6);
    assert!(list.iter().any(|p| p.id() == "kimi"));
}

#[tokio::test]
async fn listed_muse_is_opt_in_not_unsupported() {
    let home = acct_home();
    write_toml(
        &home,
        r#"
[[account]]
vendor = "muse"
id = "muse"
label = "muse"
credentials = ".config/muse/auth.json"
"#,
    );
    let list = live_registry(&Credentials::isolated(home, BTreeMap::new()), false).unwrap();
    match list[0].fetch().await {
        ProviderStatus::NotConfigured { .. } => {}
        ProviderStatus::Unsupported { reason } => {
            panic!("listed muse must not be Unsupported: {reason}")
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn listed_muse_web_is_separate_from_muse() {
    let home = acct_home();
    write_toml(
        &home,
        r#"
[[account]]
vendor = "muse"
id = "muse"
label = "muse"
credentials = ".config/muse/auth.json"

[[account]]
vendor = "muse-web"
id = "muse-web"
label = "muse web"
credentials = ".config/token-balance/muse-web.json"
"#,
    );
    let list = live_registry(&Credentials::isolated(home, BTreeMap::new()), false).unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].vendor(), "muse");
    assert_eq!(list[0].glyph_ascii(), "M");
    assert_eq!(list[1].vendor(), "muse-web");
    assert_eq!(list[1].glyph_ascii(), "W");
    match list[1].fetch().await {
        ProviderStatus::NotConfigured { hint } => {
            assert!(hint.contains("muse-web.json"), "{hint}");
        }
        other => panic!("{other:?}"),
    }
}

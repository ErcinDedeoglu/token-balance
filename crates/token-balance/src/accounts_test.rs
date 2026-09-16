use super::*;
use crate::credentials::Credentials;
use std::collections::BTreeMap;
use std::path::PathBuf;


fn unique_home() -> PathBuf {
    let n = HOME_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("tb-acct-{}-{n}", std::process::id()));
    std::fs::create_dir_all(p.join(".config/token-balance")).unwrap();
    p
}

static HOME_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn creds(home: PathBuf) -> Credentials {
    Credentials::isolated(home, BTreeMap::new())
}

fn write_accounts(home: &std::path::Path, body: &str) {
    std::fs::write(home.join(REL_PATH), body).unwrap();
}

#[test]
fn parse_two_claude_and_kimi() {
    let rows = parse_accounts(
        r#"
[[account]]
vendor = "claude"
id = "claude-work"
label = "work"
credentials = ".claude-work/.credentials.json"

[[account]]
vendor = "kimi"
id = "kimi-team"
label = "team"
api_key_env = "KIMI_CODE_API_KEY"
"#,
    )
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, "claude-work");
    assert_eq!(rows[0].label, "work");
    assert_eq!(rows[0].vendor, Vendor::Claude);
    assert_eq!(rows[1].id, "kimi-team");
    assert!(rows.iter().all(|r| r.vendor != Vendor::Muse));
}

#[test]
fn two_codex_rows_fail() {
    let err = parse_accounts(
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
credentials = ".codex-b/auth.json"
"#,
    )
    .unwrap_err();
    assert!(err.contains("Codex"), "{err}");
}

#[test]
fn inline_key_field_fails() {
    let err = parse_accounts(
        r#"
[[account]]
vendor = "kimi"
id = "kimi-1"
label = "kimi"
api_key = "not-a-real-secret"
"#,
    )
    .unwrap_err();
    assert!(err.contains("inline"), "{err}");
    assert!(err.contains("api_key"), "{err}");
}

#[test]
fn unknown_vendor_fails() {
    let err = parse_accounts(
        r#"
[[account]]
vendor = "minimax"
id = "mm"
label = "mm"
api_key_env = "MM_KEY"
"#,
    )
    .unwrap_err();
    assert!(err.contains("unknown vendor"), "{err}");
}

#[test]
fn duplicate_id_and_label_fail() {
    let dup_id = parse_accounts(
        r#"
[[account]]
vendor = "claude"
id = "same"
label = "a"
credentials = "a.json"
[[account]]
vendor = "kimi"
id = "same"
label = "b"
api_key_env = "K"
"#,
    )
    .unwrap_err();
    assert!(dup_id.contains("duplicate id"), "{dup_id}");
    let dup_label = parse_accounts(
        r#"
[[account]]
vendor = "claude"
id = "one"
label = "work"
credentials = "a.json"
[[account]]
vendor = "kimi"
id = "two"
label = "work"
api_key_env = "K"
"#,
    )
    .unwrap_err();
    assert!(dup_label.contains("duplicate label"), "{dup_label}");
}

#[test]
fn missing_file_is_none() {
    let c = creds(PathBuf::from("/nonexistent-tb-acct-home"));
    assert_eq!(load_rows(&c).unwrap(), None);
}

#[test]
fn cap_25_fails_with_24() {
    let mut body = String::new();
    for i in 0..25 {
        body.push_str(&format!(
            "[[account]]\nvendor = \"kimi\"\nid = \"kimi-{i}\"\nlabel = \"k{i}\"\napi_key_env = \"K{i}\"\n\n"
        ));
    }
    let err = parse_accounts(&body).unwrap_err();
    assert!(err.contains("24"), "{err}");
}

#[test]
fn init_template_has_no_enabled_account() {
    let home = unique_home();
    let c = creds(home);
    let p = write_init(&c).unwrap();
    let text = std::fs::read_to_string(&p).unwrap();
    assert!(p.ends_with(REL_PATH));
    for line in text.lines() {
        let t = line.trim_start();
        assert!(
            !t.starts_with("[[account]]"),
            "uncommented account row:\n{text}"
        );
    }
    assert!(parse_accounts(&text).unwrap().is_empty());
    assert!(
        text.lines().any(|l| l.trim_start().starts_with("# [[account]]")),
        "template must include commented [[account]] headers:\n{text}"
    );
}

#[test]
fn init_template_uncommented_parses_to_rows() {
    let uncommented: String = INIT_TEMPLATE
        .lines()
        .map(|line| match line.strip_prefix("# ") {
            Some(rest) => rest,
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let rows = parse_accounts(&uncommented).unwrap_or_else(|e| panic!("{e}\n{uncommented}"));
    assert_eq!(rows.len(), 6, "{uncommented}");
    assert_eq!(rows[0].vendor, Vendor::Claude);
    assert_eq!(rows[0].id, "claude-work");
    assert_eq!(rows[1].vendor, Vendor::Kimi);
    assert_eq!(rows[2].vendor, Vendor::Codex);
    assert_eq!(rows[3].vendor, Vendor::Grok);
    assert_eq!(rows[4].vendor, Vendor::Kiro);
    assert_eq!(rows[5].vendor, Vendor::Deepseek);
}

#[test]
fn load_rows_reads_written_file() {
    let home = unique_home();
    write_accounts(
        &home,
        r#"
[[account]]
vendor = "zai"
id = "zai-pack"
label = "z.ai"
api_key_env = "ZAI_API_KEY"
"#,
    );
    let rows = load_rows(&creds(home)).unwrap().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].vendor, Vendor::Zai);
}

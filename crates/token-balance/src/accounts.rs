use crate::credentials::Credentials;
use std::collections::HashSet;
use std::path::PathBuf;

pub const REL_PATH: &str = ".config/token-balance/accounts.toml";
pub const MAX_ACCOUNTS: usize = 24;

pub const INIT_TEMPLATE: &str = "\
## token-balance accounts
## One row becomes one card. Pointers only: api_key_env or credentials (path).
## Codex: at most one row. Labels and ids must be unique across the file.
##
## Enable a row by copying an example below and removing the leading '# '.
##
# [[account]]
# vendor = \"claude\"
# id = \"claude-work\"
# label = \"work\"
# credentials = \".claude/.credentials.json\"
#
# [[account]]
# vendor = \"kimi\"
# id = \"kimi-team\"
# label = \"kimi team\"
# api_key_env = \"KIMI_CODE_API_KEY\"
#
# [[account]]
# vendor = \"codex\"
# id = \"codex\"
# label = \"codex\"
# credentials = \".codex/auth.json\"
#
# [[account]]
# vendor = \"grok\"
# id = \"grok\"
# label = \"grok\"
# credentials = \".grok/auth.json\"
#
# [[account]]
# vendor = \"kiro\"
# id = \"kiro\"
# label = \"kiro\"
# credentials = \"Library/Application Support/kiro-cli/data.sqlite3\"
#
# [[account]]
# vendor = \"deepseek\"
# id = \"deepseek\"
# label = \"deepseek\"
# api_key_env = \"DEEPSEEK_API_KEY\"
#
# [[account]]
# vendor = \"muse-web\"
# id = \"muse-web\"
# label = \"muse web\"
# credentials = \".config/token-balance/muse-web.json\"
";

const INLINE_KEYS: &[&str] = &[
    "api_key",
    "token",
    "access_token",
    "secret",
    "password",
    "key",
    "authorization",
];
const ALLOWED_KEYS: &[&str] = &["vendor", "id", "label", "api_key_env", "credentials"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vendor {
    Claude,
    Codex,
    Kimi,
    Grok,
    Zai,
    Kiro,
    Deepseek,
    Muse,
    MuseWeb,
}

impl Vendor {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "claude" => Ok(Self::Claude),
            "codex" => Ok(Self::Codex),
            "kimi" => Ok(Self::Kimi),
            "grok" => Ok(Self::Grok),
            "zai" => Ok(Self::Zai),
            "kiro" => Ok(Self::Kiro),
            "deepseek" => Ok(Self::Deepseek),
            "muse" => Ok(Self::Muse),
            "muse-web" => Ok(Self::MuseWeb),
            other => Err(format!("unknown vendor '{other}'")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Kimi => "kimi",
            Self::Grok => "grok",
            Self::Zai => "zai",
            Self::Kiro => "kiro",
            Self::Deepseek => "deepseek",
            Self::Muse => "muse",
            Self::MuseWeb => "muse-web",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pointer {
    Env(String),
    File(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRow {
    pub vendor: Vendor,
    pub id: String,
    pub label: String,
    pub pointer: Pointer,
}

pub fn path(creds: &Credentials) -> PathBuf {
    creds.home().join(REL_PATH)
}

pub fn load_rows(creds: &Credentials) -> Result<Option<Vec<AccountRow>>, String> {
    let p = path(creds);
    match std::fs::read_to_string(&p) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", p.display())),
        Ok(text) => parse_accounts(&text).map(Some),
    }
}

pub fn write_init(creds: &Credentials) -> Result<PathBuf, String> {
    let p = path(creds);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    if !p.exists() {
        std::fs::write(&p, INIT_TEMPLATE).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    Ok(p)
}

pub fn parse_accounts(text: &str) -> Result<Vec<AccountRow>, String> {
    let v: toml::Value = toml::from_str(text).map_err(|e| format!("accounts.toml: {e}"))?;
    let Some(table) = v.as_table() else {
        return Ok(Vec::new());
    };
    for k in table.keys() {
        if k != "account" {
            return Err(format!("accounts.toml: unknown key '{k}'"));
        }
    }
    let Some(arr) = table.get("account").and_then(|x| x.as_array()) else {
        return Ok(Vec::new());
    };
    if arr.len() > MAX_ACCOUNTS {
        return Err(format!(
            "accounts.toml: at most {MAX_ACCOUNTS} accounts, found {}",
            arr.len()
        ));
    }
    let mut rows = Vec::new();
    let mut ids = HashSet::new();
    let mut labels = HashSet::new();
    let mut codex = 0u32;
    for (i, item) in arr.iter().enumerate() {
        let row = parse_row(item, i)?;
        if row.vendor == Vendor::Codex {
            codex += 1;
            if codex > 1 {
                return Err("accounts.toml: only one Codex account is allowed".into());
            }
        }
        if !ids.insert(row.id.clone()) {
            return Err(format!("accounts.toml: duplicate id '{}'", row.id));
        }
        if !labels.insert(row.label.clone()) {
            return Err(format!("accounts.toml: duplicate label '{}'", row.label));
        }
        rows.push(row);
    }
    Ok(rows)
}

fn parse_row(item: &toml::Value, index: usize) -> Result<AccountRow, String> {
    let loc = format!("account[{index}]");
    let Some(t) = item.as_table() else {
        return Err(format!("accounts.toml: {loc} must be a table"));
    };
    for k in t.keys() {
        if INLINE_KEYS.contains(&k.as_str()) {
            return Err(format!(
                "accounts.toml: {loc} inline key field '{k}' is not allowed; use api_key_env or credentials"
            ));
        }
        if !ALLOWED_KEYS.contains(&k.as_str()) {
            return Err(format!("accounts.toml: {loc} unknown field '{k}'"));
        }
    }
    let vendor = Vendor::parse(&req_str(t, "vendor", &loc)?)?;
    let id = req_str(t, "id", &loc)?;
    let label = req_str(t, "label", &loc)?;
    let env = opt_str(t, "api_key_env");
    let file = opt_str(t, "credentials");
    let pointer = match (env, file) {
        (Some(e), _) => Pointer::Env(e),
        (None, Some(f)) => Pointer::File(f),
        (None, None) => {
            return Err(format!(
                "accounts.toml: {loc} needs api_key_env or credentials"
            ));
        }
    };
    Ok(AccountRow {
        vendor,
        id,
        label,
        pointer,
    })
}

fn req_str(t: &toml::Table, key: &str, loc: &str) -> Result<String, String> {
    let Some(v) = t.get(key).and_then(|x| x.as_str()) else {
        return Err(format!("accounts.toml: {loc} missing {key}"));
    };
    let s = v.trim();
    if s.is_empty() {
        return Err(format!("accounts.toml: {loc} empty {key}"));
    }
    Ok(s.to_string())
}

fn opt_str(t: &toml::Table, key: &str) -> Option<String> {
    t.get(key)
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
#[path = "accounts_test.rs"]
mod tests;

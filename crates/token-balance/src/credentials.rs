use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Process or test-injected home + env. Never logs values.
#[derive(Clone, Debug)]
pub struct Credentials {
    home: PathBuf,
    env: BTreeMap<String, String>,
}

impl Credentials {
    pub fn from_process() -> Self {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/"));
        let env = std::env::vars().collect();
        Self { home, env }
    }

    #[cfg(test)]
    pub fn isolated(home: PathBuf, env: BTreeMap<String, String>) -> Self {
        Self { home, env }
    }

    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            home: PathBuf::from("/nonexistent-token-balance-home"),
            env: BTreeMap::new(),
        }
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn env(&self, key: &str) -> Option<&str> {
        self.env.get(key).map(String::as_str)
    }

    pub fn read_to_string(&self, rel: &str) -> Option<String> {
        let path = self.home.join(rel);
        std::fs::read_to_string(path).ok()
    }
}

pub fn json_field(text: &str, keys: &[&str]) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    for key in keys {
        if let Some(s) = v.get(key).and_then(|x| x.as_str()) {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }
    if let Some(obj) = v.as_object() {
        for (_, val) in obj {
            if let Some(inner) = val.as_object() {
                for key in keys {
                    if let Some(s) = inner.get(*key).and_then(|x| x.as_str()) {
                        if !s.is_empty() {
                            return Some(s.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

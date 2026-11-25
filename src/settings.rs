//! A person's own settings and folders (REQ-CFG-4 to 6), never in a world
//! or a repository: `$XDG_CONFIG_HOME/thwarter/config.toml` for settings
//! (the persona written as by default, the writing helpers' service and
//! keys), `$XDG_DATA_HOME/thwarter/worlds/` for installed worlds, and
//! `$XDG_STATE_HOME/thwarter/` for play data. Each falls back to the XDG
//! default under the home folder.

use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};

/// The XDG folder `var` names, or `home` joined with `fallback`.
fn xdg(var: Option<String>, home: Option<String>, fallback: &str) -> Option<PathBuf> {
    match var.filter(|v| !v.is_empty() && Path::new(v).is_absolute()) {
        Some(dir) => Some(PathBuf::from(dir)),
        None => home.filter(|h| !h.is_empty()).map(|h| Path::new(&h).join(fallback)),
    }
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// `$XDG_CONFIG_HOME/thwarter`, else `~/.config/thwarter`.
pub fn config_dir() -> Option<PathBuf> {
    xdg(env("XDG_CONFIG_HOME"), env("HOME"), ".config").map(|d| d.join("thwarter"))
}

/// The person at the keyboard, for now the machine's user (REQ-WS-7):
/// `$THWART_USER` when set (tests, or a shared login), else `$USER`, else
/// `$USERNAME` on Windows. Their persona id is the same name until
/// choosing another persona is built.
pub fn current_user() -> Option<String> {
    ["THWART_USER", "USER", "USERNAME"].iter().filter_map(|k| env(k)).find(|u| !u.trim().is_empty())
}

/// `$XDG_DATA_HOME/thwarter`, else `~/.local/share/thwarter`. Installed
/// worlds go in its `worlds/<persona>/<world>/` (REQ-CFG-5).
pub fn data_dir() -> Option<PathBuf> {
    xdg(env("XDG_DATA_HOME"), env("HOME"), ".local/share").map(|d| d.join("thwarter"))
}

/// `$XDG_STATE_HOME/thwarter`, else `~/.local/state/thwarter`, for play
/// data such as saves (REQ-CFG-6).
pub fn state_dir() -> Option<PathBuf> {
    xdg(env("XDG_STATE_HOME"), env("HOME"), ".local/state").map(|d| d.join("thwarter"))
}

/// The settings in `config.toml`. Keys it doesn't know are kept as they
/// are and written back, so another tool's or a later version's settings
/// survive (REQ-CFG-3).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PersonalConfig {
    table: toml::Table,
    path: Option<PathBuf>,
}

impl PersonalConfig {
    /// The settings at the usual place; empty if there's no file yet.
    pub fn load() -> Result<Self> {
        let dir = config_dir().ok_or_else(|| anyhow!("neither XDG_CONFIG_HOME nor HOME is set, so there's nowhere for settings"))?;
        Self::load_from(&dir.join("config.toml"))
    }

    /// The settings in `path`; empty if it doesn't exist.
    pub fn load_from(path: &Path) -> Result<Self> {
        let table = match std::fs::read_to_string(path) {
            Ok(text) => text.parse::<toml::Table>().with_context(|| format!("{} doesn't parse", path.display()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => toml::Table::new(),
            Err(e) => return Err(e).with_context(|| format!("can't read {}", path.display())),
        };
        Ok(Self { table, path: Some(path.to_path_buf()) })
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The persona written as by default (`persona = "bob"`).
    pub fn persona(&self) -> Option<&str> {
        self.table.get("persona").and_then(|v| v.as_str())
    }

    pub fn set_persona(&mut self, id: &str) {
        self.table.insert("persona".into(), toml::Value::String(id.into()));
    }

    /// The writing helpers' settings (`[helpers]`: service, model, key),
    /// for the console (REQ-CON-19).
    pub fn helpers(&self) -> Option<&toml::Table> {
        self.table.get("helpers").and_then(|v| v.as_table())
    }

    /// Every setting, as a table.
    pub fn table(&self) -> &toml::Table {
        &self.table
    }

    pub fn table_mut(&mut self) -> &mut toml::Table {
        &mut self.table
    }

    /// Write the settings back, atomically, readable by the owner only,
    /// since they can hold service keys.
    pub fn save(&self) -> Result<()> {
        let path = self.path.as_ref().ok_or_else(|| anyhow!("these settings have no file to save to"))?;
        let dir = path.parent().ok_or_else(|| anyhow!("{} has no folder", path.display()))?;
        std::fs::create_dir_all(dir).with_context(|| format!("can't create {}", dir.display()))?;
        let text = toml::to_string_pretty(&self.table)?;
        let tmp = dir.join(format!(".config.toml.{}.tmp", std::process::id()));
        std::fs::write(&tmp, text).with_context(|| format!("can't write {}", tmp.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
        }
        std::fs::rename(&tmp, path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            anyhow!("can't save {}: {}", path.display(), e)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_follow_xdg_then_the_home_folder() {
        let s = |v: &str| Some(v.to_string());
        assert_eq!(xdg(s("/x/config"), s("/home/a"), ".config"), Some(PathBuf::from("/x/config")));
        assert_eq!(xdg(None, s("/home/a"), ".config"), Some(PathBuf::from("/home/a/.config")));
        // XDG says a relative path is to be ignored.
        assert_eq!(xdg(s("relative"), s("/home/a"), ".local/share"), Some(PathBuf::from("/home/a/.local/share")));
        assert_eq!(xdg(s(""), None, ".config"), None);
    }

    #[test]
    fn settings_round_trip_and_keep_keys_they_dont_know() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("thwarter/config.toml");
        let mut config = PersonalConfig::load_from(&path).unwrap();
        assert_eq!(config.persona(), None);
        config.set_persona("bob");
        config.save().unwrap();

        std::fs::write(&path, format!("{}\n[helpers]\nservice = \"anthropic\"\n\n[someone-else]\nkeep = true\n", std::fs::read_to_string(&path).unwrap())).unwrap();
        let mut config = PersonalConfig::load_from(&path).unwrap();
        assert_eq!(config.persona(), Some("bob"));
        assert_eq!(config.helpers().and_then(|h| h.get("service")).and_then(|v| v.as_str()), Some("anthropic"));
        config.set_persona("ann");
        config.save().unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("persona = \"ann\"") && text.contains("[someone-else]") && text.contains("keep = true"), "{}", text);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        }
        std::fs::write(&path, "not = [toml").unwrap();
        assert!(PersonalConfig::load_from(&path).is_err());
    }
}

//! Where play mode keeps game saves (REQ-CON-11): outside the world
//! folder, so saves never end up committed with a world's content.
//!
//! `$XDG_STATE_HOME/thwarter/saves/<world>-<hash>/<name>.yaml`, with
//! `~/.local/state` when `XDG_STATE_HOME` isn't set. `<world>` is the
//! world folder's name and `<hash>` six hex digits of its full path, so
//! two worlds with the same folder name keep separate saves.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

pub const DEFAULT_SAVE: &str = "quicksave";

/// The folder for `world_root`'s saves, under `state_home`.
pub fn save_dir(world_root: &Path, state_home: &Path) -> PathBuf {
    let full = world_root.canonicalize().unwrap_or_else(|_| world_root.to_path_buf());
    let name = full.file_name().and_then(|n| n.to_str()).unwrap_or("world");
    let hash = fnv1a(full.to_string_lossy().as_bytes());
    state_home.join("thwarter").join("saves").join(format!("{}-{:06x}", name, hash & 0xff_ffff))
}

/// The file for save `name`. Names are plain words, so a save can't
/// land outside its folder.
pub fn save_path(world_root: &Path, name: &str) -> Result<PathBuf> {
    let valid = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid {
        bail!("save names use letters, digits, `-` and `_`: `{}` doesn't", name);
    }
    Ok(save_dir(world_root, &state_home()?).join(format!("{}.yaml", name)))
}

/// The names of `world_root`'s saves, sorted.
pub fn list_saves(world_root: &Path) -> Result<Vec<String>> {
    let dir = save_dir(world_root, &state_home()?);
    let Ok(entries) = std::fs::read_dir(&dir) else { return Ok(Vec::new()) };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let path = e.path();
            (path.extension().and_then(|x| x.to_str()) == Some("yaml"))
                .then(|| path.file_stem().and_then(|s| s.to_str()).map(str::to_string))
                .flatten()
        })
        .collect();
    names.sort();
    Ok(names)
}

pub(super) fn state_home() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_STATE_HOME").filter(|d| !d.is_empty()) {
        return Ok(PathBuf::from(dir));
    }
    let home = std::env::var_os("HOME").context("neither XDG_STATE_HOME nor HOME is set, so there's nowhere to save")?;
    Ok(PathBuf::from(home).join(".local").join("state"))
}

/// FNV-1a, 64-bit: a hash that stays the same across Rust versions,
/// unlike `std`'s default hasher, so a world's save folder doesn't move.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, b| (hash ^ *b as u64).wrapping_mul(0x0000_0100_0000_01b3))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_folder_name_in_two_places_gets_two_save_folders() {
        let home = Path::new("/state");
        let a = save_dir(Path::new("/no/such/authors/mx/test-world"), home);
        let b = save_dir(Path::new("/no/such/authors/someone/test-world"), home);
        assert_ne!(a, b);
        assert!(a.starts_with("/state/thwarter/saves"));
        let folder = a.file_name().unwrap().to_str().unwrap();
        assert!(folder.starts_with("test-world-") && folder.len() == "test-world-".len() + 6, "{}", folder);
        assert_eq!(a, save_dir(Path::new("/no/such/authors/mx/test-world"), home), "stable");
    }

    #[test]
    fn save_names_stay_inside_the_folder() {
        assert!(save_path(Path::new("/w"), "../escape").is_err());
        assert!(save_path(Path::new("/w"), "").is_err());
        assert!(save_path(Path::new("/w"), "before-the-well_2").is_ok());
    }
}

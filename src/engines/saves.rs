//! Where games are saved, the same for every client (REQ-PC-18,
//! REQ-CFG-6): outside the world folder, so saves never end up committed
//! with a world's content.
//!
//! `$XDG_STATE_HOME/thwarter/saves/<world>-<hash>/<name>.yaml`, with
//! `~/.local/state` when `XDG_STATE_HOME` isn't set. `<world>` is the
//! world folder's name and `<hash>` six hex digits of its full path, so
//! two worlds with the same folder name keep separate saves.
//!
//! An installed world (`$XDG_DATA_HOME/thwarter/worlds/<persona>/<world>/`)
//! keys its saves by name instead, `saves/<persona>/<world>/`, so
//! reinstalling it, or installing it somewhere else, keeps them.

use crate::engines::{load_game, save_game, WorldEngine};
use crate::models::game::PlayState;
use anyhow::{anyhow, bail, Result};
use std::path::{Path, PathBuf};

/// The save `/save` and `/load` use when given no name.
pub const DEFAULT_SAVE: &str = "quicksave";

/// The folder for `world_root`'s saves, under `state_dir` (thwarter's
/// own state folder, `settings::state_dir`).
pub fn save_dir(world_root: &Path, state_dir: &Path) -> PathBuf {
    save_dir_for(world_root, state_dir, crate::packaging::installed_id(world_root))
}

/// `save_dir`, given what `installed_id` said about the world.
fn save_dir_for(world_root: &Path, state_dir: &Path, installed: Option<(String, String)>) -> PathBuf {
    let saves = state_dir.join("saves");
    if let Some((persona, world)) = installed {
        return saves.join(persona).join(world);
    }
    let full = world_root.canonicalize().unwrap_or_else(|_| world_root.to_path_buf());
    let name = full.file_name().and_then(|n| n.to_str()).unwrap_or("world");
    let hash = fnv1a(full.to_string_lossy().as_bytes());
    saves.join(format!("{}-{:06x}", name, hash & 0xff_ffff))
}

fn state_dir() -> Result<PathBuf> {
    crate::settings::state_dir().ok_or_else(|| anyhow!("neither XDG_STATE_HOME nor HOME is set, so there's nowhere to save"))
}

/// Whether `name` can name a save: letters, digits, `-` and `_`, so a
/// save can't land outside its folder.
fn check_name(name: &str) -> Result<()> {
    let valid = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid {
        bail!("save names use letters, digits, `-` and `_`: `{}` doesn't", name);
    }
    Ok(())
}

/// The file for save `name`.
pub fn save_path(world_root: &Path, name: &str) -> Result<PathBuf> {
    check_name(name)?;
    Ok(save_dir(world_root, &state_dir()?).join(format!("{}.yaml", name)))
}

/// The names of `world_root`'s saves, sorted.
pub fn list_saves(world_root: &Path) -> Result<Vec<String>> {
    Ok(saves_in(&save_dir(world_root, &state_dir()?)))
}

fn saves_in(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
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
    names
}

/// Save `state` as `name` for the world at `world_root`; returns the file
/// written.
pub fn save_named(world_root: &Path, name: &str, state: &PlayState) -> Result<PathBuf> {
    let path = save_path(world_root, name)?;
    save_game(&path, state)?;
    Ok(path)
}

/// Load save `name` for the world at `world_root`, reconciled with
/// `world`: the state, and a warning per thing that no longer exists.
pub fn load_named(world_root: &Path, name: &str, world: &WorldEngine) -> Result<(PlayState, Vec<String>)> {
    let path = save_path(world_root, name)?;
    if !path.exists() {
        bail!("there's no save `{}`", name);
    }
    load_game(&path, world)
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
        let state = Path::new("/state/thwarter");
        let a = save_dir(Path::new("/no/such/authors/mx/test-world"), state);
        let b = save_dir(Path::new("/no/such/authors/someone/test-world"), state);
        assert_ne!(a, b);
        assert!(a.starts_with("/state/thwarter/saves"));
        let folder = a.file_name().unwrap().to_str().unwrap();
        assert!(folder.starts_with("test-world-") && folder.len() == "test-world-".len() + 6, "{}", folder);
        assert_eq!(a, save_dir(Path::new("/no/such/authors/mx/test-world"), state), "stable");
    }

    #[test]
    fn the_hash_is_the_one_the_console_used() {
        // Saves made before the move must still be found.
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn an_installed_world_keys_its_saves_by_persona_and_world() {
        let data = tempfile::tempdir().unwrap();
        let world = data.path().join("worlds/mx/test-world");
        std::fs::create_dir_all(&world).unwrap();
        let installed = crate::packaging::installed_id_in(&world, data.path());
        assert_eq!(save_dir_for(&world, Path::new("/state/thwarter"), installed), Path::new("/state/thwarter/saves/mx/test-world"));
        // Anywhere else, the folder's name and a hash of its path.
        let elsewhere = tempfile::tempdir().unwrap();
        let installed = crate::packaging::installed_id_in(elsewhere.path(), data.path());
        let dir = save_dir_for(elsewhere.path(), Path::new("/state/thwarter"), installed);
        assert!(dir.parent() == Some(Path::new("/state/thwarter/saves")), "{}", dir.display());
    }

    #[test]
    fn save_names_stay_inside_the_folder() {
        assert!(check_name("../escape").is_err());
        assert!(check_name("").is_err());
        assert!(check_name("a/b").is_err());
        assert!(check_name("before-the-well_2").is_ok());
    }

    #[test]
    fn saves_are_listed_by_name() {
        let dir = tempfile::tempdir().unwrap();
        for f in ["b.yaml", "a.yaml", "notes.txt", "c.yaml.tmp-1"] {
            std::fs::write(dir.path().join(f), "").unwrap();
        }
        assert_eq!(saves_in(dir.path()), ["a", "b"]);
        assert!(saves_in(&dir.path().join("none")).is_empty());
    }
}

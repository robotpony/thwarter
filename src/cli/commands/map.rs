//! `thwart map` (`design/CLI_DESIGN.md` › `map`): an ASCII map of the
//! world from a room, out to `--depth` exits.

use crate::engines::WorldEngine;
use crate::render::render_map;
use anyhow::{anyhow, bail, Result};

pub fn map_command(
    world_path: String,
    location: Option<String>,
    player: Option<String>,
    depth: Option<usize>,
    style: String,
    output: Option<String>,
    connections: bool,
) -> Result<()> {
    // Options specified but not built say so (REQ-CLI-7).
    if player.is_some() {
        bail!("--player (a map of what one player has seen) is not implemented yet");
    }
    if style != "ascii" {
        bail!("--style {} is not implemented yet; only ascii is", style);
    }
    if connections {
        bail!("--connections is not implemented yet");
    }
    let mut engine = WorldEngine::new();
    engine.load_world(&world_path)?;
    let from = match location {
        Some(name) => {
            let id = engine.resolve_name(name.trim_end_matches('/'), Some(crate::engines::EntityKind::Location));
            if engine.get_location(&id).is_none() {
                bail!("No location `{}` in {}", name, world_path);
            }
            id
        }
        None => engine
            .world()
            .start
            .clone()
            .filter(|s| engine.get_location(s).is_some())
            .or_else(|| {
                let mut ids: Vec<&String> = engine.loaded_locations.keys().collect();
                ids.sort();
                ids.first().map(|s| s.to_string())
            })
            .ok_or_else(|| anyhow!("{} has no locations to map", world_path))?,
    };
    let text = render_map(&engine.map_layout(&from, depth, None));
    match output {
        Some(path) => {
            std::fs::write(&path, format!("{}\n", text)).map_err(|e| anyhow!("can't write {}: {}", path, e))?;
            println!("Wrote the map to {}", path);
        }
        None => println!("{}", text),
    }
    Ok(())
}

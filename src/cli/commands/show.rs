use crate::cli::{OutputFormat, OutputFormatter};
use crate::engines::WorldEngine;
use crate::render::render_location;
use anyhow::Result;

pub fn show_command(
    world_path: String,
    location: Option<String>,
    player: Option<String>,
    state: Option<String>,
    format: String,
    verbose: bool,
) -> Result<()> {
    let formatter = OutputFormatter::new(OutputFormat::from(format.as_str())).with_verbose(verbose);

    if player.is_some() {
        formatter.print_warning("--player is not implemented yet; showing world state only");
    }
    if state.is_some() {
        formatter.print_warning("--state is not implemented yet; saved state cannot be loaded");
    }

    let mut engine = WorldEngine::new();
    let report = engine.load_world(&world_path)?;

    if !report.errors.is_empty() {
        for error in &report.errors {
            formatter.print_warning(error);
        }
    }

    match location {
        Some(loc) => match engine.get_location(&loc) {
            Some(found) => match formatter.format {
                OutputFormat::Text => {
                    if let Some(view) = engine.describe_location(&loc) {
                        println!("{}", render_location(&view));
                    }
                }
                _ => formatter.print_data(found)?,
            },
            None => {
                return Err(anyhow::anyhow!(
                    "Location `{}` not found in {} (loaded {} location(s))",
                    loc,
                    world_path,
                    report.locations_loaded
                ))
            }
        },
        None => {
            formatter.print_info(&format!(
                "World at {}: {} location(s), {} character(s), {} object(s)",
                world_path,
                report.locations_loaded,
                report.characters.len(),
                report.objects_loaded
            ));
            for id in engine.world_graph.locations.keys() {
                println!("  - {}", id);
            }
        }
    }

    Ok(())
}

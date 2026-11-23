use crate::cli::{OutputFormat, OutputFormatter};
use crate::engines::{WorldEngine, WorldLoadReport};
use crate::models::world::Location;
use anyhow::Result;

/// Compass and vertical directions in the order an author reads them.
/// Exits with any other name sort after these, alphabetically.
const DIRECTION_ORDER: &[&str] = &[
    "north", "northeast", "east", "southeast", "south", "southwest", "west", "northwest", "up", "down",
    "in", "out",
];

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
                OutputFormat::Text => print_location(found, &engine, &report),
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

/// Prints a location as an author would check it: title, description, exits,
/// then who and what is there. Unknown exit targets are flagged rather than hidden.
fn print_location(location: &Location, engine: &WorldEngine, report: &WorldLoadReport) {
    println!("{} ({})", location.name, location.id);
    if let Some(c) = &location.coordinates {
        println!("at ({}, {}, {})", c.x, c.y, c.z);
    }
    println!();
    println!("{}", location.description.trim());

    let mut exits: Vec<(&String, &String)> = location.connections.iter().collect();
    exits.sort_by_key(|(dir, _)| {
        let rank = DIRECTION_ORDER.iter().position(|d| d == &dir.as_str()).unwrap_or(DIRECTION_ORDER.len());
        (rank, dir.to_string())
    });
    println!();
    if exits.is_empty() {
        println!("Exits: none");
    } else {
        println!("Exits:");
        let width = exits.iter().map(|(dir, _)| dir.len()).max().unwrap_or(0);
        for (dir, target) in exits {
            let missing = if engine.get_location(target).is_none() { "  (missing)" } else { "" };
            println!("  {:<width$}  {}{}", dir, target, missing, width = width);
        }
    }

    print_named_list(
        "Characters",
        &location.characters,
        |id| report.characters.iter().find(|c| &c.id == id).map(|c| c.name.clone()),
    );
    print_named_list(
        "Objects",
        &location.objects,
        |id| engine.objects.get(id).map(|o| o.title.clone()),
    );
}

/// Prints `id (Name)` per entry, or `id  (not found)` when no card matches.
fn print_named_list(heading: &str, ids: &[String], lookup: impl Fn(&String) -> Option<String>) {
    println!();
    if ids.is_empty() {
        println!("{}: none", heading);
        return;
    }
    println!("{}:", heading);
    for id in ids {
        match lookup(id) {
            Some(name) => println!("  {} ({})", name, id),
            None => println!("  {}  (not found)", id),
        }
    }
}

use crate::cli::{OutputFormat, OutputFormatter};
use crate::engines::{At, EntityKind, WorldEngine};
use crate::models::game::PlayState;
use crate::parsers::ParsedEntity;
use crate::render::render_location;
use anyhow::Result;

pub fn show_command(
    world_path: String,
    location: Option<String>,
    player: Option<String>,
    state: Option<String>,
    chapter: Option<String>,
    format: String,
    verbose: bool,
) -> Result<()> {
    let formatter = OutputFormatter::new(OutputFormat::from(format.as_str())).with_verbose(verbose);

    if player.is_some() {
        formatter.print_warning("--player is not implemented yet; showing world state only");
    }

    let mut engine = WorldEngine::new();
    let report = engine.load_world(&world_path)?;

    for message in report.errors.iter().chain(&report.warnings) {
        formatter.print_warning(message);
    }

    if location.is_none() && (state.is_some() || chapter.is_some()) {
        return Err(anyhow::anyhow!("--state and --chapter need a location to show"));
    }

    match location {
        Some(loc) => {
            if engine.get_location(&loc).is_none() {
                return Err(anyhow::anyhow!(
                    "Location `{}` not found in {} (loaded {} location(s))",
                    loc,
                    world_path,
                    report.locations_loaded
                ));
            }
            // The starting version unless a state or chapter is asked for.
            let version = engine.version(EntityKind::Location, &loc, state.as_deref(), chapter.as_deref())?;
            let mut play = PlayState::new(loc.clone());
            if let (Some(name), Some(thing)) = (&version.state, engine.thing(EntityKind::Location, &loc)) {
                play.set_state("location", &loc, name, thing.starting_state.as_deref());
            }
            if let Some(key) = &version.chapter {
                if let Some(c) = engine.chapters().iter().find(|c| &c.key == key) {
                    play.chapter = Some(engine.saved_chapter(c));
                }
            }
            match formatter.format {
                OutputFormat::Text => {
                    if let Some(view) = engine.describe_location_at(&loc, At::play(&play)) {
                        println!("{}", render_location(&view));
                    }
                }
                _ => {
                    if let ParsedEntity::Location(found) = &version.entity {
                        formatter.print_data(found)?
                    }
                }
            }
        }
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

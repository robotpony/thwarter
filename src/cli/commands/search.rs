//! `thwart search` (`design/CLI_DESIGN.md` › `search`, REQ-CLI-18): text
//! over things' ids, titles and prose, or the world's broken exits.

use crate::engines::search::{Field, Matcher};
use crate::engines::{EntityKind, WorldEngine};
use crate::render::clean_text;
use anyhow::{anyhow, bail, Result};

pub fn search_command(
    world_path: String,
    query: Option<String>,
    location: Option<String>,
    search_type: Option<String>,
    case_sensitive: bool,
    regex: bool,
    missing_connections: bool,
) -> Result<()> {
    let mut engine = WorldEngine::new();
    engine.load_world(&world_path)?;

    if missing_connections {
        if query.is_some() {
            bail!("--missing-connections takes no query");
        }
        let under = location.as_deref().map(|l| l.trim_end_matches('/'));
        let broken: Vec<(String, String, String)> = engine
            .broken_connections()
            .into_iter()
            .filter(|(from, _, _)| under.map_or(true, |u| from == u || from.starts_with(&format!("{}/", u))))
            .collect();
        if broken.is_empty() {
            println!("No broken exits.");
        }
        for (from, direction, target) in broken {
            println!("{}", clean_text(&format!("{}: {} → {} (not written yet)", from, direction, target)));
        }
        return Ok(());
    }

    let query = query.ok_or_else(|| anyhow!("search needs a query, or --missing-connections"))?;
    let (kinds, text_only) = match search_type.as_deref() {
        None => (Vec::new(), false),
        Some("text") => (Vec::new(), true),
        Some(t) => (
            vec![EntityKind::from_type_name(t).ok_or_else(|| anyhow!("--type is location, object, character, door or text, not `{}`", t))?],
            false,
        ),
    };
    let matcher = if regex {
        Matcher::regex(&query, case_sensitive).map_err(|e| anyhow!("`{}` isn't a regular expression this can use: {}", query, e))?
    } else {
        Matcher::text(&query, case_sensitive)
    };
    let hits = engine.search(&matcher, &kinds, text_only, location.as_deref());
    if hits.is_empty() {
        println!("No matches.");
        return Ok(());
    }
    for hit in &hits {
        let line = match hit.field {
            Field::Id | Field::Title => format!("{} {}  {}", hit.kind.type_name(), hit.id, hit.title),
            Field::Text => format!("{} {}  {}\n    {}", hit.kind.type_name(), hit.id, hit.title, hit.snippet.as_deref().unwrap_or("")),
        };
        println!("{}", clean_text(&line));
    }
    eprintln!("{} match{}", hits.len(), if hits.len() == 1 { "" } else { "es" });
    Ok(())
}

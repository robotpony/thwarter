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

    // A lone argument that is a folder (`castle/`, or `castle` if the
    // world has that folder) is where to search, not what to search for.
    let names_folder = |q: &str| -> bool {
        if q.ends_with('/') {
            return true;
        }
        // Exactly, whatever the file system's case rules, and not when the
        // word is also a thing's id, which it more likely means.
        let path = std::path::Path::new(&world_path).join(q);
        let exact = path.parent().and_then(|p| std::fs::read_dir(p).ok()).map_or(false, |entries| {
            entries.filter_map(|e| e.ok()).any(|e| e.file_name().to_string_lossy() == q.rsplit('/').next().unwrap_or(q) && e.path().is_dir())
        });
        exact && engine.kinds_with_id(q).is_empty()
    };
    let (query, location) = match (query, location) {
        (Some(q), None) if names_folder(&q) => (None, Some(q)),
        other => other,
    };
    // No query, with a kind or a place, lists everything there.
    let query = match query.filter(|q| !q.is_empty()) {
        Some(q) => q,
        None if search_type.is_some() || location.is_some() => {
            let kinds_or_text = match search_type.as_deref() {
                Some("text") => bail!("--type text needs a query, since it matches prose"),
                Some(t) => vec![EntityKind::from_type_name(t).ok_or_else(|| anyhow!("--type is location, object, character, door or text, not `{}`", t))?],
                None => Vec::new(),
            };
            let hits = engine.search(&Matcher::All, &kinds_or_text, false, location.as_deref());
            return print_hits(&hits);
        }
        None => bail!("search needs a query, a --type or a location to list, or --missing-connections"),
    };
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
    print_hits(&hits)
}

fn print_hits(hits: &[crate::engines::search::Hit]) -> Result<()> {
    if hits.is_empty() {
        println!("No matches.");
        return Ok(());
    }
    for hit in hits {
        let line = match hit.field {
            Field::Id | Field::Title => format!("{} {}  {}", hit.kind.type_name(), hit.id, hit.title),
            Field::Text => format!("{} {}  {}\n    {}", hit.kind.type_name(), hit.id, hit.title, hit.snippet.as_deref().unwrap_or("")),
        };
        println!("{}", clean_text(&line));
    }
    eprintln!("{} match{}", hits.len(), if hits.len() == 1 { "" } else { "es" });
    Ok(())
}

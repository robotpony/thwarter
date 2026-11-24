//! `thwart show` (REQ-CLI-5, 9 to 11): the world's summary, any thing in
//! any of its versions, or a template.

use crate::cli::{OutputFormat, OutputFormatter};
use crate::engines::{At, EntityKind, WorldEngine};
use crate::models::game::PlayState;
use crate::parsers::ParsedEntity;
use crate::render::{render_character, render_location, render_object, render_thing_sources};
use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use std::path::Path;

/// What `thwart show` was asked for.
pub struct ShowRequest {
    pub world_path: String,
    /// A thing's id or link; `None` for the world's summary.
    pub name: Option<String>,
    /// `location`, `object` or `character`, when an id names more than one.
    pub kind: Option<String>,
    /// A template or built-in to show as written.
    pub template: Option<String>,
    pub player: Option<String>,
    pub state: Option<String>,
    pub chapter: Option<String>,
    pub format: String,
    pub verbose: bool,
}

pub fn show_command(req: ShowRequest) -> Result<()> {
    let formatter = OutputFormatter::new(OutputFormat::from(req.format.as_str())).with_verbose(req.verbose);
    if req.player.is_some() {
        formatter.print_warning("--player is not implemented yet; showing world state only");
    }

    let mut engine = WorldEngine::new();
    let report = engine.load_world(&req.world_path)?;
    // A count on stderr, never the list: stdout holds only the result, so
    // `--format json` is one document. `validate` lists them.
    if let Some(note) = load_note(report.errors.len(), report.warnings.len()) {
        eprintln!("{}", note);
    }

    if let Some(name) = &req.template {
        if req.name.is_some() || req.state.is_some() || req.chapter.is_some() {
            bail!("--template shows a template on its own; leave out the thing, --state and --chapter");
        }
        return show_template(&engine, name, &formatter);
    }
    match &req.name {
        Some(name) => show_thing(&engine, name, &req, &formatter),
        None => {
            if req.state.is_some() || req.chapter.is_some() {
                bail!("--state and --chapter need a thing to show");
            }
            show_world(&engine, &req.world_path, &formatter)
        }
    }
}

/// "2 problems and 27 warnings; run `thwart validate` to list them", or
/// `None` for a clean world.
fn load_note(problems: usize, warnings: usize) -> Option<String> {
    let count = |n: usize, what: &str| format!("{} {}{}", n, what, if n == 1 { "" } else { "s" });
    let parts: Vec<String> = [(problems, "problem"), (warnings, "warning")]
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, w)| count(n, w))
        .collect();
    if parts.is_empty() {
        return None;
    }
    Some(format!("{}; run `thwart validate` to list them", parts.join(" and ")))
}

/// The kinds a name could mean, in the order a bare name is tried.
fn kinds_for(engine: &WorldEngine, name: &str, kind: Option<&str>) -> Result<Vec<(EntityKind, String)>> {
    let wanted: Vec<EntityKind> = match kind {
        None => vec![EntityKind::Location, EntityKind::Object, EntityKind::Character, EntityKind::Door],
        Some(k) => vec![EntityKind::from_type_name(k).ok_or_else(|| anyhow!("--kind is location, object, character or door, not `{}`", k))?],
    };
    Ok(wanted
        .into_iter()
        .filter_map(|k| {
            let id = engine.resolve_name(name, Some(k));
            engine.thing(k, &id).map(|_| (k, id))
        })
        .collect())
}

fn show_thing(engine: &WorldEngine, name: &str, req: &ShowRequest, formatter: &OutputFormatter) -> Result<()> {
    let found = kinds_for(engine, name, req.kind.as_deref())?;
    let (kind, id) = match found.as_slice() {
        [] => {
            let what = req.kind.as_deref().unwrap_or("thing");
            let why = engine.link_ambiguity(name, None).map(|w| format!(": {}", w)).unwrap_or_default();
            bail!(
                "No {} `{}` in {} ({} location(s), {} object(s), {} character(s) loaded){}",
                what,
                name,
                req.world_path,
                engine.loaded_locations.len(),
                engine.objects.len(),
                engine.characters.len(),
                why
            )
        }
        [one] => one.clone(),
        many => {
            let kinds: Vec<String> = many
                .iter()
                .map(|(k, _)| {
                    let n = k.type_name();
                    let article = if n.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" };
                    format!("{} {}", article, n)
                })
                .collect();
            bail!("`{}` names {}; say which with --kind", name, kinds.join(" and "))
        }
    };

    // The starting version unless a state or chapter is asked for.
    let version = engine.version(kind, &id, req.state.as_deref(), req.chapter.as_deref())?;
    if !matches!(formatter.format, OutputFormat::Text) {
        return match &version.entity {
            ParsedEntity::Location(l) => formatter.print_data(l),
            ParsedEntity::Object(o) => formatter.print_data(o),
            ParsedEntity::Character(c) => formatter.print_data(c),
            ParsedEntity::Door(d) => formatter.print_data(d),
        };
    }

    let text = match &version.entity {
        ParsedEntity::Location(_) => {
            let mut play = PlayState::new(id.clone());
            if let (Some(name), Some(thing)) = (&version.state, engine.thing(kind, &id)) {
                play.set_state(kind.type_name(), &id, name, thing.starting_state.as_deref());
            }
            if let Some(key) = &version.chapter {
                if let Some(c) = engine.chapters().iter().find(|c| &c.key == key) {
                    play.chapter = Some(engine.saved_chapter(c));
                }
            }
            let view = engine.describe_location_at(&id, At::play(&play)).ok_or_else(|| anyhow!("can't describe `{}`", id))?;
            render_location(&view)
        }
        ParsedEntity::Object(o) => render_object(o),
        ParsedEntity::Character(c) => render_character(c),
        ParsedEntity::Door(d) => crate::render::render_door(d),
    };
    println!("{}", text);
    if let Some(thing) = engine.thing(kind, &id) {
        let chapter = version.chapter.as_ref().map(|k| k.path.join("/"));
        let sources = render_thing_sources(thing, req.state.as_deref(), chapter.as_deref());
        if !sources.is_empty() {
            println!("\n{}", sources);
        }
    }
    Ok(())
}

fn show_template(engine: &WorldEngine, name: &str, formatter: &OutputFormatter) -> Result<()> {
    let layer = engine.find_template(name, Path::new("")).ok_or_else(|| {
        anyhow!("No template `{}`. Templates here: {}", name, engine.template_names().join(", "))
    })?;
    let builtin = crate::engines::builtins::builtin_name(&layer.path);
    // The template's files: its one file, or a bundle's index then states.
    let files: Vec<(String, String)> = match builtin {
        Some(b) => crate::engines::builtins::builtin_files(b).into_iter().map(|(p, t)| (p.display().to_string(), t.to_string())).collect(),
        None => {
            let mut files = vec![(layer.path.display().to_string(), std::fs::read_to_string(&layer.path)?)];
            if layer.path.file_name().map_or(false, |n| n == "index.md") {
                let mut states: Vec<std::path::PathBuf> = std::fs::read_dir(layer.path.parent().unwrap())?
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.extension().map_or(false, |e| e == "md") && p.file_name().map_or(false, |n| n != "index.md"))
                    .collect();
                states.sort();
                for state in states {
                    files.push((state.display().to_string(), std::fs::read_to_string(&state)?));
                }
            }
            files
        }
    };
    if !matches!(formatter.format, OutputFormat::Text) {
        #[derive(Serialize, Debug)]
        struct FileOut {
            path: String,
            text: String,
        }
        #[derive(Serialize, Debug)]
        struct TemplateOut<'a> {
            name: &'a str,
            built_in: bool,
            files: Vec<FileOut>,
        }
        let files = files.into_iter().map(|(path, text)| FileOut { path, text }).collect();
        return formatter.print_data(&TemplateOut { name, built_in: builtin.is_some(), files });
    }
    let source = if builtin.is_some() { format!("built in, {}", files[0].0) } else { files[0].0.clone() };
    let mut out = vec![format!("Template {} ({})", name, source), String::new(), files[0].1.trim_end().to_string()];
    for (path, text) in files.iter().skip(1) {
        out.push(String::new());
        out.push(format!("State file {}:", path));
        out.push(text.trim_end().to_string());
    }
    println!("{}", out.join("\n"));
    Ok(())
}

#[derive(Serialize, Debug)]
struct WorldSummary {
    title: Option<String>,
    author: Option<String>,
    version: Option<String>,
    start: Option<String>,
    intro: String,
    story: std::collections::BTreeMap<String, String>,
    player: Option<PlayerSummary>,
    chapters: Vec<ChapterSummary>,
    locations: Vec<String>,
    objects: Vec<String>,
    characters: Vec<String>,
}

#[derive(Serialize, Debug)]
struct PlayerSummary {
    title: String,
    vars: std::collections::BTreeMap<String, String>,
}

#[derive(Serialize, Debug)]
struct ChapterSummary {
    scope: String,
    path: String,
    order: Vec<u64>,
    title: Option<String>,
}

/// The world's summary (REQ-CLI-11): its `world/` definition, story and
/// player values, chapters, and every thing's id.
fn show_world(engine: &WorldEngine, world_path: &str, formatter: &OutputFormatter) -> Result<()> {
    let w = engine.world();
    let sorted = |ids: Vec<&String>| -> Vec<String> {
        let mut v: Vec<String> = ids.into_iter().cloned().collect();
        v.sort();
        v
    };
    let summary = WorldSummary {
        title: w.title.clone(),
        author: w.author.clone(),
        version: w.version.clone(),
        start: w.start.clone(),
        intro: w.intro.clone(),
        story: w.story.clone(),
        player: w.player.as_ref().map(|p| PlayerSummary { title: p.title.clone(), vars: p.vars.clone() }),
        chapters: engine
            .chapters()
            .iter()
            .map(|c| ChapterSummary {
                scope: c.key.scope.to_string_lossy().replace('\\', "/"),
                path: c.key.path.join("/"),
                order: c.order.clone(),
                title: c.title().map(str::to_string),
            })
            .collect(),
        locations: sorted(engine.loaded_locations.keys().collect()),
        objects: sorted(engine.objects.keys().collect()),
        characters: sorted(engine.characters.keys().collect()),
    };
    if !matches!(formatter.format, OutputFormat::Text) {
        return formatter.print_data(&summary);
    }

    let mut out = vec![summary.title.clone().unwrap_or_else(|| world_path.to_string())];
    let mut byline = Vec::new();
    if let Some(a) = &summary.author {
        byline.push(format!("by {}", a));
    }
    if let Some(v) = &summary.version {
        byline.push(format!("version {}", v));
    }
    if !byline.is_empty() {
        out.push(byline.join(", "));
    }
    out.push(format!("Starts in: {}", summary.start.as_deref().unwrap_or("(not set)")));
    if !summary.intro.trim().is_empty() {
        out.push(String::new());
        out.push(summary.intro.trim().to_string());
    }
    let values = |vars: &std::collections::BTreeMap<String, String>| -> String {
        vars.iter().map(|(k, v)| format!("{} = {}", k, v)).collect::<Vec<_>>().join(", ")
    };
    out.push(String::new());
    out.push(if summary.story.is_empty() { "Story values: none".to_string() } else { format!("Story values: {}", values(&summary.story)) });
    out.push(match &summary.player {
        None => "Player: not defined (world/player.md)".to_string(),
        Some(p) if p.vars.is_empty() => format!("Player: {}", p.title),
        Some(p) => format!("Player: {} ({})", p.title, values(&p.vars)),
    });
    out.push(String::new());
    if summary.chapters.is_empty() {
        out.push("Chapters: none".to_string());
    } else {
        out.push("Chapters:".to_string());
        for c in &summary.chapters {
            let order: Vec<String> = c.order.iter().map(u64::to_string).collect();
            let scope = if c.scope.is_empty() { String::new() } else { format!("  (in {}/)", c.scope) };
            let title = c.title.as_deref().map(|t| format!("  {}", t)).unwrap_or_default();
            out.push(format!("  {}  {}{}{}", order.join("."), c.path, title, scope));
        }
    }
    for (label, ids) in [("Locations", &summary.locations), ("Objects", &summary.objects), ("Characters", &summary.characters)] {
        out.push(String::new());
        out.push(format!("{} ({}):", label, ids.len()));
        for id in ids {
            out.push(format!("  - {}", id));
        }
    }
    println!("{}", out.join("\n"));
    Ok(())
}

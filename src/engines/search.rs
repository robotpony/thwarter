//! Search (`design/CLI_DESIGN.md` › `search`, PLAN Phase 10): text over
//! every thing's id, title and prose, in its starting version. Tags wait
//! on the console's tag-search design (REQ-CF-30).

use super::{EntityKind, WorldEngine};
use crate::parsers::ParsedEntity;
use serde::Serialize;

/// How a query matches text.
pub enum Matcher {
    /// Plain text, with or without case.
    Text { query: String, case_sensitive: bool },
    Regex(regex::Regex),
    /// Everything: for listing the things of a kind or in a place.
    All,
}

impl Matcher {
    pub fn text(query: &str, case_sensitive: bool) -> Self {
        Matcher::Text { query: if case_sensitive { query.to_string() } else { query.to_lowercase() }, case_sensitive }
    }

    /// A regular expression; case-insensitive unless asked. The `regex`
    /// crate runs in linear time, so no pattern can hang a search.
    pub fn regex(pattern: &str, case_sensitive: bool) -> Result<Self, regex::Error> {
        regex::RegexBuilder::new(pattern).case_insensitive(!case_sensitive).size_limit(1 << 20).build().map(Matcher::Regex)
    }

    /// Where `text` matches, as a byte range.
    pub fn find(&self, text: &str) -> Option<std::ops::Range<usize>> {
        match self {
            Matcher::Text { query, case_sensitive } => {
                if query.is_empty() {
                    return None;
                }
                if *case_sensitive {
                    text.find(query.as_str()).map(|i| i..i + query.len())
                } else {
                    // Lowercase char by char, keeping the original's byte offsets.
                    let lower: Vec<(usize, char)> = text.char_indices().flat_map(|(i, c)| c.to_lowercase().map(move |l| (i, l))).collect();
                    let haystack: String = lower.iter().map(|(_, c)| c).collect();
                    let at = haystack.find(query.as_str())?;
                    let start_char = haystack[..at].chars().count();
                    let end_char = start_char + query.chars().count();
                    let start = lower.get(start_char)?.0;
                    let end = lower.get(end_char).map_or(text.len(), |(i, _)| *i);
                    Some(start..end.max(start))
                }
            }
            Matcher::Regex(re) => re.find(text).map(|m| m.range()),
            Matcher::All => Some(0..0),
        }
    }
}

/// Which part of a thing matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Field {
    Id,
    Title,
    Text,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Hit {
    pub kind: EntityKind,
    pub id: String,
    pub title: String,
    pub field: Field,
    /// The matching line of prose, cut to about 80 characters around the
    /// match, for a text match.
    pub snippet: Option<String>,
}

/// The line around `range`, cut to about 80 characters.
fn snippet(text: &str, range: std::ops::Range<usize>) -> String {
    let start = text[..range.start].rfind('\n').map_or(0, |i| i + 1);
    let end = text[range.end..].find('\n').map_or(text.len(), |i| range.end + i);
    let line = &text[start..end];
    let (at, len) = (range.start - start, range.end - range.start);
    if line.chars().count() <= 80 {
        return line.trim().to_string();
    }
    let before: String = line[..at].chars().rev().take(30).collect::<Vec<_>>().into_iter().rev().collect();
    let after: String = line[at + len..].chars().take(40).collect();
    format!("…{}{}{}…", before, &line[at..at + len], after)
}

impl WorldEngine {
    /// Every thing whose id, title or prose matches, of the given kinds (all
    /// if empty), sorted by kind then id. With `under`, only things whose
    /// files are in that folder (`castle/`), and the things its rooms hold
    /// or where characters start.
    pub fn search(&self, matcher: &Matcher, kinds: &[EntityKind], text_only: bool, under: Option<&str>) -> Vec<Hit> {
        let under = under.map(|u| u.trim_end_matches('/').to_string());
        let in_area = |room: &str| under.as_ref().map_or(true, |u| room == u || room.starts_with(&format!("{}/", u)));
        let rooms: Vec<&String> = self.loaded_locations.keys().filter(|r| in_area(r)).collect();
        let root = self.world_root.clone().unwrap_or_default();
        let filed_under = |kind: EntityKind, id: &str| -> bool {
            let (Some(u), Some(thing)) = (under.as_ref(), self.thing(kind, id)) else { return false };
            thing.index.path.strip_prefix(&root).map_or(false, |rel| rel.starts_with(u))
        };
        let placed = |kind: EntityKind, id: &str| -> bool {
            if under.is_none() || filed_under(kind, id) {
                return true;
            }
            match kind {
                EntityKind::Location => in_area(id),
                EntityKind::Object => rooms.iter().any(|r| self.get_location(r).map_or(false, |l| l.objects.iter().any(|o| o == id))),
                EntityKind::Character => {
                    rooms.iter().any(|r| self.get_location(r).map_or(false, |l| l.characters.iter().any(|c| c == id)))
                        || self.characters.get(id).and_then(|c| c.default_location.as_deref()).map_or(false, in_area)
                }
                EntityKind::Door => self.doors.get(id).map_or(false, |d| d.sides.iter().any(|s| in_area(&s.room))),
            }
        };
        let mut hits = Vec::new();
        let mut things: Vec<(EntityKind, String)> = self
            .things_iter()
            .map(|t| (t.kind, t.id.clone()))
            .filter(|(k, _)| kinds.is_empty() || kinds.contains(k))
            .collect();
        things.sort();
        for (kind, id) in things {
            if !placed(kind, &id) {
                continue;
            }
            let Some(entity) = self.current(kind, &id, super::At::default()) else { continue };
            let title = entity.title().to_string();
            let prose = match &entity {
                ParsedEntity::Location(l) => l.description.clone(),
                ParsedEntity::Character(c) => c.description.clone(),
                ParsedEntity::Object(o) => {
                    let mut text = o.description.clone();
                    let mut names: Vec<&String> = o.responses.keys().collect();
                    names.sort();
                    for name in names {
                        text.push('\n');
                        text.push_str(&o.responses[name]);
                    }
                    text
                }
                ParsedEntity::Door(d) => d.description.clone(),
            };
            let hit = |field, snippet| Hit { kind, id: id.clone(), title: title.clone(), field, snippet };
            if !text_only && matcher.find(&id).is_some() {
                hits.push(hit(Field::Id, None));
            } else if !text_only && matcher.find(&title).is_some() {
                hits.push(hit(Field::Title, None));
            } else if let Some(range) = matcher.find(&prose) {
                hits.push(hit(Field::Text, Some(snippet(&prose, range))));
            }
        }
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn write(dir: &Path, rel: &str, text: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn world() -> (tempfile::TempDir, WorldEngine) {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write(d, "castle/hall.md", "---\nid: castle/hall\ntitle: Great Hall\ntype: location\ncharacters: [wizard]\nobjects: [key]\n---\nBanners hang. A Golden Key glints on the table.\n");
        write(d, "forest/path.md", "---\nid: forest/path\ntitle: Path\ntype: location\n---\nTrees.\n");
        write(d, "npcs/wizard.md", "---\nid: wizard\ntitle: Old Wizard\ntype: character\n---\nHe mutters.\n");
        write(d, "npcs/ranger.md", "---\nid: ranger\ntitle: Ranger\ntype: character\ndefault_location: forest/path\n---\nA wizard once taught her.\n");
        write(d, "objects/key.md", "---\nid: key\ntitle: Golden key\ntype: object\nobject_type: key\n---\nHeavy.\n\n<!-- thwarter:action turn -->\nIt turns with a golden click.\n");
        let mut engine = WorldEngine::new();
        engine.load_world(d).unwrap();
        (dir, engine)
    }

    fn ids(hits: &[Hit]) -> Vec<(&str, Field)> {
        hits.iter().map(|h| (h.id.as_str(), h.field)).collect()
    }

    #[test]
    fn text_matches_id_title_or_prose_without_case() {
        let (_d, engine) = world();
        let hits = engine.search(&Matcher::text("golden key", false), &[], false, None);
        assert_eq!(ids(&hits), [("castle/hall", Field::Text), ("key", Field::Title)]);
        assert_eq!(hits[0].snippet.as_deref(), Some("Banners hang. A Golden Key glints on the table."));
        assert!(engine.search(&Matcher::text("golden key", true), &[], false, None).iter().all(|h| h.id != "castle/hall"));

        let wizards = engine.search(&Matcher::text("wizard", false), &[EntityKind::Character], false, None);
        assert_eq!(ids(&wizards), [("ranger", Field::Text), ("wizard", Field::Id)]);
        // Responses are searched as prose.
        assert_eq!(ids(&engine.search(&Matcher::text("click", false), &[], true, None)), [("key", Field::Text)]);
    }

    #[test]
    fn a_location_limits_search_to_its_rooms_and_what_they_hold() {
        let (_d, engine) = world();
        let in_castle = engine.search(&Matcher::text("", false), &[], false, Some("castle/"));
        assert!(in_castle.is_empty()); // an empty query matches nothing
        let hits = engine.search(&Matcher::regex(".", false).unwrap(), &[], false, Some("castle/"));
        assert_eq!(ids(&hits).iter().map(|(id, _)| *id).collect::<Vec<_>>(), ["castle/hall", "wizard", "key"]);
        let hits = engine.search(&Matcher::regex("^for", false).unwrap(), &[], false, Some("forest"));
        assert_eq!(ids(&hits), [("forest/path", Field::Id)]);
        assert!(Matcher::regex("(", false).is_err());
    }
}

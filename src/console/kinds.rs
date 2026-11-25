//! What kind of reply each `[console]` key is: done, refused, a warning,
//! a note. The kind is the console's, not the author's, so it lives here
//! and not in the text table. Colour and `/recipe`'s failure check read
//! it, so rewording a reply in config.toml changes neither.

use super::theme::Role;
use crate::engines::texts::{Entry, Texts};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Something was done: saved, made, moved.
    Done,
    /// It wasn't done, and why.
    Error,
    /// Done, but with something to watch out for.
    Warning,
    /// An author note under a reply.
    Note,
    /// Something not written yet.
    Missing,
    /// A usage line or a refusal: nothing was done, nothing went wrong.
    Refused,
}

impl Kind {
    /// How a line of this kind is drawn.
    pub fn role(self) -> Option<Role> {
        match self {
            Kind::Done => Some(Role::Message),
            Kind::Error => Some(Role::Error),
            Kind::Warning => Some(Role::Warning),
            Kind::Note => Some(Role::Note),
            Kind::Missing => Some(Role::Missing),
            Kind::Refused => None,
        }
    }

    /// Whether a command whose reply has this kind didn't do its work.
    pub fn failed(self) -> bool {
        matches!(self, Kind::Error | Kind::Refused)
    }
}

/// Each key with a kind. A key not listed is plain text.
const KINDS: &[(&str, Kind)] = &[
    ("usage_bare", Kind::Refused),
    ("unknown_command", Kind::Error),
    ("list_what", Kind::Refused),
    ("edit_look_toward_none", Kind::Missing),
    ("edit_look_toward_no_room", Kind::Refused),
    ("reload_failed", Kind::Error),
    ("reload_failed_because", Kind::Error),
    ("reloaded", Kind::Done),
    ("problems_heading", Kind::Error),
    ("warnings_heading", Kind::Warning),
    ("exit_not_written", Kind::Warning),
    ("room_thing_no_card", Kind::Missing),
    ("inspect_warnings", Kind::Warning),
    ("usage_details", Kind::Refused),
    ("usage_room_and_elsewhere", Kind::Refused),
    ("usage_new_object", Kind::Refused),
    ("usage_for_example", Kind::Refused),
    ("set_on", Kind::Done),
    ("linked", Kind::Done),
    ("unlinked", Kind::Done),
    ("moved_thing", Kind::Done),
    ("not_moved", Kind::Error),
    ("moved_room", Kind::Done),
    ("cant_write_world", Kind::Error),
    ("not_created", Kind::Error),
    ("not_saved", Kind::Error),
    ("watch_failed", Kind::Error),
    ("player_nowhere", Kind::Note),
    ("cant_start_there", Kind::Error),
    ("game_saved", Kind::Done),
    ("game_not_saved", Kind::Error),
    ("game_not_loaded_name", Kind::Error),
    ("game_not_loaded", Kind::Error),
    ("game_loaded", Kind::Done),
    ("load_author_note", Kind::Note),
    ("not_removed", Kind::Error),
    ("cant_restart", Kind::Error),
    ("restart_doesnt_start", Kind::Error),
    ("restart_cant_run", Kind::Error),
    ("restart_no_state_dir", Kind::Error),
    ("restart_cant_save", Kind::Error),
    ("restart_failed", Kind::Error),
    ("restarted_on", Kind::Done),
    ("edit_mode_at", Kind::Done),
    ("edit_saved_parses", Kind::Done),
    ("edit_saved", Kind::Done),
    ("edit_cant_open", Kind::Error),
    ("edit_not_saved", Kind::Error),
    ("edit_not_same_thing", Kind::Error),
    ("states_need_a_thing", Kind::Refused),
    ("kind_has_no_states", Kind::Refused),
    ("set_in_state", Kind::Done),
    ("bad_state_name", Kind::Refused),
    ("not_made", Kind::Error),
    ("cant_read", Kind::Error),
    ("not_written_yet", Kind::Error),
    ("not_on_to_write", Kind::Refused),
    ("cant_read_world_index", Kind::Error),
    ("watching_failed", Kind::Error),
    ("adds_problems", Kind::Warning),
    ("play_mode", Kind::Done),
    ("reply_with_note", Kind::Note),
    ("changed_on_disk", Kind::Done),
    ("editor_changed_on_disk", Kind::Warning),
    ("editor_reloaded", Kind::Done),
    ("editor_unsaved", Kind::Warning),
    ("editor_modified", Kind::Warning),
    ("usage", Kind::Refused),
    ("game_note", Kind::Note),
    ("created_room", Kind::Done),
    ("created_room_linked", Kind::Done),
    ("created_room_linked_one_way", Kind::Done),
    ("created_character", Kind::Done),
    ("created_character_placed", Kind::Done),
    ("created_character_listed", Kind::Done),
    ("created_object", Kind::Done),
    ("created_object_placed", Kind::Done),
    ("created_object_listed", Kind::Done),
    ("moved_room_updated_one", Kind::Done),
    ("moved_room_updated", Kind::Done),
    ("created_room_link_failed", Kind::Error),
    ("created_character_place_failed", Kind::Error),
    ("created_object_place_failed", Kind::Error),
    ("moved_character_and_location", Kind::Done),
    ("made_state", Kind::Done),
    ("made_state_start", Kind::Done),
    ("action_made", Kind::Done),
    ("action_made_with", Kind::Done),
    ("state_exists", Kind::Done),
    ("state_exists_start", Kind::Done),
    ("set_needs_a_thing", Kind::Refused),
    ("unlink_needs_room", Kind::Refused),
    ("cant_hold_itself", Kind::Refused),
    ("room_already_at_root", Kind::Refused),
    ("room_already_in_area", Kind::Refused),
    ("door_no_opposite", Kind::Refused),
    ("edit_kind_here", Kind::Refused),
    ("edit_folder", Kind::Refused),
    ("edit_not_a_thing", Kind::Refused),
    ("edit_needs_terminal_for", Kind::Refused),
    ("edit_only_command", Kind::Refused),
    ("cant_be_changed", Kind::Error),
    ("files_dont_parse", Kind::Error),
    ("reload_warnings", Kind::Warning),
    ("warnings_summary", Kind::Warning),
];

/// `key`'s kind, if it has one.
pub fn kind_of(key: &str) -> Option<Kind> {
    KINDS.iter().find(|(k, _)| *k == key).map(|(_, kind)| *kind)
}

/// The lines each kinded key prints, as the author's current wording
/// makes them, for telling a line's kind once it's in the output.
pub struct Kinds {
    /// A template line's literal pieces between placeholders, whether it
    /// starts and ends with a placeholder, and the key's kind.
    patterns: Vec<(Vec<String>, bool, bool, Kind)>,
}

impl Kinds {
    pub fn new(texts: &Texts) -> Self {
        let mut patterns = Vec::new();
        for (key, kind) in KINDS {
            let Some(entry) = texts.entry("console", key) else { continue };
            let templates: Vec<&String> = match entry {
                Entry::Text(text) => vec![text],
                Entry::Choices(texts) => texts.iter().collect(),
                Entry::Flag(_) => continue,
            };
            for line in templates.iter().flat_map(|t| t.lines()) {
                if let Some((pieces, open, close)) = pattern(line) {
                    patterns.push((pieces, open, close, *kind));
                }
            }
        }
        // The most literal text first, so the most specific pattern wins.
        patterns.sort_by_key(|(pieces, ..)| std::cmp::Reverse(pieces.iter().map(String::len).sum::<usize>()));
        Kinds { patterns }
    }

    /// The kind of an output line, from the template it was made from.
    pub fn of_line(&self, line: &str) -> Option<Kind> {
        self.patterns.iter().find(|(pieces, open, close, _)| matches(line, pieces, *open, *close)).map(|p| p.3)
    }
}

impl Default for Kinds {
    fn default() -> Self {
        Kinds::new(&Texts::defaults())
    }
}

/// A template line as literal pieces, or None when it's only
/// placeholders and spaces, which would match any line.
fn pattern(line: &str) -> Option<(Vec<String>, bool, bool)> {
    let mut pieces = Vec::new();
    let mut rest = line;
    let open = line.starts_with('{');
    let mut close = false;
    while let Some(at) = rest.find('{') {
        let Some(end) = rest[at..].find('}') else { break };
        if at > 0 {
            pieces.push(rest[..at].to_string());
        }
        rest = &rest[at + end + 1..];
        close = rest.is_empty();
    }
    if !rest.is_empty() {
        pieces.push(rest.to_string());
    }
    if pieces.iter().all(|p| p.trim().is_empty()) {
        return None;
    }
    Some((pieces, open, close))
}

/// Whether `line` is `pieces` with anything in the placeholders.
fn matches(line: &str, pieces: &[String], open: bool, close: bool) -> bool {
    let mut at = 0;
    for (i, piece) in pieces.iter().enumerate() {
        let first = i == 0 && !open;
        let last = i == pieces.len() - 1 && !close;
        if first {
            if !line.starts_with(piece.as_str()) {
                return false;
            }
            at = piece.len();
        } else if last {
            if line.len() < at + piece.len() || !line.ends_with(piece.as_str()) {
                return false;
            }
            at = line.len();
        } else {
            match line[at..].find(piece.as_str()) {
                Some(found) => at += found + piece.len(),
                None => return false,
            }
        }
        if first && last && at != line.len() {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kinded_key_is_a_console_key() {
        let texts = Texts::defaults();
        for (key, _) in KINDS {
            assert!(texts.entry("console", key).is_some(), "kinds.rs names `{}`, which console.toml doesn't have", key);
        }
    }

    #[test]
    fn a_line_is_known_by_its_template_even_reworded() {
        let kinds = Kinds::default();
        assert_eq!(kinds.of_line("Not saved: bad YAML"), Some(Kind::Error));
        assert_eq!(kinds.of_line("Author note: the inn isn't written"), Some(Kind::Note));
        assert_eq!(kinds.of_line("A mossy stone well."), None);
        let mut texts = Texts::defaults();
        let value: toml::Value = "[console]\ncant_restart = \"Restarting broke ({reason}).\"\n".parse().unwrap();
        texts.apply("console", value.get("console"), "config.toml", &mut Vec::new());
        let kinds = Kinds::new(&texts);
        assert_eq!(kinds.of_line("Restarting broke (no terminal)."), Some(Kind::Error));
        assert_eq!(kinds.of_line("Can't restart: no terminal"), None, "the old wording is gone");
    }
}

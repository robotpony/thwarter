//! `/remove <thing>`: deletes an object, room, character or door and every
//! reference the writer can rewrite (Bruce, 2026-11-24). There's no undo
//! yet, so it shows the writer's plan first and the next line answers y/n.

use super::author::entity_kind;
use super::doors::Asked;
use super::session::{suggest, Console};
use super::world::Kind;

impl Console {
    /// `/remove <name>`: shows what would go and asks.
    pub(super) fn remove(&mut self, args: &[&str]) -> String {
        let typed = args.join(" ");
        if typed.trim().is_empty() {
            return suggest(&self.t("remove_what", &[]), &["/remove {thing}"]);
        }
        let Some((kind, id)) = self.removable(&typed) else { return self.t("nothing_called", &[("name", &typed)]) };
        let Some(entity) = entity_kind(kind) else { return self.t("cant_be_removed", &[("id", &id)]) };
        let writer = match crate::writer::WorldWriter::new(&self.world.root) {
            Ok(w) => w,
            Err(e) => return self.t("cant_write_world", &[("reason", &e.to_string())]),
        };
        let plan = match writer.plan_remove(&self.world.engine, entity, &id) {
            Ok(plan) => plan,
            Err(e) => return self.t("not_removed", &[("reason", &e.to_string())]),
        };
        let mut out = vec![self.t("remove_ask", &[("id", &id), ("kind", kind.label())])];
        let others: Vec<String> = plan.removed.iter().skip(1).map(|(_, i)| i.clone()).collect();
        if !others.is_empty() {
            out.push(format!("  also removes  {}", others.join(", ")));
        }
        for path in &plan.deleted {
            out.push(format!("  deletes  {}", path.display()));
        }
        for (path, what) in &plan.changed {
            out.push(format!("  changes  {}  {}", path.display(), what));
        }
        if !plan.by_hand.is_empty() {
            out.push(String::new());
            out.push(self.t("left_to_fix", &[]));
            out.extend(plan.by_hand.iter().map(|b| format!("  {}", b)));
        }
        out.push(String::new());
        out.push(self.t("remove_no_undo", &[]));
        self.asked = Some(Asked::Remove { kind, id });
        out.join("\n")
    }

    /// The answer to "Remove it?".
    pub(super) fn confirm_remove(&mut self, kind: Kind, id: &str, line: &str) -> String {
        if !matches!(line.trim().to_lowercase().as_str(), "y" | "yes") {
            return self.t("remove_kept", &[("id", id)]);
        }
        let Some(entity) = entity_kind(kind) else { return self.t("cant_be_removed", &[("id", &id)]) };
        let writer = match crate::writer::WorldWriter::new(&self.world.root) {
            Ok(w) => w,
            Err(e) => return self.t("cant_write_world", &[("reason", &e.to_string())]),
        };
        if self.warnings_before.is_none() {
            self.warnings_before = Some(self.world.warnings());
        }
        let report = match writer.remove_loaded(&mut self.world.engine, entity, id, false) {
            Ok(r) => r,
            Err(e) => return self.t("not_removed", &[("reason", &e.to_string())]),
        };
        let mut summary = vec![self.t(
            "removed",
            &[
                ("ids", &report.removed.iter().map(|(_, i)| i.as_str()).collect::<Vec<_>>().join(", ")),
                ("deleted", &report.deleted.len().to_string()),
                ("changed", &report.written.len().to_string()),
            ],
        )];
        if !report.by_hand.is_empty() {
            summary.push(self.t("left_to_fix", &[]));
            summary.extend(report.by_hand.iter().map(|b| format!("  {}", b)));
        }
        let touched: Vec<_> = report.written.iter().chain(&report.deleted).cloned().collect();
        let out = self.after_writes(&touched, summary.join("\n"));
        if !self.place_exists() {
            self.move_to_existing_folder();
        }
        out
    }

    /// A thing, door or room by name.
    fn removable(&self, typed: &str) -> Option<(Kind, String)> {
        let (id, _) = super::author::id_from(typed);
        for kind in [Kind::Object, Kind::Character, Kind::Door, Kind::Room] {
            if self.world.files.values().any(|e| e.kind == kind && e.id == id) {
                return Some((kind, id));
            }
        }
        let room = self.room_id(typed);
        self.world.engine.get_location(&room).is_some().then_some((Kind::Room, room))
    }
}

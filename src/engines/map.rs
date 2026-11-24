//! Maps and paths (`design/CLI_DESIGN.md` › `map`, PLAN Phase 10). Plain
//! data, no colours or wrapping, so the console can draw it at its own
//! width and `thwart map` can print it (`render::render_map`). Agreed with
//! the console coder, 2026-11-23.

use super::{At, WorldEngine};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

/// A door an exit goes through.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MapDoor {
    pub id: String,
    pub title: String,
    pub passable: bool,
}

/// One room on the grid. `y` grows southward, `x` eastward.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MapCell {
    pub room: String,
    /// Its title; `None` for a room that isn't written yet.
    pub title: Option<String>,
    pub written: bool,
    /// The room the map was drawn from.
    pub here: bool,
    pub x: i32,
    pub y: i32,
}

/// An exit between two cells on the grid.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MapLink {
    pub from: (i32, i32),
    pub to: (i32, i32),
    /// The direction from `from`.
    pub direction: String,
    pub door: Option<MapDoor>,
}

/// An exit that doesn't fit the grid: up, down, a non-compass direction,
/// or a compass one whose cell is taken. Shown as "up → castle/tower".
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Beside {
    pub room: String,
    pub direction: String,
    pub target: String,
    pub written: bool,
    pub door: Option<MapDoor>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Map {
    pub cells: Vec<MapCell>,
    pub links: Vec<MapLink>,
    pub beside: Vec<Beside>,
}

/// Where a room reached only off the grid (up, down, or a non-compass
/// exit) is kept while its exits are followed; it never gets a cell.
const OFF_GRID: (i32, i32) = (i32::MIN, i32::MIN);

/// The grid step for a compass direction.
fn step(direction: &str) -> Option<(i32, i32)> {
    Some(match direction {
        "north" => (0, -1),
        "south" => (0, 1),
        "east" => (1, 0),
        "west" => (-1, 0),
        "northeast" => (1, -1),
        "northwest" => (-1, -1),
        "southeast" => (1, 1),
        "southwest" => (-1, 1),
        _ => return None,
    })
}

impl WorldEngine {
    /// A room's exits as `(direction, room it leads to, door)`, in compass
    /// order: through a door to its other side (in its starting version).
    pub fn exits_of(&self, room: &str) -> Vec<(String, String, Option<MapDoor>)> {
        let Some(location) = self.get_location(room) else { return Vec::new() };
        let mut exits: Vec<(String, String, Option<MapDoor>)> = location
            .connections
            .iter()
            .filter_map(|(direction, target)| {
                if self.get_location(target).is_none() {
                    if let Some(door) = self.door_at(target, At::default()) {
                        let to = door.leads_to(room)?.to_string();
                        return Some((direction.clone(), to, Some(MapDoor { id: door.id.clone(), title: door.title.clone(), passable: door.passable })));
                    }
                }
                Some((direction.clone(), target.clone(), None))
            })
            .collect();
        exits.sort_by_key(|(d, _, _)| crate::engines::location_view::direction_rank(d));
        exits
    }

    /// Every room reachable from `from`, with the fewest exits it takes,
    /// nearest first, then by id (for completion order). `from` is 0.
    pub fn distances(&self, from: &str) -> Vec<(String, usize)> {
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut queue = VecDeque::new();
        if self.get_location(from).is_none() {
            return Vec::new();
        }
        seen.insert(from.to_string(), 0);
        queue.push_back(from.to_string());
        while let Some(room) = queue.pop_front() {
            let d = seen[&room];
            for (_, to, _) in self.exits_of(&room) {
                if self.get_location(&to).is_some() && !seen.contains_key(&to) {
                    seen.insert(to.clone(), d + 1);
                    queue.push_back(to);
                }
            }
        }
        let mut out: Vec<(String, usize)> = seen.into_iter().collect();
        out.sort_by(|a, b| (a.1, &a.0).cmp(&(b.1, &b.0)));
        out
    }

    /// The shortest way from `from` to `to`, as room ids from one to the
    /// other, through doors whatever their state; `None` if there's none.
    pub fn find_path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        if self.get_location(from).is_none() || self.get_location(to).is_none() {
            return None;
        }
        let mut came_from: HashMap<String, String> = HashMap::new();
        let mut queue = VecDeque::from([from.to_string()]);
        let mut seen = BTreeSet::from([from.to_string()]);
        while let Some(room) = queue.pop_front() {
            if room == to {
                let mut path = vec![room.clone()];
                let mut at = room;
                while let Some(prev) = came_from.get(&at) {
                    path.push(prev.clone());
                    at = prev.clone();
                }
                path.reverse();
                return Some(path);
            }
            for (_, next, _) in self.exits_of(&room) {
                if self.get_location(&next).is_some() && seen.insert(next.clone()) {
                    came_from.insert(next.clone(), room.clone());
                    queue.push_back(next);
                }
            }
        }
        None
    }

    /// A map from `from`, out to `depth` exits away (all reachable rooms
    /// without one), showing only rooms in `only` if given. Rooms take grid
    /// cells by their compass exits, in the order they're reached; an exit
    /// that can't go on the grid is listed `beside` it. Unwritten rooms an
    /// exit names are cells too, marked missing.
    pub fn map_layout(&self, from: &str, depth: Option<usize>, only: Option<&BTreeSet<String>>) -> Map {
        let mut map = Map::default();
        if self.get_location(from).is_none() {
            return map;
        }
        let shown = |room: &str| only.map_or(true, |set| set.contains(room));
        let mut at: BTreeMap<String, (i32, i32)> = BTreeMap::new();
        let mut taken: HashMap<(i32, i32), String> = HashMap::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        let place = |map: &mut Map, room: &str, xy: (i32, i32), engine: &WorldEngine| {
            let title = engine.get_location(room).map(|l| l.name.clone());
            map.cells.push(MapCell { room: room.to_string(), written: title.is_some(), title, here: room == from, x: xy.0, y: xy.1 });
        };
        at.insert(from.to_string(), (0, 0));
        taken.insert((0, 0), from.to_string());
        place(&mut map, from, (0, 0), self);
        queue.push_back((from.to_string(), 0));
        let mut linked: BTreeSet<((i32, i32), (i32, i32))> = BTreeSet::new();
        while let Some((room, d)) = queue.pop_front() {
            if self.get_location(&room).is_none() {
                continue; // a missing room has no exits of its own
            }
            let here = at[&room];
            let off_grid = here == OFF_GRID;
            for (direction, target, door) in self.exits_of(&room) {
                if !shown(&target) {
                    continue;
                }
                let within = depth.map_or(true, |max| d < max);
                let written = self.get_location(&target).is_some();
                let beside = |map: &mut Map| {
                    map.beside.push(Beside { room: room.clone(), direction: direction.clone(), target: target.clone(), written, door: door.clone() });
                };
                let compass = if off_grid { None } else { step(&direction) };
                let Some((dx, dy)) = compass else {
                    if within || at.contains_key(&target) {
                        beside(&mut map);
                        if within && !at.contains_key(&target) && written {
                            // Off the grid, but still a place to go on from.
                            queue.push_back((target.clone(), d + 1));
                            at.insert(target.clone(), OFF_GRID);
                        }
                    }
                    continue;
                };
                let next = (here.0 + dx, here.1 + dy);
                match at.get(&target).copied() {
                    Some(placed) if placed == next => {
                        let key = if here <= next { (here, next) } else { (next, here) };
                        if linked.insert(key) {
                            map.links.push(MapLink { from: here, to: next, direction: direction.clone(), door: door.clone() });
                        }
                    }
                    Some(_) => beside(&mut map),
                    None if !within => {}
                    None => match taken.get(&next) {
                        Some(_) => beside(&mut map),
                        None => {
                            at.insert(target.clone(), next);
                            taken.insert(next, target.clone());
                            place(&mut map, &target, next, self);
                            let key = if here <= next { (here, next) } else { (next, here) };
                            linked.insert(key);
                            map.links.push(MapLink { from: here, to: next, direction: direction.clone(), door: door.clone() });
                            queue.push_back((target.clone(), d + 1));
                        }
                    },
                }
            }
        }
        // A room reached only off the grid has no cell; drop the marker.
        map.cells.sort_by_key(|c| (c.y, c.x));
        map
    }
}

#[cfg(test)]
mod tests {
    use crate::engines::WorldEngine;
    use std::path::Path;

    fn write(dir: &Path, rel: &str, text: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn room(dir: &Path, id: &str, exits: &str) {
        write(dir, &format!("{}.md", id), &format!("---\nid: {}\ntitle: {}\ntype: location\nconnections: {{ {} }}\n---\n", id, id.to_uppercase(), exits));
    }

    fn world() -> (tempfile::TempDir, WorldEngine) {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        room(d, "hall", "north: tower, east: gate, down: cellar");
        room(d, "tower", "south: hall, east: roof");
        room(d, "gate", "west: hall, north: roof, east: road");
        room(d, "roof", "west: tower, south: gate");
        room(d, "cellar", "up: hall, north: well");
        room(d, "well", "south: cellar");
        let mut engine = WorldEngine::new();
        engine.load_world(d).unwrap();
        (dir, engine)
    }

    #[test]
    fn rooms_take_cells_by_their_exits_and_the_rest_go_beside() {
        let (_d, engine) = world();
        let map = engine.map_layout("hall", None, None);
        let cells: Vec<(&str, i32, i32, bool)> = map.cells.iter().map(|c| (c.room.as_str(), c.x, c.y, c.written)).collect();
        assert_eq!(cells, [("tower", 0, -1, true), ("roof", 1, -1, true), ("hall", 0, 0, true), ("gate", 1, 0, true), ("road", 2, 0, false)]);
        assert!(map.cells.iter().find(|c| c.room == "hall").unwrap().here);
        assert!(map.beside.iter().any(|b| b.room == "hall" && b.direction == "down" && b.target == "cellar"));
        assert!(map.beside.iter().any(|b| b.room == "cellar" && b.direction == "up"));
        assert!(map.beside.iter().any(|b| b.room == "cellar" && b.direction == "north" && b.target == "well"));
        // Each exit pair is one link.
        assert_eq!(map.links.len(), 5);

        let near = engine.map_layout("hall", Some(1), None);
        assert_eq!(near.cells.len(), 3);
        let only: std::collections::BTreeSet<String> = ["hall", "gate"].iter().map(|s| s.to_string()).collect();
        assert_eq!(engine.map_layout("hall", None, Some(&only)).cells.len(), 2);
    }

    #[test]
    fn distances_and_paths_follow_every_exit() {
        let (_d, engine) = world();
        let d = engine.distances("hall");
        assert_eq!(d[0], ("hall".to_string(), 0));
        assert_eq!(d.iter().find(|(r, _)| r == "well").unwrap().1, 2);
        assert_eq!(engine.find_path("well", "roof").unwrap(), ["well", "cellar", "hall", "tower", "roof"]);
        assert_eq!(engine.find_path("hall", "nowhere"), None);
    }

    #[test]
    fn the_ascii_map_draws_rooms_links_doors_and_whats_beside() {
        let (dir, mut engine) = world();
        let d = dir.path();
        room(d, "hall", "north: tower, east: gate, down: cellar, west: arch");
        room(d, "yard", "east: arch");
        write(d, "arch.md", "---\nid: arch\ntitle: Arch\ntype: door\npassable: no\nsides:\n  hall: { direction: west }\n  yard: { direction: east }\n---\n");
        engine.load_world(d).unwrap();
        let text = crate::render::render_map(&engine.map_layout("hall", Some(1), None));
        assert_eq!(
            text,
            "           [TOWER ]\n               |\n[ YARD ]-#-[@ HALL]---[ GATE ]\n\nAlso:\n  HALL: down → cellar\n  cellar: up → hall"
        );
    }
}

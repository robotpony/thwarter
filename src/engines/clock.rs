//! Time in play (`design/STATES.md` › Clocks): the game's seed and the
//! draws made from it, and game time from the turn count.

/// A new game's seed. Only its spread matters: what's drawn from it is
/// the same every time the same game is replayed or loaded.
pub fn new_seed() -> u64 {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
    mix(nanos ^ (std::process::id() as u64).rotate_left(32))
}

/// A number below `n` drawn from `seed` for `salt` (what it's for: a
/// thing's id and the turn, say), the same for the same inputs.
pub fn draw(seed: u64, salt: &str, n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    // FNV-1a over the salt, then SplitMix64.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in salt.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    mix(seed ^ h) % n
}

fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// A count of turns: `20`, or a range `15-25` (both ends included).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Turns {
    pub low: u64,
    pub high: u64,
}

impl Turns {
    pub fn parse(value: &serde_yaml::Value) -> Option<Turns> {
        if let Some(n) = value.as_u64() {
            return Some(Turns { low: n, high: n });
        }
        let text = value.as_str()?.trim();
        let (low, high) = match text.split_once('-') {
            Some((a, b)) => (a.trim().parse().ok()?, b.trim().parse().ok()?),
            None => {
                let n = text.parse().ok()?;
                (n, n)
            }
        };
        (low <= high).then_some(Turns { low, high })
    }

    /// The point picked in the range for `salt`.
    pub fn pick(&self, seed: u64, salt: &str) -> u64 {
        self.low + draw(seed, salt, self.high - self.low + 1)
    }
}

/// A thing's `at:` (`at: { turn: 50, becomes: ringing }`, or with game
/// time `at: { time: "12:00", becomes: ringing }`): one map or a list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum When {
    Turn(u64),
    /// Minutes past midnight, each day.
    Time(u64),
}

/// A clock change: when, and the state it switches the thing into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timed<T> {
    pub when: T,
    pub becomes: String,
}

fn becomes_of(map: &serde_yaml::Mapping) -> Option<String> {
    map.get("becomes").and_then(|v| v.as_str()).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}

/// A state's `after:`, from the state's own file, else the nearest
/// template's state of that name. `Err` says what's wrong with it.
pub fn after_of(thing: &crate::engines::bundles::Thing, state: &str) -> Option<Result<Timed<Turns>, String>> {
    let layer = thing.states.get(state).filter(|l| l.metadata.contains_key("after")).or_else(|| {
        thing.template_states.get(state).and_then(|ls| ls.iter().rev().find(|l| l.metadata.contains_key("after")))
    })?;
    let value = layer.metadata.get("after")?;
    let shown = layer.path.display();
    Some(match value.as_mapping() {
        Some(map) => match (map.get("turns").and_then(Turns::parse), becomes_of(map)) {
            (Some(when), Some(becomes)) => Ok(Timed { when, becomes }),
            (None, _) => Err(format!("{}: `after:` needs `turns:`, a number or a range like 15-25", shown)),
            (_, None) => Err(format!("{}: `after:` needs `becomes:`, the state it changes into", shown)),
        },
        None => Err(format!("{}: `after:` is a map, like `after: {{ turns: 20, becomes: stale }}`", shown)),
    })
}

/// A thing's `at:` entries, from its index, else the nearest template's.
pub fn at_of(thing: &crate::engines::bundles::Thing) -> Vec<Result<Timed<When>, String>> {
    let Some(layer) = std::iter::once(&thing.index).chain(thing.templates.iter().rev()).find(|l| l.metadata.contains_key("at")) else {
        return Vec::new();
    };
    let shown = layer.path.display();
    let value = &layer.metadata["at"];
    let items: Vec<&serde_yaml::Value> = match value.as_sequence() {
        Some(list) => list.iter().collect(),
        None => vec![value],
    };
    items
        .into_iter()
        .map(|item| {
            let map = item.as_mapping().ok_or_else(|| format!("{}: `at:` is a map, like `at: {{ turn: 50, becomes: ringing }}`", shown))?;
            let becomes = becomes_of(map).ok_or_else(|| format!("{}: `at:` needs `becomes:`, the state it changes into", shown))?;
            if let Some(turn) = map.get("turn") {
                let turn = turn.as_u64().ok_or_else(|| format!("{}: `at: {{ turn: }}` is a turn number", shown))?;
                return Ok(Timed { when: When::Turn(turn), becomes });
            }
            match map.get("time").and_then(|t| t.as_str()).and_then(minutes_of) {
                Some(minutes) => Ok(Timed { when: When::Time(minutes), becomes }),
                None => Err(format!("{}: `at:` needs `turn:` or `time:` (a time of day, like \"12:00\")", shown)),
            }
        })
        .collect()
}

/// Minutes since midnight from `"08:00"`.
pub fn minutes_of(clock: &str) -> Option<u64> {
    let (h, m) = clock.trim().split_once(':')?;
    let (h, m): (u64, u64) = (h.trim().parse().ok()?, m.trim().parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// Game time at `turn`: the clock (`"09:15"`) and the day (from 1),
/// starting at `start` minutes past midnight on day 1.
pub fn game_time(start: u64, minutes_per_turn: u64, turn: u64) -> (String, u64) {
    let total = start + minutes_per_turn * turn;
    let day = total / (24 * 60) + 1;
    let of_day = total % (24 * 60);
    (format!("{:02}:{:02}", of_day / 60, of_day % 60), day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_repeat_for_the_same_seed_and_ranges_stay_inside() {
        assert_eq!(draw(7, "bread@3", 11), draw(7, "bread@3", 11));
        let range = Turns::parse(&serde_yaml::Value::from("15-25")).unwrap();
        assert_eq!(range, Turns { low: 15, high: 25 });
        let picks: Vec<u64> = (0..50).map(|s| range.pick(s, "bread")).collect();
        assert!(picks.iter().all(|p| (15..=25).contains(p)) && picks.iter().any(|p| *p != picks[0]));
        assert_eq!(Turns::parse(&serde_yaml::Value::from(20)), Some(Turns { low: 20, high: 20 }));
        assert_eq!(Turns::parse(&serde_yaml::Value::from("25-15")), None);
        assert_eq!(game_time(8 * 60, 5, 0), ("08:00".to_string(), 1));
        assert_eq!(game_time(23 * 60, 30, 3), ("00:30".to_string(), 2));
        assert_eq!(minutes_of("12:00"), Some(720));
        assert_eq!(minutes_of("25:00"), None);
    }
}

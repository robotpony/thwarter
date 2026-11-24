//! Scripts for actions YAML can't say (`design/SECTIONS.md` › Scripts,
//! REQ-ENG-38). A thing's `actions.rhai`, or a ```rhai block in its one
//! file, holds a function per action, named by the verb:
//!
//! ```rhai
//! fn unlock(door, turn) {
//!     if door.state != "locked" { return "no-action"; }
//!     if turn.used == () { return "no-key"; }
//!     [keep(turn.used), become("closed")]
//! }
//! ```
//!
//! A function gets the thing and the turn, read-only, and returns what
//! happens: `become("state")`, a reason (`"wrong-key"`), nothing (which
//! means `no-action`), or a list of changes. It never returns text and
//! never writes anything; the engine applies what it returns.
//!
//! Scripts are sandboxed: no files, network or modules, no `eval`, no
//! printing, and caps on operations, depth and sizes, so a world from a
//! stranger is safe to play.

use rhai::{Array, Dynamic, Engine, Map, AST};
use std::path::PathBuf;

/// The most operations one action may take before it's stopped.
pub const MAX_OPERATIONS: u64 = 100_000;

/// One script, from a file or a ```rhai block.
#[derive(Debug, Clone, PartialEq)]
pub struct Script {
    pub path: PathBuf,
    pub source: String,
}

/// One change a script asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptChange {
    /// The thing moves to this state.
    Become(String),
    /// The thing keeps what it was used with (`keep(turn.used)`).
    Keep(String),
    /// A connected thing moves to a state (`change("vault-door", "open")`).
    Change { id: String, state: String },
}

/// What a script's action returned.
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptResult {
    /// Nothing changes, for this reason (`no-action` when it returned
    /// nothing).
    Reason(String),
    Changes(Vec<ScriptChange>),
}

fn engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_max_operations(MAX_OPERATIONS);
    engine.set_max_call_levels(32);
    engine.set_max_expr_depths(64, 32);
    engine.set_max_string_size(64 * 1024);
    engine.set_max_array_size(10_000);
    engine.set_max_map_size(10_000);
    engine.set_module_resolver(rhai::module_resolvers::DummyModuleResolver::new());
    engine.disable_symbol("eval");
    engine.on_print(|_| {});
    engine.on_debug(|_, _, _| {});
    engine.register_fn("become", |state: &str| -> Map { change_map("become", state, None) });
    engine.register_fn("keep", |thing: Map| -> Map {
        let id = thing.get("id").map(|v| v.to_string()).unwrap_or_default();
        change_map("keep", "", Some(&id))
    });
    engine.register_fn("keep", |id: &str| -> Map { change_map("keep", "", Some(id)) });
    engine.register_fn("change", |id: &str, state: &str| -> Map { change_map("change", state, Some(id)) });
    engine.register_fn("holds", |thing: &mut Map, id: &str| -> bool {
        thing
            .get("held")
            .and_then(|h| h.clone().try_cast::<Array>())
            .map_or(false, |held| held.iter().any(|t| t.clone().try_cast::<Map>().and_then(|m| m.get("id").map(|v| v.to_string())).as_deref() == Some(id)))
    });
    engine
}

fn change_map(kind: &str, state: &str, id: Option<&str>) -> Map {
    let mut map = Map::new();
    map.insert("change".into(), kind.into());
    map.insert("state".into(), state.into());
    if let Some(id) = id {
        map.insert("id".into(), id.into());
    }
    map
}

/// A Rhai function name for a verb: `attempt-open` is `attempt_open`.
pub fn function_name(verb: &str) -> String {
    verb.trim().to_lowercase().replace(['-', ' '], "_")
}

/// Compile a thing's scripts, its templates' first, so its own function
/// replaces a template's of the same name. The error names the file.
pub fn compile(scripts: &[Script]) -> Result<Option<AST>, String> {
    let engine = engine();
    let mut merged: Option<AST> = None;
    for script in scripts {
        let ast = engine.compile(&script.source).map_err(|e| format!("{}: the script doesn't compile: {}", script.path.display(), e))?;
        merged = Some(match merged {
            Some(below) => below.merge(&ast),
            None => ast,
        });
    }
    Ok(merged)
}

/// Whether the compiled scripts have an action for `verb`.
pub fn has_function(ast: &AST, verb: &str) -> bool {
    let name = function_name(verb);
    ast.iter_functions().any(|f| f.name == name && f.params.len() == 2)
}

/// The names of the actions the scripts define.
pub fn functions(ast: &AST) -> Vec<String> {
    ast.iter_functions().filter(|f| f.params.len() == 2).map(|f| f.name.to_string()).collect()
}

/// Run the action for `verb`, given the thing and the turn as maps. The
/// error is the script's, for an author note.
pub fn run(ast: &AST, verb: &str, this: Map, turn: Map) -> Result<ScriptResult, String> {
    let engine = engine();
    let mut scope = rhai::Scope::new();
    let out: Dynamic = engine.call_fn(&mut scope, ast, function_name(verb), (this, turn)).map_err(|e| e.to_string())?;
    read_result(out)
}

fn read_result(out: Dynamic) -> Result<ScriptResult, String> {
    if out.is_unit() {
        return Ok(ScriptResult::Reason("no-action".into()));
    }
    if out.is_string() {
        let reason = out.into_string().unwrap_or_default();
        return Ok(ScriptResult::Reason(if reason.trim().is_empty() { "no-action".into() } else { reason }));
    }
    let items: Vec<Dynamic> = match out.clone().try_cast::<Array>() {
        Some(list) => list,
        None => vec![out],
    };
    let mut changes = Vec::new();
    for item in items {
        let Some(map) = item.clone().try_cast::<Map>() else {
            return Err(format!("an action returns become(...), keep(...), change(...), a reason or a list of them, not `{}`", item.type_name()));
        };
        let get = |k: &str| map.get(k).map(|v| v.to_string()).unwrap_or_default();
        changes.push(match get("change").as_str() {
            "become" => ScriptChange::Become(get("state")),
            "keep" => ScriptChange::Keep(get("id")),
            "change" => ScriptChange::Change { id: get("id"), state: get("state") },
            _ => return Err("an action returns become(...), keep(...), change(...), a reason or a list of them".to_string()),
        });
    }
    Ok(ScriptResult::Changes(changes))
}

/// A thing as a script sees it: `id`, `title`, `state`, each of its
/// values by name, and `held`, what it holds.
pub fn thing_map(id: &str, title: &str, state: Option<&str>, vars: &[(String, String)], held: Vec<Map>) -> Map {
    let mut map = Map::new();
    for (k, v) in vars {
        map.insert(k.as_str().into(), v.as_str().into());
    }
    map.insert("id".into(), id.into());
    map.insert("title".into(), title.into());
    map.insert("state".into(), state.map_or(Dynamic::UNIT, |s| s.into()));
    map.insert("held".into(), Dynamic::from_array(held.into_iter().map(Dynamic::from_map).collect()));
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    fn script(source: &str) -> AST {
        compile(&[Script { path: "t.rhai".into(), source: source.into() }]).unwrap().unwrap()
    }

    fn door(state: &str, key: &str, held: &[&str]) -> Map {
        let held = held.iter().map(|id| thing_map(id, id, None, &[], vec![])).collect();
        thing_map("door", "Door", Some(state), &[("key".into(), key.into())], held)
    }

    fn turn(with: Option<&str>) -> Map {
        let mut t = Map::new();
        t.insert("used".into(), with.map_or(Dynamic::UNIT, |id| Dynamic::from_map(thing_map(id, id, None, &[], vec![]))));
        t
    }

    #[test]
    fn actions_return_states_reasons_and_changes() {
        let ast = script(
            r#"
            fn unlock(door, turn) {
                if door.state != "locked" { return "no-action"; }
                if door.key == "" { return become("closed"); }
                if turn.used == () { return "no-key"; }
                if turn.used.id != door.key { return "wrong-key"; }
                [keep(turn.used), become("closed")]
            }
            fn lock(door, turn) { if door.holds(door.key) { become("locked") } }
            fn pull_hard(lever, turn) { change("vault-door", "open") }
            "#,
        );
        assert!(has_function(&ast, "unlock") && has_function(&ast, "pull-hard") && !has_function(&ast, "open"));
        let r = |d, t| run(&ast, "unlock", d, t).unwrap();
        assert_eq!(r(door("closed", "key", &[]), turn(None)), ScriptResult::Reason("no-action".into()));
        assert_eq!(r(door("locked", "key", &[]), turn(None)), ScriptResult::Reason("no-key".into()));
        assert_eq!(r(door("locked", "key", &[]), turn(Some("spoon"))), ScriptResult::Reason("wrong-key".into()));
        assert_eq!(r(door("locked", "", &[]), turn(None)), ScriptResult::Changes(vec![ScriptChange::Become("closed".into())]));
        assert_eq!(
            r(door("locked", "key", &[]), turn(Some("key"))),
            ScriptResult::Changes(vec![ScriptChange::Keep("key".into()), ScriptChange::Become("closed".into())])
        );
        assert_eq!(run(&ast, "lock", door("closed", "key", &["key"]), turn(None)).unwrap(), ScriptResult::Changes(vec![ScriptChange::Become("locked".into())]));
        assert_eq!(run(&ast, "lock", door("closed", "key", &[]), turn(None)).unwrap(), ScriptResult::Reason("no-action".into()));
        assert_eq!(
            run(&ast, "pull-hard", door("x", "", &[]), turn(None)).unwrap(),
            ScriptResult::Changes(vec![ScriptChange::Change { id: "vault-door".into(), state: "open".into() }])
        );
    }

    #[test]
    fn a_things_own_function_replaces_its_templates() {
        let ast = compile(&[
            Script { path: "template.rhai".into(), source: "fn open(t, turn) { \"stuck\" } fn close(t, turn) { \"template\" }".into() },
            Script { path: "own.rhai".into(), source: "fn open(t, turn) { become(\"open\") }".into() },
        ])
        .unwrap()
        .unwrap();
        assert_eq!(run(&ast, "open", door("closed", "", &[]), turn(None)).unwrap(), ScriptResult::Changes(vec![ScriptChange::Become("open".into())]));
        assert_eq!(run(&ast, "close", door("open", "", &[]), turn(None)).unwrap(), ScriptResult::Reason("template".into()));
    }

    #[test]
    fn scripts_are_sandboxed_and_errors_are_reported() {
        let ast = script("fn spin(t, turn) { loop { } } fn peek(t, turn) { import \"x\" as x; 1 } fn bad(t, turn) { 42 }");
        assert!(run(&ast, "spin", door("a", "", &[]), turn(None)).unwrap_err().contains("operations"));
        assert!(run(&ast, "peek", door("a", "", &[]), turn(None)).is_err());
        assert!(run(&ast, "bad", door("a", "", &[]), turn(None)).unwrap_err().contains("returns become"));
        assert!(compile(&[Script { path: "x.rhai".into(), source: "fn (".into() }]).unwrap_err().contains("x.rhai: the script doesn't compile"));
        assert!(compile(&[Script { path: "e.rhai".into(), source: "fn think(t, turn) { eval(\"1\") }".into() }]).is_err());
    }
}

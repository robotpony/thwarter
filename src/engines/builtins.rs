//! Templates the engine supplies (REQ-CF-21, `design/STATES.md` ›
//! Built-in mechanics). Each is an ordinary Markdown template file,
//! compiled in, so an author can read one and copy it to change it. A
//! world template of the same name overrides one.

use super::layers::Layer;
use std::path::Path;

/// Every built-in, by name, with its file's text.
pub const BUILTINS: &[(&str, &str)] = &[("scenery", include_str!("../builtins/scenery.md"))];

/// The built-in template `name`, as a layer whose path reads
/// `<built-in>/scenery.md` in messages.
pub fn builtin(name: &str) -> Option<Layer> {
    let (_, text) = BUILTINS.iter().find(|(n, _)| *n == name)?;
    Layer::parse(&Path::new("<built-in>").join(format!("{}.md", name)), text).ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_builtin_parses_and_is_a_template() {
        for (name, _) in super::BUILTINS {
            let layer = super::builtin(name).unwrap_or_else(|| panic!("{} doesn't parse", name));
            assert_eq!(layer.metadata.get("template"), Some(&serde_yaml::Value::Bool(true)), "{}", name);
        }
    }
}

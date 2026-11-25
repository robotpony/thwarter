//! `thwart new <world>`: makes a world for the person at the keyboard
//! to write (REQ-CLI-22).

use crate::worlds::{create_world, new_world_folder, title_from};
use anyhow::{anyhow, Result};
use std::path::Path;

/// Makes the world `name` from `from`: its folder, and a
/// `world/index.md` with the title and `authors:` the machine user.
pub fn new_command(name: &str, title: Option<&str>, from: &Path) -> Result<()> {
    let user = crate::settings::current_user().ok_or_else(|| anyhow!("no user name to make you the author: set $USER, or $THWART_USER"))?;
    let root = new_world_folder(name, from, &user);
    let shown_name = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| name.to_string());
    let title = title.map(str::to_string).unwrap_or_else(|| title_from(if name.contains(' ') { name } else { &shown_name }));
    if title.trim().is_empty() {
        return Err(anyhow!("a world needs a name, like `thwart new castle`"));
    }
    let shown = root.strip_prefix(from).map(|p| p.display().to_string()).unwrap_or_else(|_| root.display().to_string());
    create_world(&root, &title, &user).map_err(|e| anyhow!(e.to_string().replace(&root.display().to_string(), &shown)))?;
    println!("{}", crate::render::clean_text(&format!("Made {}, titled \"{}\", with you ({}) as its author.", shown, title, user)));
    println!("{}", crate::render::clean_text(&format!("Next: thwart edit {}", shown)));
    Ok(())
}

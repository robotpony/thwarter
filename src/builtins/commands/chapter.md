---
type: command
name: chapter
aliases: []
about: "a chapter, or the thing you're at's version for one"
args: "new {number} {name} | {name} [file]"
modes: [edit]
changes: true
handler: chapter
---
`/chapter new` makes a chapter at the world root, as `chapters/<number>-<name>.md`. At a room, thing or door, `/chapter <name>` adds a `## When: #<name>` section to its own file for a small change in that chapter, and `/chapter <name> file` gives it its own `chapters/<name>.md` for a big one, making it a bundle first.

    /chapter new 20 after-the-king
    /chapter after-the-king
    /chapter after-the-king file

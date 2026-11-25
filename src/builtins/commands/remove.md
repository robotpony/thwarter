---
type: command
name: remove
aliases: ["rm", "delete"]
about: "delete a thing, room or door and what refers to it, after asking"
args: "{thing}"
modes: [edit]
changes: true
handler: remove
---
Deletes a thing, room or door and every reference it can rewrite, after showing what goes and asking. There's no undo.

    /remove lantern

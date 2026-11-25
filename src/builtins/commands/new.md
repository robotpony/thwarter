---
type: command
name: new
aliases: ["make"]
about: "make a room, character or object"
args: "room|character|object {id} [{title}]"
modes: [edit]
changes: true
handler: new
---
Makes a room, character or object, then opens it in the editor.

    /new room cellar
    /new object lantern "Brass lantern"

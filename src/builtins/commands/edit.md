---
type: command
name: edit
aliases: ["e"]
about: "edit where you are, or what you name"
args: "[room|object|character] [{id}]"
modes: [edit, play]
changes: false
handler: edit
---
Opens where you are in the full-screen editor, or the room, object or character you name. `world` opens the world's own settings.

    /edit
    /edit lantern
    /edit world

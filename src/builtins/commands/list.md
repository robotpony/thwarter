---
type: command
name: list
aliases: ["ls"]
about: "every room, character, object or door in the world"
args: "{kind}"
modes: [edit, play]
changes: false
handler: list
---
Lists every room, character, object or door in the world, by id and title, wherever it is.

    /list rooms
    /list objects

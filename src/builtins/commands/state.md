---
type: command
name: state
aliases: []
about: "a state of the thing you're at; makes it a bundle"
args: "{name} [start]"
modes: [edit]
changes: true
handler: state
---
Adds a state to the thing you're at, making it a bundle. `start` makes it the starting state.

    /state lit
    /state hanging start

---
type: command
name: action
aliases: []
about: "an action from one state to another"
args: "{verb} {from} --> {to} [with {thing}]"
modes: [edit]
changes: true
handler: action
---
An action from one state to another, optionally needing a thing.

    /action light dark --> lit
    /action unlock locked --> closed with key

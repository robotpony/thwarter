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

`any` as the first state makes an action that works in every state, in the thing's own frontmatter. An action from a state on a thing with no starting state asks whether it starts there, since it couldn't run otherwise.

    /action light dark --> lit
    /action peel any --> removed
    /action unlock locked --> closed with key

---
type: command
name: set
aliases: []
about: "change one field of where you are"
args: "{key} {value}"
modes: [edit]
changes: true
handler: set
---
Changes one field of where you are. `in <state>` sets it in one of the thing's states.

    /set title "Old well"
    /set description "Dark." in closed

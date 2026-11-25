---
type: command
name: validate
aliases: ["check"]
about: "check the world as thwart validate does"
args: "[strict]"
modes: [edit, play]
changes: false
handler: validate
---
Checks the world as `thwart validate` does, edits included. `strict` counts warnings as problems.

    /validate
    /validate strict

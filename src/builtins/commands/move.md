---
type: command
name: move
aliases: ["mv"]
about: "put something somewhere else, or a room in an area"
args: "{thing} {room|character|thing}"
modes: [edit]
changes: true
handler: move
---
Puts something somewhere else: a thing into a room, person or thing, or a room into an area.

    /move lantern table
    /move cellar village

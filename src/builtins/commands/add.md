---
type: command
name: add
aliases: []
about: "put a person or thing here too; scenery if it has no card"
args: "{thing} [{room|character|thing}]"
modes: [edit]
changes: true
handler: add
---
Puts a person or thing in this room, or in a room, person or thing you name. A name with no card goes in as scenery. A thing already placed asks to move it. `/add door` makes a door.

    /add lantern
    /add rope smith
    /add door west hallway

---
type: command
name: scope
aliases: []
about: "where /new puts things: global, area or local"
args: "global|area|local"
modes: [edit]
changes: false
handler: scope
---
Sets where the next `/new` commands put things. Global is the world's `objects/` and `npcs/`, area is this area's own folders, and local is this room's bundle. The prompt shows the scope when it isn't global. A trailing `in global`, `in area` or `in here` changes it for one command, and `/move <thing> global|area|local` moves a thing that exists.

    /scope area
    /new object torch tool in here
    /move torch global

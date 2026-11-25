---
type: command
name: menu
aliases: []
about: "the quick menu for where you are; Ctrl-M opens it too"
args: ""
modes: [edit]
changes: true
handler: menu
---
Opens the quick menu for the room, thing or door you're at. Each entry asks its questions at the bottom of the console, runs the commands it stands for and shows them, so you learn the typed form. Ctrl-M opens it too, at the prompt and in the editor, in terminals with the kitty keyboard protocol (Ghostty, iTerm2 3.5+, kitty, WezTerm).

    /menu

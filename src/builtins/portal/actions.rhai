// Opening and closing (REQ-CF-19): every door built-in works as it comes.
// The replies are in closed.md and open.md, where the door ends up.

fn open(door, turn)  { if door.state == "closed" { become("open") } }
fn close(door, turn) { if door.state == "open" { become("closed") } }

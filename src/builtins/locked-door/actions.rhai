// The built-in locked door (design/STATES.md › Keys). `door.key` is the
// id of the key that opens it; "" means no key is needed.

fn unlock(door, turn) {
    if door.state != "locked" { return "no-action"; }
    if door.key == "" { return become("closed"); }      // a deadbolt: no key
    if turn.used == () { return "no-key"; }
    if turn.used.id != door.key { return "wrong-key"; }
    [keep(turn.used), become("closed")]                 // the key stays in the lock
}

fn lock(door, turn) {
    if door.state != "closed" { return "no-action"; }
    if door.key == "" || door.holds(door.key) { return become("locked"); }
    if turn.used == () { return "no-key"; }
    if turn.used.id != door.key { return "wrong-key"; }
    [keep(turn.used), become("locked")]
}

fn open(door, turn)  { if door.state == "closed" { become("open") } }
fn close(door, turn) { if door.state == "open" { become("closed") } }

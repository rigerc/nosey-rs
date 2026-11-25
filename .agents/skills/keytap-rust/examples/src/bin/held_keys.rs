use std::collections::HashSet;

use keytap::{EventKind, Key, Tap};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Listening continues even when another application's window has focus.
    let tap = Tap::new()?;
    let mut held: HashSet<Key> = HashSet::new();
    let mut shortcut_active = false;

    for event in tap.iter() {
        match event.kind {
            EventKind::KeyDown(key) => {
                held.insert(key);
            }
            EventKind::KeyUp(key) => {
                held.remove(&key);
            }
            EventKind::KeyRepeat(_) => {
                // A held key repeating does not represent another key press.
            }
        }

        let now_active = held.contains(&Key::MetaRight) && held.contains(&Key::Space);
        if now_active != shortcut_active {
            shortcut_active = now_active;
            println!("Right-Meta + Space held: {shortcut_active}");
        }
    }

    Ok(())
}

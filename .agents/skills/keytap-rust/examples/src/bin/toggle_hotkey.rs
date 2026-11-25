use keytap::{Key, chord::{Chord, ChordEvent, ChordMatcher}};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matcher = ChordMatcher::builder()
        .add_toggle("hands-free", Chord::of([Key::MetaRight, Key::AltRight, Key::Space]))
        .build()?;

    while let Ok(event) = matcher.recv() {
        match event {
            ChordEvent::Start { id: "hands-free", .. } => println!("Enabled"),
            ChordEvent::End { id: "hands-free", .. } => println!("Disabled"),
            _ => {}
        }
    }
    Ok(())
}

use keytap::{Key, chord::{Chord, ChordEvent, ChordMatcher}};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matcher = ChordMatcher::builder()
        .add("push-to-talk", Chord::of([Key::MetaRight, Key::Space]))
        .build()?;

    while let Ok(event) = matcher.recv() {
        match event {
            ChordEvent::Start { id: "push-to-talk", .. } => {
                println!("Push-to-talk: started");
                // Start microphone capture here.
            }
            ChordEvent::End { id: "push-to-talk", .. } => {
                println!("Push-to-talk: stopped");
                // Stop microphone capture here.
            }
            _ => {}
        }
    }

    // A real app must also stop capture on shutdown or loss of the event stream.
    Ok(())
}

use std::time::{Duration, Instant};

use keytap::{EventKind, Tap};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tap = Tap::new()?;
    let mut last_heartbeat = Instant::now();

    loop {
        if let Ok(event) = tap.recv_timeout(Duration::from_millis(50)) {
            match event.kind {
                EventKind::KeyDown(key) => println!("Pressed: {key:?}"),
                EventKind::KeyUp(key) => println!("Released: {key:?}"),
                EventKind::KeyRepeat(_) => {}
            }
        }
        if last_heartbeat.elapsed() >= Duration::from_secs(1) {
            // Perform non-keyboard work without blocking forever in Tap::recv.
            last_heartbeat = Instant::now();
        }
        // In production, check an application-controlled shutdown flag here.
    }
}

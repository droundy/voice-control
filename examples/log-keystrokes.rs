use rdev::{listen, Event, EventType, Key};
use voice_control::keys::Keystrokes;

fn main() {
    // This will block.
    let mut current_strokes: Vec<Keystrokes> = Vec::new();
    let mut waiting_to_lift: Vec<Key> = Vec::new();
    let nato = voice_control::keys::KeyMapping::nato();
    let mapping = voice_control::keys::KeyMapping::roundy();
    if let Err(error) = listen(move |event: Event| {
        match event {
            Event { event_type, .. } => {
                match event_type {
                    EventType::KeyPress(key) => {
                        if mapping.get_str(Keystrokes::Down(key)).is_some() {
                            // It is a modifier key.
                            current_strokes.push(Keystrokes::Down(key));
                        } else if key == Key::Return {
                            for k in current_strokes.drain(..) {
                                for s in &mapping[k][0] {
                                    print!("{} ", s);
                                }
                            }
                            println!("{}", mapping[Keystrokes::Press(key)][0][0]);
                        } else {
                            current_strokes.push(Keystrokes::Press(key));
                        }
                    }
                    EventType::KeyRelease(key) => {
                        if key == Key::ShiftLeft && current_strokes.len() > 1 {
                            if let [Keystrokes::Down(Key::ShiftLeft), Keystrokes::Press(k)] =
                                current_strokes[current_strokes.len() - 2..current_strokes.len()]
                            {
                                if nato.get_str(Keystrokes::Shift(k)).is_some() {
                                    current_strokes.pop();
                                    current_strokes.pop();
                                    current_strokes.push(Keystrokes::Shift(k));
                                    return;
                                }
                            }
                        }
                        if key == Key::ShiftRight && current_strokes.len() > 1 {
                            if let [Keystrokes::Down(Key::ShiftRight), Keystrokes::Press(k)] =
                                current_strokes[current_strokes.len() - 2..current_strokes.len()]
                            {
                                if nato.get_str(Keystrokes::Shift(k)).is_some() {
                                    current_strokes.pop();
                                    current_strokes.pop();
                                    current_strokes.push(Keystrokes::Shift(k));
                                    return;
                                }
                            }
                        }
                        if mapping.get_str(Keystrokes::Down(key)).is_some() {
                            // It is a modifier key.
                            waiting_to_lift = waiting_to_lift
                                .iter()
                                .copied()
                                .filter(|&k| k != key)
                                .collect();
                            if waiting_to_lift.len() == 0 {
                                for k in current_strokes.drain(..) {
                                    for s in &mapping[k][0] {
                                        print!("{} ", s);
                                    }
                                }
                                println!();
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
    }) {
        println!("Error: {:?}", error)
    }
}

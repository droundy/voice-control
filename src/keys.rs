use rdev::{simulate, EventType, Key, SimulateError};

fn send(event_type: &EventType) {
    let delay = std::time::Duration::from_millis(20);
    match simulate(event_type) {
        Ok(()) => (),
        Err(SimulateError) => {
            println!("We could not send {:?}", event_type);
        }
    }
    // Let ths OS catchup (at least MacOS)
    std::thread::sleep(delay);
}

fn send_key(k: Key) {
    send(&EventType::KeyPress(k));
    send(&EventType::KeyRelease(k));
}
fn send_upper(k: Key) {
    send(&EventType::KeyPress(Key::ShiftLeft));
    send(&EventType::KeyPress(k));
    send(&EventType::KeyRelease(k));
    send(&EventType::KeyRelease(Key::ShiftLeft));
}

/// Send keys to the keyboard
///
/// '🅂' | '⇧' => hold_down(Key::ShiftLeft),
/// '🄰' | '🄾' | '⎇' => hold_down(Key::Alt),
/// '🄲' => hold_down(Key::ControlLeft),
/// '🅆' | '⌘' | '❖' => hold_down(Key::MetaLeft),
/// '🅂' => hold_down(Key::ShiftLeft), //
///
/// '🅃' | '\t' => send_key(Key::Tab),
/// '⎋' | '🄴' => send_key(Key::Escape),
/// '🄱' | '␈' | '⌫' => send_key(Key::Backspace),
/// '⏎' | '\n' | '↵' => send_key(Key::Return),
/// '▤' | '☰' | '𝌆' => send_key(Key::Unknown(135)),
pub fn send_string(s: &str) {
    let mut to_lift = Vec::new();
    {
        let mut hold_down = |k: Key| {
            to_lift.push(k);
            send(&EventType::KeyPress(k));
        };
        for c in s.chars() {
            match c {
                '🅂' | '⇧' => hold_down(Key::ShiftLeft),
                '🄰' | '🄾' | '⎇' => hold_down(Key::Alt),
                '🄲' => hold_down(Key::ControlLeft),
                '🅆' | '⌘' | '❖' => hold_down(Key::MetaLeft),

                '🅃' | '\t' => send_key(Key::Tab),
                '⎋' | '🄴' => send_key(Key::Escape),
                '🄱' | '␈' | '⌫' => send_key(Key::Backspace),
                '⏎' | '\n' | '↵' => send_key(Key::Return),
                '▤' | '☰' | '𝌆' => send_key(Key::Unknown(135)),

                '[' => send_key(Key::LeftBracket),
                ']' => send_key(Key::RightBracket),
                '{' => send_upper(Key::LeftBracket),
                '}' => send_upper(Key::RightBracket),

                ';' => send_key(Key::SemiColon),
                ':' => send_upper(Key::SemiColon),

                '=' => send_key(Key::Equal),
                '+' => send_upper(Key::Equal),

                '-' => send_key(Key::Minus),
                '_' => send_upper(Key::Minus),

                '\'' => send_key(Key::Quote),
                '"' => send_upper(Key::Quote),

                '\\' => send_key(Key::BackSlash),
                '|' => send_upper(Key::BackSlash),

                '`' => send_key(Key::BackQuote),
                '~' => send_upper(Key::BackQuote),

                '1' => send_key(Key::Kp1),
                '2' => send_key(Key::Kp2),
                '3' => send_key(Key::Kp3),
                '4' => send_key(Key::Kp4),
                '5' => send_key(Key::Kp5),
                '6' => send_key(Key::Kp6),
                '7' => send_key(Key::Kp7),
                '8' => send_key(Key::Kp8),
                '9' => send_key(Key::Kp9),
                '0' => send_key(Key::Kp0),

                '!' => send_upper(Key::Kp1),
                '@' => send_upper(Key::Kp2),
                '#' => send_upper(Key::Kp3),
                '$' => send_upper(Key::Kp4),
                '%' => send_upper(Key::Kp5),
                '^' => send_upper(Key::Kp6),
                '&' => send_upper(Key::Kp7),
                '*' => send_upper(Key::Kp8),
                '(' => send_upper(Key::Kp9),
                ')' => send_upper(Key::Kp0),

                'a' => send_key(Key::KeyA),
                'b' => send_key(Key::KeyB),
                'c' => send_key(Key::KeyC),
                'd' => send_key(Key::KeyD),
                'e' => send_key(Key::KeyE),
                'f' => send_key(Key::KeyF),
                'g' => send_key(Key::KeyG),
                'h' => send_key(Key::KeyH),
                'i' => send_key(Key::KeyI),
                'j' => send_key(Key::KeyJ),
                'k' => send_key(Key::KeyK),
                'l' => send_key(Key::KeyL),
                'm' => send_key(Key::KeyM),
                'n' => send_key(Key::KeyN),
                'o' => send_key(Key::KeyO),
                'p' => send_key(Key::KeyP),
                'q' => send_key(Key::KeyQ),
                'r' => send_key(Key::KeyR),
                's' => send_key(Key::KeyS),
                't' => send_key(Key::KeyT),
                'u' => send_key(Key::KeyU),
                'v' => send_key(Key::KeyV),
                'w' => send_key(Key::KeyW),
                'x' => send_key(Key::KeyX),
                'y' => send_key(Key::KeyY),
                'z' => send_key(Key::KeyZ),
                'A' => send_upper(Key::KeyA),
                'B' => send_upper(Key::KeyB),
                'C' => send_upper(Key::KeyC),
                'D' => send_upper(Key::KeyD),
                'E' => send_upper(Key::KeyE),
                'F' => send_upper(Key::KeyF),
                'G' => send_upper(Key::KeyG),
                'H' => send_upper(Key::KeyH),
                'I' => send_upper(Key::KeyI),
                'J' => send_upper(Key::KeyJ),
                'K' => send_upper(Key::KeyK),
                'L' => send_upper(Key::KeyL),
                'M' => send_upper(Key::KeyM),
                'N' => send_upper(Key::KeyN),
                'O' => send_upper(Key::KeyO),
                'P' => send_upper(Key::KeyP),
                'Q' => send_upper(Key::KeyQ),
                'R' => send_upper(Key::KeyR),
                'S' => send_upper(Key::KeyS),
                'T' => send_upper(Key::KeyT),
                'U' => send_upper(Key::KeyU),
                'V' => send_upper(Key::KeyV),
                'W' => send_upper(Key::KeyW),
                'X' => send_upper(Key::KeyX),
                'Y' => send_upper(Key::KeyY),
                'Z' => send_upper(Key::KeyZ),
                _ => panic!("bad key character"),
            }
        }
        for k in to_lift.iter().rev() {
            send(&EventType::KeyRelease(*k));
        }
    }
}

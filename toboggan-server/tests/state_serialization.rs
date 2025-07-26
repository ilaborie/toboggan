use core::time::Duration;

use jiff::Timestamp;
use toboggan_core::{SlideId, State};

#[test]
#[allow(clippy::unwrap_used, clippy::print_stdout)] // Acceptable in test code
fn test_state_serialization_format() {
    let slide_id = SlideId::next();

    let paused_state = State::Paused {
        current: slide_id,
        total_duration: Duration::from_secs(10),
    };

    let running_state = State::Running {
        since: Timestamp::now(),
        current: slide_id,
        total_duration: Duration::from_secs(5),
    };

    let paused_json = serde_json::to_string_pretty(&paused_state).unwrap();
    let running_json = serde_json::to_string_pretty(&running_state).unwrap();

    println!("Paused state JSON:\n{paused_json}");
    println!("Running state JSON:\n{running_json}");

    // Test round-trip
    let paused_deserialized: State = serde_json::from_str(&paused_json).unwrap();
    let running_deserialized: State = serde_json::from_str(&running_json).unwrap();

    match paused_deserialized {
        State::Paused {
            current,
            total_duration,
        } => {
            assert_eq!(current, slide_id);
            assert_eq!(total_duration, Duration::from_secs(10));
        }
        _ => panic!("Expected Paused state"),
    }

    match running_deserialized {
        State::Running {
            current,
            total_duration,
            ..
        } => {
            assert_eq!(current, slide_id);
            assert_eq!(total_duration, Duration::from_secs(5));
        }
        _ => panic!("Expected Running state"),
    }
}

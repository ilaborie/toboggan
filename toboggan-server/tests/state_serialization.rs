use toboggan_core::{Duration, State, Timestamp};

#[test]
#[allow(clippy::unwrap_used, clippy::print_stdout)] // Acceptable in test code
fn test_state_serialization_format() {
    let slide_index = 1; // Use slide index instead of SlideId

    let paused_state = State::Paused {
        current: Some(slide_index),
        current_step: 0,
        total_duration: Duration::from_secs(10),
    };

    let running_state = State::Running {
        since: Timestamp::now(),
        current: slide_index,
        current_step: 0,
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
            ..
        } => {
            assert_eq!(current, Some(slide_index));
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
            assert_eq!(current, slide_index);
            assert_eq!(total_duration, Duration::from_secs(5));
        }
        _ => panic!("Expected Running state"),
    }
}

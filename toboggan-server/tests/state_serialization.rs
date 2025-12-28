use toboggan_core::{SlideId, State};

#[test]
#[allow(clippy::unwrap_used, clippy::print_stdout)] // Acceptable in test code
fn test_state_serialization_format() {
    let slide_id = SlideId::new(1);

    let running_state = State::Running {
        current: slide_id,
        current_step: 0,
    };

    let done_state = State::Done {
        current: slide_id,
        current_step: 0,
    };

    let running_json = serde_json::to_string_pretty(&running_state).unwrap();
    let done_json = serde_json::to_string_pretty(&done_state).unwrap();

    println!("Running state JSON:\n{running_json}");
    println!("Done state JSON:\n{done_json}");

    // Test round-trip
    let running_deserialized: State = serde_json::from_str(&running_json).unwrap();
    let done_deserialized: State = serde_json::from_str(&done_json).unwrap();

    match running_deserialized {
        State::Running { current, .. } => {
            assert_eq!(current, slide_id);
        }
        _ => panic!("Expected Running state"),
    }

    match done_deserialized {
        State::Done { current, .. } => {
            assert_eq!(current, slide_id);
        }
        _ => panic!("Expected Done state"),
    }
}

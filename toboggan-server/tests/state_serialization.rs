use toboggan_core::{SlideId, State};

#[test]
#[allow(clippy::unwrap_used, clippy::print_stdout)] // Acceptable in test code
fn test_state_serialization_format() {
    let slide_id = SlideId::new(1);

    let paused_state = State::Paused {
        current: Some(slide_id),
        current_step: 0,
    };

    let running_state = State::Running {
        current: slide_id,
        current_step: 0,
    };

    let paused_json = serde_json::to_string_pretty(&paused_state).unwrap();
    let running_json = serde_json::to_string_pretty(&running_state).unwrap();

    println!("Paused state JSON:\n{paused_json}");
    println!("Running state JSON:\n{running_json}");

    // Test round-trip
    let paused_deserialized: State = serde_json::from_str(&paused_json).unwrap();
    let running_deserialized: State = serde_json::from_str(&running_json).unwrap();

    match paused_deserialized {
        State::Paused { current, .. } => {
            assert_eq!(current, Some(slide_id));
        }
        _ => panic!("Expected Paused state"),
    }

    match running_deserialized {
        State::Running { current, .. } => {
            assert_eq!(current, slide_id);
        }
        _ => panic!("Expected Running state"),
    }
}

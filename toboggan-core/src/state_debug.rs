#[cfg(test)]
mod debug_tests {
    use super::*;
    use core::time::Duration;
    use jiff::Timestamp;

    #[test]
    fn test_state_serialization_format() {
        let slide_id = SlideId::new(1);
        
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
        
        println!("Paused state JSON:\n{}", paused_json);
        println!("Running state JSON:\n{}", running_json);
        
        // Test round-trip
        let paused_deserialized: State = serde_json::from_str(&paused_json).unwrap();
        let running_deserialized: State = serde_json::from_str(&running_json).unwrap();
        
        assert!(matches!(paused_deserialized, State::Paused { .. }));
        assert!(matches!(running_deserialized, State::Running { .. }));
    }
}
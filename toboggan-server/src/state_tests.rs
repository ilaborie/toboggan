#[cfg(test)]
#[allow(clippy::module_inception, clippy::unwrap_used)]
mod tests {
    use toboggan_core::{ClientId, Command, Date, Duration, Notification, Slide, State, Talk};

    use crate::TobogganState;

    fn create_test_talk() -> Talk {
        Talk::new("Test Talk")
            .with_date(Date::ymd(2025, 1, 1))
            .add_slide(Slide::cover("Cover Slide"))
            .add_slide(Slide::new("Second Slide"))
            .add_slide(Slide::new("Third Slide"))
    }

    #[tokio::test]
    async fn test_register_command() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk, 100).unwrap();
        let client_id = ClientId::new();

        let notification = state
            .handle_command(&Command::Register { client: client_id })
            .await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Init => {}
                _ => panic!("Expected initial state (Init)"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_unregister_command() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk, 100).unwrap();
        let client_id = ClientId::new();

        let notification = state
            .handle_command(&Command::Unregister { client: client_id })
            .await;

        match notification {
            Notification::State { .. } => {}
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_first_command() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk, 100).unwrap();

        // Move to last slide first (this will go to Running from Init)
        state.handle_command(&Command::Last).await;

        // Then go back to first (this should go to Running since we're not in Init anymore)
        let notification = state.handle_command(&Command::First).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Running { current, .. } => {
                    assert_eq!(current, 0); // First slide index
                }
                _ => panic!("Expected Running state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_last_command() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk, 100).unwrap();

        let notification = state.handle_command(&Command::Last).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Running { current, .. } => {
                    // From Init state, Last command should go to last slide (index 2 for 3 slides)
                    assert_eq!(current, 2); // Last slide index (3 slides = indices 0,1,2)
                }
                _ => panic!("Expected Running state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_goto_valid_slide() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk, 100).unwrap();
        let target_slide = 1; // Index 1 (second slide)

        let notification = state
            .handle_command(&Command::GoTo {
                slide: target_slide,
            })
            .await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Running { current, .. } => {
                    assert_eq!(current, target_slide);
                }
                _ => panic!("Expected Running state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_goto_invalid_slide() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk, 100).unwrap();
        let invalid_slide = 999; // Index out of bounds

        let notification = state
            .handle_command(&Command::GoTo {
                slide: invalid_slide,
            })
            .await;

        match notification {
            Notification::Error { message, .. } => {
                assert!(message.contains("not found") || message.contains("out of bounds"));
            }
            _ => panic!("Expected Error notification"),
        }
    }

    #[tokio::test]
    async fn test_next_command() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk, 100).unwrap();

        let notification = state.handle_command(&Command::Next).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Running { current, .. } => {
                    // From Init state, Next command should go to first slide
                    assert_eq!(current, 0); // First slide index
                }
                _ => panic!("Expected Running state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_next_at_last_slide() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk, 100).unwrap();

        // Go to last slide (from Init this will go to first slide)
        state.handle_command(&Command::Last).await;

        // Navigate to last slide properly
        state.handle_command(&Command::Last).await;

        // Try to go next from last slide
        let notification = state.handle_command(&Command::Next).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Done { .. } => {}
                _ => panic!("Expected Done state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_previous_command() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk, 100).unwrap();

        // Move to first slide (from Init)
        state.handle_command(&Command::Next).await;

        // Move to second slide
        state.handle_command(&Command::Next).await;

        // Then go back to first
        let notification = state.handle_command(&Command::Previous).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Running { current, .. } => {
                    assert_eq!(current, 0); // First slide index
                }
                _ => panic!("Expected Running state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_previous_at_first_slide() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk, 100).unwrap();

        // From Init state, Previous command should go to first slide
        let notification = state.handle_command(&Command::Previous).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Running { current, .. } => {
                    assert_eq!(current, 0); // First slide index
                }
                _ => panic!("Expected Running state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_pause_command() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk, 100).unwrap();

        // Get to Running state first by using a navigation command
        state.handle_command(&Command::Next).await;

        // Then pause
        let notification = state.handle_command(&Command::Pause).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Paused { .. } => {}
                _ => panic!("Expected Paused state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_resume_command() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk, 100).unwrap();

        // Get to Running state first, then pause
        state.handle_command(&Command::Next).await;
        state.handle_command(&Command::Pause).await;

        // Now resume
        let notification = state.handle_command(&Command::Resume).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Running { .. } => {}
                _ => panic!("Expected Running state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_ping_command() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk, 100).unwrap();

        let notification = state.handle_command(&Command::Ping).await;

        match notification {
            Notification::Pong => {}
            _ => panic!("Expected Pong notification"),
        }
    }

    #[tokio::test]
    async fn test_state_preservation_during_navigation() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk, 100).unwrap();

        // Start in Running state
        state.handle_command(&Command::Next).await;

        // Navigate while running
        let notification = state.handle_command(&Command::Next).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Running { .. } => {}
                _ => panic!("Expected to remain in Running state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_navigation_from_done_state() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk, 100).unwrap();

        // Go to first slide (from Init), then navigate to last slide
        state.handle_command(&Command::Next).await;
        state.handle_command(&Command::Last).await;

        // Go next from last slide to reach Done state
        state.handle_command(&Command::Next).await;

        // Navigate from Done state
        let notification = state.handle_command(&Command::Previous).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Paused { .. } => {}
                _ => panic!("Expected Paused state when navigating from Done"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_duration_tracking() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk, 100).unwrap();

        // Start tracking by getting to Running state
        state.handle_command(&Command::Next).await;

        // Wait a tiny bit to ensure duration > 0
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Pause and check duration
        let notification = state.handle_command(&Command::Pause).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Paused { total_duration, .. } => {
                    assert!(total_duration > Duration::from_secs(0));
                }
                _ => panic!("Expected Paused state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_init_to_running_transition() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk, 100).unwrap();

        // Verify initial state is Init
        let initial_state = state.current_state().await;
        assert!(matches!(initial_state, State::Init));

        // Send Next command
        let notification = state.handle_command(&Command::Next).await;

        // Verify notification contains Running state
        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Running { current, .. } => {
                    assert_eq!(current, 0); // First slide index
                }
                _ => panic!("Expected Running state in notification, got: {inner_state:?}"),
            },
            _ => panic!("Expected State notification"),
        }

        // Verify current state is also Running
        let current_state = state.current_state().await;
        assert!(matches!(current_state, State::Running { .. }));
    }

    #[tokio::test]
    async fn test_paused_navigation_to_running() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk, 100).unwrap();

        // Start running and move to second slide
        state.handle_command(&Command::Next).await; // Init -> Running (first slide)
        state.handle_command(&Command::Next).await; // Running (second slide)

        // Pause
        let notification = state.handle_command(&Command::Pause).await;
        match notification {
            Notification::State {
                state: inner_state, ..
            } => assert!(matches!(inner_state, State::Paused { .. })),
            _ => panic!("Expected State notification"),
        }

        // Now navigate next - should go to Running
        let notification = state.handle_command(&Command::Next).await;
        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Running { current, .. } => {
                    // Should be on third (last) slide
                    assert_eq!(current, 2); // Index 2 (third slide)
                }
                _ => panic!("Expected Running state, got: {inner_state:?}"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_paused_on_last_slide_next_stays_paused() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk, 100).unwrap();

        // Go to first slide, then navigate to last slide
        state.handle_command(&Command::Next).await; // Init -> Running (first slide)
        state.handle_command(&Command::Last).await; // Running -> Running (last slide)

        // Pause
        state.handle_command(&Command::Pause).await;

        // Try next from last slide - should stay paused (no next slide available)
        let notification = state.handle_command(&Command::Next).await;
        match notification {
            Notification::State {
                state: inner_state, ..
            } => assert!(matches!(inner_state, State::Paused { .. })),
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_first_command_resets_timestamp() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk, 100).unwrap();

        // Start running and navigate to second slide
        state.handle_command(&Command::Next).await; // Init -> Running (first slide)
        state.handle_command(&Command::Next).await; // Running (second slide)

        // Wait a bit to accumulate some duration
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Pause to capture current duration
        let pause_notification = state.handle_command(&Command::Pause).await;
        let Notification::State {
            state: State::Paused { total_duration, .. },
            ..
        } = pause_notification
        else {
            panic!("Expected Paused state");
        };

        // Verify we have some accumulated duration
        assert!(total_duration > Duration::from_secs(0));

        // Now go to first slide - this should reset the timestamp
        let first_notification = state.handle_command(&Command::First).await;
        match first_notification {
            Notification::State {
                state: State::Running { total_duration, .. },
                ..
            } => {
                // Duration should be reset to zero
                assert_eq!(total_duration, Duration::ZERO);
            }
            _ => panic!("Expected Running state after First command"),
        }
    }
}

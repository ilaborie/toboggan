#[cfg(test)]
#[allow(clippy::module_inception, clippy::unwrap_used)]
mod tests {
    use crate::TobogganState;
    use std::time::Duration;
    use toboggan_core::{
        ClientId, Command, Content, Notification, Renderer, Slide, SlideId, SlideKind, State,
        Style, Talk,
    };

    fn create_test_talk() -> Talk {
        Talk {
            title: Content::Text {
                text: "Test Talk".to_string(),
            },
            date: jiff::civil::Date::new(2025, 1, 1).unwrap(),
            slides: vec![
                Slide {
                    kind: SlideKind::Cover,
                    style: Style::default(),
                    title: Content::Text {
                        text: "Cover Slide".to_string(),
                    },
                    body: Content::Empty,
                    notes: Content::Empty,
                },
                Slide {
                    kind: SlideKind::Standard,
                    style: Style::default(),
                    title: Content::Text {
                        text: "Second Slide".to_string(),
                    },
                    body: Content::Empty,
                    notes: Content::Empty,
                },
                Slide {
                    kind: SlideKind::Standard,
                    style: Style::default(),
                    title: Content::Text {
                        text: "Third Slide".to_string(),
                    },
                    body: Content::Empty,
                    notes: Content::Empty,
                },
            ],
        }
    }

    #[tokio::test]
    async fn test_register_command() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk);
        let client_id = ClientId::new();

        let notification = state
            .handle_command(&Command::Register {
                client: client_id,
                renderer: Renderer::Html,
            })
            .await;

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
    async fn test_unregister_command() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk);
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
        let state = TobogganState::new(talk);

        // Move to last slide first
        state.handle_command(&Command::Last).await;

        // Then go back to first
        let notification = state.handle_command(&Command::First).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Paused { current, .. } => {
                    assert_eq!(current, *state.slide_order.first().unwrap());
                }
                _ => panic!("Expected Paused state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_last_command() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk);

        let notification = state.handle_command(&Command::Last).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Paused { current, .. } => {
                    assert_eq!(current, *state.slide_order.get(2).unwrap());
                }
                _ => panic!("Expected Paused state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_goto_valid_slide() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk);
        let target_slide = *state.slide_order.get(1).unwrap();

        let notification = state.handle_command(&Command::GoTo(target_slide)).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Paused { current, .. } => {
                    assert_eq!(current, target_slide);
                }
                _ => panic!("Expected Paused state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_goto_invalid_slide() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk);
        let invalid_slide = SlideId::next();

        let notification = state.handle_command(&Command::GoTo(invalid_slide)).await;

        match notification {
            Notification::Error { message, .. } => {
                assert!(message.contains("not found"));
            }
            _ => panic!("Expected Error notification"),
        }
    }

    #[tokio::test]
    async fn test_next_command() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk);

        let notification = state.handle_command(&Command::Next).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Paused { current, .. } => {
                    assert_eq!(current, *state.slide_order.get(1).unwrap());
                }
                _ => panic!("Expected Paused state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_next_at_last_slide() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk);

        // Go to last slide
        state.handle_command(&Command::Last).await;

        // Try to go next
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
        let state = TobogganState::new(talk);

        // Move to second slide first
        state.handle_command(&Command::Next).await;

        // Then go back
        let notification = state.handle_command(&Command::Previous).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Paused { current, .. } => {
                    assert_eq!(current, *state.slide_order.first().unwrap());
                }
                _ => panic!("Expected Paused state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_previous_at_first_slide() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk);
        let first_slide = *state.slide_order.first().unwrap();

        // Try to go previous from first slide
        let notification = state.handle_command(&Command::Previous).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Paused { current, .. } => {
                    assert_eq!(current, first_slide);
                }
                _ => panic!("Expected Paused state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_pause_command() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk);

        // Resume first
        state.handle_command(&Command::Resume).await;

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
        let state = TobogganState::new(talk);

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
        let state = TobogganState::new(talk);

        let notification = state.handle_command(&Command::Ping).await;

        match notification {
            Notification::Pong { .. } => {}
            _ => panic!("Expected Pong notification"),
        }
    }

    #[tokio::test]
    async fn test_state_preservation_during_navigation() {
        let talk = create_test_talk();
        let state = TobogganState::new(talk);

        // Start in Running state
        state.handle_command(&Command::Resume).await;

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
        let state = TobogganState::new(talk);

        // Go to last slide and then next to reach Done state
        state.handle_command(&Command::Last).await;
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
        let state = TobogganState::new(talk);

        // Resume to start tracking
        state.handle_command(&Command::Resume).await;

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
}

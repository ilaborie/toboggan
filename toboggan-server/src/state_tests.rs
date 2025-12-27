#[cfg(test)]
#[allow(clippy::module_inception, clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use axum::extract::FromRef;
    use toboggan_core::{Command, Date, Notification, Slide, SlideId, State, Talk};

    use crate::{ClientService, TalkService, TobogganState};

    fn create_test_talk() -> Talk {
        Talk::new("Test Talk")
            .with_date(Date::ymd(2025, 1, 1))
            .add_slide(Slide::cover("Cover Slide"))
            .add_slide(Slide::new("Second Slide"))
            .add_slide(Slide::new("Third Slide"))
    }

    fn create_test_state(talk: Talk) -> TobogganState {
        let talk_service = TalkService::new(talk).unwrap();
        let client_service = ClientService::new(100);
        TobogganState::new(talk_service, client_service)
    }

    #[tokio::test]
    async fn test_register_command() {
        let talk = create_test_talk();
        let state = create_test_state(talk);

        // Register command with name (server assigns client_id in ws handler)
        let notification = state
            .handle_command(&Command::Register {
                name: "Test Client".to_string(),
            })
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
        use std::net::{IpAddr, Ipv4Addr};

        let talk = create_test_talk();
        let state = create_test_state(talk);
        let ip_addr = IpAddr::V4(Ipv4Addr::LOCALHOST);

        // First register a client to get a valid ClientId
        let initial_notification = TalkService::from_ref(&state)
            .create_initial_notification()
            .await;
        let (client_id, _rx) = ClientService::from_ref(&state)
            .register_client("Test Client".to_string(), ip_addr, initial_notification)
            .await
            .expect("register client");

        let notification = state
            .handle_command(&Command::Unregister { client: client_id })
            .await;

        match notification {
            Notification::State { .. } => {}
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_register_client_and_get_clients() {
        use std::net::{IpAddr, Ipv4Addr};

        let talk = create_test_talk();
        let state = create_test_state(talk);
        let ip_addr = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let client_service = ClientService::from_ref(&state);

        // Register a client
        let initial_notification = TalkService::from_ref(&state)
            .create_initial_notification()
            .await;
        let (client_id, _rx) = client_service
            .register_client("Test Client".to_string(), ip_addr, initial_notification)
            .await
            .expect("register client");

        // Check connected clients
        let clients = client_service.connected_clients().await;
        assert_eq!(clients.clients.len(), 1);
        let first_client = clients.clients.first().expect("first client");
        // Note: The stored ClientInfo may have a different id field since we assign it after insertion
        // but the returned client_id is the real one
        assert_eq!(first_client.name, "Test Client");
        assert_eq!(first_client.ip_addr, ip_addr);

        // Unregister and check again
        client_service.unregister_client(client_id).await;
        let clients = client_service.connected_clients().await;
        assert_eq!(clients.clients.len(), 0);
    }

    #[tokio::test]
    async fn test_first_command() {
        let talk = create_test_talk();
        let state = create_test_state(talk);

        // Move to last slide first (this will go to Running from Init)
        state.handle_command(&Command::Last).await;

        // Then go back to first (this should go to Running since we're not in Init anymore)
        let notification = state.handle_command(&Command::First).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Running { current, .. } => {
                    assert_eq!(current, SlideId::FIRST); // First slide index
                }
                _ => panic!("Expected Running state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_last_command() {
        let talk = create_test_talk();
        let state = create_test_state(talk);

        let notification = state.handle_command(&Command::Last).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Running { current, .. } => {
                    // From Init state, Last command should go to last slide (index 2 for 3 slides)
                    assert_eq!(current, SlideId::new(2)); // Last slide index (3 slides = indices 0,1,2)
                }
                _ => panic!("Expected Running state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_goto_valid_slide() {
        let talk = create_test_talk();
        let state = create_test_state(talk);
        let target_slide = SlideId::new(1); // Index 1 (second slide)

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
        let state = create_test_state(talk);
        let invalid_slide = SlideId::new(999); // Index out of bounds

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
        let state = create_test_state(talk);

        let notification = state.handle_command(&Command::NextSlide).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Running { current, .. } => {
                    // From Init state, Next command should go to first slide
                    assert_eq!(current, SlideId::FIRST); // First slide index
                }
                _ => panic!("Expected Running state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_next_at_last_slide() {
        let talk = create_test_talk();
        let state = create_test_state(talk);

        // Go to last slide (from Init this will go to first slide)
        state.handle_command(&Command::Last).await;

        // Navigate to last slide properly
        state.handle_command(&Command::Last).await;

        // Try to go next from last slide
        let notification = state.handle_command(&Command::NextSlide).await;

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
        let state = create_test_state(talk);

        // Move to first slide (from Init)
        state.handle_command(&Command::NextSlide).await;

        // Move to second slide
        state.handle_command(&Command::NextSlide).await;

        // Then go back to first
        let notification = state.handle_command(&Command::PreviousSlide).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Running { current, .. } => {
                    assert_eq!(current, SlideId::FIRST); // First slide index
                }
                _ => panic!("Expected Running state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_previous_at_first_slide() {
        let talk = create_test_talk();
        let state = create_test_state(talk);

        // From Init state, Previous command should go to first slide
        let notification = state.handle_command(&Command::PreviousSlide).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Running { current, .. } => {
                    assert_eq!(current, SlideId::FIRST); // First slide index
                }
                _ => panic!("Expected Running state"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_ping_command() {
        let talk = create_test_talk();
        let state = create_test_state(talk);

        let notification = state.handle_command(&Command::Ping).await;

        match notification {
            Notification::Pong => {}
            _ => panic!("Expected Pong notification"),
        }
    }

    #[tokio::test]
    async fn test_state_preservation_during_navigation() {
        let talk = create_test_talk();
        let state = create_test_state(talk);

        // Start in Running state
        state.handle_command(&Command::NextSlide).await;

        // Navigate while running
        let notification = state.handle_command(&Command::NextSlide).await;

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
        let state = create_test_state(talk);

        // Go to first slide (from Init), then navigate to last slide
        state.handle_command(&Command::NextSlide).await;
        state.handle_command(&Command::Last).await;

        // Go next from last slide to reach Done state
        state.handle_command(&Command::NextSlide).await;

        // Navigate from Done state - should transition to Running
        let notification = state.handle_command(&Command::PreviousSlide).await;

        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Running { current, .. } => {
                    // Should be on the second-to-last slide (index 1)
                    assert_eq!(current, SlideId::new(1));
                }
                _ => panic!("Expected Running state when navigating from Done"),
            },
            _ => panic!("Expected State notification"),
        }
    }

    #[tokio::test]
    async fn test_init_to_running_transition() {
        let talk = create_test_talk();
        let state = create_test_state(talk);

        // Verify initial state is Init
        let initial_state = TalkService::from_ref(&state).current_state().await;
        assert!(matches!(initial_state, State::Init));

        // Send Next command
        let notification = state.handle_command(&Command::NextSlide).await;

        // Verify notification contains Running state
        match notification {
            Notification::State {
                state: inner_state, ..
            } => match inner_state {
                State::Running { current, .. } => {
                    assert_eq!(current, SlideId::FIRST); // First slide index
                }
                _ => panic!("Expected Running state in notification, got: {inner_state:?}"),
            },
            _ => panic!("Expected State notification"),
        }

        // Verify current state is also Running
        let current_state = TalkService::from_ref(&state).current_state().await;
        assert!(matches!(current_state, State::Running { .. }));
    }
}

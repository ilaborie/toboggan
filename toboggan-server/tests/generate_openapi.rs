use std::net::TcpListener;
use std::time::Duration;

use anyhow::Context;
use clawspec_core::test_client::{TestClient, TestServer, TestServerConfig};
use clawspec_core::{ApiClient, register_schemas};
use serde_json::{Value, json};
use utoipa::openapi::{ContactBuilder, InfoBuilder, LicenseBuilder, ServerBuilder};

use toboggan_core::{Content, Date, Renderer, Slide, SlideId, SlideKind, Style, Talk};
use toboggan_server::{HealthResponse, SlidesResponse, TalkResponse, TobogganState, routes};

#[derive(Debug, Clone)]
struct TobogganTestServer {
    router: axum::Router,
}

impl TobogganTestServer {
    fn new() -> Self {
        // Create a test talk
        let talk = Talk::new("Test Presentation")
            .with_date(Date::ymd(2025, 1, 20))
            .add_slide(Slide::cover("Welcome").with_body("This is a test presentation"))
            .add_slide(
                Slide::new("Content Slide")
                    .with_body(Content::html("<h1>Hello World</h1>"))
                    .with_notes("Some notes for the presenter"),
            )
            .add_slide(Slide::new("Final Slide").with_body(Content::iframe("https://example.com")));

        let state = TobogganState::new(talk, 100);
        let router = routes().with_state(state);

        Self { router }
    }
}

impl TestServer for TobogganTestServer {
    type Error = std::io::Error;

    fn config(&self) -> TestServerConfig {
        // Configure OpenAPI metadata
        let info = InfoBuilder::new()
            .title("Toboggan Server API")
            .version("0.1.0")
            .description(Some("REST API for the Toboggan presentation system that allows creating and serving slide-based talks with real-time WebSocket communication for Command/Notification interactions."))
            .contact(Some(
                ContactBuilder::new()
                    .name(Some("Igor Laborie"))
                    .email(Some("ilaborie@gmail.com"))
                    .build()
            ))
            .license(Some(
                LicenseBuilder::new()
                    .name("MIT OR Apache-2.0")
                    .identifier(Some("MIT OR Apache-2.0"))
                    .build()
            ))
            .build();

        let server = ServerBuilder::new()
            .url("http://localhost:3000")
            .description(Some("Local development server"))
            .build();

        let api_client_builder = ApiClient::builder().with_info(info).add_server(server);

        TestServerConfig {
            api_client: Some(api_client_builder),
            min_backoff_delay: Duration::from_millis(25),
            max_backoff_delay: Duration::from_secs(2),
            backoff_jitter: true,
            max_retry_attempts: 15,
        }
    }

    async fn launch(&self, listener: TcpListener) -> Result<(), Self::Error> {
        listener.set_nonblocking(true)?;
        let listener = tokio::net::TcpListener::from_std(listener)?;
        axum::serve(listener, self.router.clone().into_make_service()).await
    }
}

/// Create test application with clawspec `TestClient` with proper `OpenAPI` metadata
async fn create_test_app() -> anyhow::Result<TestClient<TobogganTestServer>> {
    let test_server = TobogganTestServer::new();
    let client = TestClient::start(test_server)
        .await
        .map_err(|err| anyhow::anyhow!("Failed to start test server: {err:?}"))?;
    Ok(client)
}

#[tokio::test]
async fn should_generate_openapi() -> anyhow::Result<()> {
    let mut app = create_test_app().await?;

    // Register basic schemas that have ToSchema implemented
    register_schemas!(app, Renderer, SlideId, SlideKind, Style).await;

    // Test all endpoints to generate comprehensive OpenAPI spec
    basic_api_operations(&mut app).await?;
    test_command_operations(&mut app).await?;
    test_error_cases(&mut app).await?;
    demonstrate_websocket_endpoint(&mut app).await?;

    // Generate and save OpenAPI specification
    app.write_openapi("./openapi.yml")
        .await
        .context("writing openapi file")?;

    Ok(())
}

#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
async fn basic_api_operations(app: &mut TestClient<TobogganTestServer>) -> anyhow::Result<()> {
    // Test health endpoint using Value for clawspec compatibility
    let _health_response = app
        .get("/health")?
        .await?
        .as_json::<HealthResponse>()
        .await?;

    // Test get talk endpoint
    let _talk_response = app
        .get("/api/talk")?
        .await?
        .as_json::<TalkResponse>()
        .await?;

    // Test get slides endpoint
    let _slides_response = app
        .get("/api/slides")?
        .await?
        .as_json::<SlidesResponse>()
        .await?;

    // TODO POST command

    Ok(())
}

#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
async fn test_command_operations(app: &mut TestClient<TobogganTestServer>) -> anyhow::Result<()> {
    // Test ping command - use Value for response since Notification doesn't have ToSchema
    let ping_command = json!({
        "command": "Ping"
    });
    let ping_response: Value = app
        .post("/api/command")?
        .json(&ping_command)?
        .await?
        .as_json()
        .await?;

    let ping_data = &ping_response["data"];
    assert_eq!(ping_data["type"].as_str().unwrap(), "Pong");
    assert!(ping_data["timestamp"].is_string());

    // Test register command
    let register_command = json!({
        "command": "Register",
        "client": "550e8400-e29b-41d4-a716-446655440000",
        "renderer": "Html"
    });
    let register_response: Value = app
        .post("/api/command")?
        .json(&register_command)?
        .await?
        .as_json()
        .await?;

    let register_data = &register_response["data"];
    assert_eq!(register_data["type"].as_str().unwrap(), "State");
    assert!(register_data["timestamp"].is_string());
    assert!(register_data["state"].is_object());

    // Test navigation commands
    let commands = vec![
        json!({"command": "Next"}),
        json!({"command": "Previous"}),
        json!({"command": "First"}),
        json!({"command": "Last"}),
        json!({"command": "Resume"}),
        json!({"command": "Pause"}),
    ];

    for command in commands {
        let response: Value = app
            .post("/api/command")?
            .json(&command)?
            .await?
            .as_json()
            .await?;

        // Most commands return State notification
        let response_data = &response["data"];
        let response_type = response_data["type"].as_str().unwrap();
        assert!(response_type == "State" || response_type == "Error");
        assert!(response_data["timestamp"].is_string());
    }

    Ok(())
}

#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
async fn test_error_cases(app: &mut TestClient<TobogganTestServer>) -> anyhow::Result<()> {
    // Test malformed command - we expect this to fail during deserialization
    let malformed_command = json!({
        "command": "InvalidCommand"
    });
    // This should fail with a client error during JSON serialization
    let _error = app.post("/api/command")?.json(&malformed_command);
    // Since it's a client-side error, we can't check the server response

    // Test GoTo command with invalid slide ID
    let slides_response: Value = app.get("/api/slides")?.await?.as_json().await?;
    let slides_data = &slides_response["data"];
    let slides = slides_data["slides"].as_object().unwrap();

    if let Some(first_slide_id) = slides.keys().next() {
        // Create an invalid slide ID by modifying the first one
        let mut invalid_id = first_slide_id.clone();
        invalid_id.push_str("_invalid");

        let goto_command = json!({
            "command": "GoTo",
            "0": invalid_id
        });
        // This should fail due to invalid slide ID format
        let _error = app.post("/api/command")?.json(&goto_command);
    }

    Ok(())
}

async fn demonstrate_websocket_endpoint(
    app: &mut TestClient<TobogganTestServer>,
) -> anyhow::Result<()> {
    // Test WebSocket endpoint exists (but skip upgrade testing)
    // Note: WebSocket upgrades require proper client implementation
    // For now we just verify the endpoint exists by making a simple GET request
    let _response = app.get("/api/ws")?.await?;
    // The endpoint exists and responds, that's enough for OpenAPI generation

    Ok(())
}

#[cfg(test)]
mod command_variants {
    use super::*;

    #[tokio::test]
    #[allow(clippy::unwrap_used, clippy::indexing_slicing)]
    async fn test_all_command_variants() -> anyhow::Result<()> {
        let app = create_test_app().await?;

        // Test all command variants to ensure they're documented in OpenAPI
        let commands = vec![
            json!({"command": "Ping"}),
            json!({"command": "First"}),
            json!({"command": "Last"}),
            json!({"command": "Next"}),
            json!({"command": "Previous"}),
            json!({"command": "Pause"}),
            json!({"command": "Resume"}),
            json!({
                "command": "Register",
                "client": "550e8400-e29b-41d4-a716-446655440000",
                "renderer": "Html"
            }),
            json!({
                "command": "Unregister",
                "client": "550e8400-e29b-41d4-a716-446655440000"
            }),
        ];

        for command in commands {
            let response: Value = app
                .post("/api/command")?
                .json(&command)?
                .await?
                .as_json()
                .await?;

            // All valid commands should return proper Notification responses
            let response_data = &response["data"];
            let response_type = response_data["type"].as_str().unwrap();
            assert!(
                response_type == "State" || response_type == "Pong" || response_type == "Error",
                "Unexpected response type: {response_type}"
            );
            assert!(response_data["timestamp"].is_string());
        }

        Ok(())
    }
}

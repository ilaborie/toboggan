use std::net::TcpListener;
use std::time::Duration as StdDuration;

use anyhow::Context;
use clawspec_core::test_client::{TestClient, TestServer, TestServerConfig};
use clawspec_core::{ApiClient, register_schemas};
use serde_json::{Value, json};
use toboggan_core::{
    ClientId, Command, Content, Date, Duration, Notification, Slide, SlideKind, SlidesResponse,
    State, Style, Talk, TalkResponse, Timestamp,
};
use toboggan_server::{HealthResponse, HealthResponseStatus, TobogganState, routes};
use utoipa::openapi::{ContactBuilder, InfoBuilder, LicenseBuilder, OpenApi, ServerBuilder};

#[derive(Debug, Clone)]
struct TobogganTestServer {
    router: axum::Router,
}

impl TobogganTestServer {
    #[allow(clippy::unwrap_used)]
    fn new() -> Self {
        // Create a test talk
        let talk = Talk::new("Test Presentation")
            .with_date(Date::ymd(2025, 1, 20))
            .add_slide(Slide::cover("Welcome").with_body("This is a test presentation"))
            .add_slide(
                Slide::new("Content Slide")
                    .with_body(Content::html("<h1>Hello World</h1>"))
                    .with_notes("Some notes for the presenter"),
            );

        let state = TobogganState::new(talk, 100).unwrap();
        let router = routes(None, OpenApi::default()).with_state(state);

        Self { router }
    }
}

impl TestServer for TobogganTestServer {
    type Error = std::io::Error;

    fn config(&self) -> TestServerConfig {
        // Configure OpenAPI metadata
        let info = InfoBuilder::new()
            .title(env!("CARGO_PKG_NAME"))
            .version(env!("CARGO_PKG_VERSION"))
            .description(Some(env!("CARGO_PKG_DESCRIPTION")))
            .contact(Some(
                ContactBuilder::new()
                    .name(Some("Igor Laborie"))
                    .email(Some("ilaborie@gmail.com"))
                    .build(),
            ))
            .license(Some(
                LicenseBuilder::new()
                    .name("MIT OR Apache-2.0")
                    .identifier(Some("MIT OR Apache-2.0"))
                    .build(),
            ))
            .build();

        let server = ServerBuilder::new()
            .url("http://localhost:8080")
            .description(Some("Local development server"))
            .build();

        let api_client_builder = ApiClient::builder().with_info(info).add_server(server);

        TestServerConfig {
            api_client: Some(api_client_builder),
            min_backoff_delay: StdDuration::from_millis(25),
            max_backoff_delay: StdDuration::from_secs(2),
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

    // Register all schemas that have ToSchema implemented
    register_schemas!(
        app,
        Content,
        Date,
        Duration,
        HealthResponse,
        HealthResponseStatus,
        Notification,
        Slide,
        SlideKind,
        SlidesResponse,
        State,
        Style,
        TalkResponse,
        Timestamp,
        ClientId,
    )
    .await;

    // Test all endpoints to generate comprehensive OpenAPI spec
    basic_api_operations(&mut app).await?;
    test_command_operations(&mut app).await?;
    demonstrate_websocket_endpoint(&mut app).await?;

    // Generate and save OpenAPI specification
    // Using JSON format for safer deserialization in the server
    app.write_openapi("./openapi.json")
        .await
        .context("writing openapi.json file")?;

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

    Ok(())
}

#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
async fn test_command_operations(app: &mut TestClient<TobogganTestServer>) -> anyhow::Result<()> {
    // Test ping command
    let ping_command = Command::Ping;
    let _ping_response = app
        .post("/api/command")?
        .json(&ping_command)?
        .await?
        .as_json::<Notification>()
        .await?;

    // Test register command
    let register_command = Command::Register {
        client: ClientId::new(),
    };
    let _register_response = app
        .post("/api/command")?
        .json(&register_command)?
        .await?
        .as_json::<Notification>()
        .await?;

    // Test navigation commands
    let commands = vec![
        Command::Next,
        Command::Previous,
        Command::First,
        Command::Last,
    ];

    for command in commands {
        let _response = app
            .post("/api/command")?
            .json(&command)?
            .await?
            .as_json::<Notification>()
            .await?;
    }

    Ok(())
}

async fn demonstrate_websocket_endpoint(
    app: &mut TestClient<TobogganTestServer>,
) -> anyhow::Result<()> {
    let _response = app.get("/api/ws")?.without_collection().await?;

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
            let response_type = response["type"].as_str().unwrap();
            assert!(
                response_type == "State" || response_type == "Pong" || response_type == "Error",
                "Unexpected response type: {response_type}"
            );

            // Check that the response has the expected structure for each type
            match response_type {
                "State" => {
                    // State notifications should have a state field
                    assert!(
                        response["state"].is_object(),
                        "State notification should have state field"
                    );
                }
                "Error" => {
                    // Error notifications should have a message field
                    assert!(
                        response["message"].is_string(),
                        "Error notification should have message field"
                    );
                }
                "Pong" => {
                    // Pong notifications have no additional fields
                }
                _ => panic!("Unexpected response type: {response_type}"),
            }
        }

        Ok(())
    }
}

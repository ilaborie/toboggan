#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use toboggan::{ClientConfig, ClientNotificationHandler, ConnectionStatus, State, TobogganClient};

fn main() {
    let config = ClientConfig {
        url: "http://localhost:8080".to_string(),
        max_retries: 10,
        retry_delay: Duration::from_secs(1),
    };

    let handler = Arc::new(NotificationHandler);
    let client = TobogganClient::new(config, "Test Client".to_string(), handler);

    client.connect();
    println!("state: {:?}", client.get_state());
    println!("talk: {:?}", client.get_talk());

    thread::sleep(Duration::from_secs(1));
    thread::spawn(move || {
        client.send_command(toboggan::Command::First);

        for _ in 0..10 {
            client.send_command(toboggan::Command::Next);
            thread::sleep(Duration::from_millis(200));
        }

        client.send_command(toboggan::Command::Blink);
        thread::sleep(Duration::from_millis(200));
        client.send_command(toboggan::Command::Blink);
    });

    print!("⏳ Waiting 20s...");
    thread::sleep(Duration::from_secs(20));
    print!("👋 Bye");
}

struct NotificationHandler;

impl ClientNotificationHandler for NotificationHandler {
    fn on_state_change(&self, state: State) {
        println!("🗿 {state:?}");
    }

    fn on_talk_change(&self, state: State) {
        println!("📝 Talk changed: {state:?}");
    }

    fn on_connection_status_change(&self, status: ConnectionStatus) {
        println!("🛜 {status:?}");
    }

    fn on_registered(&self, client_id: String) {
        println!("✅ Registered: {client_id}");
    }

    fn on_client_connected(&self, client_id: String, name: String) {
        println!("👋 Client connected: {client_id} ({name})");
    }

    fn on_client_disconnected(&self, client_id: String, name: String) {
        println!("👋 Client disconnected: {client_id} ({name})");
    }

    fn on_error(&self, error: String) {
        eprintln!("🚨 {error}");
    }
}

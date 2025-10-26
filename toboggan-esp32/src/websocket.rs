use std::{sync::mpsc, time::Duration};

use anyhow::{bail, Context};
use esp_idf_svc::{
    io::EspIOError,
    tls::X509,
    ws::{
        client::{
            EspWebSocketClient, EspWebSocketClientConfig, WebSocketEvent, WebSocketEventType,
        },
        FrameType,
    },
};
use heapless::spsc::{Producer, Queue};
use log::{debug, error, info, warn};
use serde::Deserialize;

use crate::{AppStateDiff, StateMode};

/// The PEM-encoded ISRG Root X1 certificate at the end of the cert chain
/// for the websocket server at echo.websocket.org.
const SERVER_ROOT_CERT: &[u8] = b"
-----BEGIN CERTIFICATE-----
MIIFazCCA1OgAwIBAgIRAIIQz7DSQONZRGPgu2OCiwAwDQYJKoZIhvcNAQELBQAw
TzELMAkGA1UEBhMCVVMxKTAnBgNVBAoTIEludGVybmV0IFNlY3VyaXR5IFJlc2Vh
cmNoIEdyb3VwMRUwEwYDVQQDEwxJU1JHIFJvb3QgWDEwHhcNMTUwNjA0MTEwNDM4
WhcNMzUwNjA0MTEwNDM4WjBPMQswCQYDVQQGEwJVUzEpMCcGA1UEChMgSW50ZXJu
ZXQgU2VjdXJpdHkgUmVzZWFyY2ggR3JvdXAxFTATBgNVBAMTDElTUkcgUm9vdCBY
MTCCAiIwDQYJKoZIhvcNAQEBBQADggIPADCCAgoCggIBAK3oJHP0FDfzm54rVygc
h77ct984kIxuPOZXoHj3dcKi/vVqbvYATyjb3miGbESTtrFj/RQSa78f0uoxmyF+
0TM8ukj13Xnfs7j/EvEhmkvBioZxaUpmZmyPfjxwv60pIgbz5MDmgK7iS4+3mX6U
A5/TR5d8mUgjU+g4rk8Kb4Mu0UlXjIB0ttov0DiNewNwIRt18jA8+o+u3dpjq+sW
T8KOEUt+zwvo/7V3LvSye0rgTBIlDHCNAymg4VMk7BPZ7hm/ELNKjD+Jo2FR3qyH
B5T0Y3HsLuJvW5iB4YlcNHlsdu87kGJ55tukmi8mxdAQ4Q7e2RCOFvu396j3x+UC
B5iPNgiV5+I3lg02dZ77DnKxHZu8A/lJBdiB3QW0KtZB6awBdpUKD9jf1b0SHzUv
KBds0pjBqAlkd25HN7rOrFleaJ1/ctaJxQZBKT5ZPt0m9STJEadao0xAH0ahmbWn
OlFuhjuefXKnEgV4We0+UXgVCwOPjdAvBbI+e0ocS3MFEvzG6uBQE3xDk3SzynTn
jh8BCNAw1FtxNrQHusEwMFxIt4I7mKZ9YIqioymCzLq9gwQbooMDQaHWBfEbwrbw
qHyGO0aoSCqI3Haadr8faqU9GY/rOPNk3sgrDQoo//fb4hVC1CLQJ13hef4Y53CI
rU7m2Ys6xt0nUW7/vGT1M0NPAgMBAAGjQjBAMA4GA1UdDwEB/wQEAwIBBjAPBgNV
HRMBAf8EBTADAQH/MB0GA1UdDgQWBBR5tFnme7bl5AFzgAiIyBpY9umbbjANBgkq
hkiG9w0BAQsFAAOCAgEAVR9YqbyyqFDQDLHYGmkgJykIrGF1XIpu+ILlaS/V9lZL
ubhzEFnTIZd+50xx+7LSYK05qAvqFyFWhfFQDlnrzuBZ6brJFe+GnY+EgPbk6ZGQ
3BebYhtF8GaV0nxvwuo77x/Py9auJ/GpsMiu/X1+mvoiBOv/2X/qkSsisRcOj/KK
NFtY2PwByVS5uCbMiogziUwthDyC3+6WVwW6LLv3xLfHTjuCvjHIInNzktHCgKQ5
ORAzI4JMPJ+GslWYHb4phowim57iaztXOoJwTdwJx4nLCgdNbOhdjsnvzqvHu7Ur
TkXWStAmzOVyyghqpZXjFaH3pO3JLF+l+/+sKAIuvtd7u+Nxe5AW0wdeRlN8NwdC
jNPElpzVmbUq4JUagEiuTDkHzsxHpFKVK7q4+63SM1N95R1NbdWhscdCb+ZAJzVc
oyi3B43njTOQ5yOf+1CceWxG1bQVs5ZufpsMljq4Ui0/1lvh+wjChP4kqKOJ2qxq
4RgqsahDYVvTH9w7jXbyLeiNdd8XM2w9U/t7y0Ff/9yi0GE44Za4rF2LN9d11TPA
mRGunUHBcnWEvgJBQl9nJEiU0Zsnvgc/ubhPgXRR4Xq37Z0j4r7g1SgEEzwxA57d
emyPxgcYxn/eR44/KJ4EBs+lVDR3veyJm+kXQ99b21/+jh5Xos1AnX5iItreGCc=
-----END CERTIFICATE-----\0";

#[derive(Debug, Clone)]
enum WsMessage {
    Connected,
    State { current: usize, mode: StateMode },
    Blink,
    Error { message: String },
    Closed,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum Message {
    Blink,
    State { state: InnerState },
    TalkChange { state: InnerState },
    Error { message: String },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "state")]
enum InnerState {
    Init,
    Paused { current: Option<usize> },
    Running { current: usize },
    Done { current: usize },
}

/// Connect to WebSocket server for real-time presentation control
///
/// # Errors
/// Returns error if WebSocket connection fails, registration fails, or message parsing fails
pub fn connect_to_ws(host: &str, port: u16, tx: &mpsc::Sender<AppStateDiff>) -> anyhow::Result<()> {
    let uri = format!("ws://{host}:{port}/api/ws");
    info!("🦋 WS using URI {uri}");

    let config = EspWebSocketClientConfig {
        server_cert: Some(X509::pem_until_nul(SERVER_ROOT_CERT)),
        ..Default::default()
    };
    let timeout = Duration::from_secs(10);

    // Use heapless queue for lock-free communication
    // Leak the queue to make it effectively static for the closure
    let ws_queue = Box::leak(Box::new(Queue::<WsMessage, 16>::new()));
    let (mut ws_producer, mut ws_consumer) = ws_queue.split();

    info!("🦋 WS connecting...");

    let mut client = EspWebSocketClient::new(&uri, &config, timeout, move |event| {
        handle_event(&mut ws_producer, event);
    })
    .context("creating WS client")?;

    // Wait for connection - poll the queue
    let mut connected = false;
    for _ in 0..100 {
        // 10 second timeout (100 * 100ms)
        if let Some(first_event) = ws_consumer.dequeue() {
            match first_event {
                WsMessage::Connected => {
                    connected = true;
                    break;
                }
                other => bail!("Expected connected event, got {other:?}"),
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    if !connected {
        bail!("WebSocket connection timeout");
    }

    info!("🦋 WS connnected");

    // Register client
    let message = r#"{"command":"Register","client":"801eb979-13c6-48c7-8ce0-78e1a3200bc6","renderer":"Html"}"#;
    info!("Websocket send, text: {message}");
    client.send(FrameType::Text(false), message.as_bytes())?;

    // Main message processing loop
    loop {
        if let Some(msg) = ws_consumer.dequeue() {
            info!("🦋 WS incomming {msg:?}");
            match msg {
                WsMessage::Connected => bail!("🦋 WS unexpected connected message"),
                WsMessage::State { current, mode } => {
                    // Send slide update diff instead of full state
                    let diff = AppStateDiff::UpdateSlide { current, mode };
                    if let Err(error) = tx.send(diff) {
                        error!("Failed to send slide update diff: {error}, stopping WebSocket");
                        break;
                    }
                }
                WsMessage::Blink => {
                    // Send blink effect diff
                    let diff = AppStateDiff::Blink;
                    if let Err(error) = tx.send(diff) {
                        error!("Failed to send blink diff: {error}, stopping WebSocket");
                        break;
                    }
                }
                WsMessage::Closed => {
                    info!("🦋 WS - closing");
                    break;
                }
                WsMessage::Error { message } => {
                    info!("🦋 WS - error {message}");
                    let diff = AppStateDiff::Error { message };
                    if let Err(error) = tx.send(diff) {
                        error!("Failed to send error diff: {error}, stopping WebSocket");
                        break;
                    }
                }
            }
        } else {
            // No message available, sleep briefly to avoid busy waiting
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    Ok(())
}

fn handle_event(
    producer: &mut Producer<WsMessage, 16>,
    event: &Result<WebSocketEvent, EspIOError>,
) {
    let event = match event {
        Ok(event) => event,
        Err(err) => {
            warn!("📥 WS connection failure {err:?}");
            return;
        }
    };

    let msg = match event.event_type {
        WebSocketEventType::BeforeConnect => {
            info!("📥 WS - before connect");
            return;
        }
        WebSocketEventType::Connected => {
            info!("📥 WS - connected");
            WsMessage::Connected
        }
        WebSocketEventType::Disconnected => {
            info!("📥 WS - disconnected");
            WsMessage::Closed
        }
        WebSocketEventType::Close(reason) => {
            info!("📥 WS - closed: {reason:?}");
            WsMessage::Closed
        }
        WebSocketEventType::Closed => {
            info!("📥 WS - closed");
            WsMessage::Closed
        }

        WebSocketEventType::Text(txt) => {
            info!("📥 WS - text: {txt}");
            let Ok(msg) = serde_json::from_str::<Message>(txt) else {
                debug!("📥 WS - skip the message");
                return;
            };

            match msg {
                Message::Blink => {
                    info!("📥 WS - ⚡️ blink event received");
                    WsMessage::Blink
                }
                Message::State { state } | Message::TalkChange { state } => {
                    let (current, mode) = match state {
                        InnerState::Init => {
                            warn!("📥 WS - Initialized");
                            return;
                        }
                        InnerState::Paused { current } => {
                            (current.unwrap_or_default(), StateMode::Paused)
                        }
                        InnerState::Running { current } => (current, StateMode::Running),
                        InnerState::Done { current } => (current, StateMode::Done),
                    };
                    WsMessage::State { current, mode }
                }

                Message::Error { message } => WsMessage::Error { message },
            }
        }
        WebSocketEventType::Binary(items) => {
            warn!("📥 WS - skip binary payload {items:?}");
            return;
        }
        WebSocketEventType::Ping => {
            debug!("📥 WS - ping");
            return;
        }
        WebSocketEventType::Pong => {
            debug!("📥 WS - pong");
            return;
        }
    };

    // Use enqueue which is lock-free
    if producer.enqueue(msg).is_err() {
        warn!("WebSocket queue full, dropping message");
    }
}

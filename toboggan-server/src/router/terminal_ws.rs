use std::io::{Read, Write};
use std::thread;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::Response;
use futures::{SinkExt, StreamExt};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use super::presenter::Presenter;
use crate::TobogganState;

#[derive(Debug, Deserialize)]
pub(super) struct TerminalParams {
    cwd: Option<String>,
    cmd: Option<String>,
    #[serde(default = "default_cols")]
    cols: u16,
    #[serde(default = "default_rows")]
    rows: u16,
}

fn default_cols() -> u16 {
    80
}

/// How long the shell's last output is given to reach the client after it dies.
///
/// Long enough for a `logout` line already through the PTY, short enough that
/// nobody notices the socket staying open for it.
const EXIT_DRAIN: Duration = Duration::from_millis(120);

fn default_rows() -> u16 {
    24
}

/// Opens a shell on the machine running the server.
///
/// The most dangerous route in the project, and the reason the presenter gate
/// exists: it spawns `$SHELL -ic <whatever the client asked for>` in a
/// client-supplied directory. Unguarded on a reachable port that is remote code
/// execution on the presenter's laptop, so `Presenter` is not optional here.
///
/// A browser cannot set headers on a `WebSocket`, so a remote presenter's token
/// travels in the query string — see [`super::presenter`].
pub(super) async fn terminal_websocket_handler(
    _: Presenter,
    ws: WebSocketUpgrade,
    State(state): State<TobogganState>,
    Query(params): Query<TerminalParams>,
) -> Response {
    let cwd = params.cwd.unwrap_or_else(|| ".".to_owned());
    let cmd = params.cmd;
    let shell = state.terminal_shell().to_owned();
    info!(%cwd, ?cmd, %shell, cols = params.cols, rows = params.rows, "Terminal WebSocket upgrade requested");
    ws.on_upgrade(move |socket| handle_terminal(socket, shell, cwd, cmd, params.cols, params.rows))
}

async fn handle_terminal(
    socket: WebSocket,
    shell: String,
    cwd: String,
    cmd: Option<String>,
    cols: u16,
    rows: u16,
) {
    let (mut ws_sender, ws_receiver) = socket.split();

    // Resolve cwd to absolute path
    let abs_cwd = match std::env::current_dir() {
        Ok(base) => base.join(&cwd),
        Err(_) => std::path::PathBuf::from(&cwd),
    };

    if !abs_cwd.is_dir() {
        error!(%cwd, ?abs_cwd, "Terminal cwd does not exist");
        let _ = ws_sender.send(Message::Close(None)).await;
        return;
    }

    let pty_system = native_pty_system();
    let size = PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = match pty_system.openpty(size) {
        Ok(pair) => pair,
        Err(err) => {
            error!(?err, "Failed to open PTY");
            let _ = ws_sender.send(Message::Close(None)).await;
            return;
        }
    };

    let mut command = CommandBuilder::new(&shell);
    command.cwd(&abs_cwd);
    command.env("TERM", "xterm-256color");

    // For commands, use interactive login shell so PATH and config are loaded
    if let Some(ref user_cmd) = cmd {
        command.args(["-ic", user_cmd]);
    }

    let mut child = match pair.slave.spawn_command(command) {
        Ok(child) => child,
        Err(err) => {
            error!(?err, %cwd, "Failed to spawn command");
            let _ = ws_sender.send(Message::Close(None)).await;
            return;
        }
    };
    drop(pair.slave);

    info!(%cwd, ?cmd, abs_cwd = %abs_cwd.display(), "Terminal session started");

    let (tx_ws, rx_ws) = mpsc::unbounded_channel::<Message>();
    let (tx_pty, rx_pty) = std::sync::mpsc::sync_channel::<Vec<u8>>(128);

    spawn_pty_reader(pair.master.try_clone_reader(), tx_ws);
    // macOS grace period (see portable-pty docs about race condition)
    tokio::time::sleep(Duration::from_millis(20)).await;
    spawn_pty_writer(pair.master.take_writer(), rx_pty);

    let ws_reader_task = spawn_ws_reader(ws_receiver, tx_pty, pair.master);
    let ws_sender_task = spawn_ws_sender(rx_ws, ws_sender);
    let reader_abort = ws_reader_task.abort_handle();
    let sender_abort = ws_sender_task.abort_handle();

    // Watching the child is the only way to learn that the shell exited.
    //
    // Nothing else reports it. The PTY reader treats a zero-length read as "no
    // output yet" and sleeps rather than as end-of-file, so it spins for the
    // life of the page; and the two socket tasks are both waiting on a client
    // that has no reason to say anything. Without this arm a shell that ran
    // `exit` left a session the browser still believed was live: a terminal
    // painting its last frame, a PTY the server had not reaped, and a click
    // that could still hand it the keyboard.
    //
    // `wait` blocks, so it goes to a blocking thread, and the killer is cloned
    // first because waiting takes the child.
    let mut killer = child.clone_killer();
    let child_exit = tokio::task::spawn_blocking(move || child.wait());

    tokio::select! {
        _ = ws_reader_task => { info!("WebSocket reader ended"); }
        _ = ws_sender_task => { info!("WebSocket sender ended"); }
        status = child_exit => {
            match status {
                Ok(Ok(status)) => info!(%status, "Shell exited"),
                Ok(Err(err)) => warn!(?err, "Could not wait for the shell"),
                Err(err) => warn!(?err, "The task waiting on the shell failed"),
            }
            // The shell's parting words are written before it exits, so they may
            // still be in flight through the reader thread. Closing the socket
            // the instant it dies would swallow them.
            tokio::time::sleep(EXIT_DRAIN).await;
        }
    }

    info!("Terminal session ended, killing child process");
    if let Err(err) = killer.kill() {
        warn!(?err, "Failed to kill child process");
    }
    // Both tasks outlive the arm that did not fire, and one of them owns the PTY
    // master: dropping it is what unblocks the reader thread, which would
    // otherwise go on polling a shell nobody is listening to.
    reader_abort.abort();
    sender_abort.abort();
}

/// Forwards PTY output to the WebSocket verbatim.
///
/// This used to also answer the terminal's own Device Attributes and
/// cursor-position queries by scanning each 4 KiB read for `\x1b[c` / `\x1b[6n`
/// — which silently missed any query straddling two reads — plus a fish-specific
/// pre-send to dodge the ~10 s wait its startup DA1 probe would otherwise incur.
/// The client's emulator (rioterm) answers those queries itself and sends the
/// replies back as ordinary input, so the server is a plain pipe again.
fn spawn_pty_reader(
    reader: Result<Box<dyn Read + Send>, anyhow::Error>,
    tx_ws: mpsc::UnboundedSender<Message>,
) {
    let mut reader = match reader {
        Ok(reader) => reader,
        Err(err) => {
            error!(?err, "Failed to clone PTY reader");
            return;
        }
    };

    thread::spawn(move || {
        let mut buffer = [0_u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => thread::sleep(Duration::from_millis(10)),
                Ok(len) => {
                    let data = buffer.get(..len).unwrap_or_default();
                    if tx_ws.send(Message::Binary(data.to_vec().into())).is_err() {
                        break;
                    }
                }
                Err(err) => {
                    warn!(?err, "PTY read error");
                    break;
                }
            }
        }
    });
}

fn spawn_pty_writer(
    writer: Result<Box<dyn Write + Send>, anyhow::Error>,
    rx_pty: std::sync::mpsc::Receiver<Vec<u8>>,
) {
    let mut writer = match writer {
        Ok(writer) => writer,
        Err(err) => {
            error!(?err, "Failed to take PTY writer");
            return;
        }
    };

    thread::spawn(move || {
        while let Ok(bytes) = rx_pty.recv() {
            if let Err(err) = writer.write_all(&bytes) {
                warn!(?err, "PTY write failed");
                break;
            }
        }
    });
}

fn spawn_ws_reader(
    mut ws_receiver: futures::stream::SplitStream<WebSocket>,
    tx_pty: std::sync::mpsc::SyncSender<Vec<u8>>,
    master: Box<dyn MasterPty + Send>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                // Text frames are control commands, binary frames are PTY input.
                // The two used to be the other way round, with every binary frame
                // first tried as control JSON and passed through when that failed
                // — so pasting `{"type":"resize",…}` into the shell resized the
                // terminal instead of reaching it. The frame type now decides.
                Ok(Message::Text(text)) => match serde_json::from_str::<TerminalControl>(&text) {
                    Ok(control) => handle_control(master.as_ref(), control),
                    Err(err) => warn!(?err, %text, "Ignoring malformed terminal control"),
                },
                Ok(Message::Binary(data)) => {
                    if tx_pty.send(data.to_vec()).is_err() {
                        break;
                    }
                }
                Ok(Message::Close(_)) => break,
                Ok(Message::Ping(_) | Message::Pong(_)) => {}
                Err(err) => {
                    warn!(?err, "Terminal WebSocket error");
                    break;
                }
            }
        }
    })
}

fn spawn_ws_sender(
    mut rx_ws: mpsc::UnboundedReceiver<Message>,
    mut ws_sender: futures::stream::SplitSink<WebSocket, Message>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(msg) = rx_ws.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    })
}

fn handle_control(master: &dyn MasterPty, control: TerminalControl) {
    match control {
        TerminalControl::Resize { cols, rows } => {
            let cols = cols.max(1);
            let rows = rows.max(1);
            let size = PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            };
            if let Err(err) = master.resize(size) {
                warn!(?err, "Failed to resize PTY");
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TerminalControl {
    Resize { cols: u16, rows: u16 },
}

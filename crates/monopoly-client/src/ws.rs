use futures::{SinkExt, StreamExt};
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use monopoly_proto::{ClientMessage, ServerMessage, SessionToken};
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;

pub fn get_storage_session_token(room_id: &str) -> Option<SessionToken> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let key = format!("monopoly_token_{}", room_id);
    storage.get_item(&key).ok()?.map(SessionToken)
}

pub fn save_storage_session_token(room_id: &str, token: &SessionToken) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let key = format!("monopoly_token_{}", room_id);
            let _ = storage.set_item(&key, &token.0);
        }
    }
}

pub struct WsClient {
    ws_write: Rc<futures::lock::Mutex<futures::stream::SplitSink<WebSocket, Message>>>,
}

impl WsClient {
    pub fn connect(
        ws_url: String,
        room_id: String,
        player_name: String,
        enable_auction: bool,
        on_message: impl Fn(ServerMessage) + 'static,
        on_close: impl Fn() + 'static,
    ) -> Result<Self, String> {
        let ws = WebSocket::open(&ws_url).map_err(|e| e.to_string())?;
        let (write, mut read) = ws.split();
        let write_mutex = Rc::new(futures::lock::Mutex::new(write));

        let write_clone = write_mutex.clone();
        let saved_token = get_storage_session_token(&room_id);

        // Send initial JoinRoom message followed by initial game configuration
        spawn_local(async move {
            let join_msg = ClientMessage::JoinRoom {
                player_name,
                session_token: saved_token,
            };
            let mut guard = write_clone.lock().await;
            if let Ok(json) = serde_json::to_string(&join_msg) {
                let _ = guard.send(Message::Text(json)).await;
            }
            let cfg_msg = ClientMessage::ConfigureGame { enable_auction };
            if let Ok(cfg_json) = serde_json::to_string(&cfg_msg) {
                let _ = guard.send(Message::Text(cfg_json)).await;
            }
        });

        // Listen for incoming server messages
        spawn_local(async move {
            while let Some(msg_res) = read.next().await {
                match msg_res {
                    Ok(Message::Text(txt)) => {
                        if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&txt) {
                            on_message(server_msg);
                        }
                    }
                    Ok(Message::Bytes(_)) => {}
                    Err(_) => break,
                }
            }
            on_close();
        });

        Ok(Self {
            ws_write: write_mutex,
        })
    }

    pub fn send(&self, msg: ClientMessage) {
        let write = self.ws_write.clone();
        spawn_local(async move {
            if let Ok(json) = serde_json::to_string(&msg) {
                let mut guard = write.lock().await;
                let _ = guard.send(Message::Text(json)).await;
            }
        });
    }
}

use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use monopoly_proto::{ClientMessage, ServerMessage};
use monopoly_server::{create_router, AppState};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

pub struct TestServer {
    pub addr: SocketAddr,
    pub state: AppState,
}

impl TestServer {
    pub async fn spawn() -> Self {
        let registry = Arc::new(DashMap::new());
        let state = AppState { registry };
        let app = create_router(state.clone());

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, app.into_make_service())
                .await
                .unwrap();
        });

        Self { addr, state }
    }

    pub fn ws_url(&self, room_id: &str) -> String {
        format!("ws://{}/ws?room={}", self.addr, room_id)
    }

    pub async fn connect_ws(
        &self,
        room_id: &str,
    ) -> (
        futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            Message,
        >,
        futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    ) {
        let url = self.ws_url(room_id);
        let (ws_stream, _) = connect_async(url).await.expect("Failed to connect WS");
        ws_stream.split()
    }
}

pub async fn send_json(
    sink: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    msg: &ClientMessage,
) {
    let json = serde_json::to_string(msg).unwrap();
    sink.send(Message::Text(json)).await.unwrap();
}

pub async fn recv_msg(
    stream: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
) -> Option<ServerMessage> {
    while let Some(Ok(msg)) = stream.next().await {
        if let Message::Text(txt) = msg {
            if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&txt) {
                return Some(server_msg);
            }
        }
    }
    None
}

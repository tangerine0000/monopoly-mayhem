use crate::room::{RoomActor, RoomCommand};
use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, Request, State,
    },
    http::{header, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use monopoly_proto::{ClientMessage, RoomId, ServerMessage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<DashMap<String, mpsc::Sender<RoomCommand>>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRoomRequest {
    pub player_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateRoomResponse {
    pub room_id: String,
}

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub room: Option<String>,
}

async fn no_cache_html(req: Request<Body>, next: Next) -> Response {
    let mut res = next.run(req).await;
    if let Some(ct) = res.headers().get(header::CONTENT_TYPE) {
        if ct.to_str().unwrap_or("").contains("text/html") {
            res.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("no-store, must-revalidate"),
            );
        }
    }
    res
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/rooms", post(create_room))
        .route("/ws", get(ws_upgrade))
        .fallback_service(ServeDir::new("crates/monopoly-client/dist"))
        .layer(middleware::from_fn(no_cache_html))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

async fn create_room(
    State(state): State<AppState>,
    Json(_req): Json<CreateRoomRequest>,
) -> impl IntoResponse {
    let raw_room_id = format!("SGN-{}", Uuid::new_v4().to_string()[..4].to_uppercase());
    let room_id = RoomId(raw_room_id.clone());

    let _tx = RoomActor::spawn(room_id, state.registry.clone());

    Json(CreateRoomResponse {
        room_id: raw_room_id,
    })
}

async fn ws_upgrade(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(state): State<AppState>,
) -> Response {
    let room_id = match query.room {
        Some(r) => {
            let normalized = r.trim().to_uppercase();
            if normalized.is_empty() {
                return (StatusCode::BAD_REQUEST, "Empty 'room' query parameter").into_response();
            }
            normalized
        }
        None => return (StatusCode::BAD_REQUEST, "Missing 'room' query parameter").into_response(),
    };

    let room_tx = match state.registry.get(&room_id) {
        Some(tx) => tx.clone(),
        None => RoomActor::spawn(RoomId(room_id.clone()), state.registry.clone()),
    };

    ws.on_upgrade(move |socket| handle_socket(socket, room_tx))
}

async fn handle_socket(socket: WebSocket, room_tx: mpsc::Sender<RoomCommand>) {
    let (mut ws_sink, mut ws_stream) = socket.split();

    // 1. Wait for JoinRoom message
    let first_msg = match ws_stream.next().await {
        Some(Ok(Message::Text(txt))) => txt,
        _ => return,
    };

    let join_action: ClientMessage = match serde_json::from_str(&first_msg) {
        Ok(msg) => msg,
        Err(e) => {
            warn!("Failed to deserialize join message: {}", e);
            return;
        }
    };

    let (player_name, session_token) = match join_action {
        ClientMessage::JoinRoom {
            player_name,
            session_token,
        } => (player_name, session_token),
        _ => {
            warn!("First message was not JoinRoom");
            return;
        }
    };

    let (client_tx, mut client_rx) = mpsc::channel::<Arc<String>>(64);
    let (reply_tx, reply_rx) = oneshot::channel();

    let connect_cmd = RoomCommand::ConnectClient {
        player_name,
        session_token,
        tx: client_tx,
        reply: reply_tx,
    };

    if room_tx.send(connect_cmd).await.is_err() {
        return;
    }

    let (player_id, _session_token, _seat) = match reply_rx.await {
        Ok(Ok(res)) => res,
        Ok(Err(err_msg)) => {
            let rej = ServerMessage::ActionRejected { reason: err_msg };
            if let Ok(json) = serde_json::to_string(&rej) {
                let _ = ws_sink.send(Message::Text(json)).await;
            }
            return;
        }
        Err(_) => return,
    };

    info!("Client joined: {}", player_id);

    // 2. Spawn pump from pre-serialized message channel to WebSocket Sink
    let mut send_task = tokio::spawn(async move {
        while let Some(json) = client_rx.recv().await {
            if ws_sink.send(Message::Text((*json).clone())).await.is_err() {
                break;
            }
        }
    });

    // 3. Receive WebSocket frames from client and route to room actor
    let room_tx_clone = room_tx.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_stream.next().await {
            match msg {
                Message::Text(txt) => {
                    if let Ok(action) = serde_json::from_str::<ClientMessage>(&txt) {
                        let _ = room_tx_clone
                            .send(RoomCommand::ClientAction(player_id, action))
                            .await;
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
        let _ = room_tx_clone
            .send(RoomCommand::DisconnectClient(player_id))
            .await;
    });

    // Wait until either sending or receiving fails
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    let _ = room_tx.send(RoomCommand::DisconnectClient(player_id)).await;
}

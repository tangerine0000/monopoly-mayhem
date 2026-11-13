use crate::bot::XeOmBot;
use crate::timer::{TimerNotification, TurnTimer};
use dashmap::DashMap;
use monopoly_core::rules::apply_action;
use monopoly_core::state::{GameState, PlayerState, TurnPhase};
use monopoly_proto::{ClientMessage, PlayerId, RoomId, ServerMessage, SessionToken};
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::sleep;
use tracing::{info, warn};
use uuid::Uuid;

pub enum RoomCommand {
    ConnectClient {
        player_name: String,
        session_token: Option<SessionToken>,
        tx: mpsc::Sender<ServerMessage>,
        reply: oneshot::Sender<Result<(PlayerId, SessionToken, u8), String>>,
    },
    DisconnectClient(PlayerId),
    ClientAction(PlayerId, ClientMessage),
    TimerEvent(TimerNotification),
    BotTurn(PlayerId),
    EmptyRoomTimeout,
}

pub struct RoomActor {
    room_id: RoomId,
    state: GameState,
    clients: HashMap<PlayerId, mpsc::Sender<ServerMessage>>,
    sessions: HashMap<SessionToken, PlayerId>,
    rng: ChaCha8Rng,
    turn_timer_handle: Option<tokio::task::JoinHandle<()>>,
    grace_timer_handle: Option<tokio::task::JoinHandle<()>>,
    empty_room_timer_handle: Option<tokio::task::JoinHandle<()>>,
    cmd_rx: mpsc::Receiver<RoomCommand>,
    self_tx: mpsc::Sender<RoomCommand>,
    registry: Arc<DashMap<String, mpsc::Sender<RoomCommand>>>,
}

impl RoomActor {
    pub fn spawn(
        room_id: RoomId,
        registry: Arc<DashMap<String, mpsc::Sender<RoomCommand>>>,
    ) -> mpsc::Sender<RoomCommand> {
        let (cmd_tx, cmd_rx) = mpsc::channel(64);
        let initial_tx = cmd_tx.clone();
        let empty_timer = tokio::spawn(async move {
            sleep(Duration::from_secs(60)).await;
            let _ = initial_tx.send(RoomCommand::EmptyRoomTimeout).await;
        });

        let actor = Self {
            room_id: room_id.clone(),
            state: GameState::new(),
            clients: HashMap::new(),
            sessions: HashMap::new(),
            rng: ChaCha8Rng::from_entropy(),
            turn_timer_handle: None,
            grace_timer_handle: None,
            empty_room_timer_handle: Some(empty_timer),
            cmd_rx,
            self_tx: cmd_tx.clone(),
            registry: registry.clone(),
        };

        registry.insert(room_id.0, cmd_tx.clone());
        tokio::spawn(actor.run());
        cmd_tx
    }

    pub async fn run(mut self) {
        info!("Room actor started for room {}", self.room_id);

        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                RoomCommand::ConnectClient {
                    player_name,
                    session_token,
                    tx,
                    reply,
                } => {
                    self.handle_connect(player_name, session_token, tx, reply)
                        .await;
                }
                RoomCommand::DisconnectClient(player_id) => {
                    self.handle_disconnect(player_id).await;
                }
                RoomCommand::ClientAction(player_id, action) => {
                    self.handle_action(player_id, action).await;
                }
                RoomCommand::TimerEvent(notif) => {
                    self.handle_timer(notif).await;
                }
                RoomCommand::BotTurn(bot_id) => {
                    self.execute_bot_sequence(bot_id).await;
                }
                RoomCommand::EmptyRoomTimeout => {
                    if self.clients.is_empty() {
                        info!(
                            "Room {} is empty, shutting down and destroying actor",
                            self.room_id
                        );
                        if let Some(h) = self.turn_timer_handle.take() {
                            h.abort();
                        }
                        if let Some(h) = self.grace_timer_handle.take() {
                            h.abort();
                        }
                        self.registry.remove(&self.room_id.0);
                        return;
                    }
                }
            }
        }

        self.registry.remove(&self.room_id.0);
    }

    async fn handle_connect(
        &mut self,
        player_name: String,
        session_token: Option<SessionToken>,
        tx: mpsc::Sender<ServerMessage>,
        reply: oneshot::Sender<Result<(PlayerId, SessionToken, u8), String>>,
    ) {
        // Cancel empty room cleanup if active
        if let Some(handle) = self.empty_room_timer_handle.take() {
            handle.abort();
        }

        // 1. Reconnection
        if let Some(ref tok) = session_token {
            if let Some(&p_id) = self.sessions.get(tok) {
                if let Some(player) = self.state.find_player_mut(p_id) {
                    player.is_connected = true;
                    let seat = player.seat_index;
                    self.clients.insert(p_id, tx.clone());

                    // Cancel grace timer if running for this player
                    if let Some(handle) = self.grace_timer_handle.take() {
                        handle.abort();
                    }

                    let _ = reply.send(Ok((p_id, tok.clone(), seat)));

                    // Send RoomJoined & SyncSnapshot to reconnecting player
                    let _ = tx
                        .send(ServerMessage::RoomJoined {
                            room_id: self.room_id.clone(),
                            player_id: p_id,
                            session_token: tok.clone(),
                            seat_index: seat,
                        })
                        .await;

                    let _ = tx
                        .send(ServerMessage::SyncSnapshot {
                            state: serde_json::to_value(&self.state).unwrap_or_default(),
                        })
                        .await;

                    self.broadcast(ServerMessage::PlayerReconnected { player_id: p_id })
                        .await;
                    return;
                }
            }
        }

        // 2. New Player Join
        if self.state.is_game_started {
            let _ = reply.send(Err("Game already started".to_string()));
            return;
        }
        if self.state.players.len() >= 4 {
            let _ = reply.send(Err("Room is full".to_string()));
            return;
        }

        let p_id = PlayerId(Uuid::new_v4());
        let tok = SessionToken(Uuid::new_v4().to_string());
        let seat = self.state.players.len() as u8;

        let player = PlayerState::new(p_id, player_name, seat, Some(tok.clone()));
        self.state.players.push(player);
        self.state.turn_order.push(p_id);
        self.sessions.insert(tok.clone(), p_id);
        self.clients.insert(p_id, tx.clone());

        let _ = reply.send(Ok((p_id, tok.clone(), seat)));

        let _ = tx
            .send(ServerMessage::RoomJoined {
                room_id: self.room_id.clone(),
                player_id: p_id,
                session_token: tok,
                seat_index: seat,
            })
            .await;

        self.broadcast(ServerMessage::SyncSnapshot {
            state: serde_json::to_value(&self.state).unwrap_or_default(),
        })
        .await;

        if self.state.players.len() == 4 {
            self.start_game().await;
        }
    }

    pub async fn start_game(&mut self) {
        self.state.is_game_started = true;
        self.state.turn_order = self.state.players.iter().map(|p| p.id).collect();
        self.state.current_player_index = 0;
        self.state.turn_phase = TurnPhase::WaitingForRoll;

        self.broadcast(ServerMessage::GameStarted {
            turn_order: self.state.turn_order.clone(),
        })
        .await;

        let active_id = self.state.turn_order[0];
        self.start_player_turn(active_id).await;
    }

    async fn start_player_turn(&mut self, player_id: PlayerId) {
        if let Some(h) = self.turn_timer_handle.take() {
            h.abort();
        }
        if let Some(h) = self.grace_timer_handle.take() {
            h.abort();
        }

        self.broadcast(ServerMessage::TurnStarted {
            active_player: player_id,
            time_limit_seconds: 120,
        })
        .await;

        self.broadcast(ServerMessage::SyncSnapshot {
            state: serde_json::to_value(&self.state).unwrap_or_default(),
        })
        .await;

        let (is_connected, is_bot) = self
            .state
            .find_player(player_id)
            .map(|p| (p.is_connected, p.is_bot_controlled))
            .unwrap_or((false, false));

        if is_bot {
            let tx = self.self_tx.clone();
            tokio::spawn(async move {
                sleep(Duration::from_millis(600)).await;
                let _ = tx.send(RoomCommand::BotTurn(player_id)).await;
            });
            return;
        }

        if !is_connected {
            // Player disconnected: trigger bot after short delay
            let tx = self.self_tx.clone();
            tokio::spawn(async move {
                sleep(Duration::from_millis(500)).await;
                let _ = tx
                    .send(RoomCommand::TimerEvent(TimerNotification::GraceExpired(
                        player_id,
                    )))
                    .await;
            });
            return;
        }

        // Spawn 2-minute (120-second) turn countdown
        let (notify_tx, mut notify_rx) = mpsc::channel(32);
        let handle = TurnTimer::spawn_turn_countdown(player_id, 120, notify_tx);
        self.turn_timer_handle = Some(handle);

        let self_tx = self.self_tx.clone();
        tokio::spawn(async move {
            while let Some(notif) = notify_rx.recv().await {
                if self_tx.send(RoomCommand::TimerEvent(notif)).await.is_err() {
                    break;
                }
            }
        });
    }

    async fn handle_disconnect(&mut self, player_id: PlayerId) {
        self.clients.remove(&player_id);
        if let Some(p) = self.state.find_player_mut(player_id) {
            p.is_connected = false;
        }

        self.broadcast(ServerMessage::SyncSnapshot {
            state: serde_json::to_value(&self.state).unwrap_or_default(),
        })
        .await;

        // If disconnected player was active player, start 15s grace timer
        if let Some(cur) = self.state.current_player() {
            if cur.id == player_id && self.state.is_game_started && !self.state.is_game_over {
                let (notify_tx, mut notify_rx) = mpsc::channel(16);
                let handle = TurnTimer::spawn_disconnect_grace(player_id, 15, notify_tx);
                self.grace_timer_handle = Some(handle);

                let self_tx = self.self_tx.clone();
                tokio::spawn(async move {
                    if let Some(notif) = notify_rx.recv().await {
                        let _ = self_tx.send(RoomCommand::TimerEvent(notif)).await;
                    }
                });
            }
        }

        // If all clients disconnected, auto-destroy room shortly (5s if unstarted, 15s grace if started)
        if self.clients.is_empty() {
            let self_tx = self.self_tx.clone();
            let delay_secs = if self.state.is_game_started { 15 } else { 5 };
            self.empty_room_timer_handle = Some(tokio::spawn(async move {
                sleep(Duration::from_secs(delay_secs)).await;
                let _ = self_tx.send(RoomCommand::EmptyRoomTimeout).await;
            }));
        }
    }

    async fn handle_action(&mut self, player_id: PlayerId, action: ClientMessage) {
        if !self.state.is_game_started {
            let is_host = self
                .state
                .find_player(player_id)
                .map(|p| p.seat_index == 0)
                .unwrap_or(false);

            match action {
                ClientMessage::ConfigureGame { enable_auction } => {
                    if is_host {
                        self.state.config.enable_auction = enable_auction;
                        self.broadcast(ServerMessage::SyncSnapshot {
                            state: serde_json::to_value(&self.state).unwrap_or_default(),
                        })
                        .await;
                    }
                }
                ClientMessage::AddBot => {
                    if !is_host {
                        if let Some(tx) = self.clients.get(&player_id) {
                            let _ = tx
                                .send(ServerMessage::ActionRejected {
                                    reason: "Chỉ chủ phòng mới có thể thêm máy AI".to_string(),
                                })
                                .await;
                        }
                        return;
                    }
                    if self.state.players.len() >= 4 {
                        if let Some(tx) = self.clients.get(&player_id) {
                            let _ = tx
                                .send(ServerMessage::ActionRejected {
                                    reason: "Phòng đã đủ 4 người chơi".to_string(),
                                })
                                .await;
                        }
                        return;
                    }
                    let bot_names = [
                        "Máy Phú Nhuận",
                        "Máy Chợ Lớn",
                        "Máy Thảo Điền",
                        "Máy Tân Định",
                    ];
                    let seat = self.state.players.len() as u8;
                    let bot_name = bot_names
                        .get(seat as usize)
                        .copied()
                        .unwrap_or("Máy AI")
                        .to_string();
                    let bot_id = PlayerId(Uuid::new_v4());
                    let mut bot = PlayerState::new(bot_id, bot_name, seat, None);
                    bot.is_bot_controlled = true;
                    bot.is_connected = true;
                    self.state.players.push(bot);
                    self.state.turn_order.push(bot_id);
                    self.broadcast(ServerMessage::SyncSnapshot {
                        state: serde_json::to_value(&self.state).unwrap_or_default(),
                    })
                    .await;
                }
                ClientMessage::StartGame => {
                    if !is_host {
                        if let Some(tx) = self.clients.get(&player_id) {
                            let _ = tx
                                .send(ServerMessage::ActionRejected {
                                    reason: "Chỉ chủ phòng mới có thể bắt đầu ván đấu".to_string(),
                                })
                                .await;
                        }
                        return;
                    }
                    if self.state.players.len() < 2 {
                        if let Some(tx) = self.clients.get(&player_id) {
                            let _ = tx
                                .send(ServerMessage::ActionRejected {
                                    reason: "Cần ít nhất 2 người chơi để bắt đầu".to_string(),
                                })
                                .await;
                        }
                        return;
                    }
                    self.start_game().await;
                }
                _ => {
                    if let Some(tx) = self.clients.get(&player_id) {
                        let _ = tx
                            .send(ServerMessage::ActionRejected {
                                reason: "Ván đấu chưa bắt đầu".to_string(),
                            })
                            .await;
                    }
                }
            }
            return;
        }

        match apply_action(&self.state, player_id, &action, &mut self.rng) {
            Ok((new_state, events)) => {
                let old_player_index = self.state.current_player_index;
                let old_turn_phase = self.state.turn_phase;

                self.state = new_state;

                for ev in events {
                    self.broadcast(ev).await;
                }

                self.broadcast(ServerMessage::SyncSnapshot {
                    state: serde_json::to_value(&self.state).unwrap_or_default(),
                })
                .await;

                // Check if turn advanced
                if self.state.current_player_index != old_player_index
                    || (old_turn_phase == TurnPhase::TurnEnded
                        && self.state.turn_phase == TurnPhase::WaitingForRoll)
                {
                    if let Some(active) = self.state.current_player() {
                        let active_id = active.id;
                        self.start_player_turn(active_id).await;
                    }
                } else if self.state.turn_phase == TurnPhase::Auction {
                    if let Some(auc) = &self.state.auction {
                        if let Some(&bot_bidder) = auc.active_bidders.iter().find(|bidder_id| {
                            self.state
                                .find_player(**bidder_id)
                                .map(|p| p.is_bot_controlled)
                                .unwrap_or(false)
                        }) {
                            let tx = self.self_tx.clone();
                            tokio::spawn(async move {
                                sleep(Duration::from_millis(500)).await;
                                let _ = tx.send(RoomCommand::BotTurn(bot_bidder)).await;
                            });
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Action rejected for {}: {}", player_id, e);
                if let Some(tx) = self.clients.get(&player_id) {
                    let _ = tx
                        .send(ServerMessage::ActionRejected {
                            reason: e.to_string(),
                        })
                        .await;
                }
            }
        }
    }

    async fn handle_timer(&mut self, notif: TimerNotification) {
        match notif {
            TimerNotification::Tick(s) => {
                self.broadcast(ServerMessage::TimerTick {
                    seconds_remaining: s,
                })
                .await;
            }
            TimerNotification::TurnExpired(p_id) => {
                self.broadcast(ServerMessage::BotAssumedControl {
                    player_id: p_id,
                    reason: "Timeout".to_string(),
                })
                .await;
                self.execute_bot_sequence(p_id).await;
            }
            TimerNotification::GraceExpired(p_id) => {
                self.broadcast(ServerMessage::BotAssumedControl {
                    player_id: p_id,
                    reason: "Disconnected".to_string(),
                })
                .await;
                self.execute_bot_sequence(p_id).await;
            }
        }
    }

    async fn execute_bot_sequence(&mut self, player_id: PlayerId) {
        while let Some(bot_action) = XeOmBot::decide_action(&self.state, player_id) {
            let is_end_turn = matches!(bot_action, ClientMessage::EndTurn);
            self.handle_action(player_id, bot_action).await;
            if is_end_turn || self.state.is_game_over {
                break;
            }
        }
    }

    async fn broadcast(&self, msg: ServerMessage) {
        for tx in self.clients.values() {
            let _ = tx.send(msg.clone()).await;
        }
    }
}

use crate::registry::LobbyRegistry;
use monopoly_core::bot::BotPolicy;
use monopoly_core::rules::apply_action;
use monopoly_core::state::{GameState, PlayerState, TurnPhase};
use monopoly_proto::{
    ClientMessage, MessageKey, PlayerId, RoomId, ServerMessage, SessionToken,
};
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;
use tracing::{info, warn};
use uuid::Uuid;

pub enum RoomCommand {
    ConnectClient {
        player_name: String,
        session_token: Option<SessionToken>,
        tx: mpsc::Sender<Arc<String>>,
        reply: oneshot::Sender<Result<(PlayerId, SessionToken, u8), MessageKey>>,
    },
    DisconnectClient(PlayerId),
    ClientAction(PlayerId, ClientMessage),
    EmptyRoomTimeout,
}

pub struct RoomActor {
    room_id: RoomId,
    state: GameState,
    clients: HashMap<PlayerId, mpsc::Sender<Arc<String>>>,
    sessions: HashMap<SessionToken, PlayerId>,
    rng: ChaCha8Rng,
    turn_deadline: Option<(PlayerId, Instant)>,
    grace_deadline: Option<(PlayerId, Instant)>,
    bot_deadline: Option<(PlayerId, Instant)>,
    empty_deadline: Option<Instant>,
    cmd_rx: mpsc::Receiver<RoomCommand>,
    registry: LobbyRegistry,
}

impl RoomActor {
    pub fn spawn(room_id: RoomId, registry: LobbyRegistry) -> mpsc::Sender<RoomCommand> {
        let (cmd_tx, cmd_rx) = mpsc::channel(64);
        let now = Instant::now();

        let actor = Self {
            room_id: room_id.clone(),
            state: GameState::new(),
            clients: HashMap::new(),
            sessions: HashMap::new(),
            rng: ChaCha8Rng::from_entropy(),
            turn_deadline: None,
            grace_deadline: None,
            bot_deadline: None,
            empty_deadline: Some(now + Duration::from_secs(60)),
            cmd_rx,
            registry: registry.clone(),
        };

        registry.insert(room_id.0, cmd_tx.clone());
        tokio::spawn(actor.run());
        cmd_tx
    }

    fn next_deadline(&self) -> Option<Instant> {
        [
            self.turn_deadline.map(|(_, t)| t),
            self.grace_deadline.map(|(_, t)| t),
            self.bot_deadline.map(|(_, t)| t),
            self.empty_deadline,
        ]
        .into_iter()
        .flatten()
        .min()
    }

    pub async fn run(mut self) {
        info!("Room actor started for room {}", self.room_id);

        loop {
            let next = self.next_deadline();
            tokio::select! {
                cmd = self.cmd_rx.recv() => {
                    match cmd {
                        Some(RoomCommand::ConnectClient {
                            player_name,
                            session_token,
                            tx,
                            reply,
                        }) => {
                            self.handle_connect(player_name, session_token, tx, reply).await;
                        }
                        Some(RoomCommand::DisconnectClient(player_id)) => {
                            self.handle_disconnect(player_id).await;
                        }
                        Some(RoomCommand::ClientAction(player_id, action)) => {
                            self.handle_action(player_id, action).await;
                        }
                        Some(RoomCommand::EmptyRoomTimeout) => {
                            if self.clients.is_empty() {
                                self.registry.remove(&self.room_id.0);
                                return;
                            }
                        }
                        None => break,
                    }
                }
                _ = async {
                    match next {
                        Some(deadline) => tokio::time::sleep_until(deadline).await,
                        None => std::future::pending::<()>().await,
                    }
                } => {
                    let now = Instant::now();
                    if let Some((_, d)) = self.empty_deadline.map(|d| ((), d)).filter(|(_, d)| *d <= now) {
                        let _ = d;
                        self.empty_deadline = None;
                        if self.clients.is_empty() {
                            info!(
                                "Room {} is empty, shutting down and destroying actor",
                                self.room_id
                            );
                            self.registry.remove(&self.room_id.0);
                            return;
                        }
                    }
                    if let Some((p_id, d)) = self.grace_deadline {
                        if d <= now {
                            self.grace_deadline = None;
                            self.broadcast(ServerMessage::BotAssumedControl {
                                player_id: p_id,
                                reason: MessageKey::Disconnected,
                            });
                            self.execute_bot_sequence(p_id).await;
                            continue;
                        }
                    }
                    if let Some((p_id, d)) = self.turn_deadline {
                        if d <= now {
                            self.turn_deadline = None;
                            self.broadcast(ServerMessage::BotAssumedControl {
                                player_id: p_id,
                                reason: MessageKey::Timeout,
                            });
                            self.execute_bot_sequence(p_id).await;
                            continue;
                        }
                    }
                    if let Some((bot_id, d)) = self.bot_deadline {
                        if d <= now {
                            self.bot_deadline = None;
                            self.execute_bot_sequence(bot_id).await;
                        }
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
        tx: mpsc::Sender<Arc<String>>,
        reply: oneshot::Sender<Result<(PlayerId, SessionToken, u8), MessageKey>>,
    ) {
        self.empty_deadline = None;

        // 1. Reconnection
        if let Some(ref tok) = session_token {
            if let Some(&p_id) = self.sessions.get(tok) {
                if let Some(player) = self.state.find_player_mut(p_id) {
                    player.is_connected = true;
                    let seat = player.seat_index;
                    self.clients.insert(p_id, tx.clone());

                    if self.grace_deadline.is_some_and(|(id, _)| id == p_id) {
                        self.grace_deadline = None;
                    }

                    let _ = reply.send(Ok((p_id, tok.clone(), seat)));

                    Self::send_to_client(
                        &tx,
                        &ServerMessage::RoomJoined {
                            room_id: self.room_id.clone(),
                            player_id: p_id,
                            session_token: tok.clone(),
                            seat_index: seat,
                        },
                    );

                    Self::send_to_client(
                        &tx,
                        &ServerMessage::SyncView {
                            view: self.state.to_public_view(),
                        },
                    );

                    self.broadcast(ServerMessage::PlayerReconnected { player_id: p_id });
                    return;
                }
            }
        }

        // 2. New Player Join
        if self.state.is_game_started {
            let _ = reply.send(Err(MessageKey::RoomAlreadyStarted));
            return;
        }

        let max_players = self.state.config.max_players.clamp(2, 4) as usize;
        if self.state.players.len() >= max_players {
            let _ = reply.send(Err(MessageKey::RoomFull));
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

        Self::send_to_client(
            &tx,
            &ServerMessage::RoomJoined {
                room_id: self.room_id.clone(),
                player_id: p_id,
                session_token: tok,
                seat_index: seat,
            },
        );

        if self.state.players.len() >= max_players {
            self.start_game().await;
        } else {
            self.broadcast(ServerMessage::SyncView {
                view: self.state.to_public_view(),
            });
        }
    }

    pub async fn start_game(&mut self) {
        self.state.is_game_started = true;
        self.state.turn_order = self.state.players.iter().map(|p| p.id).collect();
        self.state.current_player_index = 0;
        self.state.turn_phase = TurnPhase::WaitingForRoll;

        self.broadcast(ServerMessage::GameStarted {
            turn_order: self.state.turn_order.clone(),
        });

        let active_id = self.state.turn_order[0];
        self.schedule_player_turn(active_id, true);
    }

    fn schedule_player_turn(&mut self, player_id: PlayerId, emit_turn_started: bool) {
        self.turn_deadline = None;
        self.grace_deadline = None;
        self.bot_deadline = None;

        if emit_turn_started {
            self.broadcast(ServerMessage::TurnStarted {
                active_player: player_id,
                time_limit_seconds: self.state.config.turn_time_limit_seconds,
            });
            self.broadcast(ServerMessage::SyncView {
                view: self.state.to_public_view(),
            });
        }

        let (is_connected, is_bot) = self
            .state
            .find_player(player_id)
            .map(|p| (p.is_connected, p.is_bot_controlled))
            .unwrap_or((false, false));

        let now = Instant::now();
        if is_bot {
            self.bot_deadline = Some((player_id, now + Duration::from_millis(600)));
        } else if !is_connected {
            self.grace_deadline = Some((player_id, now + Duration::from_millis(500)));
        } else {
            let secs = self.state.config.turn_time_limit_seconds as u64;
            self.turn_deadline = Some((player_id, now + Duration::from_secs(secs)));
        }
    }

    async fn handle_disconnect(&mut self, player_id: PlayerId) {
        self.clients.remove(&player_id);
        if let Some(p) = self.state.find_player_mut(player_id) {
            p.is_connected = false;
        }

        self.broadcast(ServerMessage::SyncView {
            view: self.state.to_public_view(),
        });

        let now = Instant::now();
        if let Some(cur) = self.state.current_player() {
            if cur.id == player_id && self.state.is_game_started && !self.state.is_game_over {
                self.grace_deadline = Some((player_id, now + Duration::from_secs(15)));
            }
        }

        if self.clients.is_empty() {
            let delay_secs = if self.state.is_game_started { 15 } else { 5 };
            self.empty_deadline = Some(now + Duration::from_secs(delay_secs));
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
                ClientMessage::ConfigureGame {
                    enable_auction,
                    max_players,
                } => {
                    if is_host {
                        self.state.config.enable_auction = enable_auction;
                        if let Some(mp) = max_players {
                            let min_allowed = self.state.players.len().max(2) as u8;
                            self.state.config.max_players = mp.clamp(2, 4).max(min_allowed);
                        }
                        self.broadcast(ServerMessage::SyncView {
                            view: self.state.to_public_view(),
                        });
                    }
                }
                ClientMessage::AddBot => {
                    if !is_host {
                        self.reject_player(player_id, MessageKey::HostOnlyAddBot);
                        return;
                    }
                    let max_players = self.state.config.max_players.clamp(2, 4) as usize;
                    if self.state.players.len() >= max_players {
                        self.reject_player(player_id, MessageKey::RoomFull);
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
                    self.broadcast(ServerMessage::SyncView {
                        view: self.state.to_public_view(),
                    });
                }
                ClientMessage::StartGame => {
                    if !is_host {
                        self.reject_player(player_id, MessageKey::HostOnlyStart);
                        return;
                    }
                    if self.state.players.len() < 2 {
                        self.reject_player(player_id, MessageKey::NeedAtLeastTwoPlayers);
                        return;
                    }
                    self.start_game().await;
                }
                _ => {
                    self.reject_player(player_id, MessageKey::GameNotStarted);
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
                    self.broadcast(ev);
                }

                self.broadcast(ServerMessage::SyncView {
                    view: self.state.to_public_view(),
                });

                if self.state.current_player_index != old_player_index
                    || (old_turn_phase == TurnPhase::TurnEnded
                        && self.state.turn_phase == TurnPhase::WaitingForRoll)
                {
                    if let Some(active) = self.state.current_player() {
                        let active_id = active.id;
                        self.schedule_player_turn(active_id, false);
                    }
                } else if self.state.turn_phase == TurnPhase::Auction {
                    if let Some(auc) = &self.state.auction {
                        if let Some(&bot_bidder) = auc.active_bidders.iter().find(|bidder_id| {
                            self.state
                                .find_player(**bidder_id)
                                .map(|p| p.is_bot_controlled)
                                .unwrap_or(false)
                        }) {
                            self.bot_deadline =
                                Some((bot_bidder, Instant::now() + Duration::from_millis(500)));
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Action rejected for {}: {}", player_id, e);
                self.reject_player(player_id, e.to_message_key());
            }
        }
    }

    async fn execute_bot_sequence(&mut self, player_id: PlayerId) {
        while let Some(bot_action) = BotPolicy::decide_action_state(&self.state, player_id) {
            let is_end_turn = matches!(bot_action, ClientMessage::EndTurn);
            Box::pin(self.handle_action(player_id, bot_action)).await;
            if is_end_turn || self.state.is_game_over {
                break;
            }
        }
    }

    fn reject_player(&self, player_id: PlayerId, reason: MessageKey) {
        if let Some(tx) = self.clients.get(&player_id) {
            Self::send_to_client(tx, &ServerMessage::ActionRejected { reason });
        }
    }

    fn send_to_client(tx: &mpsc::Sender<Arc<String>>, msg: &ServerMessage) {
        if let Ok(json) = serde_json::to_string(msg) {
            let _ = tx.try_send(Arc::new(json));
        }
    }

    fn broadcast(&self, msg: ServerMessage) {
        if let Ok(json) = serde_json::to_string(&msg) {
            let payload = Arc::new(json);
            for tx in self.clients.values() {
                let _ = tx.try_send(payload.clone());
            }
        }
    }
}

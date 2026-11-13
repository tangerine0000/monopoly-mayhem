use crate::components::board::{Board, BoardFloatingPopup, PlayerAvatar};
use crate::components::log::LogFeed;
use crate::palette::{
    get_district_color, ACCENT_PRIMARY, BG_SECONDARY, BORDER_DEFAULT, DISTRICT_1_DARK_BLUE,
    STATUS_DANGER, STATUS_SUCCESS, STATUS_WARNING, SURFACE_HOVER, TEXT_MUTED, TEXT_PRIMARY,
    TEXT_SECONDARY,
};
use crate::ws::{save_storage_session_token, WsClient};
use leptos::*;
use monopoly_core::board::{get_board, District, TileKind};
use monopoly_core::money::Money;
use monopoly_core::rules::apply_action;
use monopoly_core::state::{GameState, PlayerState, TurnPhase};
use monopoly_proto::{ClientMessage, PlayerId, ServerMessage};
use std::rc::Rc;
use std::time::Duration;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = playMonopolySound)]
    fn play_monopoly_sound(kind: &str);

    #[wasm_bindgen(js_namespace = window, js_name = setMonopolySoundMuted)]
    fn set_monopoly_sound_muted(muted: bool);
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Lobby,
    Game,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LobbyTab {
    Online,
    Local,
}

#[derive(Clone, PartialEq, Eq)]
pub struct LocalPlayerSlot {
    pub name: String,
    pub is_bot: bool,
}

fn decide_local_bot_action(state: &GameState, player_id: PlayerId) -> Option<ClientMessage> {
    let player = state.find_player(player_id)?;
    if player.is_bankrupt {
        return None;
    }

    match state.turn_phase {
        TurnPhase::WaitingForRoll => {
            if player.in_jail {
                if player.get_out_of_jail_cards > 0 {
                    Some(ClientMessage::UseJailCard)
                } else if player.cash > Money::from_vnd(5_000_000) {
                    Some(ClientMessage::PayJailBail)
                } else {
                    Some(ClientMessage::RollDice)
                }
            } else {
                Some(ClientMessage::RollDice)
            }
        }
        TurnPhase::TileActionPending => {
            let pos = player.position as usize;
            let board = get_board();
            let tile = &board[pos];
            let cost = match &tile.kind {
                TileKind::Property(p) => p.cost,
                TileKind::Station(s) => s.cost,
                TileKind::Utility(u) => u.cost,
                _ => Money::ZERO,
            };

            if cost > Money::ZERO && player.cash >= Money(cost.0 * 2) {
                Some(ClientMessage::BuyProperty)
            } else {
                Some(ClientMessage::DeclineProperty)
            }
        }
        TurnPhase::Auction => {
            let auction = state.auction.as_ref()?;
            let board = get_board();
            let tile = &board[auction.tile_index as usize];
            let cost = match &tile.kind {
                TileKind::Property(p) => p.cost,
                TileKind::Station(s) => s.cost,
                TileKind::Utility(u) => u.cost,
                _ => Money::ZERO,
            };

            let half_cost = Money(cost.0 / 2);
            let next_bid = auction.current_bid + Money::from_vnd(50_000);

            if auction.current_bid < half_cost
                && player.cash >= Money(auction.current_bid.0 * 3)
                && player.cash >= next_bid
            {
                Some(ClientMessage::PlaceBid { amount: next_bid })
            } else {
                Some(ClientMessage::PassAuction)
            }
        }
        TurnPhase::TurnEnded => Some(ClientMessage::EndTurn),
    }
}

fn schedule_when_animation_done(
    is_rolling: ReadSignal<bool>,
    is_animating_move: ReadSignal<bool>,
    delay_ms: u64,
    task: std::rc::Rc<std::cell::RefCell<Option<Box<dyn FnOnce()>>>>,
) {
    set_timeout(
        move || {
            if is_rolling.get_untracked() || is_animating_move.get_untracked() {
                schedule_when_animation_done(is_rolling, is_animating_move, 120, task);
            } else if let Some(f) = task.borrow_mut().take() {
                f();
            }
        },
        Duration::from_millis(delay_ms),
    );
}

fn execute_local_action(
    actor_id: PlayerId,
    action: ClientMessage,
    local_state: ReadSignal<GameState>,
    set_local_state: WriteSignal<GameState>,
    set_my_player_id: WriteSignal<Option<PlayerId>>,
    is_local_mode: ReadSignal<bool>,
    is_rolling: ReadSignal<bool>,
    is_animating_move: ReadSignal<bool>,
    on_msg: Callback<ServerMessage>,
) {
    if !is_local_mode.get_untracked() {
        return;
    }
    let current_state = local_state.get_untracked();
    if current_state.is_game_over {
        return;
    }

    match apply_action(&current_state, actor_id, &action, &mut rand::thread_rng()) {
        Ok((mut new_state, mut events)) => {
            let had_roll = events
                .iter()
                .any(|ev| matches!(ev, ServerMessage::DiceRolled { .. }));
            let actor_is_bot = current_state
                .find_player(actor_id)
                .map(|p| p.is_bot_controlled)
                .unwrap_or(false);

            if new_state.players.len() == 1 {
                new_state.config.enable_auction = false;
            }

            if !actor_is_bot
                && !new_state.is_game_over
                && new_state.turn_phase == TurnPhase::TurnEnded
                && new_state.players.len() == 1
            {
                if let Ok((end_state, end_events)) =
                    apply_action(&new_state, actor_id, &ClientMessage::EndTurn, &mut rand::thread_rng())
                {
                    new_state = end_state;
                    events.extend(end_events);
                }
            }

            set_local_state.set(new_state.clone());

            for ev in events {
                on_msg.call(ev);
            }
            on_msg.call(ServerMessage::SyncSnapshot {
                state: serde_json::to_value(&new_state).unwrap_or_default(),
            });

            if new_state.is_game_over {
                return;
            }

            if new_state.turn_phase == TurnPhase::Auction {
                if let Some(auc) = new_state.auction.as_ref() {
                    let human_bidders: Vec<PlayerId> = auc
                        .active_bidders
                        .iter()
                        .copied()
                        .filter(|id| {
                            new_state
                                .find_player(*id)
                                .map(|p| !p.is_bot_controlled && !p.is_bankrupt)
                                .unwrap_or(false)
                        })
                        .collect();
                    if !human_bidders.is_empty() {
                        let next_human = if let Some(pos) =
                            human_bidders.iter().position(|id| *id == actor_id)
                        {
                            human_bidders[(pos + 1) % human_bidders.len()]
                        } else {
                            human_bidders[0]
                        };
                        set_my_player_id.set(Some(next_human));
                    }
                }
            } else if let Some(cur) = new_state.current_player() {
                if !cur.is_bot_controlled {
                    set_my_player_id.set(Some(cur.id));
                }
            }

            if !actor_is_bot
                && new_state.turn_phase == TurnPhase::TurnEnded
                && new_state.players.len() > 1
            {
                let delay_ms = if had_roll { 350 } else { 300 };
                let task: std::rc::Rc<std::cell::RefCell<Option<Box<dyn FnOnce()>>>> =
                    std::rc::Rc::new(std::cell::RefCell::new(Some(Box::new(move || {
                        if !is_local_mode.get_untracked() {
                            return;
                        }
                        let st = local_state.get_untracked();
                        if st.is_game_over
                            || st.turn_phase != TurnPhase::TurnEnded
                            || st.current_player().map(|p| p.id) != Some(actor_id)
                        {
                            return;
                        }
                        execute_local_action(
                            actor_id,
                            ClientMessage::EndTurn,
                            local_state,
                            set_local_state,
                            set_my_player_id,
                            is_local_mode,
                            is_rolling,
                            is_animating_move,
                            on_msg,
                        );
                    }))));
                schedule_when_animation_done(is_rolling, is_animating_move, delay_ms, task);
            }

            let next_bot_id = if new_state.turn_phase == TurnPhase::Auction {
                new_state.auction.as_ref().and_then(|auc| {
                    auc.active_bidders
                        .iter()
                        .copied()
                        .find(|bidder_id| {
                            new_state
                                .find_player(*bidder_id)
                                .map(|p| p.is_bot_controlled)
                                .unwrap_or(false)
                        })
                })
            } else {
                new_state
                    .current_player()
                    .filter(|p| p.is_bot_controlled)
                    .map(|p| p.id)
            };

            if let Some(bot_id) = next_bot_id {
                let delay_ms = if had_roll { 450 } else { 550 };
                let task: std::rc::Rc<std::cell::RefCell<Option<Box<dyn FnOnce()>>>> =
                    std::rc::Rc::new(std::cell::RefCell::new(Some(Box::new(move || {
                        if !is_local_mode.get_untracked() {
                            return;
                        }
                        let st = local_state.get_untracked();
                        if st.is_game_over {
                            return;
                        }
                        if let Some(bot_action) = decide_local_bot_action(&st, bot_id) {
                            execute_local_action(
                                bot_id,
                                bot_action,
                                local_state,
                                set_local_state,
                                set_my_player_id,
                                is_local_mode,
                                is_rolling,
                                is_animating_move,
                                on_msg,
                            );
                        }
                    }))));
                schedule_when_animation_done(is_rolling, is_animating_move, delay_ms, task);
            }
        }
        Err(e) => {
            on_msg.call(ServerMessage::ActionRejected {
                reason: e.to_string(),
            });
        }
    }
}

#[component]
pub fn App() -> impl IntoView {
    let (view_mode, set_view_mode) = create_signal(ViewMode::Lobby);
    let (lobby_tab, set_lobby_tab) = create_signal(LobbyTab::Online);
    let (local_player_count, set_local_player_count) = create_signal(2usize);
    let (local_slots, set_local_slots) = create_signal(vec![
        LocalPlayerSlot {
            name: "Người chơi 1".to_string(),
            is_bot: false,
        },
        LocalPlayerSlot {
            name: "Người chơi 2".to_string(),
            is_bot: false,
        },
        LocalPlayerSlot {
            name: "Người chơi 3".to_string(),
            is_bot: false,
        },
        LocalPlayerSlot {
            name: "Người chơi 4".to_string(),
            is_bot: false,
        },
    ]);
    let (player_name, set_player_name) = create_signal("Nguyễn Minh".to_string());
    let (room_id, set_room_id) = create_signal("SGN-888".to_string());
    let (enable_auction_config, set_enable_auction_config) = create_signal(false);
    let (game_state, set_game_state) = create_signal(GameState::new());
    let (my_player_id, set_my_player_id) = create_signal::<Option<PlayerId>>(None);
    let (timer_seconds, set_timer_seconds) = create_signal(120u32);
    let (logs, set_logs) = create_signal(vec!["Phiên bàn cờ đã sẵn sàng".to_string()]);
    let (show_history, set_show_history) = create_signal(false);
    let (show_rules, set_show_rules) = create_signal(false);
    let (selected_tile, set_selected_tile) = create_signal::<Option<u8>>(None);
    let (sound_muted, set_sound_muted) = create_signal(false);

    // Animation signals for sequenced dice tumble, step-by-step pawn hopping, trail, and popups
    let (is_rolling, set_is_rolling) = create_signal(false);
    let (is_animating_move, set_is_animating_move) = create_signal(false);
    let (display_dice, set_display_dice) = create_signal((1u8, 1u8));
    let (visual_positions, set_visual_positions) = create_signal([0u8; 4]);
    let (hopping_seat, set_hopping_seat) = create_signal::<Option<usize>>(None);
    let (step_trail, set_step_trail) = create_signal::<Vec<u8>>(Vec::new());
    let (landing_tile_burst, set_landing_tile_burst) = create_signal::<Option<u8>>(None);
    let (recent_acquired_tile, set_recent_acquired_tile) = create_signal::<Option<u8>>(None);
    let (recent_house_tile, set_recent_house_tile) = create_signal::<Option<u8>>(None);
    let (floating_popups, set_floating_popups) = create_signal::<Vec<BoardFloatingPopup>>(Vec::new());
    let (popup_seq, set_popup_seq) = create_signal(1u64);
    let (cash_delta_badge, set_cash_delta_badge) = create_signal::<Option<(String, bool)>>(None);
    let (drawn_card_banner, set_drawn_card_banner) =
        create_signal::<Option<(String, String, String)>>(None);

    // Buffered state/card while dice tumble & pawn hops so UI updates right when landing
    let (pending_snapshot, set_pending_snapshot) = create_signal::<Option<GameState>>(None);
    let (pending_card, set_pending_card) =
        create_signal::<Option<(String, String, String)>>(None);

    let (ws_client, set_ws_client) = create_signal::<Option<Rc<WsClient>>>(None);
    let (error_toast, set_error_toast) = create_signal::<Option<String>>(None);

    let add_log = move |msg: String| {
        set_logs.update(|l| l.push(msg));
    };

    let spawn_board_popup = move |tile_index: u8, text: String, is_positive: bool| {
        let id = popup_seq.get_untracked();
        set_popup_seq.set(id + 1);
        set_floating_popups.update(|list| {
            list.push(BoardFloatingPopup {
                id,
                tile_index,
                text,
                is_positive,
            });
        });
        set_timeout(
            move || {
                set_floating_popups.update(|list| {
                    list.retain(|item| item.id != id);
                });
            },
            Duration::from_millis(1600),
        );
    };

    let commit_snapshot = move |new_state: GameState| {
        if let Some(my_id) = my_player_id.get_untracked() {
            let old_cash = game_state
                .get_untracked()
                .find_player(my_id)
                .map(|p| p.cash.0);
            let new_cash = new_state.find_player(my_id).map(|p| p.cash.0);
            if let (Some(old_c), Some(new_c)) = (old_cash, new_cash) {
                let diff = new_c - old_c;
                if diff != 0 {
                    let formatted = if diff > 0 {
                        format!("+{}", Money(diff))
                    } else {
                        format!("{}", Money(diff))
                    };
                    set_cash_delta_badge.set(Some((formatted, diff > 0)));
                    set_timeout(
                        move || {
                            set_cash_delta_badge.set(None);
                        },
                        Duration::from_millis(1500),
                    );
                }
            }
        }
        set_game_state.set(new_state);
    };

    let (is_local_mode, set_is_local_mode) = create_signal(false);
    let (local_authoritative_state, set_local_authoritative_state) = create_signal(GameState::new());

    let process_server_message = Callback::new(move |server_msg: ServerMessage| match server_msg {
        ServerMessage::RoomJoined {
            room_id: joined_room,
            player_id,
            session_token,
            seat_index,
        } => {
            set_my_player_id.set(Some(player_id));
            save_storage_session_token(&joined_room.0, &session_token);
            add_log(format!(
                "Đã vào phòng {} tại vị trí T{}",
                joined_room.0,
                seat_index + 1
            ));
        }
                ServerMessage::GameStarted { turn_order: _ } => {
                    add_log("Ván đấu bắt đầu".to_string());
                }
                ServerMessage::TurnStarted {
                    active_player,
                    time_limit_seconds,
                } => {
                    set_timer_seconds.set(time_limit_seconds);
                    let latest_state = pending_snapshot
                        .get_untracked()
                        .unwrap_or_else(|| game_state.get_untracked());
                    let name = latest_state
                        .find_player(active_player)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| "Người chơi".to_string());
                    add_log(format!("Đến lượt của {}", name));
                }
                ServerMessage::TimerTick { seconds_remaining } => {
                    set_timer_seconds.set(seconds_remaining);
                    if seconds_remaining <= 5 && seconds_remaining > 0 {
                        let is_my_turn = my_player_id.get_untracked().is_some()
                            && game_state.get_untracked().current_player().map(|p| p.id)
                                == my_player_id.get_untracked();
                        if is_my_turn {
                            play_monopoly_sound("urgent");
                        }
                    }
                    if seconds_remaining == 30 {
                        let is_my_turn = my_player_id.get_untracked().is_some()
                            && game_state.get_untracked().current_player().map(|p| p.id)
                                == my_player_id.get_untracked();
                        if is_my_turn {
                            let warn_msg = "Cảnh báo: Còn 30 giây trước khi hết lượt!".to_string();
                            set_error_toast.set(Some(warn_msg.clone()));
                            add_log(warn_msg);
                        }
                    }
                }
                ServerMessage::DiceRolled {
                    player_id,
                    dice,
                    new_tile_index,
                    is_doubles,
                } => {
                    play_monopoly_sound("roll");
                    set_drawn_card_banner.set(None);
                    set_pending_card.set(None);
                    set_landing_tile_burst.set(None);
                    set_step_trail.set(Vec::new());
                    set_is_rolling.set(true);
                    set_is_animating_move.set(true);

                    let shuffle_faces: [(u8, u8); 7] = [
                        (6, 2),
                        (3, 5),
                        (1, 4),
                        (5, 6),
                        (2, 3),
                        (4, 1),
                        (6, 5),
                    ];
                    for (frame_idx, pair) in shuffle_faces.into_iter().enumerate() {
                        set_timeout(
                            move || {
                                if is_rolling.get_untracked() {
                                    set_display_dice.set(pair);
                                }
                            },
                            Duration::from_millis((frame_idx as u64) * 70),
                        );
                    }

                    let seat_idx = game_state
                        .get_untracked()
                        .players
                        .iter()
                        .position(|p| p.id == player_id)
                        .unwrap_or(0)
                        % 4;
                    let start_tile = visual_positions.get_untracked()[seat_idx];
                    let steps = ((new_tile_index + 40 - start_tile) % 40) as u64;

                    set_timeout(
                        move || {
                            set_display_dice.set(dice);
                            set_is_rolling.set(false);

                            if steps == 0 {
                                set_landing_tile_burst.set(Some(new_tile_index));
                                set_selected_tile.set(Some(new_tile_index));
                                set_hopping_seat.set(None);
                                set_is_animating_move.set(false);
                                if let Some(snap) = pending_snapshot.get_untracked() {
                                    set_pending_snapshot.set(None);
                                    commit_snapshot(snap);
                                }
                                return;
                            }

                            set_hopping_seat.set(Some(seat_idx));
                            for step in 1..=steps {
                                let intermediate_tile = ((start_tile as u64 + step) % 40) as u8;
                                let is_last_step = step == steps;
                                set_timeout(
                                    move || {
                                        set_visual_positions.update(|positions| {
                                            positions[seat_idx] = intermediate_tile;
                                        });
                                        set_step_trail.update(|trail| {
                                            trail.push(intermediate_tile);
                                            if trail.len() > 3 {
                                                trail.remove(0);
                                            }
                                        });

                                        if intermediate_tile == 0 && new_tile_index != 10 {
                                            spawn_board_popup(
                                                0,
                                                "+2.000.000 đ · Qua Khởi hành".to_string(),
                                                true,
                                            );
                                        }

                                        if is_last_step {
                                            set_step_trail.set(Vec::new());
                                            set_landing_tile_burst.set(Some(new_tile_index));
                                            set_selected_tile.set(Some(new_tile_index));

                                            if let Some(card_info) = pending_card.get_untracked() {
                                                set_pending_card.set(None);
                                                set_drawn_card_banner.set(Some(card_info));
                                                spawn_board_popup(
                                                    new_tile_index,
                                                    "Rút thẻ sự kiện".to_string(),
                                                    true,
                                                );
                                            }

                                            let latest_snap = pending_snapshot
                                                .get_untracked()
                                                .unwrap_or_else(|| game_state.get_untracked());
                                            let (final_server_pos, is_jailed) = latest_snap
                                                .find_player(player_id)
                                                .map(|p| (p.position, p.in_jail))
                                                .unwrap_or((intermediate_tile, false));

                                            if final_server_pos != intermediate_tile {
                                                if is_jailed && final_server_pos == 10 {
                                                    play_monopoly_sound("jail");
                                                    spawn_board_popup(
                                                        intermediate_tile,
                                                        "Chuyển vào Khu tạm giam".to_string(),
                                                        false,
                                                    );
                                                    set_timeout(
                                                        move || {
                                                            set_visual_positions.update(|positions| {
                                                                positions[seat_idx] = 10;
                                                            });
                                                            set_hopping_seat.set(None);
                                                            set_is_animating_move.set(false);
                                                            set_landing_tile_burst.set(Some(10));
                                                            set_selected_tile.set(Some(10));
                                                            if let Some(snap) =
                                                                pending_snapshot.get_untracked()
                                                            {
                                                                set_pending_snapshot.set(None);
                                                                commit_snapshot(snap);
                                                            }
                                                        },
                                                        Duration::from_millis(450),
                                                    );
                                                } else {
                                                    let extra_steps = ((final_server_pos + 40
                                                        - intermediate_tile)
                                                        % 40)
                                                        as u64;
                                                    for extra in 1..=extra_steps {
                                                        let next_t = ((intermediate_tile as u64
                                                            + extra)
                                                            % 40)
                                                            as u8;
                                                        let is_final_extra = extra == extra_steps;
                                                        set_timeout(
                                                            move || {
                                                                set_visual_positions.update(
                                                                    |positions| {
                                                                        positions[seat_idx] =
                                                                            next_t;
                                                                    },
                                                                );
                                                                if next_t == 0 {
                                                                    spawn_board_popup(
                                                                        0,
                                                                        "+2.000.000 đ · Qua Khởi hành"
                                                                            .to_string(),
                                                                        true,
                                                                    );
                                                                }
                                                                if is_final_extra {
                                                                    set_hopping_seat.set(None);
                                                                    set_is_animating_move
                                                                        .set(false);
                                                                    set_landing_tile_burst
                                                                        .set(Some(final_server_pos));
                                                                    set_selected_tile
                                                                        .set(Some(final_server_pos));
                                                                    if let Some(snap) =
                                                                        pending_snapshot
                                                                            .get_untracked()
                                                                    {
                                                                        set_pending_snapshot
                                                                            .set(None);
                                                                        commit_snapshot(snap);
                                                                    }
                                                                }
                                                            },
                                                            Duration::from_millis(
                                                                380 + extra * 75,
                                                            ),
                                                        );
                                                    }
                                                }
                                            } else {
                                                if let Some(TileKind::Tax(t)) = get_board()
                                                    .get(intermediate_tile as usize)
                                                    .map(|tile| tile.kind.clone())
                                                {
                                                    spawn_board_popup(
                                                        intermediate_tile,
                                                        format!("-{} · Nộp thuế", t.amount),
                                                        false,
                                                    );
                                                }

                                                set_hopping_seat.set(None);
                                                set_is_animating_move.set(false);
                                                if let Some(snap) = pending_snapshot.get_untracked()
                                                {
                                                    set_pending_snapshot.set(None);
                                                    commit_snapshot(snap);
                                                }
                                            }
                                        }
                                    },
                                    Duration::from_millis(step * 110),
                                );
                            }
                        },
                        Duration::from_millis(520),
                    );

                    let name = game_state
                        .get_untracked()
                        .find_player(player_id)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| "Người chơi".to_string());
                    let doubles_str = if is_doubles {
                        " · Xúc xắc đôi"
                    } else {
                        ""
                    };
                    add_log(format!(
                        "{} tung ({}, {}){} · Tới ô #{}",
                        name, dice.0, dice.1, doubles_str, new_tile_index
                    ));
                }
                ServerMessage::PropertyAcquired {
                    player_id,
                    tile_index,
                    cost,
                } => {
                    play_monopoly_sound("buy");
                    set_recent_acquired_tile.set(Some(tile_index));
                    set_landing_tile_burst.set(Some(tile_index));
                    spawn_board_popup(
                        tile_index,
                        format!("Mua đất · -{}", cost),
                        false,
                    );
                    set_timeout(
                        move || {
                            if recent_acquired_tile.get_untracked() == Some(tile_index) {
                                set_recent_acquired_tile.set(None);
                            }
                        },
                        Duration::from_millis(900),
                    );

                    let name = game_state
                        .get_untracked()
                        .find_player(player_id)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| "Người chơi".to_string());
                    add_log(format!(
                        "{} mua ô #{} với giá {}",
                        name, tile_index, cost
                    ));
                }
                ServerMessage::RentPaid {
                    payer,
                    recipient,
                    tile_index,
                    amount,
                } => {
                    let my_id_now = my_player_id.get_untracked();
                    if my_id_now == Some(recipient) {
                        play_monopoly_sound("cash_in");
                    } else if my_id_now == Some(payer) {
                        play_monopoly_sound("cash_out");
                    }
                    spawn_board_popup(
                        tile_index,
                        format!("Tiền thuê · -{}", amount),
                        false,
                    );
                    let p_name = game_state
                        .get_untracked()
                        .find_player(payer)
                        .map(|p| p.name.clone())
                        .unwrap_or_default();
                    let r_name = game_state
                        .get_untracked()
                        .find_player(recipient)
                        .map(|p| p.name.clone())
                        .unwrap_or_default();
                    add_log(format!(
                        "{} trả {} tiền thuê ô #{} cho {}",
                        p_name, amount, tile_index, r_name
                    ));
                }
                ServerMessage::HouseModified {
                    tile_index,
                    house_count,
                } => {
                    play_monopoly_sound("buy");
                    set_recent_house_tile.set(Some(tile_index));
                    set_landing_tile_burst.set(Some(tile_index));
                    let label = if house_count == 5 {
                        "Nâng cấp Khách sạn".to_string()
                    } else {
                        format!("Cấp nhà · {} căn", house_count)
                    };
                    spawn_board_popup(tile_index, label.clone(), true);
                    set_timeout(
                        move || {
                            if recent_house_tile.get_untracked() == Some(tile_index) {
                                set_recent_house_tile.set(None);
                            }
                        },
                        Duration::from_millis(900),
                    );
                    add_log(format!("Ô #{} · {}", tile_index, label));
                }
                ServerMessage::PropertyMortgageChanged {
                    tile_index,
                    is_mortgaged,
                } => {
                    let msg = if is_mortgaged {
                        "Đã thế chấp ngân hàng"
                    } else {
                        "Đã chuộc lại tài sản"
                    };
                    spawn_board_popup(tile_index, msg.to_string(), is_mortgaged);
                    add_log(format!("{} tại ô #{}", msg, tile_index));
                }
                ServerMessage::CardDrawn {
                    player_id: _,
                    deck,
                    description,
                    effect_summary,
                } => {
                    let card_tuple = (
                        deck.clone(),
                        description.clone(),
                        effect_summary.clone(),
                    );
                    if is_rolling.get_untracked() || is_animating_move.get_untracked() {
                        set_pending_card.set(Some(card_tuple));
                    } else {
                        set_drawn_card_banner.set(Some(card_tuple));
                    }
                    add_log(format!(
                        "[{}] {}: {}",
                        deck, description, effect_summary
                    ));
                }
                ServerMessage::BotAssumedControl { player_id, reason } => {
                    let name = game_state
                        .get_untracked()
                        .find_player(player_id)
                        .map(|p| p.name.clone())
                        .unwrap_or_default();
                    add_log(format!(
                        "Hệ thống tự động đi thay lượt của {} ({})",
                        name, reason
                    ));
                }
                ServerMessage::SyncSnapshot { state } => {
                    if let Ok(parsed) = serde_json::from_value::<GameState>(state) {
                        if is_rolling.get_untracked() || is_animating_move.get_untracked() {
                            set_pending_snapshot.set(Some(parsed));
                        } else {
                            let mut pos_arr = visual_positions.get_untracked();
                            for (i, p) in parsed.players.iter().enumerate().take(4) {
                                pos_arr[i] = p.position;
                            }
                            set_visual_positions.set(pos_arr);
                            set_display_dice.set(parsed.last_dice);
                            commit_snapshot(parsed);
                        }
                    }
                }
                ServerMessage::ActionRejected { reason } => {
                    set_is_rolling.set(false);
                    set_is_animating_move.set(false);
                    set_hopping_seat.set(None);
                    set_error_toast.set(Some(reason.clone()));
                    add_log(format!("Thao tác bị từ chối: {}", reason));
                }
                ServerMessage::PlayerBankrupt { player_id } => {
                    let name = game_state
                        .get_untracked()
                        .find_player(player_id)
                        .map(|p| p.name.clone())
                        .unwrap_or_default();
                    add_log(format!("{} đã phá sản", name));
                }
                _ => {}
    });

    let start_local_game_custom = move |slots: Vec<LocalPlayerSlot>| {
        set_ws_client.set(None);
        set_is_local_mode.set(true);
        set_selected_tile.set(None);
        set_visual_positions.set([0u8; 4]);
        set_display_dice.set((1, 1));
        set_is_rolling.set(false);
        set_is_animating_move.set(false);
        set_hopping_seat.set(None);
        set_step_trail.set(Vec::new());
        set_landing_tile_burst.set(None);
        set_drawn_card_banner.set(None);
        set_pending_snapshot.set(None);
        set_pending_card.set(None);
        set_timer_seconds.set(120);

        let mut st = GameState::new();
        let count = slots.len().clamp(1, 4);
        st.config.enable_auction = if count > 1 {
            enable_auction_config.get_untracked()
        } else {
            false
        };
        st.is_game_started = true;
        st.turn_phase = TurnPhase::WaitingForRoll;

        let mut first_human_id = None;
        for (idx, slot) in slots.into_iter().take(4).enumerate() {
            let trimmed = slot.name.trim().to_string();
            let final_name = if trimmed.is_empty() {
                if slot.is_bot {
                    format!("Máy AI {}", idx + 1)
                } else {
                    format!("Người chơi {}", idx + 1)
                }
            } else {
                trimmed
            };
            let pid = PlayerId(uuid::Uuid::new_v4());
            let mut player = PlayerState::new(pid, final_name, idx as u8, None);
            player.is_bot_controlled = slot.is_bot && idx > 0;
            if !player.is_bot_controlled && first_human_id.is_none() {
                first_human_id = Some(pid);
            }
            st.players.push(player);
            st.turn_order.push(pid);
        }

        let first_name = st
            .players
            .first()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "Người chơi 1".to_string());
        let bot_count = st.players.iter().filter(|p| p.is_bot_controlled).count();
        let human_count = st.players.len().saturating_sub(bot_count);

        if st.players.len() == 1 {
            set_room_id.set("SOLO-1P".to_string());
            set_logs.set(vec![format!(
                "Chế độ chơi thử 1 người (Solo) đã sẵn sàng · Lượt của {}",
                first_name
            )]);
        } else if bot_count == 0 {
            set_room_id.set(format!("LOCAL-{}P", human_count));
            set_logs.set(vec![format!(
                "Chế độ {} người chơi trên cùng máy đã sẵn sàng · Lượt của {}",
                human_count, first_name
            )]);
        } else {
            set_room_id.set(format!("LOCAL-{}H{}AI", human_count, bot_count));
            set_logs.set(vec![format!(
                "Ván nội bộ ({} người + {} máy AI) đã sẵn sàng · Lượt của {}",
                human_count, bot_count, first_name
            )]);
        }

        set_my_player_id.set(first_human_id);
        set_local_authoritative_state.set(st.clone());
        set_game_state.set(st);
        set_view_mode.set(ViewMode::Game);
    };

    let start_local_game = move |with_bots: bool| {
        let human_name = {
            let trimmed = player_name.get_untracked().trim().to_string();
            if trimmed.is_empty() {
                "Người chơi 1".to_string()
            } else {
                trimmed
            }
        };
        let mut slots = vec![LocalPlayerSlot {
            name: human_name,
            is_bot: false,
        }];
        if with_bots {
            for bot_name in ["Máy Phú Nhuận", "Máy Chợ Lớn", "Máy Thảo Điền"] {
                slots.push(LocalPlayerSlot {
                    name: bot_name.to_string(),
                    is_bot: true,
                });
            }
        }
        start_local_game_custom(slots);
    };

    let connect_to_room = move |target_room: String, name: String, auction_enabled: bool| {
        set_is_local_mode.set(false);
        set_my_player_id.set(None);
        set_room_id.set(target_room.clone());

        let ws_protocol = if web_sys::window()
            .map(|w| w.location().protocol().unwrap_or_default())
            .unwrap_or_default()
            == "https:"
        {
            "wss:"
        } else {
            "ws:"
        };
        let host = web_sys::window()
            .map(|w| w.location().host().unwrap_or_default())
            .unwrap_or_else(|| "127.0.0.1:8080".to_string());
        let ws_url = format!("{}//{}/ws?room={}", ws_protocol, host, target_room);

        let client_res = WsClient::connect(
            ws_url,
            target_room,
            name,
            auction_enabled,
            move |server_msg| {
                process_server_message.call(server_msg);
            },
            move || {
                if my_player_id.get_untracked().is_none() && !is_local_mode.get_untracked() {
                    start_local_game(true);
                    add_log("Không thấy máy chủ WebSocket, tự động chuyển sang chế độ 1 người + 3 máy AI".to_string());
                } else {
                    add_log("Mất kết nối máy chủ, đang kết nối lại".to_string());
                }
            },
        );

        match client_res {
            Ok(client) => {
                set_ws_client.set(Some(Rc::new(client)));
                set_view_mode.set(ViewMode::Game);
            }
            Err(_) => {
                start_local_game(true);
                add_log("Không kết nối được máy chủ trực tuyến, chuyển sang chế độ ngoại tuyến 1 người + 3 máy AI".to_string());
            }
        }
    };

    let handle_action = Callback::new(move |action: ClientMessage| {
        if is_local_mode.get_untracked() {
            if let Some(my_id) = my_player_id.get_untracked() {
                execute_local_action(
                    my_id,
                    action,
                    local_authoritative_state,
                    set_local_authoritative_state,
                    set_my_player_id,
                    is_local_mode,
                    is_rolling,
                    is_animating_move,
                    process_server_message,
                );
            }
        } else if let Some(ref client) = ws_client.get() {
            client.send(action);
        }
    });

    let trigger_roll_dice = Callback::new(move |_: ()| {
        if is_rolling.get_untracked() || is_animating_move.get_untracked() {
            return;
        }
        let cur = game_state.get_untracked();
        let my_id = my_player_id.get_untracked();
        let is_my_turn = my_id.is_some() && cur.current_player().map(|p| p.id) == my_id;
        if cur.is_game_over || !is_my_turn {
            return;
        }

        if is_local_mode.get_untracked() {
            match cur.turn_phase {
                TurnPhase::WaitingForRoll => {
                    handle_action.call(ClientMessage::RollDice);
                }
                TurnPhase::TileActionPending => {
                    if cur.players.len() == 1 {
                        handle_action.call(ClientMessage::DeclineProperty);
                        handle_action.call(ClientMessage::RollDice);
                    } else if !cur.config.enable_auction {
                        handle_action.call(ClientMessage::DeclineProperty);
                    }
                }
                TurnPhase::TurnEnded => {
                    handle_action.call(ClientMessage::EndTurn);
                    if cur.players.len() == 1 {
                        handle_action.call(ClientMessage::RollDice);
                    }
                }
                TurnPhase::Auction => {}
            }
        } else {
            match cur.turn_phase {
                TurnPhase::WaitingForRoll => handle_action.call(ClientMessage::RollDice),
                TurnPhase::TurnEnded => handle_action.call(ClientMessage::EndTurn),
                _ => {}
            }
        }
    });

    let _keydown_handle = window_event_listener(ev::keydown, move |ev| {
        let key = ev.key();
        if key == "Escape" {
            set_selected_tile.set(None);
            set_show_history.set(false);
            set_show_rules.set(false);
            return;
        }
        if key == " " && view_mode.get_untracked() == ViewMode::Game {
            if let Some(target) = ev.target().and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()) {
                let tag = target.tag_name();
                if tag == "INPUT" || tag == "TEXTAREA" || tag == "BUTTON" {
                    return;
                }
            }
            let st = game_state.get_untracked();
            let my_id = my_player_id.get_untracked();
            let is_my_turn = my_id.is_some() && st.current_player().map(|p| p.id) == my_id;
            if is_my_turn
                && st.is_game_started
                && !st.is_game_over
                && !is_rolling.get_untracked()
                && !is_animating_move.get_untracked()
                && st.turn_phase == TurnPhase::WaitingForRoll
            {
                ev.prevent_default();
                trigger_roll_dice.call(());
            }
        }
    });

    view! {
        <div class="app-shell">
            <header class="game-header">
                <div class="header-left">
                    <span style=format!(
                        "width: 10px; height: 10px; border-radius: 2px; background: {}; display: inline-block; flex-shrink: 0;",
                        ACCENT_PRIMARY
                    )></span>
                    <span
                        class="header-title-text"
                        style=format!(
                            "font-size: 16px; line-height: 20px; font-weight: 600; color: {};",
                            TEXT_PRIMARY
                        )
                    >
                        "Cờ tỷ phú Sài Gòn"
                    </span>
                    <span
                        class="mono header-room-badge"
                        style=format!(
                            "font-size: 12px; line-height: 16px; background: {}; color: {}; border: 1px solid {}; padding: 4px 8px; border-radius: 4px; font-weight: 400;",
                            BG_SECONDARY, TEXT_MUTED, BORDER_DEFAULT
                        )
                    >
                        {move || format!("Phòng {}", room_id.get())}
                    </span>
                </div>

                <div class="header-right">
                    <button
                        class="btn-outline"
                        style=format!(
                            "font-size: 12px; line-height: 16px; height: 32px; background: {}; color: {}; border: 1px solid {}; padding: 0 10px; border-radius: 8px; font-weight: 500; cursor: pointer; display: inline-flex; align-items: center; gap: 6px;",
                            BG_SECONDARY, TEXT_PRIMARY, BORDER_DEFAULT
                        )
                        on:click=move |_| set_show_rules.update(|v| *v = !*v)
                    >
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
                            <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" />
                        </svg>
                        <span>"Luật chơi"</span>
                    </button>
                    {move || {
                        if view_mode.get() == ViewMode::Game {
                            let st = game_state.get();
                            let my_id = my_player_id.get();
                            let my_seat = my_id
                                .and_then(|id| st.players.iter().position(|p| p.id == id))
                                .unwrap_or(0);
                            let my_cash = my_id
                                .and_then(|id| st.find_player(id))
                                .map(|p| p.cash.to_string())
                                .unwrap_or_else(|| "15.000.000 đ".to_string());
                            let log_count = logs.get().len();
                            let history_open = show_history.get();
                            view! {
                                <div style=format!(
                                    "font-size: 12px; line-height: 16px; height: 32px; background: {}; color: {}; border: 1px solid {}; padding: 0 10px 0 6px; border-radius: 8px; font-weight: 600; display: inline-flex; align-items: center; gap: 6px;",
                                    BG_SECONDARY, TEXT_PRIMARY, BORDER_DEFAULT
                                )>
                                    <PlayerAvatar seat_idx=my_seat size=22 />
                                    <span class="mono">{my_cash}</span>
                                    {move || {
                                        if let Some((delta_txt, is_pos)) = cash_delta_badge.get() {
                                            let col = if is_pos { STATUS_SUCCESS } else { STATUS_DANGER };
                                            view! {
                                                <span class="mono ui-cash-delta" style=format!("font-size: 11px; color: {};", col)>
                                                    {delta_txt}
                                                </span>
                                            }.into_view()
                                        } else {
                                            view! {}.into_view()
                                        }
                                    }}
                                </div>
                                <button
                                    class="btn-outline"
                                    style=format!(
                                        "font-size: 12px; line-height: 16px; height: 32px; background: {}; color: {}; border: 1px solid {}; padding: 0 10px; border-radius: 8px; font-weight: 500; cursor: pointer; display: inline-flex; align-items: center; gap: 6px;",
                                        BG_SECONDARY,
                                        if sound_muted.get() { TEXT_MUTED } else { TEXT_PRIMARY },
                                        BORDER_DEFAULT
                                    )
                                    title="Bật/tắt âm thanh"
                                    on:click=move |_| {
                                        let next = !sound_muted.get_untracked();
                                        set_sound_muted.set(next);
                                        set_monopoly_sound_muted(next);
                                    }
                                >
                                    <span>{move || if sound_muted.get() { "Âm: Tắt" } else { "Âm: Bật" }}</span>
                                </button>
                                <button
                                    class="btn-outline"
                                    style=format!(
                                        "font-size: 12px; line-height: 16px; height: 32px; background: {}; color: {}; border: 1px solid {}; padding: 0 10px; border-radius: 8px; font-weight: 500; cursor: pointer; display: inline-flex; align-items: center; gap: 6px;",
                                        if history_open { SURFACE_HOVER } else { BG_SECONDARY },
                                        if history_open { TEXT_PRIMARY } else { TEXT_SECONDARY },
                                        BORDER_DEFAULT
                                    )
                                    on:click=move |_| set_show_history.update(|v| *v = !*v)
                                >
                                    <span>{if history_open { "Ẩn lịch sử" } else { "Lịch sử" }}</span>
                                    <span class="mono" style=format!("font-size: 11px; color: {};", TEXT_MUTED)>
                                        {format!("({})", log_count)}
                                    </span>
                                </button>
                                <button
                                    class="btn-outline"
                                    style=format!(
                                        "font-size: 12px; line-height: 16px; height: 32px; background: {}; color: {}; border: 1px solid {}; padding: 0 10px; border-radius: 8px; font-weight: 500; cursor: pointer; display: inline-flex; align-items: center; gap: 6px;",
                                        BG_SECONDARY, TEXT_SECONDARY, BORDER_DEFAULT
                                    )
                                    on:click=move |_| {
                                        set_show_history.set(false);
                                        set_is_local_mode.set(false);
                                        set_my_player_id.set(None);
                                        set_ws_client.set(None);
                                        set_view_mode.set(ViewMode::Lobby);
                                    }
                                >
                                    "Về sảnh"
                                </button>
                            }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}
                    {move || {
                        if let Some(err) = error_toast.get() {
                            view! {
                                <div style=format!(
                                    "background: {}; color: {}; border: 1px solid {}; padding: 4px 10px; border-radius: 8px; font-size: 12px; line-height: 18px; font-weight: 500; display: flex; align-items: center; gap: 6px;",
                                    BG_SECONDARY, STATUS_DANGER, BORDER_DEFAULT
                                )>
                                    <span>{err}</span>
                                    <button
                                        style=format!("background: transparent; border: none; color: {}; cursor: pointer; font-size: 12px; padding: 0 4px;", TEXT_MUTED)
                                        on:click=move |_| set_error_toast.set(None)
                                    >
                                        "Đóng"
                                    </button>
                                </div>
                            }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}
                </div>
            </header>

            {move || {
                match view_mode.get() {
                    ViewMode::Lobby => {
                        view! {
                            <div class="lobby-shell">
                                <div class="lobby-grid lobby-card-enter">
                                    <div style="display: flex; flex-direction: column; gap: 24px;">
                                        <div style="display: flex; flex-direction: column; gap: 12px;">
                                            <div style="display: inline-flex; align-items: center; gap: 8px;">
                                                <span style=format!("width: 6px; height: 6px; border-radius: 50%; background: {};", STATUS_WARNING)></span>
                                                <span class="mono" style=format!("font-size: 12px; line-height: 16px; color: {};", TEXT_MUTED)>
                                                    "Bản đồ địa chính · 40 ô phố Sài Gòn"
                                                </span>
                                            </div>
                                            <h1 style=format!("font-size: 32px; line-height: 36px; font-weight: 500; color: {}; letter-spacing: -0.02em;", TEXT_PRIMARY)>
                                                "Đầu tư và đấu giá qua 8 quận nội thành Sài Gòn"
                                            </h1>
                                            <p style=format!("color: {}; font-size: 14px; line-height: 20px;", TEXT_SECONDARY)>
                                                "Ván đấu trực tuyến 2 đến 4 người chơi theo chuẩn khế ước địa ốc, hỗ trợ đấu giá công khai khi bỏ qua quyền mua và tự động đi thay khi hết 2 phút"
                                            </p>
                                        </div>

                                        <div class="panel" style="padding: 24px; gap: 16px;">
                                            <div class="lobby-tabs">
                                                <button
                                                    class=move || if lobby_tab.get() == LobbyTab::Online { "lobby-tab active" } else { "lobby-tab" }
                                                    on:click=move |_| set_lobby_tab.set(LobbyTab::Online)
                                                >
                                                    "Trực tuyến (Online)"
                                                </button>
                                                <button
                                                    class=move || if lobby_tab.get() == LobbyTab::Local { "lobby-tab active" } else { "lobby-tab" }
                                                    on:click=move |_| set_lobby_tab.set(LobbyTab::Local)
                                                >
                                                    "Cùng máy / AI"
                                                </button>
                                            </div>

                                            {move || match lobby_tab.get() {
                                                LobbyTab::Online => view! {
                                                    <div style="display: flex; flex-direction: column; gap: 16px;">
                                                        <div style="display: flex; flex-direction: column; gap: 8px;">
                                                            <label style=format!("font-size: 13px; line-height: 20px; font-weight: 500; color: {};", TEXT_SECONDARY)>
                                                                "Tên người chơi"
                                                            </label>
                                                            <input
                                                                type="text"
                                                                prop:value=player_name
                                                                on:input=move |ev| set_player_name.set(event_target_value(&ev))
                                                            />
                                                        </div>

                                                        <div style="display: flex; flex-direction: column; gap: 8px;">
                                                            <label style=format!("font-size: 13px; line-height: 20px; font-weight: 500; color: {};", TEXT_SECONDARY)>
                                                                "Mã phòng"
                                                            </label>
                                                            <input
                                                                class="mono"
                                                                type="text"
                                                                prop:value=room_id
                                                                on:input=move |ev| set_room_id.set(event_target_value(&ev))
                                                            />
                                                        </div>

                                                        <div
                                                            style=format!(
                                                                "padding: 12px; border-radius: 8px; background: {}; border: 1px solid {}; display: flex; align-items: center; justify-content: space-between; gap: 12px; cursor: pointer;",
                                                                BG_SECONDARY, BORDER_DEFAULT
                                                            )
                                                            on:click=move |_| set_enable_auction_config.update(|v| *v = !*v)
                                                        >
                                                            <div style="display: flex; flex-direction: column; gap: 4px;">
                                                                <span style=format!("font-size: 14px; line-height: 20px; font-weight: 500; color: {};", TEXT_PRIMARY)>
                                                                    "Đấu giá khi bỏ lượt mua"
                                                                </span>
                                                                <span style=format!("font-size: 12px; line-height: 16px; color: {};", TEXT_MUTED)>
                                                                    {move || if enable_auction_config.get() {
                                                                        "Mở phiên trả giá công khai cho cả bàn khi không mua đất"
                                                                    } else {
                                                                        "Kết thúc lượt ngay khi người chơi từ chối mua đất"
                                                                    }}
                                                                </span>
                                                            </div>
                                                            <div style=format!(
                                                                "padding: 4px 8px; border-radius: 4px; background: {}; border: 1px solid {}; display: inline-flex; align-items: center; gap: 8px; flex-shrink: 0;",
                                                                SURFACE_HOVER, BORDER_DEFAULT
                                                            )>
                                                                <span style=move || format!(
                                                                    "width: 6px; height: 6px; border-radius: 50%; background: {};",
                                                                    if enable_auction_config.get() { STATUS_SUCCESS } else { TEXT_MUTED }
                                                                )></span>
                                                                <span class="mono" style=move || format!(
                                                                    "font-size: 12px; line-height: 16px; font-weight: 500; color: {};",
                                                                    if enable_auction_config.get() { TEXT_PRIMARY } else { TEXT_MUTED }
                                                                )>
                                                                    {move || if enable_auction_config.get() { "Đang bật" } else { "Đang tắt" }}
                                                                </span>
                                                            </div>
                                                        </div>

                                                        <div style="display: flex; gap: 12px; margin-top: 4px;">
                                                            <button
                                                                class="btn"
                                                                style="flex: 1;"
                                                                on:click=move |_| {
                                                                    connect_to_room(room_id.get(), player_name.get(), enable_auction_config.get());
                                                                }
                                                            >
                                                                "Vào phòng"
                                                            </button>
                                                            <button
                                                                class="btn btn-secondary"
                                                                style="flex: 1;"
                                                                on:click=move |_| {
                                                                    let p_name = player_name.get();
                                                                    let auction_opt = enable_auction_config.get();
                                                                    spawn_local(async move {
                                                                        let generated_room = format!("SGN-{}", uuid::Uuid::new_v4().to_string()[..4].to_uppercase());
                                                                        set_room_id.set(generated_room.clone());
                                                                        connect_to_room(generated_room, p_name, auction_opt);
                                                                    });
                                                                }
                                                            >
                                                                "Tạo phòng mới"
                                                            </button>
                                                        </div>
                                                    </div>
                                                }.into_view(),
                                                LobbyTab::Local => view! {
                                                    <div style="display: flex; flex-direction: column; gap: 14px;">
                                                        <div style="display: flex; flex-direction: column; gap: 8px;">
                                                            <label style=format!("font-size: 13px; line-height: 20px; font-weight: 500; color: {};", TEXT_SECONDARY)>
                                                                "Số người chơi trên bàn"
                                                            </label>
                                                            <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px;">
                                                                {[1usize, 2, 3, 4].into_iter().map(|cnt| {
                                                                    let label = if cnt == 1 { "1 (Solo)".to_string() } else { format!("{} Người", cnt) };
                                                                    view! {
                                                                        <button
                                                                            class=move || if local_player_count.get() == cnt { "btn" } else { "btn btn-outline" }
                                                                            style="min-height: 36px; font-size: 13px; padding: 0 8px;"
                                                                            on:click=move |_| set_local_player_count.set(cnt)
                                                                        >
                                                                            {label}
                                                                        </button>
                                                                    }
                                                                }).collect_view()}
                                                            </div>
                                                        </div>

                                                        <div style="display: flex; flex-direction: column; gap: 8px;">
                                                            {move || {
                                                                let cnt = local_player_count.get();
                                                                let slots = local_slots.get();
                                                                (0..cnt).map(|idx| {
                                                                    let slot = slots[idx].clone();
                                                                    view! {
                                                                        <div style="display: flex; align-items: center; gap: 8px;">
                                                                            <PlayerAvatar seat_idx=idx size=28 />
                                                                            <input
                                                                                type="text"
                                                                                style="flex: 1; min-width: 0;"
                                                                                prop:value=slot.name
                                                                                on:input=move |ev| {
                                                                                    let val = event_target_value(&ev);
                                                                                    set_local_slots.update(|list| {
                                                                                        if let Some(item) = list.get_mut(idx) {
                                                                                            item.name = val;
                                                                                        }
                                                                                    });
                                                                                }
                                                                            />
                                                                            {if idx > 0 {
                                                                                let is_bot = slot.is_bot;
                                                                                view! {
                                                                                    <button
                                                                                        class="btn btn-outline"
                                                                                        style=format!(
                                                                                            "min-height: 36px; width: 88px; font-size: 12px; padding: 0 8px; flex-shrink: 0; color: {};",
                                                                                            if is_bot { STATUS_WARNING } else { STATUS_SUCCESS }
                                                                                        )
                                                                                        on:click=move |_| {
                                                                                            set_local_slots.update(|list| {
                                                                                                if let Some(item) = list.get_mut(idx) {
                                                                                                    item.is_bot = !item.is_bot;
                                                                                                    if item.is_bot && item.name.starts_with("Người chơi") {
                                                                                                        let bot_defaults = ["", "Máy Phú Nhuận", "Máy Chợ Lớn", "Máy Thảo Điền"];
                                                                                                        item.name = bot_defaults[idx].to_string();
                                                                                                    } else if !item.is_bot && item.name.starts_with("Máy ") {
                                                                                                        item.name = format!("Người chơi {}", idx + 1);
                                                                                                    }
                                                                                                }
                                                                                            });
                                                                                        }
                                                                                    >
                                                                                        {if is_bot { "Máy AI" } else { "Người" }}
                                                                                    </button>
                                                                                }.into_view()
                                                                            } else {
                                                                                view! {
                                                                                    <span class="mono" style=format!("width: 88px; text-align: center; font-size: 12px; color: {}; flex-shrink: 0;", TEXT_MUTED)>
                                                                                        "Chủ bàn"
                                                                                    </span>
                                                                                }.into_view()
                                                                            }}
                                                                        </div>
                                                                    }
                                                                }).collect_view()
                                                            }}
                                                        </div>

                                                        {move || if local_player_count.get() > 1 {
                                                            view! {
                                                                <div
                                                                    style=format!(
                                                                        "padding: 10px 12px; border-radius: 8px; background: {}; border: 1px solid {}; display: flex; align-items: center; justify-content: space-between; gap: 12px; cursor: pointer;",
                                                                        BG_SECONDARY, BORDER_DEFAULT
                                                                    )
                                                                    on:click=move |_| set_enable_auction_config.update(|v| *v = !*v)
                                                                >
                                                                    <span style=format!("font-size: 13px; font-weight: 500; color: {};", TEXT_PRIMARY)>
                                                                        "Đấu giá khi bỏ lượt mua"
                                                                    </span>
                                                                    <span class="mono" style=move || format!(
                                                                        "font-size: 12px; font-weight: 500; color: {};",
                                                                        if enable_auction_config.get() { STATUS_SUCCESS } else { TEXT_MUTED }
                                                                    )>
                                                                        {move || if enable_auction_config.get() { "Đang bật" } else { "Đang tắt" }}
                                                                    </span>
                                                                </div>
                                                            }.into_view()
                                                        } else {
                                                            view! {}.into_view()
                                                        }}

                                                        <div style="display: flex; gap: 10px; margin-top: 2px;">
                                                            <button
                                                                class="btn"
                                                                style="flex: 1;"
                                                                on:click=move |_| {
                                                                    let cnt = local_player_count.get_untracked();
                                                                    let slots = local_slots.get_untracked().into_iter().take(cnt).collect();
                                                                    start_local_game_custom(slots);
                                                                }
                                                            >
                                                                "Bắt đầu ván đấu"
                                                            </button>
                                                            <button
                                                                class="btn btn-secondary"
                                                                on:click=move |_| start_local_game(true)
                                                            >
                                                                "1P + 3 AI"
                                                            </button>
                                                        </div>
                                                    </div>
                                                }.into_view(),
                                            }}
                                        </div>
                                    </div>

                                    // Right Column: Tactile Saigon Title Deed Preview Moment
                                    <div class="panel" style="padding: 20px; gap: 16px;">
                                        <div style=format!(
                                            "border-radius: 8px; background: {}; padding: 16px; display: flex; flex-direction: column; gap: 4px; border: 1px solid {};",
                                            DISTRICT_1_DARK_BLUE, BORDER_DEFAULT
                                        )>
                                            <div style="display: flex; align-items: center; justify-content: space-between;">
                                                <span class="mono" style=format!("font-size: 12px; line-height: 16px; color: {}; opacity: 0.85;", TEXT_PRIMARY)>
                                                    "Ô #39 · Quận 1 · Bến Nghé"
                                                </span>
                                                <span class="mono" style=format!("font-size: 12px; line-height: 16px; color: {}; opacity: 0.85;", TEXT_PRIMARY)>
                                                    "Khế ước mẫu"
                                                </span>
                                            </div>
                                            <span style=format!("font-size: 20px; line-height: 28px; font-weight: 600; color: {};", TEXT_PRIMARY)>
                                                "Đồng Khởi"
                                            </span>
                                        </div>

                                        <div style=format!(
                                            "display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; padding: 12px; border-radius: 8px; background: {}; border: 1px solid {};",
                                            BG_SECONDARY, BORDER_DEFAULT
                                        )>
                                            <div style="display: flex; flex-direction: column; gap: 4px;">
                                                <span style=format!("font-size: 12px; line-height: 16px; color: {};", TEXT_MUTED)>"Giá mua đất"</span>
                                                <span class="mono" style=format!("font-size: 14px; line-height: 20px; font-weight: 500; color: {};", TEXT_PRIMARY)>"4.000.000 đ"</span>
                                            </div>
                                            <div style="display: flex; flex-direction: column; gap: 4px;">
                                                <span style=format!("font-size: 12px; line-height: 16px; color: {};", TEXT_MUTED)>"Chi phí xây nhà"</span>
                                                <span class="mono" style=format!("font-size: 14px; line-height: 20px; font-weight: 500; color: {};", TEXT_PRIMARY)>"2.000.000 đ"</span>
                                            </div>
                                            <div style="display: flex; flex-direction: column; gap: 4px;">
                                                <span style=format!("font-size: 12px; line-height: 16px; color: {};", TEXT_MUTED)>"Giá trị thế chấp"</span>
                                                <span class="mono" style=format!("font-size: 14px; line-height: 20px; font-weight: 500; color: {};", TEXT_SECONDARY)>"2.000.000 đ"</span>
                                            </div>
                                        </div>

                                        <div style=format!(
                                            "display: flex; flex-direction: column; gap: 4px; padding: 12px; border-radius: 8px; background: {}; border: 1px solid {};",
                                            BG_SECONDARY, BORDER_DEFAULT
                                        )>
                                            <div style="display: flex; justify-content: space-between; height: 28px; align-items: center;">
                                                <span style=format!("font-size: 13px; color: {};", TEXT_SECONDARY)>"Thuê đất trống"</span>
                                                <span class="mono" style=format!("font-size: 13px; color: {}; font-weight: 500;", TEXT_PRIMARY)>"500.000 đ"</span>
                                            </div>
                                            <div style="display: flex; justify-content: space-between; height: 28px; align-items: center;">
                                                <span style=format!("font-size: 13px; color: {};", TEXT_SECONDARY)>"Sở hữu trọn bộ Quận 1"</span>
                                                <span class="mono" style=format!("font-size: 13px; color: {}; font-weight: 500;", TEXT_PRIMARY)>"1.000.000 đ"</span>
                                            </div>
                                            <div style="display: flex; justify-content: space-between; height: 28px; align-items: center;">
                                                <span style=format!("font-size: 13px; color: {};", TEXT_SECONDARY)>"1 căn nhà phố"</span>
                                                <span class="mono" style=format!("font-size: 13px; color: {};", TEXT_SECONDARY)>"2.000.000 đ"</span>
                                            </div>
                                            <div style="display: flex; justify-content: space-between; height: 28px; align-items: center;">
                                                <span style=format!("font-size: 13px; color: {};", TEXT_SECONDARY)>"2 căn nhà phố"</span>
                                                <span class="mono" style=format!("font-size: 13px; color: {};", TEXT_SECONDARY)>"6.000.000 đ"</span>
                                            </div>
                                            <div style="display: flex; justify-content: space-between; height: 28px; align-items: center;">
                                                <span style=format!("font-size: 13px; color: {};", TEXT_SECONDARY)>"3 căn nhà phố"</span>
                                                <span class="mono" style=format!("font-size: 13px; color: {};", TEXT_SECONDARY)>"14.000.000 đ"</span>
                                            </div>
                                            <div style="display: flex; justify-content: space-between; height: 28px; align-items: center;">
                                                <span style=format!("font-size: 13px; color: {};", TEXT_SECONDARY)>"4 căn nhà phố"</span>
                                                <span class="mono" style=format!("font-size: 13px; color: {};", TEXT_SECONDARY)>"17.000.000 đ"</span>
                                            </div>
                                            <div style=format!(
                                                "display: flex; justify-content: space-between; height: 32px; align-items: center; padding: 0 8px; margin: 4px -8px -4px -8px; border-radius: 4px; background: {};",
                                                SURFACE_HOVER
                                            )>
                                                <span style=format!("font-size: 13px; font-weight: 500; color: {};", STATUS_WARNING)>"Cấp khách sạn"</span>
                                                <span class="mono" style=format!("font-size: 13px; font-weight: 500; color: {};", STATUS_WARNING)>"20.000.000 đ"</span>
                                            </div>
                                        </div>

                                        <div style="display: flex; align-items: center; justify-content: space-between;">
                                            <span class="mono" style=format!("font-size: 12px; line-height: 16px; color: {};", TEXT_MUTED)>
                                                "Vốn khởi điểm: 15.000.000 đ"
                                            </span>
                                            <span class="mono" style=format!("font-size: 12px; line-height: 16px; color: {};", TEXT_MUTED)>
                                                "Qua Khởi hành: +2.000.000 đ"
                                            </span>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        }.into_view()
                    }
                    ViewMode::Game => {
                        view! {
                            <div class="game-layout">
                                {move || {
                                    let st = game_state.get();
                                    if !st.is_game_started {
                                        return view! {}.into_view();
                                    }
                                    let cur_id = st.current_player().map(|p| p.id);
                                    let my_id = my_player_id.get();
                                    view! {
                                        <div class="standings-strip">
                                            {st.players.iter().enumerate().map(|(idx, p)| {
                                                let is_active = cur_id == Some(p.id);
                                                let is_me = my_id == Some(p.id);
                                                let cls = if p.is_bankrupt {
                                                    "standing-pill bankrupt"
                                                } else if is_active {
                                                    "standing-pill active"
                                                } else {
                                                    "standing-pill"
                                                };
                                                let tag = if p.is_bot_controlled {
                                                    " · AI"
                                                } else if is_me {
                                                    " · Bạn"
                                                } else if !p.is_connected {
                                                    " · Mất kết nối"
                                                } else if p.in_jail {
                                                    " · Tạm giam"
                                                } else {
                                                    ""
                                                };

                                                let all_districts = [
                                                    District::District4,
                                                    District::District8,
                                                    District::PhuNhuan,
                                                    District::TanBinh,
                                                    District::District5,
                                                    District::District3,
                                                    District::District2,
                                                    District::District1,
                                                ];
                                                let board_tiles = get_board();
                                                let owned_districts: Vec<(District, bool)> = all_districts
                                                    .into_iter()
                                                    .filter_map(|d| {
                                                        let has_any = board_tiles.iter().any(|t| {
                                                            matches!(&t.kind, TileKind::Property(pr) if pr.district == d)
                                                                && st.properties[t.index as usize].owner == Some(p.id)
                                                        });
                                                        if has_any {
                                                            Some((d, st.owns_full_district(p.id, d)))
                                                        } else {
                                                            None
                                                        }
                                                    })
                                                    .collect();

                                                view! {
                                                    <div class=cls>
                                                        <PlayerAvatar seat_idx=idx size=24 />
                                                        <div style="display: flex; flex-direction: column; line-height: 1.15;">
                                                            <span style=format!("font-size: 12px; font-weight: 600; color: {};", TEXT_PRIMARY)>
                                                                {format!("{}{}", p.name, tag)}
                                                            </span>
                                                            <span class="mono" style=format!(
                                                                "font-size: 11px; color: {};",
                                                                if p.is_bankrupt { STATUS_DANGER } else if is_active { STATUS_WARNING } else { TEXT_SECONDARY }
                                                            )>
                                                                {if p.is_bankrupt { "Phá sản".to_string() } else { p.cash.to_string() }}
                                                            </span>
                                                            {if !owned_districts.is_empty() {
                                                                view! {
                                                                    <div class="standing-districts">
                                                                        {owned_districts.into_iter().map(|(d, is_mono)| {
                                                                            let d_cls = if is_mono { "standing-district-bar monopoly" } else { "standing-district-bar" };
                                                                            view! {
                                                                                <span
                                                                                    class=d_cls
                                                                                    style=format!("background: {};", get_district_color(d))
                                                                                ></span>
                                                                            }
                                                                        }).collect_view()}
                                                                    </div>
                                                                }.into_view()
                                                            } else {
                                                                view! {}.into_view()
                                                            }}
                                                        </div>
                                                    </div>
                                                }
                                            }).collect_view()}
                                        </div>
                                    }.into_view()
                                }}

                                <Board
                                    state=game_state
                                    selected_tile=selected_tile
                                    is_rolling=is_rolling
                                    is_animating_move=is_animating_move
                                    display_dice=display_dice
                                    visual_positions=visual_positions
                                    hopping_seat=hopping_seat
                                    step_trail=step_trail
                                    landing_tile_burst=landing_tile_burst
                                    recent_acquired_tile=recent_acquired_tile
                                    recent_house_tile=recent_house_tile
                                    floating_popups=floating_popups
                                    drawn_card_banner=drawn_card_banner
                                    my_player_id=my_player_id
                                    timer_seconds=timer_seconds
                                    is_local_mode=is_local_mode
                                    on_tile_click=Some(Callback::new(move |idx| {
                                        set_selected_tile.update(|cur| {
                                            *cur = if *cur == Some(idx) { None } else { Some(idx) };
                                        });
                                    }))
                                    on_roll_dice=Some(trigger_roll_dice)
                                    on_action=Some(handle_action)
                                />

                                {move || {
                                    let st = game_state.get();
                                    if is_local_mode.get() || st.is_game_started {
                                        return view! {}.into_view();
                                    }
                                    let my_id = my_player_id.get();
                                    let is_host = my_id
                                        .and_then(|id| st.find_player(id))
                                        .map(|p| p.seat_index == 0)
                                        .unwrap_or(false);
                                    let player_count = st.players.len();
                                    let r_id = room_id.get();

                                    view! {
                                        <div class="waiting-room-backdrop">
                                            <div class="waiting-room-card">
                                                <div style="display: flex; align-items: center; justify-content: space-between; gap: 12px;">
                                                    <div style="display: flex; flex-direction: column; gap: 4px;">
                                                        <span class="mono" style=format!("font-size: 12px; color: {};", STATUS_WARNING)>
                                                            {format!("Phòng chờ · {}", r_id)}
                                                        </span>
                                                        <h2 style=format!("font-size: 20px; font-weight: 600; color: {};", TEXT_PRIMARY)>
                                                            {format!("Đang chờ người chơi ({}/4)", player_count)}
                                                        </h2>
                                                    </div>
                                                    <span class="mono" style=format!(
                                                        "padding: 4px 10px; border-radius: 6px; background: {}; border: 1px solid {}; font-size: 12px; color: {};",
                                                        BG_SECONDARY, BORDER_DEFAULT, TEXT_SECONDARY
                                                    )>
                                                        {if st.config.enable_auction { "Đấu giá: Bật" } else { "Đấu giá: Tắt" }}
                                                    </span>
                                                </div>

                                                <div style="display: flex; flex-direction: column; gap: 8px;">
                                                    {(0..4usize).map(|seat_idx| {
                                                        if let Some(p) = st.players.get(seat_idx) {
                                                            let is_me = my_id == Some(p.id);
                                                            let badge = if seat_idx == 0 {
                                                                "Chủ phòng"
                                                            } else if p.is_bot_controlled {
                                                                "Máy AI"
                                                            } else if is_me {
                                                                "Bạn"
                                                            } else {
                                                                "Sẵn sàng"
                                                            };
                                                            view! {
                                                                <div style=format!(
                                                                    "height: 44px; padding: 0 12px; border-radius: 10px; background: {}; border: 1px solid {}; display: flex; align-items: center; justify-content: space-between;",
                                                                    BG_SECONDARY, BORDER_DEFAULT
                                                                )>
                                                                    <div style="display: flex; align-items: center; gap: 10px;">
                                                                        <PlayerAvatar seat_idx=seat_idx size=26 />
                                                                        <span style=format!("font-size: 14px; font-weight: 500; color: {};", TEXT_PRIMARY)>
                                                                            {p.name.clone()}
                                                                        </span>
                                                                    </div>
                                                                    <span class="mono" style=format!(
                                                                        "font-size: 12px; color: {};",
                                                                        if seat_idx == 0 { STATUS_WARNING } else if is_me { STATUS_SUCCESS } else { TEXT_MUTED }
                                                                    )>
                                                                        {badge}
                                                                    </span>
                                                                </div>
                                                            }.into_view()
                                                        } else {
                                                            view! {
                                                                <div style=format!(
                                                                    "height: 44px; padding: 0 12px; border-radius: 10px; background: transparent; border: 1px dashed {}; display: flex; align-items: center; justify-content: space-between;",
                                                                    BORDER_DEFAULT
                                                                )>
                                                                    <span style=format!("font-size: 13px; color: {};", TEXT_MUTED)>
                                                                        {format!("Vị trí T{} · Đang chờ người chơi...", seat_idx + 1)}
                                                                    </span>
                                                                </div>
                                                            }.into_view()
                                                        }
                                                    }).collect_view()}
                                                </div>

                                                {if is_host {
                                                    view! {
                                                        <div style="display: flex; gap: 10px;">
                                                            <button
                                                                class="btn btn-secondary"
                                                                style="flex: 1;"
                                                                disabled=move || player_count >= 4
                                                                on:click=move |_| handle_action.call(ClientMessage::AddBot)
                                                            >
                                                                "Thêm máy AI"
                                                            </button>
                                                            <button
                                                                class="btn"
                                                                style="flex: 1;"
                                                                disabled=move || player_count < 2
                                                                on:click=move |_| handle_action.call(ClientMessage::StartGame)
                                                            >
                                                                "Bắt đầu ván đấu"
                                                            </button>
                                                        </div>
                                                    }.into_view()
                                                } else {
                                                    view! {
                                                        <div style=format!("text-align: center; font-size: 13px; color: {}; padding: 4px 0;", TEXT_SECONDARY)>
                                                            "Đang chờ chủ phòng bắt đầu ván đấu (tự động bắt đầu khi đủ 4 người)..."
                                                        </div>
                                                    }.into_view()
                                                }}
                                            </div>
                                        </div>
                                    }.into_view()
                                }}

                                <div class="floating-dice-hud">
                                    {move || {
                                        let st = game_state.get();
                                        let my_id = my_player_id.get();
                                        let is_my_turn = my_id.is_some()
                                            && st.current_player().map(|p| p.id) == my_id;
                                        let me_opt = my_id.and_then(|id| st.find_player(id));
                                        let busy = is_rolling.get() || is_animating_move.get();
                                        let local_mode = is_local_mode.get();
                                        let (d1, d2) = display_dice.get();

                                        if st.is_game_over {
                                            let winner_name = st
                                                .winner
                                                .and_then(|wid| st.find_player(wid))
                                                .map(|p| p.name.clone())
                                                .unwrap_or_else(|| "Hoàn tất".to_string());
                                            return view! {
                                                <span style=format!("font-size: 13px; font-weight: 600; color: {}; padding: 0 8px;", STATUS_SUCCESS)>
                                                    {format!("Người thắng: {}", winner_name)}
                                                </span>
                                            }.into_view();
                                        }

                                        let mut context_buttons: Vec<View> = Vec::new();

                                        if !busy && st.turn_phase == TurnPhase::Auction {
                                            if let (Some(me), Some(auc)) = (me_opt, st.auction.as_ref()) {
                                                let next_bid = auc.current_bid + Money::from_vnd(50_000);
                                                let is_active = auc.active_bidders.contains(&me.id);
                                                let can_bid = is_active && me.cash >= next_bid;
                                                if is_active {
                                                    if local_mode {
                                                        let bidder_name = me.name.clone();
                                                        context_buttons.push(view! {
                                                            <span class="mono" style=format!("font-size: 12px; font-weight: 600; color: {}; padding: 0 4px;", STATUS_WARNING)>
                                                                {format!("Đấu giá · {}", bidder_name)}
                                                            </span>
                                                        }.into_view());
                                                    }
                                                    context_buttons.push(view! {
                                                        <button
                                                            class="btn-hud-pill btn-hud-buy"
                                                            disabled=move || !can_bid
                                                            on:click=move |_| handle_action.call(ClientMessage::PlaceBid { amount: next_bid })
                                                        >
                                                            {format!("Trả {}", next_bid)}
                                                        </button>
                                                        <button
                                                            class="btn-hud-pill btn-hud-secondary"
                                                            on:click=move |_| handle_action.call(ClientMessage::PassAuction)
                                                        >
                                                            "Bỏ đấu giá"
                                                        </button>
                                                    }.into_view());
                                                }
                                            }
                                        } else if !busy && is_my_turn {
                                            match st.turn_phase {
                                                TurnPhase::TileActionPending => {
                                                    if let Some(me) = me_opt {
                                                        let board = get_board();
                                                        let tile = &board[me.position as usize];
                                                        let cost = match &tile.kind {
                                                            TileKind::Property(p) => p.cost,
                                                            TileKind::Station(s) => s.cost,
                                                            TileKind::Utility(u) => u.cost,
                                                            _ => Money::ZERO,
                                                        };
                                                        let can_buy = cost > Money::ZERO && me.cash >= cost;
                                                        let buy_label = format!("Mua {} ({})", tile.name, cost);
                                                        context_buttons.push(view! {
                                                            <button
                                                                class="btn-hud-pill btn-hud-buy"
                                                                disabled=move || !can_buy
                                                                on:click=move |_| handle_action.call(ClientMessage::BuyProperty)
                                                            >
                                                                {buy_label}
                                                            </button>
                                                            <button
                                                                class="btn-hud-pill btn-hud-secondary"
                                                                on:click=move |_| handle_action.call(ClientMessage::DeclineProperty)
                                                            >
                                                                "Bỏ qua"
                                                            </button>
                                                        }.into_view());
                                                    }
                                                }
                                                TurnPhase::WaitingForRoll => {
                                                    if let Some(me) = me_opt {
                                                        if me.in_jail {
                                                            let can_bail = me.cash >= Money::from_vnd(500_000);
                                                            if me.get_out_of_jail_cards > 0 {
                                                                context_buttons.push(view! {
                                                                    <button
                                                                        class="btn-hud-pill btn-hud-buy"
                                                                        on:click=move |_| handle_action.call(ClientMessage::UseJailCard)
                                                                    >
                                                                        "Dùng thẻ ra tù"
                                                                    </button>
                                                                }.into_view());
                                                            }
                                                            context_buttons.push(view! {
                                                                <button
                                                                    class="btn-hud-pill btn-hud-secondary"
                                                                    disabled=move || !can_bail
                                                                    on:click=move |_| handle_action.call(ClientMessage::PayJailBail)
                                                                >
                                                                    "Bảo lãnh 500k"
                                                                </button>
                                                            }.into_view());
                                                        }
                                                    }
                                                }
                                                TurnPhase::Auction => {}
                                                TurnPhase::TurnEnded => {
                                                    if !local_mode {
                                                        context_buttons.push(view! {
                                                            <button
                                                                class="btn-hud-pill btn-hud-secondary"
                                                                on:click=move |_| handle_action.call(ClientMessage::EndTurn)
                                                            >
                                                                "Kết thúc lượt"
                                                            </button>
                                                        }.into_view());
                                                    }
                                                }
                                            }
                                        }

                                        if !local_mode && st.is_game_started && !st.is_game_over {
                                            let t_sec = timer_seconds.get();
                                            let mins = t_sec / 60;
                                            let secs = t_sec % 60;
                                            let is_urgent = t_sec <= 30;
                                            let t_col = if is_urgent { STATUS_DANGER } else { TEXT_SECONDARY };
                                            let label = if is_urgent && is_my_turn {
                                                format!("Còn {}s!", t_sec)
                                            } else {
                                                format!("{}:{:02}", mins, secs)
                                            };
                                            context_buttons.push(view! {
                                                <span
                                                    class="mono"
                                                    style=format!(
                                                        "font-size: 12px; font-weight: 600; color: {}; padding: {}; border-radius: 999px; background: {}; border: 1px solid {};",
                                                        t_col,
                                                        if is_urgent && is_my_turn { "4px 10px" } else { "0 4px" },
                                                        if is_urgent && is_my_turn { "rgba(229, 72, 77, 0.16)" } else { "transparent" },
                                                        if is_urgent && is_my_turn { STATUS_DANGER } else { "transparent" }
                                                    )
                                                >
                                                    {label}
                                                </span>
                                            }.into_view());
                                        }

                                        let roll_disabled = busy
                                            || !is_my_turn
                                            || st.turn_phase == TurnPhase::Auction
                                            || (st.turn_phase == TurnPhase::TileActionPending
                                                && (!local_mode
                                                    || (st.players.len() > 1
                                                        && st.config.enable_auction)))
                                            || (st.turn_phase == TurnPhase::TurnEnded && !local_mode);

                                        let roll_label = if is_rolling.get() {
                                            "Đang lắc...".to_string()
                                        } else if is_animating_move.get() {
                                            format!("Đi {} bước...", d1 + d2)
                                        } else if !is_my_turn {
                                            let cur_name = st
                                                .current_player()
                                                .map(|p| p.name.clone())
                                                .unwrap_or_else(|| "...".to_string());
                                            format!("Lượt {}", cur_name)
                                        } else {
                                            "Lắc xúc xắc".to_string()
                                        };

                                        view! {
                                            {context_buttons.into_iter().collect_view()}
                                            <button
                                                class="btn-roll-dice"
                                                disabled=roll_disabled
                                                on:click=move |_| trigger_roll_dice.call(())
                                            >
                                                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                                    <rect x="3" y="3" width="18" height="18" rx="4" />
                                                    <circle cx="8.5" cy="8.5" r="1.2" fill="currentColor" />
                                                    <circle cx="15.5" cy="8.5" r="1.2" fill="currentColor" />
                                                    <circle cx="12" cy="12" r="1.2" fill="currentColor" />
                                                    <circle cx="8.5" cy="15.5" r="1.2" fill="currentColor" />
                                                    <circle cx="15.5" cy="15.5" r="1.2" fill="currentColor" />
                                                </svg>
                                                <span>{roll_label}</span>
                                                <span class="mono" style="font-size: 13px; opacity: 0.9; font-weight: 500;">
                                                    {format!("({}+{})", d1, d2)}
                                                </span>
                                            </button>
                                        }.into_view()
                                    }}
                                </div>

                                {move || {
                                    if show_history.get() {
                                        view! {
                                            <div
                                                class="history-popover-backdrop"
                                                on:click=move |_| set_show_history.set(false)
                                            ></div>
                                            <div class="history-popover">
                                                <div style="display: flex; align-items: center; justify-content: space-between; padding-bottom: 4px;">
                                                    <span style=format!("font-size: 13px; font-weight: 600; color: {};", TEXT_PRIMARY)>
                                                        "Lịch sử ván đấu"
                                                    </span>
                                                    <button
                                                        class="btn-outline"
                                                        style=format!(
                                                            "height: 28px; padding: 0 10px; border-radius: 6px; font-size: 12px; color: {}; border: 1px solid {}; cursor: pointer;",
                                                            TEXT_SECONDARY, BORDER_DEFAULT
                                                        )
                                                        on:click=move |_| set_show_history.set(false)
                                                    >
                                                        "Đóng"
                                                    </button>
                                                </div>
                                                <LogFeed logs=logs />
                                            </div>
                                        }.into_view()
                                    } else {
                                        view! {}.into_view()
                                    }
                                }}
                            </div>
                        }.into_view()
                    }
                }
            }}

            {move || {
                if show_rules.get() {
                    view! {
                        <div
                            class="rules-modal-backdrop"
                            on:click=move |_| set_show_rules.set(false)
                        >
                            <div
                                class="rules-modal"
                                on:click=move |ev| ev.stop_propagation()
                            >
                                <div class="rules-modal-header">
                                    <div style="display: flex; align-items: center; gap: 10px;">
                                        <span style="width: 10px; height: 10px; border-radius: 3px; background: #d4a853; display: inline-block;"></span>
                                        <span style="font-size: 16px; font-weight: 600; color: #fbf7ee;">
                                            "Luật chơi nhanh · Cờ tỷ phú Sài Gòn"
                                        </span>
                                    </div>
                                    <button
                                        class="btn-outline"
                                        style=format!(
                                            "height: 30px; padding: 0 12px; border-radius: 8px; font-size: 12px; color: {}; border: 1px solid rgba(239, 233, 222, 0.2); cursor: pointer;",
                                            TEXT_PRIMARY
                                        )
                                        on:click=move |_| set_show_rules.set(false)
                                    >
                                        "Đóng"
                                    </button>
                                </div>

                                <ol class="rules-list">
                                    <li class="rule-item">
                                        <div class="rule-illust">
                                            <svg width="64" height="46" viewBox="0 0 64 46">
                                                <rect x="6" y="10" width="20" height="20" rx="4" fill="#fbf7ee" stroke="#18283a" stroke-width="1.5" />
                                                <circle cx="16" cy="20" r="3" fill="#d9383a" />
                                                <rect x="22" y="18" width="20" height="20" rx="4" fill="#fbf7ee" stroke="#18283a" stroke-width="1.5" />
                                                <circle cx="28" cy="24" r="2" fill="#18283a" />
                                                <circle cx="36" cy="32" r="2" fill="#18283a" />
                                                <path d="M 46 22 L 58 22 M 54 17 L 59 22 L 54 27" fill="none" stroke="#d9383a" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" />
                                                <text x="51" y="12" font-size="9" font-weight="700" font-family="'Roboto Mono', monospace" fill="#2b7a53" text-anchor="middle">"+2M"</text>
                                            </svg>
                                        </div>
                                        <div class="rule-body">
                                            <div class="rule-title-row">
                                                <span class="rule-step-badge">"1"</span>
                                                <span class="rule-title">"Lắc xúc xắc & Đi qua ô Khởi hành"</span>
                                            </div>
                                            <p class="rule-desc">
                                                "Mỗi người bắt đầu với 15.000.000 đ. Nhấn «Lắc xúc xắc» để di chuyển; đi qua hoặc dừng tại ô Khởi hành nhận +2.000.000 đ. Tung được đôi (2 mặt giống nhau) sẽ được thêm 1 lượt."
                                            </p>
                                        </div>
                                    </li>

                                    <li class="rule-item">
                                        <div class="rule-illust">
                                            <svg width="64" height="46" viewBox="0 0 64 46">
                                                <rect x="8" y="6" width="28" height="34" rx="4" fill="#fbf7ee" stroke="#18283a" stroke-width="1.5" />
                                                <rect x="8" y="6" width="28" height="9" rx="3" fill="#3b66a8" />
                                                <line x1="13" y1="21" x2="31" y2="21" stroke="#4a5d73" stroke-width="2" />
                                                <line x1="13" y1="27" x2="27" y2="27" stroke="#4a5d73" stroke-width="2" />
                                                <circle cx="48" cy="28" r="10" fill="#d4a853" stroke="#18283a" stroke-width="1.5" />
                                                <text x="48" y="31.5" font-size="11" font-weight="700" fill="#18283a" text-anchor="middle">"đ"</text>
                                            </svg>
                                        </div>
                                        <div class="rule-body">
                                            <div class="rule-title-row">
                                                <span class="rule-step-badge">"2"</span>
                                                <span class="rule-title">"Mua đất, Bến xe & Thu tiền thuê"</span>
                                            </div>
                                            <p class="rule-desc">
                                                "Dừng vào ô đất trống chưa có chủ, bạn có thể bấm «Mua» để sở hữu (hoặc «Bỏ qua» để đấu giá nếu đang bật). Người khác đi vào ô của bạn phải trả tiền thuê theo biểu phí trên khế ước."
                                            </p>
                                        </div>
                                    </li>

                                    <li class="rule-item">
                                        <div class="rule-illust">
                                            <svg width="64" height="46" viewBox="0 0 64 46">
                                                <polygon points="8,24 16,16 24,24" fill="#2e8b57" stroke="#18283a" stroke-width="1.2" />
                                                <rect x="10" y="24" width="12" height="10" fill="#4cc38a" stroke="#18283a" stroke-width="1.2" />
                                                <polygon points="20,24 28,16 36,24" fill="#2e8b57" stroke="#18283a" stroke-width="1.2" />
                                                <rect x="22" y="24" width="12" height="10" fill="#4cc38a" stroke="#18283a" stroke-width="1.2" />
                                                <path d="M 38 24 L 44 24" stroke="#18283a" stroke-width="2" stroke-linecap="round" />
                                                <polygon points="45,20 53,11 61,20" fill="#b82d2f" stroke="#18283a" stroke-width="1.2" />
                                                <rect x="47" y="20" width="12" height="15" fill="#d9383a" stroke="#18283a" stroke-width="1.2" />
                                            </svg>
                                        </div>
                                        <div class="rule-body">
                                            <div class="rule-title-row">
                                                <span class="rule-step-badge">"3"</span>
                                                <span class="rule-title">"Độc quyền Quận · Xây Nhà & Khách sạn"</span>
                                            </div>
                                            <p class="rule-desc">
                                                "Sở hữu đủ các ô cùng màu một quận sẽ tự động nhân đôi tiền thuê đất trống và cho phép nâng cấp tối đa 4 căn nhà phố rồi lên Khách sạn để thu lợi nhuận cực cao."
                                            </p>
                                        </div>
                                    </li>

                                    <li class="rule-item">
                                        <div class="rule-illust">
                                            <svg width="64" height="46" viewBox="0 0 64 46">
                                                <rect x="6" y="10" width="22" height="26" rx="4" fill="#e67e35" stroke="#18283a" stroke-width="1.5" />
                                                <text x="17" y="29" font-size="18" font-weight="700" fill="#fbf7ee" text-anchor="middle">"?"</text>
                                                <rect x="34" y="10" width="24" height="26" rx="4" fill="#f5f0e4" stroke="#18283a" stroke-width="1.5" />
                                                <line x1="40" y1="10" x2="40" y2="36" stroke="#18283a" stroke-width="2" />
                                                <line x1="46" y1="10" x2="46" y2="36" stroke="#18283a" stroke-width="2" />
                                                <line x1="52" y1="10" x2="52" y2="36" stroke="#18283a" stroke-width="2" />
                                            </svg>
                                        </div>
                                        <div class="rule-body">
                                            <div class="rule-title-row">
                                                <span class="rule-step-badge">"4"</span>
                                                <span class="rule-title">"Thẻ Cơ hội, Khí vận & Khu Tạm giam"</span>
                                            </div>
                                            <p class="rule-desc">
                                                "Dừng ở ô Cơ hội / Khí vận để rút thẻ thưởng hoặc phạt bất ngờ. Nếu vào Khu tạm giam (ô #10), bạn có thể tung được đôi trong 3 lượt hoặc nộp bảo lãnh 500.000 đ để ra ngay."
                                            </p>
                                        </div>
                                    </li>

                                    <li class="rule-item">
                                        <div class="rule-illust">
                                            <svg width="64" height="46" viewBox="0 0 64 46">
                                                <path d="M 22 10 L 42 10 L 39 24 C 38 28 35 30 32 30 C 29 30 26 28 25 24 Z" fill="#d4a853" stroke="#18283a" stroke-width="1.5" />
                                                <rect x="29" y="30" width="6" height="6" fill="#d4a853" stroke="#18283a" stroke-width="1.2" />
                                                <rect x="23" y="36" width="18" height="4" rx="2" fill="#18283a" />
                                                <circle cx="15" cy="18" r="4" fill="#4cc38a" stroke="#18283a" stroke-width="1.2" />
                                                <circle cx="49" cy="18" r="4" fill="#e5484d" stroke="#18283a" stroke-width="1.2" />
                                            </svg>
                                        </div>
                                        <div class="rule-body">
                                            <div class="rule-title-row">
                                                <span class="rule-step-badge">"5"</span>
                                                <span class="rule-title">"Phá sản & Chiến thắng"</span>
                                            </div>
                                            <p class="rule-desc">
                                                "Nếu không đủ tiền trả nợ thuê hoặc thuế, người chơi sẽ bị phá sản và bàn giao tài sản lại cho chủ nợ. Người trụ lại cuối cùng trên bàn cờ Sài Gòn là người chiến thắng!"
                                            </p>
                                        </div>
                                    </li>
                                </ol>
                            </div>
                        </div>
                    }.into_view()
                } else {
                    view! {}.into_view()
                }
            }}
        </div>
    }
}

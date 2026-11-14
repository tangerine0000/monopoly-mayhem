use crate::components::board::{tile_rect_pct, Board, BoardFloatingPopup, MiniMap};
use crate::components::log::LogFeed;
use crate::components::players::PlayersRail;
use crate::components::portfolio::Portfolio;
use crate::components::stage::Stage;
use crate::palette::{
    fmt_short, get_district_color, get_district_label, player_color, player_vehicle_svg,
    ICON_CLOSE, ICON_DICE, ICON_INFO, ICON_STAR,
};
use crate::ws::{save_storage_session_token, WsClient};
use leptos::prelude::*;
use monopoly_core::board::{get_board, Tile, TileKind};
use monopoly_core::bot::decide_action;
use monopoly_core::rules::{apply_action, close_auction};
use monopoly_core::state::{GameState, PlayerState, TurnPhase};
use monopoly_proto::{ClientMessage, Money, PlayerId, ServerMessage};
use std::rc::Rc;
use std::time::Duration;
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = playMonopolySound)]
    fn js_play_sound(kind: &str);

    #[wasm_bindgen(js_namespace = window, js_name = setMonopolySoundMuted)]
    fn js_set_sound_muted(muted: bool);

    #[wasm_bindgen(js_namespace = window, js_name = copyMonopolyText)]
    fn js_copy_text(text: &str);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LobbyTab {
    Online,
    Local,
}

#[derive(Clone, Debug)]
struct LocalSlotConfig {
    name: String,
    is_bot: bool,
}

fn default_local_slots() -> Vec<LocalSlotConfig> {
    vec![
        LocalSlotConfig {
            name: "Bạn".to_string(),
            is_bot: false,
        },
        LocalSlotConfig {
            name: "Máy Tám".to_string(),
            is_bot: true,
        },
        LocalSlotConfig {
            name: "Máy Tư".to_string(),
            is_bot: true,
        },
        LocalSlotConfig {
            name: "Máy Út".to_string(),
            is_bot: true,
        },
    ]
}

fn fmt_vnd(amount: Money) -> String {
    format!("{}", amount)
}

fn check_monopoly_completed(state: &GameState, tile_index: u8, owner: PlayerId) -> bool {
    let board = get_board();
    let Some(Tile {
        kind: TileKind::Property(prop),
        ..
    }) = board.get(tile_index as usize)
    else {
        return false;
    };
    state.owns_full_district(owner, prop.district)
}

fn parse_room_query_param() -> Option<String> {
    let window = web_sys::window()?;
    let search = window.location().search().ok()?;
    let trimmed = search.trim_start_matches('?');
    for pair in trimmed.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == "room" && !v.trim().is_empty() {
                return Some(v.trim().to_uppercase());
            }
        }
    }
    None
}

fn build_invite_url(room_id: &str) -> String {
    if let Some(window) = web_sys::window() {
        if let (Ok(origin), Ok(pathname)) =
            (window.location().origin(), window.location().pathname())
        {
            return format!("{}{}?room={}", origin, pathname, room_id);
        }
    }
    format!("?room={}", room_id)
}

#[component]
pub fn App() -> impl IntoView {
    let initial_room = parse_room_query_param().unwrap_or_else(|| "SGN-01".to_string());

    // Lobby & mode state
    let (in_game, set_in_game) = create_signal(false);
    let (is_local_mode, set_is_local_mode) = create_signal(false);
    let (lobby_tab, set_lobby_tab) = create_signal(LobbyTab::Online);
    let (room_id, set_room_id) = create_signal(initial_room);
    let (player_name, set_player_name) = create_signal("Khách Sài Gòn".to_string());
    let (enable_auction_cfg, set_enable_auction_cfg) = create_signal(true);
    let (local_player_count, set_local_player_count) = create_signal(4usize);
    let (local_slots, set_local_slots) = create_signal(default_local_slots());

    // Connection & core state
    let (ws_connected, set_ws_connected) = create_signal(false);
    let (my_player_id, set_my_player_id) = create_signal::<Option<PlayerId>>(None);
    let (game_state, set_game_state) = create_signal(GameState::new());
    let (logs, set_logs) = create_signal::<Vec<String>>(Vec::new());
    let (error_msg, set_error_msg) = create_signal::<Option<String>>(None);
    let (selected_tile, set_selected_tile) = create_signal::<Option<u8>>(None);
    let (trade_open, set_trade_open) = create_signal(false);
    let (show_rules, set_show_rules) = create_signal(false);
    let (is_muted, set_is_muted) = create_signal(false);
    let (mobile_side_open, set_mobile_side_open) = create_signal(false);
    let (copied_invite, set_copied_invite) = create_signal(false);

    // Timers & animations
    let (timer_seconds, set_timer_seconds) = create_signal(120u32);
    let (auction_seconds, set_auction_seconds) = create_signal(8u32);
    let (is_rolling, set_is_rolling) = create_signal(false);
    let (is_animating_move, set_is_animating_move) = create_signal(false);
    let (display_dice, set_display_dice) = create_signal((1u8, 1u8));
    let (visual_positions, set_visual_positions) = create_signal([0u8; 4]);
    let (hopping_seat, set_hopping_seat) = create_signal::<Option<usize>>(None);
    let (step_trail, set_step_trail) = create_signal::<Vec<u8>>(Vec::new());
    let (landing_tile_burst, set_landing_tile_burst) = create_signal::<Option<u8>>(None);
    let (recent_acquired_tile, set_recent_acquired_tile) = create_signal::<Option<u8>>(None);
    let (recent_house_tile, set_recent_house_tile) = create_signal::<Option<u8>>(None);
    let (floating_popups, set_floating_popups) =
        create_signal::<Vec<BoardFloatingPopup>>(Vec::new());
    let (cash_delta_badge, set_cash_delta_badge) = create_signal::<Option<(String, bool)>>(None);
    let (drawn_card_banner, set_drawn_card_banner) =
        create_signal::<Option<(String, String, String)>>(None);
    let (manual_cam_tile, set_manual_cam_tile) = create_signal::<Option<u8>>(None);

    // Stored references for timers, narrative pacing, WS client, and local bots
    let ws_client_ref: StoredValue<Option<Rc<WsClient>>, LocalStorage> =
        StoredValue::new_local(None);
    let local_bot_map: StoredValue<Vec<(PlayerId, bool)>> = store_value(Vec::new());
    let local_bot_gen: StoredValue<u64> = store_value(0);
    let local_auction_gen: StoredValue<u64> = store_value(0);
    let turn_timer_gen: StoredValue<u64> = store_value(0);
    let roll_gen: StoredValue<u64> = store_value(0);
    let move_gen: StoredValue<u64> = store_value(0);
    let card_seq: StoredValue<u64> = store_value(0);
    let popup_seq: StoredValue<u64> = store_value(0);
    let delta_seq: StoredValue<u64> = store_value(0);
    let error_seq: StoredValue<u64> = store_value(0);
    let pending_logs: StoredValue<Vec<String>> = store_value(Vec::new());
    let pending_sounds: StoredValue<Vec<&'static str>> = store_value(Vec::new());
    let pending_popups: StoredValue<Vec<(u8, String, bool)>> = store_value(Vec::new());
    let pending_banner: StoredValue<Option<(String, String, String)>> = store_value(None);
    let pending_cash_delta: StoredValue<Option<(String, bool)>> = store_value(None);

    let play_sfx = move |kind: &'static str| {
        if !is_muted.get_untracked() {
            js_play_sound(kind);
        }
    };

    let push_log = move |line: String| {
        set_logs.update(|list| {
            list.insert(0, line);
            if list.len() > 60 {
                list.truncate(60);
            }
        });
    };

    let spawn_popup = move |tile_index: u8, text: String, is_positive: bool| {
        let id = popup_seq.get_value() + 1;
        popup_seq.set_value(id);
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
                set_floating_popups.update(|list| list.retain(|p| p.id != id));
            },
            Duration::from_millis(1500),
        );
    };

    let show_cash_delta = move |text: String, is_positive: bool| {
        let seq = delta_seq.get_value() + 1;
        delta_seq.set_value(seq);
        set_cash_delta_badge.set(Some((text, is_positive)));
        set_timeout(
            move || {
                if delta_seq.get_value() == seq {
                    set_cash_delta_badge.set(None);
                }
            },
            Duration::from_millis(1600),
        );
    };

    let show_banner = move |kind: String, title: String, desc: String| {
        let seq = card_seq.get_value() + 1;
        card_seq.set_value(seq);
        set_drawn_card_banner.set(Some((kind, title, desc)));
        set_timeout(
            move || {
                if card_seq.get_value() == seq {
                    set_drawn_card_banner.set(None);
                }
            },
            Duration::from_millis(3500),
        );
    };

    let show_error = move |msg: String| {
        let seq = error_seq.get_value() + 1;
        error_seq.set_value(seq);
        set_error_msg.set(Some(msg));
        set_timeout(
            move || {
                if error_seq.get_value() == seq {
                    set_error_msg.set(None);
                }
            },
            Duration::from_millis(4500),
        );
    };

    let flush_narrative = move || {
        let mut logs_to_flush = Vec::new();
        pending_logs.update_value(|v| logs_to_flush = std::mem::take(v));
        for line in logs_to_flush {
            push_log(line);
        }
        let mut sounds_to_flush = Vec::new();
        pending_sounds.update_value(|v| sounds_to_flush = std::mem::take(v));
        for s in sounds_to_flush {
            play_sfx(s);
        }
        let mut popups_to_flush = Vec::new();
        pending_popups.update_value(|v| popups_to_flush = std::mem::take(v));
        for (tile_idx, txt, pos) in popups_to_flush {
            spawn_popup(tile_idx, txt, pos);
        }
        let mut delta_to_flush = None;
        pending_cash_delta.update_value(|v| delta_to_flush = v.take());
        if let Some((txt, pos)) = delta_to_flush {
            show_cash_delta(txt, pos);
        }
        let mut banner_to_flush = None;
        pending_banner.update_value(|v| banner_to_flush = v.take());
        if let Some((k, t, d)) = banner_to_flush {
            show_banner(k, t, d);
        }
    };

    let is_animating_narrative =
        move || is_rolling.get_untracked() || is_animating_move.get_untracked();

    let enqueue_log = move |line: String| {
        if is_animating_narrative() {
            pending_logs.update_value(|v| v.push(line));
        } else {
            push_log(line);
        }
    };

    let enqueue_sound = move |kind: &'static str| {
        if is_animating_narrative() {
            pending_sounds.update_value(|v| {
                if !v.contains(&kind) {
                    v.push(kind);
                }
            });
        } else {
            play_sfx(kind);
        }
    };

    let enqueue_popup = move |tile_idx: u8, text: String, is_positive: bool| {
        if is_animating_narrative() {
            pending_popups.update_value(|v| v.push((tile_idx, text, is_positive)));
        } else {
            spawn_popup(tile_idx, text, is_positive);
        }
    };

    let enqueue_cash_delta = move |text: String, is_positive: bool| {
        if is_animating_narrative() {
            pending_cash_delta.set_value(Some((text, is_positive)));
        } else {
            show_cash_delta(text, is_positive);
        }
    };

    let enqueue_banner = move |kind: String, title: String, desc: String| {
        if is_animating_narrative() {
            pending_banner.set_value(Some((kind, title, desc)));
        } else {
            show_banner(kind, title, desc);
        }
    };

    let restart_local_turn_clock = move |is_started: bool, is_over: bool| {
        let gen = turn_timer_gen.get_value() + 1;
        turn_timer_gen.set_value(gen);
        set_timer_seconds.set(120);
        if !is_started || is_over {
            return;
        }
        fn tick_turn(
            gen: u64,
            remaining: u32,
            turn_timer_gen: StoredValue<u64>,
            set_timer: WriteSignal<u32>,
            is_muted: ReadSignal<bool>,
        ) {
            set_timeout(
                move || {
                    if turn_timer_gen.get_value() != gen || remaining == 0 {
                        return;
                    }
                    let next = remaining - 1;
                    set_timer.set(next);
                    if next > 0 && next <= 10 && !is_muted.get_untracked() {
                        js_play_sound("urgent");
                    }
                    if next > 0 {
                        tick_turn(gen, next, turn_timer_gen, set_timer, is_muted);
                    }
                },
                Duration::from_secs(1),
            );
        }
        tick_turn(gen, 120, turn_timer_gen, set_timer_seconds, is_muted);
    };

    let player_name_of = move |pid: PlayerId| -> String {
        game_state.with_untracked(|s| {
            s.players
                .iter()
                .find(|p| p.id == pid)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "Người chơi".to_string())
        })
    };

    let start_hop_animation = move |seat: usize, from_pos: u8, to_pos: u8, wait_for_roll: bool| {
        let steps = ((to_pos + 40 - from_pos) % 40) as usize;
        if steps == 0 {
            return;
        }
        if steps > 12 {
            // Teleport (ví dụ vào trại tạm giam)
            set_visual_positions.update(|vp| {
                if seat < 4 {
                    vp[seat] = to_pos;
                }
            });
            set_landing_tile_burst.set(Some(to_pos));
            set_timeout(
                move || set_landing_tile_burst.set(None),
                Duration::from_millis(500),
            );
            return;
        }

        let gen = move_gen.get_value() + 1;
        move_gen.set_value(gen);
        set_is_animating_move.set(true);
        let initial_delay = if wait_for_roll { 380 } else { 40 };

        for step in 1..=steps {
            let tile = (from_pos + step as u8) % 40;
            let delay_ms = initial_delay + ((step - 1) as u64) * 135;
            set_timeout(
                move || {
                    if move_gen.get_value() != gen {
                        return;
                    }
                    set_hopping_seat.set(Some(seat));
                    set_visual_positions.update(|vp| {
                        if seat < 4 {
                            vp[seat] = tile;
                        }
                    });
                    set_step_trail.update(|trail| {
                        if !trail.contains(&tile) {
                            trail.push(tile);
                        }
                    });
                    play_sfx("step");
                },
                Duration::from_millis(delay_ms),
            );
        }

        let done_delay = initial_delay + (steps as u64) * 135 + 160;
        set_timeout(
            move || {
                if move_gen.get_value() != gen {
                    return;
                }
                set_hopping_seat.set(None);
                set_step_trail.set(Vec::new());
                set_landing_tile_burst.set(Some(to_pos));
                set_is_animating_move.set(false);
                flush_narrative();
                set_timeout(
                    move || set_landing_tile_burst.set(None),
                    Duration::from_millis(500),
                );
            },
            Duration::from_millis(done_delay),
        );
    };

    let apply_state_snapshot = move |new_state: GameState| {
        let old_state = game_state.get_untracked();
        let turn_changed = old_state.current_player_index != new_state.current_player_index
            || old_state.is_game_started != new_state.is_game_started;
        let just_finished = !old_state.is_game_over && new_state.is_game_over;

        let old_vp = visual_positions.get_untracked();
        let mut moved_seat: Option<(usize, u8, u8)> = None;

        if old_state.players.len() == new_state.players.len() && new_state.is_game_started {
            for (idx, np) in new_state.players.iter().enumerate().take(4) {
                let prev = old_vp[idx];
                if prev != np.position {
                    moved_seat = Some((idx, prev, np.position));
                    break;
                }
            }
        } else {
            let mut init_vp = [0u8; 4];
            for (idx, p) in new_state.players.iter().enumerate().take(4) {
                init_vp[idx] = p.position;
            }
            set_visual_positions.set(init_vp);
        }

        if let Some(me) = my_player_id.get_untracked() {
            if let (Some(old_me), Some(new_me)) = (
                old_state.players.iter().find(|p| p.id == me),
                new_state.players.iter().find(|p| p.id == me),
            ) {
                let diff = new_me.cash.0 - old_me.cash.0;
                if diff != 0 {
                    let abs_money = Money(diff.abs());
                    let label = if diff > 0 {
                        format!("+{}", fmt_short(abs_money))
                    } else {
                        format!("-{}", fmt_short(abs_money))
                    };
                    enqueue_cash_delta(label, diff > 0);
                }
            }
        }

        if new_state.turn_phase != TurnPhase::Auction {
            set_auction_seconds.set(8);
        }
        set_display_dice.set(new_state.last_dice);
        set_game_state.set(new_state.clone());

        if (turn_changed || just_finished) && is_local_mode.get_untracked() {
            restart_local_turn_clock(new_state.is_game_started, new_state.is_game_over);
        }
        if just_finished {
            enqueue_sound("win");
        }
        if let Some((seat, from_pos, to_pos)) = moved_seat {
            start_hop_animation(seat, from_pos, to_pos, is_rolling.get_untracked());
        } else if !is_rolling.get_untracked() && !is_animating_move.get_untracked() {
            flush_narrative();
        }
    };

    let process_server_message = move |msg: ServerMessage| {
        let board = get_board();
        match msg {
            ServerMessage::RoomJoined {
                player_id,
                session_token,
                ..
            } => {
                set_my_player_id.set(Some(player_id));
                let r_id = room_id.get_untracked();
                save_storage_session_token(&r_id, &session_token);
            }
            ServerMessage::SyncSnapshot { state } => {
                if let Ok(new_state) = serde_json::from_value::<GameState>(state) {
                    apply_state_snapshot(new_state);
                }
            }
            ServerMessage::GameStarted { .. } => {
                push_log("Ván đấu bắt đầu!".to_string());
                play_sfx("buy");
            }
            ServerMessage::TurnStarted {
                time_limit_seconds, ..
            } => {
                set_timer_seconds.set(time_limit_seconds);
            }
            ServerMessage::TimerTick { seconds_remaining } => {
                set_timer_seconds.set(seconds_remaining);
                if seconds_remaining > 0 && seconds_remaining <= 10 {
                    play_sfx("urgent");
                }
            }
            ServerMessage::DiceRolled {
                player_id,
                dice: (d1, d2),
                is_doubles,
                ..
            } => {
                let rgen = roll_gen.get_value() + 1;
                roll_gen.set_value(rgen);
                set_is_rolling.set(true);
                set_display_dice.set((d1, d2));
                play_sfx("roll");
                let pname = player_name_of(player_id);
                let doubles = if is_doubles { " (Đôi!)" } else { "" };
                enqueue_log(format!("{} tung được {} + {}{}", pname, d1, d2, doubles));
                set_timeout(
                    move || {
                        if roll_gen.get_value() == rgen {
                            set_is_rolling.set(false);
                            if !is_animating_move.get_untracked() {
                                flush_narrative();
                            }
                        }
                    },
                    Duration::from_millis(420),
                );
            }
            ServerMessage::PropertyAcquired {
                player_id,
                tile_index,
                cost,
            } => {
                let pname = player_name_of(player_id);
                let tname = &board[tile_index as usize].name;
                enqueue_log(format!("{} mua {} giá {}", pname, tname, fmt_short(cost)));
                enqueue_popup(tile_index, format!("-{}", fmt_short(cost)), false);
                let st = game_state.get_untracked();
                if check_monopoly_completed(&st, tile_index, player_id) {
                    enqueue_sound("monopoly");
                } else {
                    enqueue_sound("buy");
                }
                set_recent_acquired_tile.set(Some(tile_index));
                set_timeout(
                    move || set_recent_acquired_tile.set(None),
                    Duration::from_millis(650),
                );
            }
            ServerMessage::HouseModified {
                tile_index,
                house_count,
            } => {
                let tname = &board[tile_index as usize].name;
                if house_count >= 5 {
                    enqueue_log(format!("Nâng cấp khách sạn tại {}", tname));
                } else if house_count == 0 {
                    enqueue_log(format!("Đã bán hết nhà tại {}", tname));
                } else {
                    enqueue_log(format!("{} hiện có {} căn nhà", tname, house_count));
                }
                enqueue_sound("buy");
                set_recent_house_tile.set(Some(tile_index));
                set_timeout(
                    move || set_recent_house_tile.set(None),
                    Duration::from_millis(650),
                );
            }
            ServerMessage::PropertyMortgageChanged {
                tile_index,
                is_mortgaged,
            } => {
                let tname = &board[tile_index as usize].name;
                if is_mortgaged {
                    enqueue_log(format!("Đã thế chấp {}", tname));
                    enqueue_sound("cash_in");
                } else {
                    enqueue_log(format!("Đã chuộc lại {}", tname));
                    enqueue_sound("cash_out");
                }
            }
            ServerMessage::RentPaid {
                payer,
                recipient,
                tile_index,
                amount,
            } => {
                let from_n = player_name_of(payer);
                let to_n = player_name_of(recipient);
                let tname = &board[tile_index as usize].name;
                enqueue_log(format!(
                    "{} trả {} tiền thuê {} cho {}",
                    from_n,
                    fmt_short(amount),
                    tname,
                    to_n
                ));
                enqueue_popup(tile_index, format!("-{}", fmt_short(amount)), false);
                let me = my_player_id.get_untracked();
                if me == Some(recipient) {
                    enqueue_sound("cash_in");
                } else {
                    enqueue_sound("cash_out");
                }
            }
            ServerMessage::CardDrawn {
                player_id,
                deck,
                description,
                effect_summary,
            } => {
                let pname = player_name_of(player_id);
                let kind_label = if deck.contains("Cơ") || deck.to_lowercase().contains("chance") {
                    "Cơ hội"
                } else {
                    "Khí vận"
                };
                enqueue_log(format!("{} rút {}: {}", pname, kind_label, description));
                let full_desc = if effect_summary.is_empty() {
                    description
                } else {
                    format!("{} ({})", description, effect_summary)
                };
                enqueue_banner(kind_label.to_string(), pname, full_desc);
                enqueue_sound("cash_in");
            }
            ServerMessage::PlayerBankrupt { player_id } => {
                let pname = player_name_of(player_id);
                enqueue_log(format!("{} đã tuyên bố phá sản!", pname));
                enqueue_sound("jail");
            }
            ServerMessage::DebtIncurred {
                debtor,
                creditor,
                amount,
            } => {
                let d_name = player_name_of(debtor);
                let c_name = creditor
                    .map(player_name_of)
                    .unwrap_or_else(|| "Ngân hàng".to_string());
                enqueue_log(format!(
                    "{} đang thiếu {} cho {} — cần xoay tiền trả nợ",
                    d_name,
                    fmt_short(amount),
                    c_name
                ));
                enqueue_sound("urgent");
            }
            ServerMessage::TradeProposed { from, to } => {
                let f_name = player_name_of(from);
                let t_name = player_name_of(to);
                push_log(format!("{} gửi đề nghị giao dịch tới {}", f_name, t_name));
                set_trade_open.set(false);
                play_sfx("cash_in");
            }
            ServerMessage::TradeResolved { from, to, accepted } => {
                let f_name = player_name_of(from);
                let t_name = player_name_of(to);
                if accepted {
                    push_log(format!("{} đã đồng ý giao dịch với {}!", t_name, f_name));
                    play_sfx("monopoly");
                } else {
                    push_log(format!("{} từ chối giao dịch của {}.", t_name, f_name));
                    play_sfx("cash_out");
                }
            }
            ServerMessage::AuctionStarted { tile_index, .. } => {
                let tname = &board[tile_index as usize].name;
                push_log(format!("Bắt đầu đấu giá ô {}!", tname));
                set_auction_seconds.set(8);
            }
            ServerMessage::BidPlaced { bidder, amount } => {
                let pname = player_name_of(bidder);
                push_log(format!("{} đặt giá {}", pname, fmt_short(amount)));
                set_auction_seconds.set(8);
                play_sfx("buy");
            }
            ServerMessage::AuctionTimer { seconds_remaining } => {
                set_auction_seconds.set(seconds_remaining);
            }
            ServerMessage::AuctionConcluded {
                winner,
                final_price,
            } => {
                if let Some(w) = winner {
                    let wname = player_name_of(w);
                    push_log(format!(
                        "{} trúng đấu giá với {}",
                        wname,
                        fmt_short(final_price)
                    ));
                } else {
                    push_log("Không ai trả giá, đất về Ngân hàng.".to_string());
                }
                set_auction_seconds.set(8);
                play_sfx("gavel");
            }
            ServerMessage::ActionRejected { reason } => {
                show_error(reason);
            }
            ServerMessage::BotAssumedControl { player_id, reason } => {
                let pname = player_name_of(player_id);
                push_log(format!("AI tạm điều khiển {} ({})", pname, reason));
            }
            ServerMessage::PlayerReconnected { player_id } => {
                let pname = player_name_of(player_id);
                push_log(format!("{} đã kết nối lại phòng.", pname));
            }
        }
    };

    // Local mode execution + 8s auction countdown + bot scheduler
    let is_local_bot = move |pid: PlayerId| -> bool {
        local_bot_map.with_value(|list| list.iter().any(|&(id, bot)| id == pid && bot))
    };

    let any_human_can_act_in_local_auction = move |st: &GameState| -> bool {
        let Some(ref auc) = st.auction else {
            return false;
        };
        st.players.iter().any(|p| {
            !p.is_bankrupt
                && !is_local_bot(p.id)
                && auc.highest_bidder != Some(p.id)
                && auc.active_bidders.contains(&p.id)
        })
    };

    let sync_local_human_focus = move |st: &GameState| {
        if let Some(ref debt) = st.debt {
            if !is_local_bot(debt.debtor) {
                set_my_player_id.set(Some(debt.debtor));
                return;
            }
        }
        if let Some(ref trade) = st.trade {
            if !is_local_bot(trade.to) {
                set_my_player_id.set(Some(trade.to));
                return;
            }
        }
        if st.turn_phase == TurnPhase::Auction {
            if let Some(ref auc) = st.auction {
                if let Some(next_human) = st.players.iter().find(|p| {
                    !p.is_bankrupt
                        && !is_local_bot(p.id)
                        && auc.highest_bidder != Some(p.id)
                        && auc.active_bidders.contains(&p.id)
                }) {
                    set_my_player_id.set(Some(next_human.id));
                    return;
                }
            }
        }
        if let Some(cur) = st.current_player() {
            if !is_local_bot(cur.id) {
                set_my_player_id.set(Some(cur.id));
            }
        }
    };

    fn schedule_local_auction_clock(
        local_auction_gen: StoredValue<u64>,
        game_state: ReadSignal<GameState>,
        set_auction_seconds: WriteSignal<u32>,
        is_local_mode: ReadSignal<bool>,
        process_fn: Rc<dyn Fn(ServerMessage)>,
        apply_snapshot_fn: Rc<dyn Fn(GameState)>,
        schedule_bots_fn: Rc<dyn Fn()>,
    ) {
        let gen = local_auction_gen.get_value() + 1;
        local_auction_gen.set_value(gen);
        set_auction_seconds.set(8);

        fn step(
            gen: u64,
            rem: u32,
            local_auction_gen: StoredValue<u64>,
            game_state: ReadSignal<GameState>,
            set_auction_seconds: WriteSignal<u32>,
            is_local_mode: ReadSignal<bool>,
            process_fn: Rc<dyn Fn(ServerMessage)>,
            apply_snapshot_fn: Rc<dyn Fn(GameState)>,
            schedule_bots_fn: Rc<dyn Fn()>,
        ) {
            set_timeout(
                move || {
                    if local_auction_gen.get_value() != gen || !is_local_mode.get_untracked() {
                        return;
                    }
                    let mut st = game_state.get_untracked();
                    if st.turn_phase != TurnPhase::Auction || st.is_game_over {
                        return;
                    }
                    if rem > 1 {
                        let next = rem - 1;
                        set_auction_seconds.set(next);
                        step(
                            gen,
                            next,
                            local_auction_gen,
                            game_state,
                            set_auction_seconds,
                            is_local_mode,
                            process_fn,
                            apply_snapshot_fn,
                            schedule_bots_fn,
                        );
                    } else {
                        let mut msgs = Vec::new();
                        close_auction(&mut st, &mut msgs);
                        for m in msgs {
                            process_fn(m);
                        }
                        apply_snapshot_fn(st);
                        schedule_bots_fn();
                    }
                },
                Duration::from_secs(1),
            );
        }

        step(
            gen,
            8,
            local_auction_gen,
            game_state,
            set_auction_seconds,
            is_local_mode,
            process_fn,
            apply_snapshot_fn,
            schedule_bots_fn,
        );
    }

    let process_msg_rc: Rc<dyn Fn(ServerMessage)> = Rc::new(process_server_message);
    let apply_snap_rc: Rc<dyn Fn(GameState)> = Rc::new(apply_state_snapshot);
    let schedule_bots_slot: Rc<std::cell::RefCell<Option<Rc<dyn Fn()>>>> =
        Rc::new(std::cell::RefCell::new(None));

    let execute_local_action = {
        let process_rc = process_msg_rc.clone();
        let snap_rc = apply_snap_rc.clone();
        let bots_slot = schedule_bots_slot.clone();
        move |actor: PlayerId, msg: ClientMessage| {
            let st = game_state.get_untracked();
            if !st.is_game_started || st.is_game_over {
                return;
            }
            let was_auction = st.turn_phase == TurnPhase::Auction;
            let is_bid = matches!(msg, ClientMessage::PlaceBid { .. });
            let mut rng = rand::thread_rng();
            let (st, msgs) = match apply_action(&st, actor, &msg, &mut rng) {
                Ok(res) => res,
                Err(e) => {
                    show_error(e.to_string());
                    return;
                }
            };

            for m in msgs {
                process_rc(m);
            }
            let is_now_auction = st.turn_phase == TurnPhase::Auction;
            if is_now_auction && (!was_auction || is_bid) {
                if let Some(ref sched) = *bots_slot.borrow() {
                    schedule_local_auction_clock(
                        local_auction_gen,
                        game_state,
                        set_auction_seconds,
                        is_local_mode,
                        process_rc.clone(),
                        snap_rc.clone(),
                        sched.clone(),
                    );
                }
            }
            sync_local_human_focus(&st);
            snap_rc(st);

            if let Some(ref sched) = *bots_slot.borrow() {
                sched();
            }
        }
    };

    fn pick_local_bot_action(
        st: &GameState,
        is_bot: impl Fn(PlayerId) -> bool,
        human_in_auction: impl Fn(&GameState) -> bool,
    ) -> Option<(PlayerId, ClientMessage)> {
        if !st.is_game_started || st.is_game_over {
            return None;
        }
        if let Some(ref debt) = st.debt {
            if is_bot(debt.debtor) {
                return decide_action(st, debt.debtor).map(|a| (debt.debtor, a));
            }
            return None;
        }
        if let Some(ref trade) = st.trade {
            if is_bot(trade.to) {
                return decide_action(st, trade.to).map(|a| (trade.to, a));
            }
            return None;
        }
        if st.turn_phase == TurnPhase::Auction {
            for p in &st.players {
                if !p.is_bankrupt && is_bot(p.id) {
                    if let Some(act) = decide_action(st, p.id) {
                        if matches!(act, ClientMessage::PlaceBid { .. }) || !human_in_auction(st) {
                            return Some((p.id, act));
                        }
                    }
                }
            }
            return None;
        }
        let cur = st.current_player()?;
        if is_bot(cur.id) {
            decide_action(st, cur.id).map(|a| (cur.id, a))
        } else {
            None
        }
    }

    let execute_local_rc = Rc::new(execute_local_action);
    {
        let bots_slot = schedule_bots_slot.clone();
        let exec_rc = execute_local_rc.clone();
        let bots_fn: Rc<dyn Fn()> = Rc::new(move || {
            let gen = local_bot_gen.get_value() + 1;
            local_bot_gen.set_value(gen);
            let wait_ms = if is_rolling.get_untracked() || is_animating_move.get_untracked() {
                950
            } else {
                650
            };
            let exec_inner = exec_rc.clone();
            set_timeout(
                move || {
                    if local_bot_gen.get_value() != gen
                        || !is_local_mode.get_untracked()
                        || !in_game.get_untracked()
                    {
                        return;
                    }
                    if is_rolling.get_untracked() || is_animating_move.get_untracked() {
                        let exec_retry = exec_inner.clone();
                        set_timeout(
                            move || {
                                if local_bot_gen.get_value() != gen {
                                    return;
                                }
                                let st = game_state.get_untracked();
                                if let Some((actor, act)) = pick_local_bot_action(
                                    &st,
                                    is_local_bot,
                                    any_human_can_act_in_local_auction,
                                ) {
                                    exec_retry(actor, act);
                                }
                            },
                            Duration::from_millis(550),
                        );
                        return;
                    }
                    let st = game_state.get_untracked();
                    if let Some((actor, act)) =
                        pick_local_bot_action(&st, is_local_bot, any_human_can_act_in_local_auction)
                    {
                        exec_inner(actor, act);
                    }
                },
                Duration::from_millis(wait_ms),
            );
        });
        *bots_slot.borrow_mut() = Some(bots_fn);
    }

    let exec_sv = StoredValue::new_local(execute_local_rc.clone());
    let send_action = Callback::new(move |msg: ClientMessage| {
        if is_local_mode.get_untracked() {
            let st = game_state.get_untracked();
            let actor = match msg {
                ClientMessage::PayDebt | ClientMessage::DeclareBankruptcy => st
                    .debt
                    .as_ref()
                    .map(|d| d.debtor)
                    .or_else(|| my_player_id.get_untracked()),
                ClientMessage::RespondTrade { .. } => st
                    .trade
                    .as_ref()
                    .map(|t| t.to)
                    .or_else(|| my_player_id.get_untracked()),
                ClientMessage::CancelTrade => st
                    .trade
                    .as_ref()
                    .map(|t| t.from)
                    .or_else(|| my_player_id.get_untracked()),
                _ => my_player_id
                    .get_untracked()
                    .or_else(|| st.current_player().map(|p| p.id)),
            };
            if let Some(actor_id) = actor {
                exec_sv.with_value(|f| f(actor_id, msg));
            }
        } else {
            ws_client_ref.with_value(|c| {
                if let Some(client) = c {
                    client.send(msg);
                }
            });
        }
    });

    let connect_online: Rc<dyn Fn(String, String, bool)> = Rc::new(
        move |target_room: String, p_name: String, auction_on: bool| {
            let trimmed_room = if target_room.trim().is_empty() {
                format!("SGN-{}", Uuid::new_v4().to_string()[..4].to_uppercase())
            } else {
                target_room.trim().to_uppercase()
            };
            let trimmed_name = if p_name.trim().is_empty() {
                "Khách Sài Gòn".to_string()
            } else {
                p_name.trim().to_string()
            };

            set_room_id.set(trimmed_room.clone());
            set_player_name.set(trimmed_name.clone());
            set_is_local_mode.set(false);
            set_in_game.set(true);
            set_logs.set(Vec::new());
            set_selected_tile.set(None);
            set_trade_open.set(false);
            set_manual_cam_tile.set(None);

            let window = web_sys::window().expect("window");
            let loc = window.location();
            let proto = if loc.protocol().unwrap_or_default() == "https:" {
                "wss"
            } else {
                "ws"
            };
            let host = loc.host().unwrap_or_else(|_| "127.0.0.1:8080".to_string());
            let ws_url = format!("{}://{}/ws/{}", proto, host, trimmed_room);

            let proc_rc = process_msg_rc.clone();
            match WsClient::connect(
                ws_url,
                trimmed_room,
                trimmed_name,
                auction_on,
                move |srv_msg| {
                    set_ws_connected.set(true);
                    proc_rc(srv_msg);
                },
                move || {
                    set_ws_connected.set(false);
                },
            ) {
                Ok(client) => {
                    ws_client_ref.set_value(Some(Rc::new(client)));
                    set_ws_connected.set(true);
                }
                Err(e) => {
                    show_error(format!("Không thể kết nối máy chủ: {}", e));
                    set_in_game.set(false);
                }
            }
        },
    );

    let start_local_game = {
        let bots_slot = schedule_bots_slot.clone();
        let snap_rc = apply_snap_rc.clone();
        move || {
            let count = local_player_count.get_untracked().clamp(1, 4);
            let slots = local_slots.get_untracked();
            let auction_on = enable_auction_cfg.get_untracked();

            let mut players = Vec::with_capacity(count);
            let mut turn_order = Vec::with_capacity(count);
            let mut bot_flags = Vec::with_capacity(count);
            for (idx, slot) in slots.iter().take(count).enumerate() {
                let id = PlayerId(Uuid::new_v4());
                let is_bot = idx > 0 && slot.is_bot;
                let default_name = if idx == 0 {
                    "Bạn".to_string()
                } else {
                    format!("Người {}", idx + 1)
                };
                let name = if slot.name.trim().is_empty() {
                    default_name
                } else {
                    slot.name.trim().to_string()
                };
                players.push(PlayerState::new(id, name, idx as u8, None));
                turn_order.push(id);
                bot_flags.push((id, is_bot));
            }

            let first_human = bot_flags
                .iter()
                .find(|&&(_, b)| !b)
                .map(|&(id, _)| id)
                .or_else(|| players.first().map(|p| p.id));

            let mut st = GameState::new();
            st.players = players;
            st.turn_order = turn_order;
            st.config.enable_auction = auction_on;
            st.is_game_started = true;

            ws_client_ref.set_value(None);
            local_bot_map.set_value(bot_flags);
            set_is_local_mode.set(true);
            set_ws_connected.set(false);
            set_my_player_id.set(first_human);
            set_in_game.set(true);
            set_logs.set(Vec::new());
            set_selected_tile.set(None);
            set_trade_open.set(false);
            set_manual_cam_tile.set(None);
            set_visual_positions.set([0u8; 4]);
            set_game_state.set(GameState::new());

            push_log("Bắt đầu ván Cờ Tỷ Phú Sài Gòn tại máy!".to_string());
            snap_rc(st);
            if let Some(ref sched) = *bots_slot.borrow() {
                sched();
            }
        }
    };

    let back_to_lobby = Callback::new(move |_: ()| {
        ws_client_ref.set_value(None);
        local_bot_gen.set_value(local_bot_gen.get_value() + 1);
        local_auction_gen.set_value(local_auction_gen.get_value() + 1);
        turn_timer_gen.set_value(turn_timer_gen.get_value() + 1);
        set_in_game.set(false);
        set_ws_connected.set(false);
        set_is_local_mode.set(false);
        set_selected_tile.set(None);
        set_trade_open.set(false);
        set_drawn_card_banner.set(None);
        set_mobile_side_open.set(false);
        set_game_state.set(GameState::new());
    });

    // Global keyboard shortcuts (Space to roll, Escape to dismiss modals/inspect)
    create_effect(move |_| {
        let Some(window) = web_sys::window() else {
            return;
        };
        let cb = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::wrap(Box::new(
            move |ev: web_sys::KeyboardEvent| {
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    if let Some(active) = doc.active_element() {
                        let tag = active.tag_name().to_uppercase();
                        if tag == "INPUT" || tag == "TEXTAREA" {
                            return;
                        }
                    }
                }
                match ev.key().as_str() {
                    "Escape" => {
                        if show_rules.get_untracked() {
                            set_show_rules.set(false);
                        } else if trade_open.get_untracked() {
                            set_trade_open.set(false);
                        } else if selected_tile.get_untracked().is_some() {
                            set_selected_tile.set(None);
                        } else if mobile_side_open.get_untracked() {
                            set_mobile_side_open.set(false);
                        }
                    }
                    " " => {
                        if !in_game.get_untracked()
                            || show_rules.get_untracked()
                            || trade_open.get_untracked()
                            || is_rolling.get_untracked()
                            || is_animating_move.get_untracked()
                        {
                            return;
                        }
                        let st = game_state.get_untracked();
                        if !st.is_game_started
                            || st.is_game_over
                            || st.turn_phase != TurnPhase::WaitingForRoll
                        {
                            return;
                        }
                        let is_my_turn =
                            st.current_player().map(|p| p.id) == my_player_id.get_untracked();
                        if is_my_turn {
                            ev.prevent_default();
                            send_action.run(ClientMessage::RollDice);
                        }
                    }
                    _ => {}
                }
            },
        ));
        let _ = window.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
        cb.forget();
    });

    // Mobile camera coordinates derived smoothly from active player or user minimap tap
    let cam_center_pct = Signal::derive(move || {
        let tile = if let Some(manual) = manual_cam_tile.get() {
            manual
        } else {
            let st = game_state.get();
            let active_seat = hopping_seat.get().unwrap_or_else(|| {
                st.current_player()
                    .and_then(|cp| st.players.iter().position(|p| p.id == cp.id))
                    .unwrap_or(0)
            });
            visual_positions.get()[active_seat.min(3)]
        };
        let (x, y, w, h) = tile_rect_pct(tile);
        let cx = (x + w * 0.5).clamp(23.0, 77.0);
        let cy = (y + h * 0.5).clamp(23.0, 77.0);
        (cx, cy)
    });

    // Clear manual camera override when active turn changes
    create_effect(move |prev_turn: Option<usize>| {
        let cur = game_state.with(|s| s.current_player_index);
        if prev_turn.is_some() && prev_turn != Some(cur) {
            set_manual_cam_tile.set(None);
        }
        cur
    });

    // #39 real sample deed data for Lobby preview
    let sample_tile = get_board()[39].clone();
    let start_local_sv = StoredValue::new_local(start_local_game.clone());
    let connect_online_sv = StoredValue::new_local(connect_online.clone());

    view! {
        <div class="app-shell">
            <header class="topbar">
                <div class="group">
                    <div class="logo">
                        <span class="mark" inner_html=ICON_STAR></span>
                        <span>"Cờ Tỷ Phú Sài Gòn"</span>
                    </div>
                    <Show when=move || in_game.get()>
                        <span class="pill-dark hide-mobile">
                            <span class=move || {
                                if is_local_mode.get() || ws_connected.get() {
                                    "dot on"
                                } else {
                                    "dot"
                                }
                            }></span>
                            {move || {
                                if is_local_mode.get() {
                                    "Chơi tại máy".to_string()
                                } else {
                                    format!("Phòng {}", room_id.get())
                                }
                            }}
                        </span>
                    </Show>
                </div>

                <div class="group">
                    <Show when=move || error_msg.get().is_some()>
                        <div class="toast-error" role="alert">
                            <span>{move || error_msg.get().unwrap_or_default()}</span>
                            <button
                                on:click=move |_| set_error_msg.set(None)
                                aria-label="Đóng thông báo"
                                inner_html=ICON_CLOSE
                            ></button>
                        </div>
                    </Show>

                    <Show when=move || in_game.get()>
                        <button
                            class="pill-dark mobile-only-btn"
                            on:click=move |_| set_mobile_side_open.update(|v| *v = !*v)
                        >
                            {move || if mobile_side_open.get() { "Đóng sổ" } else { "Tài sản & Tin" }}
                        </button>
                    </Show>

                    <button
                        class="pill-dark"
                        on:click=move |_| {
                            let next = !is_muted.get_untracked();
                            set_is_muted.set(next);
                            js_set_sound_muted(next);
                        }
                    >
                        {move || if is_muted.get() { "Âm: Tắt" } else { "Âm: Bật" }}
                    </button>

                    <button
                        class="pill-dark"
                        on:click=move |_| set_show_rules.set(true)
                    >
                        <span style="width: 14px; height: 14px; display: inline-flex;" inner_html=ICON_INFO></span>
                        <span class="hide-mobile">"Luật chơi"</span>
                    </button>

                    <Show when=move || in_game.get()>
                        <button
                            class="pill-dark"
                            on:click=move |_| back_to_lobby.run(())
                        >
                            "Về sảnh"
                        </button>
                    </Show>
                </div>
            </header>

            <Show
                when=move || in_game.get()
                fallback=move || {
                    let deed_view = if let TileKind::Property(ref p) = sample_tile.kind {
                        let color = get_district_color(p.district);
                        let district = get_district_label(p.district);
                        let name = p.name.clone();
                        let price_s = fmt_vnd(p.cost);
                        let house_s = fmt_vnd(p.house_cost);
                        let mort_s = fmt_vnd(p.mortgage_value);
                        let r0 = fmt_vnd(p.rent_table[0]);
                        let r_mono = fmt_vnd(Money(p.rent_table[0].0 * 2));
                        let r1 = fmt_vnd(p.rent_table[1]);
                        let r2 = fmt_vnd(p.rent_table[2]);
                        let r3 = fmt_vnd(p.rent_table[3]);
                        let r4 = fmt_vnd(p.rent_table[4]);
                        let r5 = fmt_vnd(p.rent_table[5]);
                        view! {
                            <div class="lobby-preview">
                                <div class="sheet">
                                    <div class="sheet-head" style=format!("background: {};", color)>
                                        <span class="grp">{format!("Ô #39 · {} · Khế ước mẫu", district)}</span>
                                        <h2>{name}</h2>
                                    </div>
                                    <div class="sheet-body">
                                        <div class="rent-box">
                                            <div class="rent-row"><span>"Giá mua đất"</span><span class="v">{price_s}</span></div>
                                            <div class="rent-row"><span>"Chi phí xây nhà"</span><span class="v">{house_s}</span></div>
                                            <div class="rent-row"><span>"Thế chấp"</span><span class="v">{mort_s}</span></div>
                                        </div>
                                        <div class="rent-box">
                                            <div class="rent-row"><span>"Đất trống"</span><span class="v">{r0}</span></div>
                                            <div class="rent-row"><span>"Trọn bộ quận (×2)"</span><span class="v">{r_mono}</span></div>
                                            <div class="rent-row"><span>"1 căn nhà phố"</span><span class="v">{r1}</span></div>
                                            <div class="rent-row"><span>"2 căn nhà phố"</span><span class="v">{r2}</span></div>
                                            <div class="rent-row"><span>"3 căn nhà phố"</span><span class="v">{r3}</span></div>
                                            <div class="rent-row"><span>"4 căn nhà phố"</span><span class="v">{r4}</span></div>
                                            <div class="rent-row hi"><span>"Khách sạn"</span><span class="v">{r5}</span></div>
                                        </div>
                                    </div>
                                </div>
                                <div class="lobby-facts">
                                    <span>"Vốn 15 tr · 40 ô Sài Gòn"</span>
                                    <span>"Gạch bông & Bảng hiệu"</span>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    };

                    view! {
                        <div class="lobby-shell">
                            <div class="lobby-grid">
                                <div class="lobby-hero">
                                    <h1>"Cờ Tỷ Phú Sài Gòn"</h1>
                                    <p>"Mua đất khu Bến Thành, xây phố Nguyễn Huệ, đấu giá và thương lượng trên bàn gạch bông cổ điển."</p>

                                    <div class="card lobby-form">
                                        <div class="lobby-tabs" role="tablist">
                                            <button
                                                class=move || if lobby_tab.get() == LobbyTab::Online { "lobby-tab active" } else { "lobby-tab" }
                                                on:click=move |_| set_lobby_tab.set(LobbyTab::Online)
                                            >
                                                "Chơi trực tuyến"
                                            </button>
                                            <button
                                                class=move || if lobby_tab.get() == LobbyTab::Local { "lobby-tab active" } else { "lobby-tab" }
                                                on:click=move |_| set_lobby_tab.set(LobbyTab::Local)
                                            >
                                                "Cùng máy & AI"
                                            </button>
                                        </div>

                                        {move || match lobby_tab.get() {
                                            LobbyTab::Online => {
                                                view! {
                                                <div style="display: flex; flex-direction: column; gap: 12px;">
                                                    <div class="field">
                                                        <label for="player-name-input">"Tên hiển thị của bạn"</label>
                                                        <input
                                                            id="player-name-input"
                                                            type="text"
                                                            maxlength="20"
                                                            prop:value=move || player_name.get()
                                                            on:input=move |ev| set_player_name.set(event_target_value(&ev))
                                                        />
                                                    </div>
                                                    <div class="field">
                                                        <label for="room-id-input">"Mã phòng"</label>
                                                        <input
                                                            id="room-id-input"
                                                            type="text"
                                                            maxlength="16"
                                                            prop:value=move || room_id.get()
                                                            on:input=move |ev| set_room_id.set(event_target_value(&ev).to_uppercase())
                                                        />
                                                    </div>
                                                    <div class="switch-row">
                                                        <div>
                                                            "Đấu giá khi bỏ qua đất"
                                                            <small>"Bấm Bỏ qua sẽ mở sàn đấu giá 8 giây"</small>
                                                        </div>
                                                        <button
                                                            type="button"
                                                            class="switch"
                                                            role="switch"
                                                            aria-checked=move || if enable_auction_cfg.get() { "true" } else { "false" }
                                                            on:click=move |_| set_enable_auction_cfg.update(|v| *v = !*v)
                                                        ></button>
                                                    </div>
                                                    <div class="lobby-actions">
                                                        <button
                                                            class="btn btn-primary"
                                                            on:click=move |_| {
                                                                connect_online_sv.with_value(|f| f(room_id.get(), player_name.get(), enable_auction_cfg.get()));
                                                            }
                                                        >
                                                            "Vào phòng"
                                                        </button>
                                                        <button
                                                            class="btn btn-ghost"
                                                            on:click=move |_| {
                                                                let generated = format!("SGN-{}", Uuid::new_v4().to_string()[..4].to_uppercase());
                                                                set_room_id.set(generated.clone());
                                                                connect_online_sv.with_value(|f| f(generated, player_name.get(), enable_auction_cfg.get()));
                                                            }
                                                        >
                                                            "Tạo phòng mới"
                                                        </button>
                                                    </div>
                                                </div>
                                            }.into_any()
                                            },
                                            LobbyTab::Local => {
                                                view! {
                                                    <div style="display: flex; flex-direction: column; gap: 12px;">
                                                        <div class="field">
                                                            <label>"Số người chơi trên bàn"</label>
                                                            <div class="cnt-grid">
                                                                {(1..=4).map(|cnt| {
                                                                    let label = if cnt == 1 { "1 (Solo)".to_string() } else { format!("{} người", cnt) };
                                                                    view! {
                                                                        <button
                                                                            type="button"
                                                                            class=move || if local_player_count.get() == cnt { "cnt-btn active" } else { "cnt-btn" }
                                                                            on:click=move |_| set_local_player_count.set(cnt)
                                                                        >
                                                                            {label}
                                                                        </button>
                                                                    }
                                                                }).collect_view()}
                                                            </div>
                                                        </div>

                                                        <div style="display: flex; flex-direction: column; gap: 6px;">
                                                            {move || {
                                                                let count = local_player_count.get();
                                                                let slots = local_slots.get();
                                                                (0..count).map(|idx| {
                                                                    let slot = slots[idx].clone();
                                                                    let col = player_color(idx);
                                                                    view! {
                                                                        <div class="slot-row">
                                                                            <span
                                                                                class="avatar"
                                                                                style=format!("width: 34px; height: 34px; background: {};", col)
                                                                                inner_html=player_vehicle_svg(idx)
                                                                            ></span>
                                                                            <input
                                                                                type="text"
                                                                                maxlength="18"
                                                                                prop:value=slot.name.clone()
                                                                                on:input=move |ev| {
                                                                                    let val = event_target_value(&ev);
                                                                                    set_local_slots.update(|s| s[idx].name = val);
                                                                                }
                                                                            />
                                                                            {if idx == 0 {
                                                                                view! { <span class="slot-host">"Chủ bàn"</span> }.into_any()
                                                                            } else {
                                                                                view! {
                                                                                    <button
                                                                                        type="button"
                                                                                        class=if slot.is_bot { "slot-role bot" } else { "slot-role" }
                                                                                        on:click=move |_| {
                                                                                            set_local_slots.update(|s| s[idx].is_bot = !s[idx].is_bot);
                                                                                        }
                                                                                    >
                                                                                        {if slot.is_bot { "Máy AI" } else { "Người" }}
                                                                                    </button>
                                                                                }.into_any()
                                                                            }}
                                                                        </div>
                                                                    }
                                                                }).collect_view()
                                                            }}
                                                        </div>

                                                        <div class="switch-row">
                                                            <div>
                                                                "Đấu giá khi bỏ qua đất"
                                                                <small>"Bật để cạnh tranh giá khởi điểm"</small>
                                                            </div>
                                                            <button
                                                                type="button"
                                                                class="switch"
                                                                role="switch"
                                                                aria-checked=move || if enable_auction_cfg.get() { "true" } else { "false" }
                                                                on:click=move |_| set_enable_auction_cfg.update(|v| *v = !*v)
                                                            ></button>
                                                        </div>

                                                        <button
                                                            class="btn btn-primary"
                                                            style="width: 100%;"
                                                            on:click=move |_| start_local_sv.with_value(|f| f())
                                                        >
                                                            <span style="width: 18px; height: 18px; display: inline-flex;" inner_html=ICON_DICE></span>
                                                            "Bắt đầu ván tại máy"
                                                        </button>
                                                    </div>
                                                }.into_any()
                                            }
                                        }}
                                    </div>
                                </div>

                                {deed_view}
                            </div>
                        </div>
                    }
                }
            >
                <div class="game-layout">
                    <PlayersRail
                        state=game_state
                        my_player_id=my_player_id
                        timer_seconds=timer_seconds
                        cash_delta_badge=cash_delta_badge
                    />

                    <main class="board-col">
                        <div class="board-cam">
                            <div
                                class="board-wrap"
                                style=move || {
                                    let (cx, cy) = cam_center_pct.get();
                                    format!("--cam-cx: {:.2}%; --cam-cy: {:.2}%;", cx, cy)
                                }
                            >
                                <Board
                                    state=game_state
                                    my_player_id=my_player_id
                                    selected_tile=selected_tile
                                    visual_positions=visual_positions
                                    hopping_seat=hopping_seat
                                    step_trail=step_trail
                                    landing_tile_burst=landing_tile_burst
                                    recent_acquired_tile=recent_acquired_tile
                                    recent_house_tile=recent_house_tile
                                    floating_popups=floating_popups
                                    on_tile_click=Callback::new(move |idx: u8| {
                                        set_selected_tile.set(Some(idx));
                                    })
                                />
                            </div>

                            <MiniMap
                                state=game_state
                                visual_positions=visual_positions
                                cam_center_pct=cam_center_pct
                                on_jump_tile=Callback::new(move |idx: u8| {
                                    set_manual_cam_tile.set(Some(idx));
                                })
                            />

                            <Show when=move || manual_cam_tile.get().is_some()>
                                <button
                                    class="recenter"
                                    on:click=move |_| set_manual_cam_tile.set(None)
                                >
                                    "Theo quân"
                                </button>
                            </Show>

                            <Show when=move || logs.with(|l| !l.is_empty())>
                                <div class="mobile-toast">
                                    {move || logs.with(|l| l.first().cloned().unwrap_or_default())}
                                </div>
                            </Show>
                        </div>

                        <Stage
                            state=game_state
                            my_player_id=my_player_id
                            timer_seconds=timer_seconds
                            auction_seconds=auction_seconds
                            is_rolling=is_rolling
                            is_animating_move=is_animating_move
                            display_dice=display_dice
                            drawn_card_banner=drawn_card_banner
                            on_dismiss_card=Callback::new(move |_| {
                                card_seq.set_value(card_seq.get_value() + 1);
                                set_drawn_card_banner.set(None);
                            })
                            selected_tile=selected_tile
                            set_selected_tile=set_selected_tile
                            trade_open=trade_open
                            set_trade_open=set_trade_open
                            on_action=send_action
                            on_back_to_lobby=back_to_lobby
                        />
                    </main>

                    <aside class=move || {
                        if mobile_side_open.get() {
                            "rail side-rail mobile-open"
                        } else {
                            "rail side-rail"
                        }
                    }>
                        <Portfolio
                            state=game_state
                            my_player_id=my_player_id
                            set_selected_tile=set_selected_tile
                            on_action=send_action
                        />
                        <LogFeed logs=logs />
                    </aside>
                </div>

                // Online Waiting Room Modal
                <Show when=move || {
                    !is_local_mode.get() && !game_state.with(|s| s.is_game_started)
                }>
                    {move || {
                        let st = game_state.get();
                        let me = my_player_id.get();
                        let r_id = room_id.get();
                        let invite = build_invite_url(&r_id);
                        let is_host = st.players.first().map(|p| Some(p.id) == me).unwrap_or(false);
                        let p_count = st.players.len();
                        let invite_copy = invite.clone();

                        view! {
                            <div class="modal-backdrop">
                                <div class="waiting-room-card">
                                    <div class="wr-head">
                                        <div>
                                            <span class="wr-code">{format!("Phòng {}", r_id)}</span>
                                            <h2>"Phòng chờ"</h2>
                                        </div>
                                        <button
                                            class="btn btn-ghost btn-sm"
                                            on:click=move |_| back_to_lobby.run(())
                                        >
                                            "Rời phòng"
                                        </button>
                                    </div>

                                    <div class="wr-seats">
                                        {(0..4).map(|seat| {
                                            if let Some(p) = st.players.get(seat) {
                                                let col = player_color(seat);
                                                let is_me = Some(p.id) == me;
                                                view! {
                                                    <div class="wr-seat">
                                                        <span
                                                            class="avatar"
                                                            style=format!("width: 34px; height: 34px; background: {};", col)
                                                            inner_html=player_vehicle_svg(seat)
                                                        ></span>
                                                        <span class="wr-name">{p.name.clone()}</span>
                                                        {if seat == 0 {
                                                            view! { <span class="wr-badge host">"Chủ phòng"</span> }.into_any()
                                                        } else if is_me {
                                                            view! { <span class="wr-badge me">"Bạn"</span> }.into_any()
                                                        } else {
                                                            view! { <span class="wr-badge">"Sẵn sàng"</span> }.into_any()
                                                        }}
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <div class="wr-seat empty">
                                                        {format!("Ghế #{} · Đang chờ người chơi...", seat + 1)}
                                                    </div>
                                                }.into_any()
                                            }
                                        }).collect_view()}
                                    </div>

                                    <div class="invite-row">
                                        <span title=invite.clone()>{invite.clone()}</span>
                                        <button
                                            class="btn btn-ghost btn-sm"
                                            on:click=move |_| {
                                                js_copy_text(&invite_copy);
                                                set_copied_invite.set(true);
                                                set_timeout(move || set_copied_invite.set(false), Duration::from_secs(2));
                                            }
                                        >
                                            {move || if copied_invite.get() { "Đã chép!" } else { "Chép link" }}
                                        </button>
                                    </div>

                                    {if is_host {
                                        let cur_auc = st.config.enable_auction;
                                        view! {
                                            <div style="display: flex; flex-direction: column; gap: 12px;">
                                                <div class="switch-row">
                                                    <div>
                                                        "Đấu giá khi bỏ qua đất"
                                                        <small>"Chỉ chủ phòng mới đổi được trước khi bắt đầu"</small>
                                                    </div>
                                                    <button
                                                        type="button"
                                                        class="switch"
                                                        role="switch"
                                                        aria-checked=if cur_auc { "true" } else { "false" }
                                                        on:click=move |_| {
                                                            send_action.run(ClientMessage::ConfigureGame {
                                                                enable_auction: !cur_auc,
                                                                max_players: None,
                                                            });
                                                        }
                                                    ></button>
                                                </div>
                                                <div class="lobby-actions">
                                                    <button
                                                        class="btn btn-ghost"
                                                        disabled=p_count >= 4
                                                        on:click=move |_| send_action.run(ClientMessage::AddBot)
                                                    >
                                                        "Thêm Máy AI"
                                                    </button>
                                                    <button
                                                        class="btn btn-primary"
                                                        disabled=p_count < 2
                                                        on:click=move |_| send_action.run(ClientMessage::StartGame)
                                                    >
                                                        "Bắt đầu ván"
                                                    </button>
                                                </div>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class="waiting" style="justify-content: center;">
                                                <span>"Đang chờ "<b>"Chủ phòng"</b>" bắt đầu ván..."</span>
                                            </div>
                                        }.into_any()
                                    }}
                                </div>
                            </div>
                        }
                    }}
                </Show>
            </Show>

            // Rules Modal
            <Show when=move || show_rules.get()>
                <div class="modal-backdrop" on:click=move |_| set_show_rules.set(false)>
                    <div class="rules-modal" on:click=move |ev| ev.stop_propagation()>
                        <div class="rules-modal-header">
                            <span>"Luật chơi nhanh · Cờ Tỷ Phú Sài Gòn"</span>
                            <button
                                class="pill-dark"
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
                                        <text x="51" y="12" font-size="9" font-weight="700" fill="#2b7a53" text-anchor="middle">"+2M"</text>
                                    </svg>
                                </div>
                                <div class="rule-body">
                                    <div class="rule-title-row">
                                        <span class="rule-step-badge">"1"</span>
                                        <span class="rule-title">"Lắc xúc xắc & Đi qua ô Khởi hành"</span>
                                    </div>
                                    <p class="rule-desc">
                                        "Mỗi người bắt đầu với 15.000.000 đ. Nhấn «Lắc xúc xắc» (hoặc phím Space) để di chuyển; đi qua hoặc dừng tại ô Khởi hành nhận +2.000.000 đ. Tung được đôi sẽ được thêm 1 lượt."
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
                                        <span class="rule-title">"Mua đất, Đấu giá 8s & Giao dịch"</span>
                                    </div>
                                    <p class="rule-desc">
                                        "Dừng vào ô đất trống có thể bấm «Mua» hoặc «Bỏ qua» để mở đấu giá đếm ngược 8 giây. Trong lượt của mình, bạn có thể bấm «Giao dịch» để đổi đất và tiền mặt với người chơi khác."
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
                                        "Sở hữu đủ các ô cùng màu một quận sẽ tự nhân đôi tiền thuê đất trống và cho phép nâng cấp tối đa 4 căn nhà phố rồi lên Khách sạn."
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
                                        <span class="rule-title">"Cơ hội, Khí vận & Xoay tiền trả nợ"</span>
                                    </div>
                                    <p class="rule-desc">
                                        "Nếu thiếu tiền trả thuê hoặc thuế, ván không kết thúc ngay: bạn có thể bán nhà hoặc thế chấp đất để xoay đủ tiền trả nợ trước khi phải tuyên bố phá sản."
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
                                        "Khi không thể trả nợ và tuyên bố phá sản, tài sản sẽ bàn giao cho chủ nợ. Người trụ lại cuối cùng trên bàn cờ Sài Gòn là người chiến thắng!"
                                    </p>
                                </div>
                            </li>
                        </ol>
                    </div>
                </div>
            </Show>
        </div>
    }
}

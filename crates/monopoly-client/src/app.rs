use crate::components::board::{Board, BoardFloatingPopup};
use crate::components::controls::Controls;
use crate::components::log::LogFeed;
use crate::palette::{
    ACCENT_PRIMARY, BG_SECONDARY, BORDER_DEFAULT, DISTRICT_1_DARK_BLUE, STATUS_DANGER,
    STATUS_SUCCESS, STATUS_WARNING, SURFACE_HOVER, SURFACE_PANEL, TEXT_MUTED, TEXT_PRIMARY,
    TEXT_SECONDARY,
};
use crate::ws::{save_storage_session_token, WsClient};
use leptos::*;
use monopoly_core::board::{get_board, TileKind};
use monopoly_core::money::Money;
use monopoly_core::state::GameState;
use monopoly_proto::{ClientMessage, PlayerId, ServerMessage};
use std::rc::Rc;
use std::time::Duration;
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Lobby,
    Game,
}

#[component]
pub fn App() -> impl IntoView {
    let (view_mode, set_view_mode) = create_signal(ViewMode::Lobby);
    let (player_name, set_player_name) = create_signal("Nguyễn Minh".to_string());
    let (room_id, set_room_id) = create_signal("SGN-888".to_string());
    let (enable_auction_config, set_enable_auction_config) = create_signal(false);
    let (game_state, set_game_state) = create_signal(GameState::new());
    let (my_player_id, set_my_player_id) = create_signal::<Option<PlayerId>>(None);
    let (timer_seconds, set_timer_seconds) = create_signal(30u32);
    let (logs, set_logs) = create_signal(vec!["Phiên bàn cờ đã sẵn sàng".to_string()]);
    let (selected_tile, set_selected_tile) = create_signal::<Option<u8>>(Some(0));

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

    let connect_to_room = move |target_room: String, name: String, auction_enabled: bool| {
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

        let target_room_clone = target_room.clone();
        let client_res = WsClient::connect(
            ws_url,
            target_room.clone(),
            name,
            auction_enabled,
            move |server_msg| match server_msg {
                ServerMessage::RoomJoined {
                    room_id: _,
                    player_id,
                    session_token,
                    seat_index,
                } => {
                    set_my_player_id.set(Some(player_id));
                    save_storage_session_token(&target_room_clone, &session_token);
                    add_log(format!(
                        "Đã vào phòng {} tại vị trí T{}",
                        target_room_clone,
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
                }
                ServerMessage::DiceRolled {
                    player_id,
                    dice,
                    new_tile_index,
                    is_doubles,
                } => {
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
            },
            move || {
                add_log("Mất kết nối máy chủ, đang kết nối lại".to_string());
            },
        );

        match client_res {
            Ok(client) => {
                set_ws_client.set(Some(Rc::new(client)));
                set_view_mode.set(ViewMode::Game);
            }
            Err(e) => {
                set_error_toast.set(Some(format!("Không thể kết nối máy chủ: {}", e)));
            }
        }
    };

    let handle_action = Callback::new(move |action: ClientMessage| {
        if matches!(action, ClientMessage::RollDice) {
            set_is_rolling.set(true);
            set_is_animating_move.set(true);
        }
        if let Some(ref client) = ws_client.get() {
            client.send(action);
        }
    });

    view! {
        <div class="app-shell">
            <header style=format!(
                "background: {}; border-bottom: 1px solid {}; height: 56px; padding: 0 24px; display: flex; align-items: center; justify-content: space-between; flex-shrink: 0;",
                SURFACE_PANEL, BORDER_DEFAULT
            )>
                <div style="display: flex; align-items: center; gap: 12px;">
                    <span style=format!(
                        "width: 10px; height: 10px; border-radius: 2px; background: {}; display: inline-block; flex-shrink: 0;",
                        ACCENT_PRIMARY
                    )></span>
                    <span style=format!(
                        "font-size: 16px; line-height: 20px; font-weight: 600; color: {};",
                        TEXT_PRIMARY
                    )>
                        "Cờ tỷ phú Sài Gòn"
                    </span>
                    <span
                        class="mono"
                        style=format!(
                            "font-size: 12px; line-height: 16px; background: {}; color: {}; border: 1px solid {}; padding: 4px 8px; border-radius: 4px; font-weight: 400;",
                            BG_SECONDARY, TEXT_MUTED, BORDER_DEFAULT
                        )
                    >
                        {move || format!("Phòng {}", room_id.get())}
                    </span>
                    {move || {
                        if view_mode.get() == ViewMode::Game {
                            let auction_on = game_state.get().config.enable_auction;
                            let dot_col = if auction_on { STATUS_SUCCESS } else { TEXT_MUTED };
                            let label = if auction_on { "Đấu giá · Đang bật" } else { "Đấu giá · Đang tắt" };
                            view! {
                                <button
                                    class="btn-outline"
                                    style=format!(
                                        "font-size: 12px; line-height: 16px; height: 32px; background: {}; color: {}; border: 1px solid {}; padding: 0 12px; border-radius: 8px; font-weight: 500; cursor: pointer; display: inline-flex; align-items: center; gap: 8px;",
                                        BG_SECONDARY, TEXT_SECONDARY, BORDER_DEFAULT
                                    )
                                    title="Bật hoặc tắt đấu giá khi người chơi bỏ lượt mua đất"
                                    on:click=move |_| {
                                        let next_val = !game_state.get_untracked().config.enable_auction;
                                        set_enable_auction_config.set(next_val);
                                        handle_action.call(ClientMessage::ConfigureGame { enable_auction: next_val });
                                    }
                                >
                                    <span style=format!("width: 6px; height: 6px; border-radius: 50%; background: {};", dot_col)></span>
                                    <span>{label}</span>
                                </button>
                            }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}
                </div>
                {move || {
                    if let Some(err) = error_toast.get() {
                        view! {
                            <div style=format!(
                                "background: {}; color: {}; border: 1px solid {}; padding: 4px 12px; border-radius: 8px; font-size: 13px; line-height: 20px; font-weight: 500; display: flex; align-items: center; gap: 8px;",
                                BG_SECONDARY, STATUS_DANGER, BORDER_DEFAULT
                            )>
                                <span>{err}</span>
                                <button
                                    style=format!("background: transparent; border: none; color: {}; cursor: pointer; font-size: 13px; padding: 0 4px;", TEXT_MUTED)
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
                                                "Ván đấu trực tuyến 2 đến 4 người chơi theo chuẩn khế ước địa ốc, hỗ trợ đấu giá công khai khi bỏ qua quyền mua và tự động đi thay khi hết 30 giây"
                                            </p>
                                        </div>

                                        <div class="panel" style="padding: 24px; gap: 16px;">
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
                                <Board
                                    state=game_state
                                    selected_tile=selected_tile
                                    is_rolling=is_rolling
                                    display_dice=display_dice
                                    visual_positions=visual_positions
                                    hopping_seat=hopping_seat
                                    step_trail=step_trail
                                    landing_tile_burst=landing_tile_burst
                                    recent_acquired_tile=recent_acquired_tile
                                    recent_house_tile=recent_house_tile
                                    floating_popups=floating_popups
                                    drawn_card_banner=drawn_card_banner
                                    on_tile_click=Some(Callback::new(move |idx| {
                                        set_selected_tile.set(Some(idx));
                                    }))
                                />

                                <div class="right-column">
                                    <Controls
                                        state=game_state
                                        my_player_id=my_player_id
                                        timer_seconds=timer_seconds
                                        is_rolling=is_rolling
                                        is_animating_move=is_animating_move
                                        display_dice=display_dice
                                        cash_delta_badge=cash_delta_badge
                                        on_action=handle_action
                                        selected_tile=selected_tile
                                    />
                                    <LogFeed logs=logs />
                                </div>
                            </div>
                        }.into_view()
                    }
                }
            }}
        </div>
    }
}

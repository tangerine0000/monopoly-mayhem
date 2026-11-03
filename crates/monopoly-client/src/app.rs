use crate::components::board::Board;
use crate::components::controls::Controls;
use crate::components::ledger::Ledger;
use crate::components::log::LogFeed;
use crate::components::minimap::Minimap;
use crate::ws::{save_storage_session_token, WsClient};
use leptos::*;
use monopoly_core::state::GameState;
use monopoly_proto::{ClientMessage, PlayerId, ServerMessage};
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Lobby,
    Game,
}

#[component]
pub fn App() -> impl IntoView {
    let (view_mode, set_view_mode) = create_signal(ViewMode::Lobby);
    let (player_name, set_player_name) = create_signal("Sài Gòn Pro".to_string());
    let (room_id, set_room_id) = create_signal("SGN-888".to_string());
    let (game_state, set_game_state) = create_signal(GameState::new());
    let (my_player_id, set_my_player_id) = create_signal::<Option<PlayerId>>(None);
    let (timer_seconds, set_timer_seconds) = create_signal(30u32);
    let (logs, set_logs) = create_signal(vec!["Chào mừng đến với Cờ Tỷ Phú Sài Gòn!".to_string()]);
    let (selected_tile, set_selected_tile) = create_signal::<Option<u8>>(None);
    let (ws_client, set_ws_client) = create_signal::<Option<Rc<WsClient>>>(None);
    let (error_toast, set_error_toast) = create_signal::<Option<String>>(None);

    let add_log = move |msg: String| {
        set_logs.update(|l| l.push(msg));
    };

    let connect_to_room = move |target_room: String, name: String| {
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
                        "Đã vào phòng {} ở ghế số #{}",
                        target_room_clone,
                        seat_index + 1
                    ));
                }
                ServerMessage::GameStarted { turn_order: _ } => {
                    add_log("Trận đấu chính thức bắt đầu!".to_string());
                }
                ServerMessage::TurnStarted {
                    active_player,
                    time_limit_seconds,
                } => {
                    set_timer_seconds.set(time_limit_seconds);
                    let name = game_state
                        .get()
                        .find_player(active_player)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| "Người chơi".to_string());
                    add_log(format!("Bắt đầu lượt đi của {}", name));
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
                    let name = game_state
                        .get()
                        .find_player(player_id)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| "Người chơi".to_string());
                    let doubles_str = if is_doubles {
                        " (ĐÔI! ĐƯỢC ĐI TIẾP)"
                    } else {
                        ""
                    };
                    add_log(format!(
                        "🎲 {} lắc được ({}, {}) -> Đến ô #{}{}",
                        name, dice.0, dice.1, new_tile_index, doubles_str
                    ));
                }
                ServerMessage::PropertyAcquired {
                    player_id,
                    tile_index,
                    cost,
                } => {
                    let name = game_state
                        .get()
                        .find_player(player_id)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| "Người chơi".to_string());
                    add_log(format!(
                        "🏢 {} đã mua ô #{} với giá {}",
                        name, tile_index, cost
                    ));
                }
                ServerMessage::RentPaid {
                    payer,
                    recipient,
                    tile_index,
                    amount,
                } => {
                    let p_name = game_state
                        .get()
                        .find_player(payer)
                        .map(|p| p.name.clone())
                        .unwrap_or_default();
                    let r_name = game_state
                        .get()
                        .find_player(recipient)
                        .map(|p| p.name.clone())
                        .unwrap_or_default();
                    add_log(format!(
                        "💰 {} đã trả {} tiền thuê ô #{} cho {}",
                        p_name, amount, tile_index, r_name
                    ));
                }
                ServerMessage::CardDrawn {
                    player_id,
                    deck,
                    description,
                    effect_summary,
                } => {
                    let name = game_state
                        .get()
                        .find_player(player_id)
                        .map(|p| p.name.clone())
                        .unwrap_or_default();
                    add_log(format!(
                        "🃏 {} rút thẻ {}: {} ({})",
                        name, deck, description, effect_summary
                    ));
                }
                ServerMessage::BotAssumedControl { player_id, reason } => {
                    let name = game_state
                        .get()
                        .find_player(player_id)
                        .map(|p| p.name.clone())
                        .unwrap_or_default();
                    add_log(format!(
                        "🚨 Bot Xe Ôm thay mặt {} thao tác (Lý do: {})",
                        name, reason
                    ));
                }
                ServerMessage::SyncSnapshot { state } => {
                    if let Ok(st) = serde_json::from_value::<GameState>(state) {
                        set_game_state.set(st);
                    }
                }
                ServerMessage::ActionRejected { reason } => {
                    set_error_toast.set(Some(reason.clone()));
                    add_log(format!("❌ Thao tác bị từ chối: {}", reason));
                }
                ServerMessage::PlayerBankrupt { player_id } => {
                    let name = game_state
                        .get()
                        .find_player(player_id)
                        .map(|p| p.name.clone())
                        .unwrap_or_default();
                    add_log(format!("💥 {} đã tuyên bố phá sản!", name));
                }
                _ => {}
            },
            move || {
                add_log("Mất kết nối tới máy chủ! Đang thử kết nối lại...".to_string());
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
        if let Some(ref client) = ws_client.get() {
            client.send(action);
        }
    });

    view! {
        <div style="min-height: 100vh; display: flex; flex-direction: column;">
            <header style="background: #1e293b; border-bottom: 1px solid #334155; padding: 12px 20px; display: flex; align-items: center; justify-content: space-between;">
                <div style="display: flex; align-items: center; gap: 12px;">
                    <span style="font-size: 1.3rem; font-weight: 800; color: #e11d48;">
                        "CỜ TỶ PHÚ SÀI GÒN"
                    </span>
                    <span style="font-size: 0.85rem; background: #0f172a; color: #94a3b8; padding: 4px 8px; border-radius: 6px; font-weight: 600;">
                        {move || format!("Phòng: {}", room_id.get())}
                    </span>
                </div>
                {move || {
                    if let Some(err) = error_toast.get() {
                        view! {
                            <div style="background: #ef4444; color: white; padding: 6px 12px; border-radius: 6px; font-size: 0.85rem; font-weight: 600; display: flex; align-items: center; gap: 8px;">
                                <span>{err}</span>
                                <button style="background: transparent; border: none; color: white; cursor: pointer;" on:click=move |_| set_error_toast.set(None)>"✕"</button>
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
                            <div style="flex: 1; display: flex; align-items: center; justify-content: center; padding: 20px;">
                                <div class="panel" style="max-width: 440px; width: 100%; padding: 30px; gap: 20px; text-align: center;">
                                    <h2 style="font-size: 1.8rem; font-weight: 800; color: #f8fafc;">
                                        "Tham Gia Trận Đấu"
                                    </h2>
                                    <p style="color: #94a3b8; font-size: 0.95rem;">
                                        "Trải nghiệm bản đồ 40 ô địa danh Sài Gòn, luật chơi chuẩn, đấu giá tài sản và bot Xe Ôm thông minh!"
                                    </p>

                                    <div style="display: flex; flex-direction: column; gap: 12px; text-align: left;">
                                        <label style="font-size: 0.85rem; font-weight: 600; color: #cbd5e1;">"Tên Người Chơi:"</label>
                                        <input
                                            type="text"
                                            prop:value=player_name
                                            on:input=move |ev| set_player_name.set(event_target_value(&ev))
                                        />

                                        <label style="font-size: 0.85rem; font-weight: 600; color: #cbd5e1; margin-top: 6px;">"Mã Phòng (Room ID):"</label>
                                        <input
                                            type="text"
                                            prop:value=room_id
                                            on:input=move |ev| set_room_id.set(event_target_value(&ev))
                                        />
                                    </div>

                                    <div style="display: flex; gap: 10px; margin-top: 10px;">
                                        <button
                                            class="btn"
                                            style="flex: 1;"
                                            on:click=move |_| {
                                                connect_to_room(room_id.get(), player_name.get());
                                            }
                                        >
                                            "Vào Phòng"
                                        </button>
                                        <button
                                            class="btn btn-secondary"
                                            style="flex: 1;"
                                            on:click=move |_| {
                                                let p_name = player_name.get();
                                                spawn_local(async move {
                                                    let generated_room = format!("SGN-{}", uuid::Uuid::new_v4().to_string()[..4].to_uppercase());
                                                    set_room_id.set(generated_room.clone());
                                                    connect_to_room(generated_room, p_name);
                                                });
                                            }
                                        >
                                            "Tạo Phòng Mới"
                                        </button>
                                    </div>
                                </div>
                            </div>
                        }.into_view()
                    }
                    ViewMode::Game => {
                        view! {
                            <div class="game-layout">
                                <div style="display: flex; flex-direction: column; gap: 16px; overflow-y: auto;">
                                    <Ledger state=game_state my_player_id=my_player_id />
                                    <div class="panel">
                                        <Minimap
                                            state=game_state
                                            on_select_quadrant=Callback::new(move |q| {
                                                add_log(format!("Chuyển góc nhìn tới góc phần tư Q{}", q));
                                            })
                                        />
                                    </div>
                                </div>

                                <Board
                                    state=game_state
                                    on_tile_click=Some(Callback::new(move |idx| {
                                        set_selected_tile.set(Some(idx));
                                    }))
                                />

                                <div class="right-column" style="display: flex; flex-direction: column; gap: 16px; overflow-y: auto;">
                                    <Controls
                                        state=game_state
                                        my_player_id=my_player_id
                                        timer_seconds=timer_seconds
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

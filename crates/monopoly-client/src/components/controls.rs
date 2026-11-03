use leptos::*;
use monopoly_core::state::{GameState, TurnPhase};
use monopoly_proto::{ClientMessage, Money, PlayerId};

#[component]
pub fn Controls(
    state: ReadSignal<GameState>,
    my_player_id: ReadSignal<Option<PlayerId>>,
    timer_seconds: ReadSignal<u32>,
    on_action: Callback<ClientMessage>,
    selected_tile: ReadSignal<Option<u8>>,
) -> impl IntoView {
    let is_my_turn = move || {
        let cur = state.get().current_player().map(|p| p.id);
        my_player_id.get().is_some() && cur == my_player_id.get()
    };

    let me = move || {
        let pid = my_player_id.get()?;
        state.get().find_player(pid).cloned()
    };

    view! {
        <div class="panel">
            <h3 style="font-size: 1.1rem; font-weight: 700; color: #f8fafc; display: flex; align-items: center; justify-content: space-between;">
                <span>"Bảng Điều Khiển"</span>
                {move || {
                    let s = timer_seconds.get();
                    view! {
                        <span style="font-size: 0.9rem; font-weight: 600; color: #fbbf24;">
                            {format!("⏱️ {}s", s)}
                        </span>
                    }
                }}
            </h3>

            <div class="timer-container">
                <div
                    class="timer-fill"
                    style=move || {
                        let pct = (timer_seconds.get() as f64 / 30.0) * 100.0;
                        format!("width: {}%;", pct.clamp(0.0, 100.0))
                    }
                ></div>
            </div>

            {move || {
                let (d1, d2) = state.get().last_dice;
                let sum = d1 + d2;
                view! {
                    <div style="display: flex; gap: 10px; align-items: center; justify-content: center; padding: 10px; background: #0f172a; border-radius: 8px;">
                        <span style="font-size: 1.5rem; font-weight: 800; background: #e11d48; width: 44px; height: 44px; display: inline-flex; align-items: center; justify-content: center; border-radius: 8px; color: white;">
                            {d1.to_string()}
                        </span>
                        <span style="font-size: 1.5rem; font-weight: 800; background: #e11d48; width: 44px; height: 44px; display: inline-flex; align-items: center; justify-content: center; border-radius: 8px; color: white;">
                            {d2.to_string()}
                        </span>
                        <span style="font-size: 1rem; color: #94a3b8; font-weight: 600; margin-left: 8px;">
                            {format!("(Tổng: {})", sum)}
                        </span>
                    </div>
                }
            }}

            <div style="display: flex; flex-direction: column; gap: 8px;">
                {move || {
                    let st = state.get();
                    let my_turn = is_my_turn();
                    let player = me();

                    match st.turn_phase {
                        TurnPhase::WaitingForRoll => {
                            let in_jail = player.as_ref().map(|p| p.in_jail).unwrap_or(false);
                            let cards = player.as_ref().map(|p| p.get_out_of_jail_cards).unwrap_or(0);

                            view! {
                                <div style="display: flex; flex-direction: column; gap: 8px;">
                                    <button
                                        class="btn"
                                        disabled=!my_turn
                                        on:click=move |_| on_action.call(ClientMessage::RollDice)
                                    >
                                        "🎲 Lắc Xúc Xắc"
                                    </button>
                                    {if in_jail {
                                        view! {
                                            <div style="display: flex; gap: 6px;">
                                                <button
                                                    class="btn btn-secondary"
                                                    style="flex: 1;"
                                                    disabled=!my_turn
                                                    on:click=move |_| on_action.call(ClientMessage::PayJailBail)
                                                >
                                                    "Nộp Phạt (500k)"
                                                </button>
                                                {if cards > 0 {
                                                    view! {
                                                        <button
                                                            class="btn btn-secondary"
                                                            style="flex: 1;"
                                                            disabled=!my_turn
                                                            on:click=move |_| on_action.call(ClientMessage::UseJailCard)
                                                        >
                                                            "Dùng Thẻ Thoát Tù"
                                                        </button>
                                                    }.into_view()
                                                } else {
                                                    view! {}.into_view()
                                                }}
                                            </div>
                                        }.into_view()
                                    } else {
                                        view! {}.into_view()
                                    }}
                                </div>
                            }.into_view()
                        }
                        TurnPhase::TileActionPending => {
                            view! {
                                <div style="display: flex; gap: 8px;">
                                    <button
                                        class="btn"
                                        style="flex: 1;"
                                        disabled=!my_turn
                                        on:click=move |_| on_action.call(ClientMessage::BuyProperty)
                                    >
                                        "🏢 Mua Bất Động Sản"
                                    </button>
                                    <button
                                        class="btn btn-outline"
                                        style="flex: 1;"
                                        disabled=!my_turn
                                        on:click=move |_| on_action.call(ClientMessage::DeclineProperty)
                                    >
                                        "Bỏ Qua / Đấu Giá"
                                    </button>
                                </div>
                            }.into_view()
                        }
                        TurnPhase::Auction => {
                            let cur_bid = st.auction.as_ref().map(|a| a.current_bid).unwrap_or(Money::ZERO);
                            let next_bid = cur_bid + Money::from_vnd(50_000);
                            view! {
                                <div style="display: flex; gap: 8px;">
                                    <button
                                        class="btn"
                                        style="flex: 1;"
                                        on:click=move |_| on_action.call(ClientMessage::PlaceBid { amount: next_bid })
                                    >
                                        "Đặt Giá (+50k)"
                                    </button>
                                    <button
                                        class="btn btn-outline"
                                        style="flex: 1;"
                                        on:click=move |_| on_action.call(ClientMessage::PassAuction)
                                    >
                                        "Bỏ Đấu Giá"
                                    </button>
                                </div>
                            }.into_view()
                        }
                        TurnPhase::TurnEnded => {
                            view! {
                                <button
                                    class="btn"
                                    disabled=!my_turn
                                    on:click=move |_| on_action.call(ClientMessage::EndTurn)
                                >
                                    "➡️ Kết Thúc Lượt"
                                </button>
                            }.into_view()
                        }
                    }
                }}
            </div>

            {move || {
                if let Some(t_idx) = selected_tile.get() {
                    view! {
                        <div style="margin-top: 10px; padding: 10px; background: #0f172a; border-radius: 8px; display: flex; flex-direction: column; gap: 6px;">
                            <span style="font-size: 0.85rem; font-weight: 600; color: #94a3b8;">
                                {format!("Thao Tác Ô Đất #{}:", t_idx)}
                            </span>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 6px;">
                                <button class="btn btn-outline" style="min-height: 38px; font-size: 0.85rem;" on:click=move |_| on_action.call(ClientMessage::BuildHouse { tile_index: t_idx })>
                                    "+ Xây Nhà"
                                </button>
                                <button class="btn btn-outline" style="min-height: 38px; font-size: 0.85rem;" on:click=move |_| on_action.call(ClientMessage::SellHouse { tile_index: t_idx })>
                                    "- Bán Nhà"
                                </button>
                                <button class="btn btn-outline" style="min-height: 38px; font-size: 0.85rem;" on:click=move |_| on_action.call(ClientMessage::MortgageProperty { tile_index: t_idx })>
                                    "Thế Chấp"
                                </button>
                                <button class="btn btn-outline" style="min-height: 38px; font-size: 0.85rem;" on:click=move |_| on_action.call(ClientMessage::UnmortgageProperty { tile_index: t_idx })>
                                    "Chuộc Lại"
                                </button>
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

#![allow(dead_code)]

use crate::palette::{
    get_district_color, get_district_label, BG_SECONDARY, BORDER_DEFAULT, BORDER_SUBTLE,
    PLAYER_AVATARS, PLAYER_COLORS, STATUS_DANGER, STATUS_SUCCESS, SURFACE_HOVER, TEXT_MUTED,
    TEXT_ON_ACCENT, TEXT_PRIMARY, TEXT_SECONDARY, TILE_CHANCE, TILE_CHEST, TILE_STATION, TILE_TAX,
    TILE_UTILITY,
};
use leptos::prelude::*;
use monopoly_core::board::{get_board, get_district_tiles, TileKind};
use monopoly_core::money::Money as CoreMoney;
use monopoly_core::state::{GameState, TurnPhase};
use monopoly_proto::{ClientMessage, Money, PlayerId};

#[component]
pub fn Controls(
    state: ReadSignal<GameState>,
    my_player_id: ReadSignal<Option<PlayerId>>,
    timer_seconds: ReadSignal<u32>,
    is_rolling: ReadSignal<bool>,
    is_animating_move: ReadSignal<bool>,
    display_dice: ReadSignal<(u8, u8)>,
    cash_delta_badge: ReadSignal<Option<(String, bool)>>,
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
            // Header & Countdown Timer
            <div style="display: flex; align-items: center; justify-content: space-between; height: 24px;">
                <span style=format!(
                    "font-size: 14px; line-height: 20px; font-weight: 500; color: {};",
                    TEXT_PRIMARY
                )>
                    "Điều khiển lượt"
                </span>
                {move || {
                    let s = timer_seconds.get();
                    let urgent = s <= 30;
                    let col = if urgent { STATUS_DANGER } else { TEXT_SECONDARY };
                    view! {
                        <span
                            class="mono"
                            style=format!(
                                "font-size: 12px; line-height: 16px; font-weight: 500; color: {}; display: inline-flex; align-items: center; gap: 6px;",
                                col
                            )
                        >
                            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                                <circle cx="12" cy="12" r="9"></circle>
                                <polyline points="12 7 12 12 15 15"></polyline>
                            </svg>
                            <span>{format!("{:02}s", s)}</span>
                        </span>
                    }
                }}
            </div>

            // Timer Progress Track (uses transform scaleX, zero layout animation)
            <div class="timer-container">
                <div
                    class=move || if timer_seconds.get() <= 30 { "timer-fill urgent" } else { "timer-fill" }
                    style=move || {
                        let ratio = (timer_seconds.get() as f64 / 120.0).clamp(0.0, 1.0);
                        format!("transform: scaleX({:.3});", ratio)
                    }
                ></div>
            </div>

            // Player Standings Ledger (aligned columns, 36px rows, 4px gap)
            <div style="display: flex; flex-direction: column; gap: 4px;">
                {move || {
                    let st = state.get();
                    let cur_active_id = st.current_player().map(|p| p.id);
                    let my_id = my_player_id.get();
                    let delta = cash_delta_badge.get();

                    st.players.into_iter().enumerate().map(|(idx, p)| {
                        let code = PLAYER_AVATARS[idx % 4];
                        let col = PLAYER_COLORS[idx % 4];
                        let is_active = cur_active_id == Some(p.id);
                        let is_me = my_id == Some(p.id);
                        let row_bg = if is_active { SURFACE_HOVER } else { BG_SECONDARY };
                        let row_border = if is_active { BORDER_SUBTLE } else { BORDER_DEFAULT };
                        let flash_cls = if is_me {
                            match delta {
                                Some((_, true)) => "cash-pill-gain",
                                Some((_, false)) => "cash-pill-loss",
                                None => "",
                            }
                        } else {
                            ""
                        };

                        view! {
                            <div
                                class=flash_cls
                                style=format!(
                                    "height: 36px; padding: 0 12px; border-radius: 8px; background: {}; border: 1px solid {}; display: flex; align-items: center; justify-content: space-between; gap: 8px;",
                                    row_bg, row_border
                                )
                            >
                                <div style="display: flex; align-items: center; gap: 8px; min-width: 0;">
                                    <span
                                        class="mono"
                                        style=format!(
                                            "width: 24px; height: 20px; border-radius: 4px; background: {}; color: {}; font-size: 11px; line-height: 20px; font-weight: 500; text-align: center; flex-shrink: 0;",
                                            col, TEXT_ON_ACCENT
                                        )
                                    >
                                        {code}
                                    </span>
                                    <span style=format!(
                                        "font-size: 13px; line-height: 20px; font-weight: {}; color: {}; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                                        if is_active || is_me { "500" } else { "400" },
                                        if p.is_bankrupt { TEXT_MUTED } else { TEXT_PRIMARY }
                                    )>
                                        {p.name.clone()}
                                    </span>
                                    {if is_me {
                                        view! {
                                            <span class="mono" style=format!("font-size: 11px; color: {};", TEXT_MUTED)>
                                                "Bạn"
                                            </span>
                                        }.into_any()
                                    } else {
                                        ().into_any()
                                    }}
                                </div>

                                <div style="display: flex; align-items: center; gap: 8px; flex-shrink: 0;">
                                    {if is_me {
                                        if let Some((ref delta_str, is_pos)) = delta {
                                            let d_col = if is_pos { STATUS_SUCCESS } else { STATUS_DANGER };
                                            view! {
                                                <span class="ui-cash-delta mono" style=format!("font-size: 11px; font-weight: 500; color: {};", d_col)>
                                                    {delta_str.clone()}
                                                </span>
                                            }.into_any()
                                        } else {
                                            ().into_any()
                                        }
                                    } else {
                                        ().into_any()
                                    }}
                                    <span
                                        class="mono"
                                        style=format!(
                                            "font-size: 13px; line-height: 20px; font-weight: 500; color: {};",
                                            if p.is_bankrupt { STATUS_DANGER } else { TEXT_PRIMARY }
                                        )
                                    >
                                        {if p.is_bankrupt { "Phá sản".to_string() } else { p.cash.to_string() }}
                                    </span>
                                </div>
                            </div>
                        }
                    }).collect_view()
                }}
            </div>

            // Compact Dice Status Bar
            {move || {
                let (d1, d2) = display_dice.get();
                let sum = d1 + d2;
                let rolling = is_rolling.get();
                view! {
                    <div style=format!(
                        "height: 36px; padding: 0 12px; border-radius: 8px; background: {}; border: 1px solid {}; display: flex; align-items: center; justify-content: space-between;",
                        BG_SECONDARY, BORDER_DEFAULT
                    )>
                        <span style=format!("font-size: 12px; color: {};", TEXT_MUTED)>
                            {if rolling {
                                "Trạng thái xúc xắc"
                            } else if d1 == d2 {
                                "Xúc xắc đôi"
                            } else {
                                "Kết quả xúc xắc"
                            }}
                        </span>
                        <span class="mono" style=format!("font-size: 13px; font-weight: 500; color: {};", TEXT_PRIMARY)>
                            {if rolling {
                                "Đang lắc".to_string()
                            } else {
                                format!("{} + {} = {} nút", d1, d2, sum)
                            }}
                        </span>
                    </div>
                }
            }}

            // Primary Turn Phase Actions
            <div style="display: flex; flex-direction: column; gap: 8px;">
                {move || {
                    let st = state.get();
                    let my_turn = is_my_turn();
                    let player = me();
                    let moving = is_rolling.get() || is_animating_move.get();

                    if moving {
                        return view! {
                            <div class="phase-action-group" style="display: flex; flex-direction: column; gap: 8px;">
                                <button class="btn" disabled=true>
                                    {move || if is_rolling.get() { "Đang tung xúc xắc" } else { "Đang di chuyển trên bàn cờ" }}
                                </button>
                            </div>
                        }.into_any();
                    }

                    match st.turn_phase {
                        TurnPhase::WaitingForRoll => {
                            let in_jail = player.as_ref().map(|p| p.in_jail).unwrap_or(false);
                            let cards = player.as_ref().map(|p| p.get_out_of_jail_cards).unwrap_or(0);
                            let can_bail = my_turn
                                && player
                                    .as_ref()
                                    .map(|p| p.cash >= Money::from_vnd(500_000))
                                    .unwrap_or(false);

                            view! {
                                <div class="phase-action-group" style="display: flex; flex-direction: column; gap: 8px;">
                                    <button
                                        class="btn"
                                        disabled=move || !my_turn || is_rolling.get()
                                        on:click=move |_| on_action.run(ClientMessage::RollDice)
                                    >
                                        "Lắc xúc xắc"
                                    </button>
                                    {if in_jail {
                                        view! {
                                            <div style="display: flex; gap: 8px;">
                                                <button
                                                    class="btn btn-secondary"
                                                    style="flex: 1;"
                                                    disabled=!can_bail
                                                    on:click=move |_| on_action.run(ClientMessage::PayJailBail)
                                                >
                                                    "Nộp phạt 500.000 đ"
                                                </button>
                                                {if cards > 0 {
                                                    view! {
                                                        <button
                                                            class="btn btn-secondary"
                                                            style="flex: 1;"
                                                            disabled=!my_turn
                                                            on:click=move |_| on_action.run(ClientMessage::UseJailCard)
                                                        >
                                                            "Dùng thẻ ra tù"
                                                        </button>
                                                    }.into_any()
                                                } else {
                                                    ().into_any()
                                                }}
                                            </div>
                                        }.into_any()
                                    } else {
                                        ().into_any()
                                    }}
                                </div>
                            }.into_any()
                        }
                        TurnPhase::TileActionPending => {
                            let auction_enabled = st.config.enable_auction;
                            let decline_label = if auction_enabled {
                                "Bỏ qua · Đấu giá"
                            } else {
                                "Bỏ qua"
                            };
                            let can_buy = if let Some(ref p) = player {
                                let board = get_board();
                                let cost = match &board[p.position as usize].kind {
                                    TileKind::Property(pr) => pr.cost,
                                    TileKind::Station(s) => s.cost,
                                    TileKind::Utility(u) => u.cost,
                                    _ => CoreMoney::ZERO,
                                };
                                st.properties[p.position as usize].owner.is_none()
                                    && cost > CoreMoney::ZERO
                                    && p.cash >= cost
                            } else {
                                false
                            };
                            view! {
                                <div class="phase-action-group" style="display: flex; gap: 8px;">
                                    <button
                                        class="btn"
                                        style="flex: 1;"
                                        disabled=!(my_turn && can_buy)
                                        on:click=move |_| on_action.run(ClientMessage::BuyProperty)
                                    >
                                        "Mua bất động sản"
                                    </button>
                                    <button
                                        class="btn btn-outline"
                                        style="flex: 1;"
                                        disabled=!my_turn
                                        on:click=move |_| on_action.run(ClientMessage::DeclineProperty)
                                    >
                                        {decline_label}
                                    </button>
                                </div>
                            }.into_any()
                        }
                        TurnPhase::Auction => {
                            let cur_bid = st.auction.as_ref().map(|a| a.current_bid).unwrap_or(Money::ZERO);
                            let next_bid = cur_bid + Money::from_vnd(50_000);
                            let (can_bid, can_pass) = match (&st.auction, &player) {
                                (Some(auc), Some(p)) => {
                                    let is_active = auc.active_bidders.contains(&p.id);
                                    let is_leading = auc.highest_bidder == Some(p.id);
                                    (is_active && !is_leading && p.cash >= next_bid, is_active && !is_leading)
                                }
                                _ => (false, false),
                            };
                            view! {
                                <div class="phase-action-group" style="display: flex; gap: 8px;">
                                    <button
                                        class="btn"
                                        style="flex: 1;"
                                        disabled=!can_bid
                                        on:click=move |_| on_action.run(ClientMessage::PlaceBid { amount: next_bid })
                                    >
                                        "Trả giá +50.000 đ"
                                    </button>
                                    <button
                                        class="btn btn-outline"
                                        style="flex: 1;"
                                        disabled=!can_pass
                                        on:click=move |_| on_action.run(ClientMessage::PassAuction)
                                    >
                                        "Bỏ đấu giá"
                                    </button>
                                </div>
                            }.into_any()
                        }
                        TurnPhase::TurnEnded => {
                            view! {
                                <div class="phase-action-group" style="display: flex; flex-direction: column;">
                                    <button
                                        class="btn"
                                        disabled=!my_turn
                                        on:click=move |_| on_action.run(ClientMessage::EndTurn)
                                    >
                                        "Kết thúc lượt"
                                    </button>
                                </div>
                            }.into_any()
                        }
                    }
                }}
            </div>

            // Compact Selected Tile Management Section (non-redundant with center board deed)
            {move || {
                if let Some(t_idx) = selected_tile.get() {
                    let st = state.get();
                    let board = get_board();
                    let tile = board[t_idx as usize].clone();
                    let prop_state = st.properties[t_idx as usize];

                    let (dot_col, cat_name) = match &tile.kind {
                        TileKind::Property(p) => (get_district_color(p.district), get_district_label(p.district)),
                        TileKind::Station(_) => (TILE_STATION, "Bến xe & Nhà ga"),
                        TileKind::Utility(_) => (TILE_UTILITY, "Công trình công cộng"),
                        TileKind::Tax(_) => (TILE_TAX, "Thuế nhà nước"),
                        TileKind::Chance => (TILE_CHANCE, "Ô Cơ hội"),
                        TileKind::CommunityChest => (TILE_CHEST, "Ô Khí vận"),
                        _ => (BORDER_SUBTLE, "Ô chức năng"),
                    };

                    let owner_text = if let Some(owner_id) = prop_state.owner {
                        if let Some((idx, p)) = st.players.iter().enumerate().find(|(_, p)| p.id == owner_id) {
                            format!("{} · {}", PLAYER_AVATARS[idx % 4], p.name)
                        } else {
                            "Đã có chủ".to_string()
                        }
                    } else {
                        "Ngân hàng".to_string()
                    };

                    let entry_cost_text = match &tile.kind {
                        TileKind::Property(p) => {
                            if prop_state.is_mortgaged {
                                "Đang thế chấp".to_string()
                            } else if let Some(owner_id) = prop_state.owner {
                                let rent = if prop_state.house_count > 0 {
                                    p.rent_table[prop_state.house_count as usize]
                                } else if st.owns_full_district(owner_id, p.district) {
                                    CoreMoney(p.rent_table[0].0 * 2)
                                } else {
                                    p.rent_table[0]
                                };
                                format!("Thuê: {}", rent)
                            } else {
                                format!("Giá: {}", p.cost)
                            }
                        }
                        TileKind::Station(s) => {
                            if let Some(owner_id) = prop_state.owner {
                                let cnt = st.stations_owned_by(owner_id);
                                let rent = CoreMoney::from_vnd(250_000 * (1i64 << cnt.saturating_sub(1)));
                                format!("Thuê ({} bến): {}", cnt, rent)
                            } else {
                                format!("Giá: {}", s.cost)
                            }
                        }
                        TileKind::Utility(u) => {
                            if let Some(owner_id) = prop_state.owner {
                                let cnt = st.utilities_owned_by(owner_id);
                                let mult = if cnt >= 2 { 10 } else { 4 };
                                format!("Thuê: {}× xúc xắc", mult)
                            } else {
                                format!("Giá: {}", u.cost)
                            }
                        }
                        TileKind::Tax(t) => format!("Thuế: {}", t.amount),
                        _ => "Không mua bán".to_string(),
                    };

                    let is_buildable = matches!(tile.kind, TileKind::Property(_));
                    let is_mortgageable = matches!(
                        tile.kind,
                        TileKind::Property(_) | TileKind::Station(_) | TileKind::Utility(_)
                    );
                    let my_id = my_player_id.get();
                    let my_turn = is_my_turn();
                    let moving = is_rolling.get() || is_animating_move.get();
                    let is_owned_by_me = my_id.is_some() && prop_state.owner == my_id;
                    let my_cash = my_id.and_then(|id| st.find_player(id)).map(|p| p.cash);
                    let can_manage = !moving && my_turn && is_owned_by_me && st.is_game_started && !st.is_game_over;
                    let non_prop_flags = |m_val: CoreMoney| {
                        let unmortgage_cost = CoreMoney((m_val.0 * 11) / 10);
                        (
                            false,
                            false,
                            !prop_state.is_mortgaged,
                            prop_state.is_mortgaged && my_cash.map(|c| c >= unmortgage_cost).unwrap_or(false),
                        )
                    };
                    let (can_build, can_sell, can_mortgage, can_unmortgage) = if !can_manage {
                        (false, false, false, false)
                    } else {
                        match &tile.kind {
                            TileKind::Property(p) => {
                                let d_tiles = get_district_tiles(p.district);
                                let full_set = my_id
                                    .map(|id| st.owns_full_district(id, p.district))
                                    .unwrap_or(false);
                                let any_mortgaged = d_tiles
                                    .iter()
                                    .any(|&t| st.properties[t as usize].is_mortgaged);
                                let min_h = d_tiles
                                    .iter()
                                    .map(|&t| st.properties[t as usize].house_count)
                                    .min()
                                    .unwrap_or(0);
                                let max_h = d_tiles
                                    .iter()
                                    .map(|&t| st.properties[t as usize].house_count)
                                    .max()
                                    .unwrap_or(0);
                                let h_cnt = prop_state.house_count;
                                let unmortgage_cost = CoreMoney((p.mortgage_value.0 * 11) / 10);
                                (
                                    full_set
                                        && !any_mortgaged
                                        && h_cnt < 5
                                        && h_cnt == min_h
                                        && my_cash.map(|c| c >= p.house_cost).unwrap_or(false),
                                    h_cnt > 0 && h_cnt == max_h,
                                    !prop_state.is_mortgaged && max_h == 0,
                                    prop_state.is_mortgaged
                                        && my_cash.map(|c| c >= unmortgage_cost).unwrap_or(false),
                                )
                            }
                            TileKind::Station(s) => non_prop_flags(s.mortgage_value),
                            TileKind::Utility(u) => non_prop_flags(u.mortgage_value),
                            _ => (false, false, false, false),
                        }
                    };

                    view! {
                        <div style=format!(
                            "padding: 12px; border-radius: 8px; background: {}; border: 1px solid {}; display: flex; flex-direction: column; gap: 8px;",
                            BG_SECONDARY, BORDER_DEFAULT
                        )>
                            <div style="display: flex; align-items: center; justify-content: space-between; gap: 8px;">
                                <div style="display: inline-flex; align-items: center; gap: 8px;">
                                    <span style=format!("width: 8px; height: 8px; border-radius: 2px; background: {}; flex-shrink: 0;", dot_col)></span>
                                    <span class="mono" style=format!("font-size: 12px; line-height: 16px; color: {};", TEXT_MUTED)>
                                        {format!("Ô #{:02} · {}", t_idx, cat_name)}
                                    </span>
                                </div>
                                <span class="mono" style=format!("font-size: 12px; line-height: 16px; color: {};", TEXT_SECONDARY)>
                                    {entry_cost_text}
                                </span>
                            </div>

                            <div style="display: flex; align-items: center; justify-content: space-between; gap: 8px;">
                                <span style=format!("font-size: 14px; line-height: 20px; font-weight: 500; color: {};", TEXT_PRIMARY)>
                                    {tile.name}
                                </span>
                                <span style=format!("font-size: 12px; line-height: 16px; color: {};", TEXT_MUTED)>
                                    {owner_text}
                                </span>
                            </div>

                            {if is_mortgageable {
                                view! {
                                    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 8px; margin-top: 4px;">
                                        {if is_buildable {
                                            view! {
                                                <button
                                                    class="btn btn-outline"
                                                    style="min-height: 36px; font-size: 13px; padding: 4px 12px;"
                                                    disabled=!can_build
                                                    on:click=move |_| on_action.run(ClientMessage::BuildHouse { tile_index: t_idx })
                                                >
                                                    "Xây nhà"
                                                </button>
                                                <button
                                                    class="btn btn-outline"
                                                    style="min-height: 36px; font-size: 13px; padding: 4px 12px;"
                                                    disabled=!can_sell
                                                    on:click=move |_| on_action.run(ClientMessage::SellHouse { tile_index: t_idx })
                                                >
                                                    "Bán nhà"
                                                </button>
                                            }.into_any()
                                        } else {
                                            ().into_any()
                                        }}
                                        <button
                                            class="btn btn-outline"
                                            style="min-height: 36px; font-size: 13px; padding: 4px 12px;"
                                            disabled=!can_mortgage
                                            on:click=move |_| on_action.run(ClientMessage::MortgageProperty { tile_index: t_idx })
                                        >
                                            "Thế chấp"
                                        </button>
                                        <button
                                            class="btn btn-outline"
                                            style="min-height: 36px; font-size: 13px; padding: 4px 12px;"
                                            disabled=!can_unmortgage
                                            on:click=move |_| on_action.run(ClientMessage::UnmortgageProperty { tile_index: t_idx })
                                        >
                                            "Chuộc lại"
                                        </button>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <span style=format!("color: {}; font-size: 12px; line-height: 16px;", TEXT_MUTED)>
                                        "Chọn một ô bất động sản trên bàn cờ để xây nhà hoặc thế chấp"
                                    </span>
                                }.into_any()
                            }}
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div style=format!(
                            "padding: 12px; border-radius: 8px; background: {}; color: {}; font-size: 13px; line-height: 20px;",
                            BG_SECONDARY, TEXT_MUTED
                        )>
                            "Chọn một ô đất trên bàn cờ để tra cứu chủ sở hữu và biểu phí thuê"
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

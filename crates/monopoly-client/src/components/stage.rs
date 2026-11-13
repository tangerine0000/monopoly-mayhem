//! "Sân khấu" giữa bàn cờ: mọi quyết định hiện ở nơi mắt người chơi đang nhìn.
//! Mỗi thẻ trả lời đúng một câu hỏi; chi tiết (bảng tiền thuê) để sau một link.

use crate::palette::{
    fmt_short, get_district_label, player_color, player_vehicle_svg, tile_color, tile_group_label,
    tile_short_name, ICON_BARS, ICON_CLOSE, ICON_INFO, ICON_STAR,
};
use leptos::*;
use monopoly_core::board::{get_board, get_district_tiles, District, Tile, TileKind};
use monopoly_core::money::Money;
use monopoly_core::state::{GameState, PropertyState, TradeOffer, TurnPhase, JAIL_BAIL_VND};
use monopoly_proto::{ClientMessage, PlayerId};

fn seat_of(s: &GameState, id: PlayerId) -> Option<usize> {
    s.players.iter().position(|p| p.id == id)
}

fn tile_cost(tile: &Tile) -> Option<Money> {
    match &tile.kind {
        TileKind::Property(p) => Some(p.cost),
        TileKind::Station(st) => Some(st.cost),
        TileKind::Utility(u) => Some(u.cost),
        _ => None,
    }
}

fn mortgage_value(tile: &Tile) -> Option<Money> {
    match &tile.kind {
        TileKind::Property(p) => Some(p.mortgage_value),
        TileKind::Station(st) => Some(st.mortgage_value),
        TileKind::Utility(u) => Some(u.mortgage_value),
        _ => None,
    }
}

fn station_rent(count: usize) -> Money {
    Money::from_vnd(250_000 * (1i64 << count.saturating_sub(1).min(3)))
}

/// Tiền thuê hiện tại của một ô đã có chủ (dạng chữ ngắn).
fn current_rent_text(s: &GameState, tile: &Tile, prop: PropertyState) -> Option<String> {
    let owner = prop.owner?;
    if prop.is_mortgaged {
        return Some("Đang thế chấp · 0 đ".to_string());
    }
    Some(match &tile.kind {
        TileKind::Property(p) => {
            let rent = if prop.house_count > 0 {
                p.rent_table[prop.house_count as usize]
            } else if s.owns_full_district(owner, p.district) {
                Money(p.rent_table[0].0 * 2)
            } else {
                p.rent_table[0]
            };
            fmt_short(rent)
        }
        TileKind::Station(_) => fmt_short(station_rent(s.stations_owned_by(owner))),
        TileKind::Utility(_) => {
            let mult = if s.utilities_owned_by(owner) >= 2 { 10 } else { 4 };
            format!("{mult} × xúc xắc")
        }
        _ => return None,
    })
}

/// Kiểm tra xem sau khi nhận `gained` (và mất `lost`), `player` có hoàn thành một quận mới không.
fn completed_district_after_trade(
    s: &GameState,
    player: PlayerId,
    gained: &[u8],
    lost: &[u8],
) -> Option<District> {
    let board = get_board();
    for &t in gained {
        if let TileKind::Property(p) = &board[t as usize].kind {
            if s.owns_full_district(player, p.district) {
                continue;
            }
            let d_tiles = get_district_tiles(p.district);
            let will_own_all = d_tiles.iter().all(|&dt| {
                !lost.contains(&dt)
                    && (gained.contains(&dt) || s.properties[dt as usize].owner == Some(player))
            });
            if will_own_all {
                return Some(p.district);
            }
        }
    }
    None
}

fn trade_effect_line(s: &GameState, offer: &TradeOffer, viewer_is_receiver: bool) -> Option<String> {
    let from_name = s.find_player(offer.from).map(|p| p.name.as_str()).unwrap_or("Đối thủ");
    let to_name = s.find_player(offer.to).map(|p| p.name.as_str()).unwrap_or("Đối thủ");

    if let Some(d) = completed_district_after_trade(s, offer.to, &offer.give_tiles, &offer.take_tiles) {
        let who = if viewer_is_receiver { "bạn" } else { to_name };
        return Some(format!("Sau giao dịch, {who} đủ bộ {}", get_district_label(d)));
    }
    if let Some(d) = completed_district_after_trade(s, offer.from, &offer.take_tiles, &offer.give_tiles) {
        let who = if viewer_is_receiver { from_name } else { "bạn" };
        return Some(format!("Sau giao dịch, {who} đủ bộ {}", get_district_label(d)));
    }
    None
}

fn tradable_tiles_of(s: &GameState, owner: PlayerId) -> Vec<(u8, String, &'static str)> {
    let board = get_board();
    board
        .iter()
        .filter(|t| s.properties[t.index as usize].owner == Some(owner))
        .filter(|t| match &t.kind {
            TileKind::Property(p) => get_district_tiles(p.district)
                .iter()
                .all(|&d| s.properties[d as usize].house_count == 0),
            TileKind::Station(_) | TileKind::Utility(_) => true,
            _ => false,
        })
        .map(|t| (t.index, tile_short_name(t), tile_color(t)))
        .collect()
}

fn pip_cells(v: u8) -> &'static [(u8, u8)] {
    match v {
        1 => &[(2, 2)],
        2 => &[(1, 1), (3, 3)],
        3 => &[(1, 1), (2, 2), (3, 3)],
        4 => &[(1, 1), (1, 3), (3, 1), (3, 3)],
        5 => &[(1, 1), (1, 3), (2, 2), (3, 1), (3, 3)],
        _ => &[(1, 1), (2, 1), (3, 1), (1, 3), (2, 3), (3, 3)],
    }
}

fn die_view(value: u8, class: String) -> View {
    let red = if value == 1 || value == 4 { " red" } else { "" };
    view! {
        <div class=format!("die{red} {class}")>
            {pip_cells(value).iter().map(|(r, c)| view! { <i style=format!("grid-area: {r} / {c};")></i> }).collect_view()}
        </div>
    }
    .into_view()
}

#[component]
fn RentTable(tile: Tile, prop: PropertyState, state: ReadSignal<GameState>) -> impl IntoView {
    let rows: Vec<(String, String, bool)> = state.with(|s| match &tile.kind {
        TileKind::Property(p) => {
            let full = prop.owner.map(|o| s.owns_full_district(o, p.district)).unwrap_or(false);
            let h = prop.house_count;
            let owned = prop.owner.is_some();
            let mut v = vec![
                ("Đất trống".to_string(), fmt_short(p.rent_table[0]), owned && h == 0 && !full),
                ("Đủ bộ quận (×2)".to_string(), fmt_short(Money(p.rent_table[0].0 * 2)), owned && h == 0 && full),
            ];
            for n in 1..=4u8 {
                v.push((format!("{n} nhà"), fmt_short(p.rent_table[n as usize]), h == n));
            }
            v.push(("Khách sạn".to_string(), fmt_short(p.rent_table[5]), h == 5));
            v.push((
                "Xây mỗi nhà · thế chấp".to_string(),
                format!("{} · {}", fmt_short(p.house_cost), fmt_short(p.mortgage_value)),
                false,
            ));
            v
        }
        TileKind::Station(_) => {
            let cnt = prop.owner.map(|o| s.stations_owned_by(o)).unwrap_or(0);
            (1..=4usize)
                .map(|n| (format!("Có {n} bến"), fmt_short(station_rent(n)), cnt == n))
                .collect()
        }
        TileKind::Utility(_) => {
            let cnt = prop.owner.map(|o| s.utilities_owned_by(o)).unwrap_or(0);
            vec![
                ("Có 1 công trình".to_string(), "4 × xúc xắc".to_string(), cnt == 1),
                ("Có 2 công trình".to_string(), "10 × xúc xắc".to_string(), cnt == 2),
            ]
        }
        _ => vec![],
    });
    view! {
        <div class="rent-table">
            {rows.into_iter().map(|(label, val, cur)| view! {
                <div class=if cur { "cur" } else { "" }><span>{label}</span><b>{val}</b></div>
            }).collect_view()}
        </div>
    }
}

#[derive(Clone, Copy, PartialEq)]
enum MainCard {
    None,
    GameOver,
    Auction,
    IncomingTrade,
    Debt,
    Decision,
    ProposeTrade,
    OutgoingTrade,
    Inspect(u8),
}

#[component]
pub fn Stage(
    state: ReadSignal<GameState>,
    my_player_id: ReadSignal<Option<PlayerId>>,
    timer_seconds: ReadSignal<u32>,
    auction_seconds: ReadSignal<u32>,
    is_rolling: ReadSignal<bool>,
    is_animating_move: ReadSignal<bool>,
    display_dice: ReadSignal<(u8, u8)>,
    drawn_card_banner: ReadSignal<Option<(String, String, String)>>,
    on_dismiss_card: Callback<()>,
    selected_tile: ReadSignal<Option<u8>>,
    set_selected_tile: WriteSignal<Option<u8>>,
    trade_open: ReadSignal<bool>,
    set_trade_open: WriteSignal<bool>,
    on_action: Callback<ClientMessage>,
    on_back_to_lobby: Callback<()>,
) -> impl IntoView {
    let (show_rent, set_show_rent) = create_signal(false);

    // State cho form đề nghị giao dịch
    let (trade_to, set_trade_to) = create_signal::<Option<PlayerId>>(None);
    let (give_tiles, set_give_tiles) = create_signal::<Vec<u8>>(Vec::new());
    let (take_tiles, set_take_tiles) = create_signal::<Vec<u8>>(Vec::new());
    let (give_cash_k, set_give_cash_k) = create_signal(0i64); // đơn vị: nghìn đồng
    let (take_cash_k, set_take_cash_k) = create_signal(0i64);

    let moving = move || is_rolling.get() || is_animating_move.get();
    let is_my_turn = move || {
        let me = my_player_id.get();
        me.is_some() && state.with(|s| s.current_player().map(|p| p.id)) == me
    };
    let phase = move || state.with(|s| s.turn_phase);

    let main = create_memo(move |_| {
        let st_started = state.with(|s| s.is_game_started);
        if moving() || !st_started {
            return MainCard::None;
        }
        if state.with(|s| s.is_game_over) {
            return MainCard::GameOver;
        }
        if phase() == TurnPhase::Auction && state.with(|s| s.auction.is_some()) {
            return MainCard::Auction;
        }
        let me = my_player_id.get();
        if state.with(|s| s.trade.as_ref().is_some_and(|t| Some(t.to) == me)) {
            return MainCard::IncomingTrade;
        }
        if phase() == TurnPhase::Debt
            && state.with(|s| s.debt.as_ref().is_some_and(|d| Some(d.debtor) == me))
        {
            return MainCard::Debt;
        }
        if is_my_turn() && phase() == TurnPhase::TileActionPending {
            return MainCard::Decision;
        }
        if trade_open.get()
            && is_my_turn()
            && matches!(phase(), TurnPhase::WaitingForRoll | TurnPhase::TurnEnded)
            && state.with(|s| s.trade.is_none())
        {
            return MainCard::ProposeTrade;
        }
        if state.with(|s| s.trade.as_ref().is_some_and(|t| Some(t.from) == me)) {
            return MainCard::OutgoingTrade;
        }
        match selected_tile.get() {
            Some(i) if drawn_card_banner.with(|c| c.is_none()) => MainCard::Inspect(i),
            _ => MainCard::None,
        }
    });

    create_effect(move |_| {
        main.track();
        set_show_rent.set(false);
    });

    // Khi mở form giao dịch thì reset lựa chọn và chọn sẵn đối thủ đầu tiên
    create_effect(move |_| {
        if trade_open.get() {
            let me = my_player_id.get_untracked();
            let first_opp = state.with_untracked(|s| {
                s.players
                    .iter()
                    .find(|p| !p.is_bankrupt && Some(p.id) != me)
                    .map(|p| p.id)
            });
            set_trade_to.set(first_opp);
            set_give_tiles.set(Vec::new());
            set_take_tiles.set(Vec::new());
            set_give_cash_k.set(0);
            set_take_cash_k.set(0);
        }
    });

    // ---------- Thanh lượt ----------
    let turn_pill = move || {
        let (seat, name, is_me) = state.with(|s| {
            let cur = s.current_player()?;
            Some((seat_of(s, cur.id)?, cur.name.clone(), Some(cur.id) == my_player_id.get()))
        })?;
        if state.with(|s| s.is_game_over || !s.is_game_started) {
            return None;
        }
        let secs = timer_seconds.get();
        let label = if is_me {
            "Lượt của bạn".to_string()
        } else {
            format!("Lượt của {name}")
        };
        Some(view! {
            <div class="turn-pill" role="status">
                <span class="avatar" style=format!("--c: {};", player_color(seat)) inner_html=player_vehicle_svg(seat)></span>
                <span>{label}</span>
                <span class=if secs <= 10 { "t urgent" } else { "t" }>{format!("{}:{:02}", secs / 60, secs % 60)}</span>
            </div>
        })
    };

    // ---------- Xúc xắc ----------
    let dice = move || {
        if !is_rolling.get() && main.get() != MainCard::None {
            return None;
        }
        if !state.with(|s| s.is_game_started) {
            return None;
        }
        let (d1, d2) = display_dice.get();
        let rolling = is_rolling.get();
        let doubles = !rolling && d1 == d2;
        let cls = |side: &str| {
            let mut c = if rolling {
                format!("rolling-{side}")
            } else {
                "settled".to_string()
            };
            if doubles {
                c.push_str(" doubles");
            }
            c
        };
        Some(view! {
            <div class="dice" aria-label=format!("Xúc xắc {d1} và {d2}")>
                {die_view(d1, cls("l"))}
                {die_view(d2, cls("r"))}
            </div>
        })
    };

    // ---------- Vé số Cơ hội / phong bì Khí vận ----------
    let ticket = move || {
        let (deck, title, desc) = drawn_card_banner.get()?;
        let chance = deck.contains("Cơ Hội");
        let stub = if chance {
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6"><circle cx="12" cy="12" r="8"/><path d="M12 7v10M9.5 9.5c0-1 1-1.5 2.5-1.5s2.5.6 2.5 1.6c0 2.4-5 1.2-5 3.8 0 1 1.1 1.6 2.5 1.6s2.5-.5 2.5-1.5"/></svg>"#
        } else {
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"><rect x="5" y="3" width="14" height="18" rx="2"/><circle cx="12" cy="10" r="3"/><path d="M8 16h8"/></svg>"#
        };
        Some(view! {
            <div class="ticket-wrap">
                <div class=if chance { "ticket" } else { "ticket chest" } role="status">
                    <div class="main">
                        <span class="tag">{if chance { "Cơ hội · vé số kiến thiết" } else { "Khí vận · phong bì" }}</span>
                        <h3>{title}</h3>
                        <p>{desc}</p>
                    </div>
                    <div class="stub" inner_html=stub></div>
                </div>
                <button class="btn btn-gold btn-sm ticket-ack" on:click=move |_| on_dismiss_card.call(())>
                    <span>"Đã hiểu"</span>
                    <span class="ticket-bar"></span>
                </button>
            </div>
        })
    };

    // ---------- Thẻ mua đất ----------
    let decision = move || {
        let me_id = my_player_id.get()?;
        let (tile, cash, enable_auction, why) = state.with(|s| {
            let me = s.find_player(me_id)?;
            let tile = get_board()[me.position as usize].clone();
            let why: (String, String, bool) = match &tile.kind {
                TileKind::Property(p) => {
                    let all = get_district_tiles(p.district);
                    let mine = all
                        .iter()
                        .filter(|&&t| s.properties[t as usize].owner == Some(me_id))
                        .count();
                    let label = get_district_label(p.district);
                    if mine + 1 == all.len() {
                        (format!("Đủ bộ {label}"), "Thuê ×2 và được xây nhà".to_string(), true)
                    } else if mine == 0 {
                        (format!("Ô đầu tiên ở {label}"), format!("Gom đủ {} ô để thuê ×2", all.len()), false)
                    } else {
                        (format!("Ô thứ {}/{} ở {label}", mine + 1, all.len()), "Cần đủ bộ để xây nhà".to_string(), false)
                    }
                }
                TileKind::Station(_) => {
                    let n = s.stations_owned_by(me_id) + 1;
                    (format!("Bến thứ {n}/4"), format!("Thu vé {}", fmt_short(station_rent(n))), n == 4)
                }
                TileKind::Utility(_) => {
                    let n = s.utilities_owned_by(me_id) + 1;
                    let mult = if n >= 2 { 10 } else { 4 };
                    (format!("Công trình thứ {n}/2"), format!("Thu {mult} × xúc xắc"), n == 2)
                }
                _ => (String::new(), String::new(), false),
            };
            Some((tile, me.cash, s.config.enable_auction, why))
        })?;
        let cost = tile_cost(&tile)?;
        let affordable = cash >= cost;
        let color = tile_color(&tile);
        let group = tile_group_label(&tile);
        let prop = PropertyState::default();
        let rent_tile = tile.clone();
        let (why_title, why_sub, star) = why;
        Some(view! {
            <div class="sheet" role="dialog" aria-label=format!("Mua {}", tile.name)>
                <div class="sheet-head" style=format!("background: {color};")>
                    <div class="grp">{group}</div>
                    <h2>{tile.name.clone()}</h2>
                </div>
                <div class="sheet-body">
                    <div class="price-row">
                        <b>{cost.to_string()}</b>
                        {affordable.then(|| view! { <span>{format!("còn lại {}", fmt_short(cash - cost))}</span> })}
                    </div>
                    {if affordable {
                        view! {
                            <div class="why">
                                <span class="badge" inner_html=if star { ICON_STAR } else { ICON_INFO }></span>
                                <span>{why_title}<small>{why_sub}</small></span>
                            </div>
                        }.into_view()
                    } else {
                        view! {
                            <div class="why bad">
                                <span class="badge" inner_html=ICON_INFO></span>
                                <span>"Không đủ tiền"<small>{format!("Thiếu {}", fmt_short(cost - cash))}</small></span>
                            </div>
                        }.into_view()
                    }}
                    <div class="actions">
                        <button class="btn btn-primary" disabled=!affordable on:click=move |_| on_action.call(ClientMessage::BuyProperty)>"Mua"</button>
                        <button class="btn btn-ghost" on:click=move |_| on_action.call(ClientMessage::DeclineProperty)>
                            {if enable_auction { "Đấu giá" } else { "Bỏ qua" }}
                        </button>
                    </div>
                    {move || if show_rent.get() {
                        view! { <RentTable tile=rent_tile.clone() prop=prop state=state /> }.into_view()
                    } else {
                        ().into_view()
                    }}
                    <div class="center-link">
                        <button class="linkbtn" on:click=move |_| set_show_rent.update(|v| *v = !*v)>
                            {move || if show_rent.get() { "Ẩn bảng tiền thuê" } else { "Xem bảng tiền thuê" }}
                        </button>
                    </div>
                </div>
            </div>
        })
    };

    // ---------- Đấu giá (kèm vòng đếm 8 giây) ----------
    let auction = move || {
        let me = my_player_id.get();
        let (tile, bid, leader, bidders, my_cash, i_am_in, i_am_leading) = state.with(|s| {
            let a = s.auction.as_ref()?;
            let tile = get_board()[a.tile_index as usize].clone();
            let leader = a
                .highest_bidder
                .and_then(|id| Some((seat_of(s, id)?, s.find_player(id)?.name.clone(), Some(id) == me)));
            let bidders: Vec<(usize, bool)> = s
                .players
                .iter()
                .enumerate()
                .filter(|(_, p)| !p.is_bankrupt)
                .map(|(i, p)| (i, a.active_bidders.contains(&p.id)))
                .collect();
            let my_cash = me.and_then(|id| s.find_player(id)).map(|p| p.cash).unwrap_or(Money::ZERO);
            let i_am_in = me.map(|id| a.active_bidders.contains(&id)).unwrap_or(false);
            let i_am_leading = me.is_some() && a.highest_bidder == me;
            Some((tile, a.current_bid, leader, bidders, my_cash, i_am_in, i_am_leading))
        })?;
        let color = tile_color(&tile);
        let base = tile_cost(&tile).map(fmt_short).unwrap_or_default();
        let bid_small = bid + Money::from_vnd(100_000);
        let bid_big = bid + Money::from_vnd(500_000);
        let auc_secs = auction_seconds.get().min(8);
        let auc_ring_len = 125.7f64;
        let auc_offset = auc_ring_len * (1.0 - (auc_secs as f64 / 8.0).clamp(0.0, 1.0));
        Some(view! {
            <div class="sheet" role="dialog" aria-label=format!("Đấu giá {}", tile.name)>
                <div class="sheet-head auction-head" style=format!("background: {color};")>
                    <div>
                        <div class="grp">{format!("Đấu giá · giá gốc {base}")}</div>
                        <h2>{tile.name.clone()}</h2>
                    </div>
                    <div class="auc-ring num" title="Đồng hồ đặt lại 8 giây mỗi lượt trả giá">
                        <svg viewBox="0 0 46 46" aria-hidden="true">
                            <circle cx="23" cy="23" r="20" fill="none" stroke="rgba(255,255,255,.28)" stroke-width="4"/>
                            <circle cx="23" cy="23" r="20" fill="none" stroke="#fff" stroke-width="4" stroke-linecap="round"
                                stroke-dasharray=auc_ring_len stroke-dashoffset=auc_offset/>
                        </svg>
                        <span>{auc_secs}</span>
                    </div>
                </div>
                <div class="sheet-body">
                    <div class="auction-bid">
                        {if bid == Money::ZERO {
                            view! { <span class="v" style="font-size: 26px;">"Chưa ai trả giá"</span> }.into_view()
                        } else {
                            view! { <span class="v">{bid.to_string()}</span> }.into_view()
                        }}
                        {leader.map(|(seat, name, is_me)| view! {
                            <span class="leader">
                                <span class="avatar sm" style=format!("--c: {};", player_color(seat)) inner_html=player_vehicle_svg(seat)></span>
                                {if is_me { "Bạn đang dẫn".to_string() } else { format!("{name} đang dẫn") }}
                            </span>
                        })}
                    </div>
                    <div class="bidders">
                        {bidders.into_iter().map(|(seat, active)| view! {
                            <span class=if active { "avatar sm" } else { "avatar sm out" } style=format!("--c: {};", player_color(seat)) inner_html=player_vehicle_svg(seat)></span>
                        }).collect_view()}
                    </div>
                    {if i_am_leading {
                        view! { <div class="center-link"><span class="pf-hint">"Bạn đang trả giá cao nhất · chờ người khác"</span></div> }.into_view()
                    } else if i_am_in {
                        view! {
                            <div class="actions three">
                                <button class="btn btn-gold" disabled={my_cash < bid_small} on:click=move |_| on_action.call(ClientMessage::PlaceBid { amount: bid_small })>"+100k"</button>
                                <button class="btn btn-gold" disabled={my_cash < bid_big} on:click=move |_| on_action.call(ClientMessage::PlaceBid { amount: bid_big })>"+500k"</button>
                                <button class="btn btn-ghost" on:click=move |_| on_action.call(ClientMessage::PassAuction)>"Bỏ"</button>
                            </div>
                        }.into_view()
                    } else {
                        view! { <div class="center-link"><span class="pf-hint">"Bạn đã rời phiên đấu giá"</span></div> }.into_view()
                    }}
                </div>
            </div>
        })
    };

    // ---------- Màn Thiếu tiền / Gom tiền trước khi phá sản ----------
    let debt_card = move || {
        let me_id = my_player_id.get()?;
        let (creditor_label, debt_amt, my_cash, sell_opts, mortgage_opts) = state.with(|s| {
            let debt = s.debt.as_ref()?;
            if debt.debtor != me_id {
                return None;
            }
            let me = s.find_player(me_id)?;
            let creditor_label = match debt.creditors.as_slice() {
                [] => "Ngân hàng".to_string(),
                [single] => s.find_player(*single).map(|p| p.name.clone()).unwrap_or_else(|| "Đối thủ".to_string()),
                _ => "tất cả người chơi".to_string(),
            };
            let board = get_board();
            let mut sell_opts: Vec<(u8, String, Money)> = Vec::new();
            let mut mortgage_opts: Vec<(u8, String, Money)> = Vec::new();
            for (idx, prop) in s.properties.iter().enumerate() {
                if prop.owner != Some(me_id) {
                    continue;
                }
                let tile = &board[idx];
                match &tile.kind {
                    TileKind::Property(p) => {
                        let d_tiles = get_district_tiles(p.district);
                        let max_h = d_tiles.iter().map(|&d| s.properties[d as usize].house_count).max().unwrap_or(0);
                        if prop.house_count > 0 && prop.house_count == max_h {
                            sell_opts.push((idx as u8, tile_short_name(tile), Money(p.house_cost.0 / 2)));
                        }
                        if !prop.is_mortgaged && max_h == 0 {
                            mortgage_opts.push((idx as u8, tile_short_name(tile), p.mortgage_value));
                        }
                    }
                    TileKind::Station(st) if !prop.is_mortgaged => {
                        mortgage_opts.push((idx as u8, tile_short_name(tile), st.mortgage_value));
                    }
                    TileKind::Utility(u) if !prop.is_mortgaged => {
                        mortgage_opts.push((idx as u8, tile_short_name(tile), u.mortgage_value));
                    }
                    _ => {}
                }
            }
            Some((creditor_label, debt.amount, me.cash, sell_opts, mortgage_opts))
        })?;
        let enough = my_cash >= debt_amt;
        let pct = if debt_amt.0 > 0 {
            ((my_cash.0 as f64 / debt_amt.0 as f64) * 100.0).clamp(0.0, 100.0)
        } else {
            100.0
        };
        let lack = if enough { Money::ZERO } else { debt_amt - my_cash };
        Some(view! {
            <div class="sheet" role="dialog" aria-label="Gom tiền trả nợ">
                <div class="sheet-head" style="background: var(--son);">
                    <div class="grp">"Khoản nợ đang treo"</div>
                    <h2>{format!("Trả {creditor_label} {}", fmt_short(debt_amt))}</h2>
                </div>
                <div class="sheet-body">
                    <div class="debt-sub">
                        {format!("Bạn đang có {}. Chọn bán nhà hoặc thế chấp để bù.", fmt_short(my_cash))}
                    </div>
                    <div class="meter">
                        <div class="bar"><i style=format!("width: {pct:.1}%;")></i></div>
                        <div class="lg num">
                            <span>{format!("{} / {}", fmt_short(my_cash), fmt_short(debt_amt))}</span>
                            <b class=if enough { "ok" } else { "no" }>
                                {if enough { "Đã đủ để trả".to_string() } else { format!("Còn thiếu {}", fmt_short(lack)) }}
                            </b>
                        </div>
                    </div>
                    {(!enough && (!sell_opts.is_empty() || !mortgage_opts.is_empty())).then(|| view! {
                        <div class="opts">
                            {sell_opts.into_iter().map(|(idx, name, val)| view! {
                                <button class="opt" on:click=move |_| on_action.call(ClientMessage::SellHouse { tile_index: idx })>
                                    <span>{format!("Bán 1 nhà · {name}")}</span>
                                    <b class="num">{format!("+{}", fmt_short(val))}</b>
                                </button>
                            }).collect_view()}
                            {mortgage_opts.into_iter().map(|(idx, name, val)| view! {
                                <button class="opt" on:click=move |_| on_action.call(ClientMessage::MortgageProperty { tile_index: idx })>
                                    <span>{format!("Thế chấp · {name}")}</span>
                                    <b class="num">{format!("+{}", fmt_short(val))}</b>
                                </button>
                            }).collect_view()}
                        </div>
                    })}
                    <div class="action-stack" style="width: 100%;">
                        <button class="btn btn-primary btn-wide" disabled=!enough on:click=move |_| on_action.call(ClientMessage::PayDebt)>
                            {format!("Trả tiền ({})", fmt_short(debt_amt))}
                        </button>
                        <button class="linkbtn" on:click=move |_| on_action.call(ClientMessage::DeclareBankruptcy)>
                            "Tuyên bố phá sản"
                        </button>
                    </div>
                </div>
            </div>
        })
    };

    // ---------- Nhận đề nghị giao dịch ----------
    let incoming_trade = move || {
        let me_id = my_player_id.get()?;
        let (offer, from_seat, from_name, effect_line) = state.with(|s| {
            let t = s.trade.as_ref()?.clone();
            if t.to != me_id {
                return None;
            }
            let seat = seat_of(s, t.from)?;
            let name = s.find_player(t.from)?.name.clone();
            let eff = trade_effect_line(s, &t, true);
            Some((t, seat, name, eff))
        })?;
        let board = get_board();
        let secs = timer_seconds.get();
        Some(view! {
            <div class="sheet trade-sheet" role="dialog" aria-label="Đề nghị giao dịch">
                <div class="trade-head">
                    <span class="avatar sm" style=format!("--c: {};", player_color(from_seat)) inner_html=player_vehicle_svg(from_seat)></span>
                    <span class="t">{format!("{from_name} muốn đổi đất")}</span>
                    <span class="trade-t num">{format!("{}:{:02}", secs / 60, secs % 60)}</span>
                </div>
                <div class="sheet-body">
                    <div class="trade-cols">
                        <div class="trade-side">
                            <span class="sub">"Bạn nhận"</span>
                            {offer.give_tiles.iter().map(|&idx| {
                                let tile = &board[idx as usize];
                                view! { <div class="dm"><i style=format!("background: {};", tile_color(tile))></i>{tile_short_name(tile)}</div> }
                            }).collect_view()}
                            {(offer.give_cash > Money::ZERO).then(|| view! {
                                <div class="dm money num">{format!("+{}", offer.give_cash)}</div>
                            })}
                            {(offer.give_tiles.is_empty() && offer.give_cash == Money::ZERO).then(|| view! {
                                <div class="dm empty-dm">"Không có"</div>
                            })}
                        </div>
                        <span class="trade-arrow" aria-hidden="true">"⇄"</span>
                        <div class="trade-side">
                            <span class="sub">"Bạn đưa"</span>
                            {offer.take_tiles.iter().map(|&idx| {
                                let tile = &board[idx as usize];
                                view! { <div class="dm"><i style=format!("background: {};", tile_color(tile))></i>{tile_short_name(tile)}</div> }
                            }).collect_view()}
                            {(offer.take_cash > Money::ZERO).then(|| view! {
                                <div class="dm money num">{format!("+{}", offer.take_cash)}</div>
                            })}
                            {(offer.take_tiles.is_empty() && offer.take_cash == Money::ZERO).then(|| view! {
                                <div class="dm empty-dm">"Không có"</div>
                            })}
                        </div>
                    </div>
                    {effect_line.map(|line| view! {
                        <div class="why"><span class="badge" inner_html=ICON_STAR></span><span>{line}</span></div>
                    })}
                    <div class="actions">
                        <button class="btn btn-primary" on:click=move |_| on_action.call(ClientMessage::RespondTrade { accept: true })>"Đồng ý"</button>
                        <button class="btn btn-ghost" on:click=move |_| on_action.call(ClientMessage::RespondTrade { accept: false })>"Từ chối"</button>
                    </div>
                </div>
            </div>
        })
    };

    // ---------- Soạn đề nghị giao dịch ----------
    let propose_trade = move || {
        let me_id = my_player_id.get()?;
        let (opponents, my_tiles, my_cash, target_id, target_tiles, target_cash, effect_line) = state.with(|s| {
            let opponents: Vec<(PlayerId, usize, String)> = s
                .players
                .iter()
                .enumerate()
                .filter(|(_, p)| !p.is_bankrupt && p.id != me_id)
                .map(|(i, p)| (p.id, i, p.name.clone()))
                .collect();
            let my_tiles = tradable_tiles_of(s, me_id);
            let my_cash = s.find_player(me_id).map(|p| p.cash).unwrap_or(Money::ZERO);
            let target_id = trade_to.get().filter(|id| opponents.iter().any(|o| o.0 == *id)).or_else(|| opponents.first().map(|o| o.0));
            let target_tiles = target_id.map(|id| tradable_tiles_of(s, id)).unwrap_or_default();
            let target_cash = target_id.and_then(|id| s.find_player(id)).map(|p| p.cash).unwrap_or(Money::ZERO);
            let preview_offer = target_id.map(|to| TradeOffer {
                from: me_id,
                to,
                give_tiles: give_tiles.get(),
                give_cash: Money::from_vnd(give_cash_k.get() * 1_000),
                take_tiles: take_tiles.get(),
                take_cash: Money::from_vnd(take_cash_k.get() * 1_000),
            });
            let eff = preview_offer.as_ref().and_then(|o| trade_effect_line(s, o, false));
            (opponents, my_tiles, my_cash, target_id, target_tiles, target_cash, eff)
        });
        let target_id = target_id?;
        let g_cash = Money::from_vnd(give_cash_k.get() * 1_000);
        let t_cash = Money::from_vnd(take_cash_k.get() * 1_000);
        let valid = (!give_tiles.with(|v| v.is_empty()) || !take_tiles.with(|v| v.is_empty()) || g_cash > Money::ZERO || t_cash > Money::ZERO)
            && g_cash <= my_cash
            && t_cash <= target_cash;

        Some(view! {
            <div class="sheet trade-sheet" role="dialog" aria-label="Đề nghị giao dịch">
                <div class="trade-head">
                    <span class="t">"Giao dịch với"</span>
                    <div class="trade-targets">
                        {opponents.into_iter().map(|(id, seat, name)| {
                            let active = id == target_id;
                            view! {
                                <button
                                    class=if active { "target-chip active" } else { "target-chip" }
                                    on:click=move |_| {
                                        set_trade_to.set(Some(id));
                                        set_take_tiles.set(Vec::new());
                                        set_take_cash_k.set(0);
                                    }
                                >
                                    <span class="avatar sm" style=format!("--c: {};", player_color(seat)) inner_html=player_vehicle_svg(seat)></span>
                                    <span>{name}</span>
                                </button>
                            }
                        }).collect_view()}
                    </div>
                    <button class="close dark" aria-label="Đóng" inner_html=ICON_CLOSE on:click=move |_| set_trade_open.set(false)></button>
                </div>
                <div class="sheet-body">
                    <div class="trade-compose-cols">
                        <div class="trade-side">
                            <span class="sub">"Bạn đưa"</span>
                            <div class="tile-chips">
                                {if my_tiles.is_empty() {
                                    view! { <span class="pf-hint">"Không có đất"</span> }.into_view()
                                } else {
                                    my_tiles.into_iter().map(|(idx, name, col)| {
                                        let sel = give_tiles.with(|v| v.contains(&idx));
                                        view! {
                                            <button
                                                class=if sel { "tchip sel" } else { "tchip" }
                                                on:click=move |_| set_give_tiles.update(|v| {
                                                    if let Some(p) = v.iter().position(|&x| x == idx) { v.remove(p); } else { v.push(idx); }
                                                })
                                            >
                                                <i style=format!("background: {col};")></i>{name}
                                            </button>
                                        }
                                    }).collect_view()
                                }}
                            </div>
                            <div class="cash-step">
                                <span class="num">{format!("Tiền: {}", fmt_short(g_cash))}</span>
                                <div class="step-btns">
                                    <button class="sbtn" on:click=move |_| set_give_cash_k.update(|c| *c = (*c - 200).max(0))>"−200k"</button>
                                    <button class="sbtn" on:click=move |_| set_give_cash_k.update(|c| *c = (*c + 200).min(my_cash.as_vnd() / 1_000))>"+200k"</button>
                                    <button class="sbtn" on:click=move |_| set_give_cash_k.update(|c| *c = (*c + 1_000).min(my_cash.as_vnd() / 1_000))>"+1tr"</button>
                                </div>
                            </div>
                        </div>
                        <div class="trade-side">
                            <span class="sub">"Bạn nhận"</span>
                            <div class="tile-chips">
                                {if target_tiles.is_empty() {
                                    view! { <span class="pf-hint">"Không có đất"</span> }.into_view()
                                } else {
                                    target_tiles.into_iter().map(|(idx, name, col)| {
                                        let sel = take_tiles.with(|v| v.contains(&idx));
                                        view! {
                                            <button
                                                class=if sel { "tchip sel" } else { "tchip" }
                                                on:click=move |_| set_take_tiles.update(|v| {
                                                    if let Some(p) = v.iter().position(|&x| x == idx) { v.remove(p); } else { v.push(idx); }
                                                })
                                            >
                                                <i style=format!("background: {col};")></i>{name}
                                            </button>
                                        }
                                    }).collect_view()
                                }}
                            </div>
                            <div class="cash-step">
                                <span class="num">{format!("Tiền: {}", fmt_short(t_cash))}</span>
                                <div class="step-btns">
                                    <button class="sbtn" on:click=move |_| set_take_cash_k.update(|c| *c = (*c - 200).max(0))>"−200k"</button>
                                    <button class="sbtn" on:click=move |_| set_take_cash_k.update(|c| *c = (*c + 200).min(target_cash.as_vnd() / 1_000))>"+200k"</button>
                                    <button class="sbtn" on:click=move |_| set_take_cash_k.update(|c| *c = (*c + 1_000).min(target_cash.as_vnd() / 1_000))>"+1tr"</button>
                                </div>
                            </div>
                        </div>
                    </div>
                    {effect_line.map(|line| view! {
                        <div class="why"><span class="badge" inner_html=ICON_STAR></span><span>{line}</span></div>
                    })}
                    <div class="actions">
                        <button
                            class="btn btn-primary"
                            disabled=!valid
                            on:click=move |_| {
                                on_action.call(ClientMessage::ProposeTrade {
                                    to: target_id,
                                    give_tiles: give_tiles.get_untracked(),
                                    give_cash: Money::from_vnd(give_cash_k.get_untracked() * 1_000),
                                    take_tiles: take_tiles.get_untracked(),
                                    take_cash: Money::from_vnd(take_cash_k.get_untracked() * 1_000),
                                });
                                set_trade_open.set(false);
                            }
                        >"Gửi đề nghị"</button>
                        <button class="btn btn-ghost" on:click=move |_| set_trade_open.set(false)>"Hủy"</button>
                    </div>
                </div>
            </div>
        })
    };

    // ---------- Đang chờ đối thủ trả lời giao dịch ----------
    let outgoing_trade = move || {
        let to_name = state.with(|s| {
            let t = s.trade.as_ref()?;
            s.find_player(t.to).map(|p| p.name.clone())
        })?;
        Some(view! {
            <div class="waiting-card card">
                <span>"Đang chờ "<b>{to_name}</b>" trả lời đề nghị giao dịch…"</span>
                <button class="btn btn-ghost btn-sm" on:click=move |_| on_action.call(ClientMessage::CancelTrade)>"Rút lại"</button>
            </div>
        })
    };

    // ---------- Màn kết thúc ván (bục vinh quang + danh hiệu) ----------
    let game_over = move || {
        let standings = state.with(|s| {
            let mut list: Vec<(usize, String, Money, bool, usize, u8)> = s
                .players
                .iter()
                .enumerate()
                .map(|(seat, p)| {
                    let props = s.properties.iter().filter(|pr| pr.owner == Some(p.id)).count();
                    let houses: u8 = s
                        .properties
                        .iter()
                        .filter(|pr| pr.owner == Some(p.id))
                        .map(|pr| pr.house_count)
                        .sum();
                    (seat, p.name.clone(), s.net_worth(p.id), p.is_bankrupt, props, houses)
                })
                .collect();
            list.sort_by(|a, b| a.3.cmp(&b.3).then_with(|| b.2.cmp(&a.2)));
            list
        });
        let winner_name = standings.first().map(|x| x.1.clone()).unwrap_or_else(|| "Hoàn tất".to_string());
        view! {
            <div class="sheet" role="dialog" aria-label="Kết thúc ván đấu">
                <div class="sheet-head" style="background: var(--ochre); color: var(--ink);">
                    <div class="grp">"Ván đấu kết thúc"</div>
                    <h2>{format!("{winner_name} chiến thắng!")}</h2>
                </div>
                <div class="sheet-body">
                    <div class="podium">
                        {standings.into_iter().enumerate().map(|(rank, (seat, name, worth, bankrupt, props, houses))| {
                            let badge = if rank == 0 {
                                "Quán quân"
                            } else if houses >= 3 {
                                "Vua xây dựng"
                            } else if props >= 4 {
                                "Trùm đất"
                            } else if bankrupt {
                                "Phá sản"
                            } else {
                                "Á quân"
                            };
                            view! {
                                <div class=if rank == 0 { "pod-row first" } else { "pod-row" }>
                                    <span class="rank num">{format!("#{}", rank + 1)}</span>
                                    <span class="avatar sm" style=format!("--c: {};", player_color(seat)) inner_html=player_vehicle_svg(seat)></span>
                                    <div class="pod-info">
                                        <b>{name}</b>
                                        <small>{badge}</small>
                                    </div>
                                    <span class="pod-worth num">{if bankrupt { "0 đ".to_string() } else { fmt_short(worth) }}</span>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                    <button class="btn btn-primary btn-wide" on:click=move |_| on_back_to_lobby.call(())>"Về sảnh chơi lại"</button>
                </div>
            </div>
        }
    };

    // ---------- Tra cứu ô ----------
    let inspect = move |idx: u8| {
        let me = my_player_id.get();
        let tile = get_board()[idx as usize].clone();
        let color = tile_color(&tile);
        let group = tile_group_label(&tile);
        let (prop, owner, rent) = state.with(|s| {
            let prop = s.properties[idx as usize];
            let owner = prop
                .owner
                .and_then(|id| Some((seat_of(s, id)?, s.find_player(id)?.name.clone(), Some(id) == me)));
            (prop, owner, current_rent_text(s, &tile, prop))
        });
        let purchasable = tile_cost(&tile).is_some();
        let summary = match &tile.kind {
            TileKind::Go => Some("Đi qua hoặc dừng lại nhận +2.000.000 đ".to_string()),
            TileKind::Tax(t) => Some(format!("Dừng ở đây nộp {}", t.amount)),
            TileKind::Chance => Some("Dừng ở đây rút một thẻ Cơ hội".to_string()),
            TileKind::CommunityChest => Some("Dừng ở đây rút một thẻ Khí vận".to_string()),
            TileKind::Jail => Some(format!("Chỉ ghé thăm. Bị giam thì nộp {} để ra", Money::from_vnd(JAIL_BAIL_VND))),
            TileKind::FreeParking => Some("Nghỉ chân, không mất gì".to_string()),
            TileKind::GoToJail => Some("Đi thẳng vào tạm giam, không qua Khởi hành".to_string()),
            _ => None,
        };
        let mine = owner.as_ref().map(|o| o.2).unwrap_or(false);
        let is_property = matches!(tile.kind, TileKind::Property(_));
        let mval = mortgage_value(&tile).map(fmt_short).unwrap_or_default();
        let rent_tile = tile.clone();
        let light = matches!(tile.kind, TileKind::Chance);
        let can_manage = mine && is_my_turn();

        view! {
            <div class="sheet" role="dialog" aria-label=tile.name.clone()>
                <div class=if light { "sheet-head light" } else { "sheet-head" } style=format!("background: {color};")>
                    <div class="grp">{group}</div>
                    <h2>{tile.name.clone()}</h2>
                    <button class="close" aria-label="Đóng" inner_html=ICON_CLOSE on:click=move |_| set_selected_tile.set(None)></button>
                </div>
                <div class="sheet-body">
                    {summary.map(|t| view! { <div class="why"><span class="badge" inner_html=ICON_INFO></span><span>{t}</span></div> })}
                    {purchasable.then(|| match owner.clone() {
                        Some((seat, name, is_me)) => {
                            let rent_txt = rent.clone().unwrap_or_default();
                            let sub_text = if is_me {
                                format!("Thuê {rent_txt}")
                            } else if prop.is_mortgaged {
                                "Không thu tiền thuê".to_string()
                            } else {
                                format!("Nếu dừng: trả {name} {rent_txt}")
                            };
                            view! {
                                <div class="owner-line">
                                    <span class="avatar sm" style=format!("--c: {};", player_color(seat)) inner_html=player_vehicle_svg(seat)></span>
                                    <span>{if is_me { "Của bạn".to_string() } else { name }}</span>
                                    <span class="sub">{sub_text}</span>
                                </div>
                            }.into_view()
                        }
                        None => view! {
                            <div class="price-row"><b>{tile_cost(&tile).map(|c| c.to_string()).unwrap_or_default()}</b><span>"Chưa có chủ"</span></div>
                        }.into_view(),
                    })}
                    {prop.is_mortgaged.then(|| view! {
                        <div class="why bad"><span class="badge" inner_html=ICON_BARS></span><span>"Đang thế chấp"<small>"Không thu tiền thuê tới khi chuộc lại"</small></span></div>
                    })}
                    {mine.then(|| view! {
                        <div class="actions two-even">
                            {is_property.then(|| view! {
                                <button class="btn btn-gold btn-sm" disabled={!can_manage} on:click=move |_| on_action.call(ClientMessage::BuildHouse { tile_index: idx })>"Xây nhà"</button>
                                <button class="btn btn-ghost btn-sm" disabled={!can_manage || prop.house_count == 0} on:click=move |_| on_action.call(ClientMessage::SellHouse { tile_index: idx })>"Bán nhà"</button>
                            })}
                            {if prop.is_mortgaged {
                                view! { <button class="btn btn-ghost btn-sm" disabled={!can_manage} on:click=move |_| on_action.call(ClientMessage::UnmortgageProperty { tile_index: idx })>"Chuộc lại"</button> }.into_view()
                            } else {
                                view! { <button class="btn btn-ghost btn-sm" disabled={!can_manage || prop.house_count > 0} on:click=move |_| on_action.call(ClientMessage::MortgageProperty { tile_index: idx })>{format!("Thế chấp +{mval}")}</button> }.into_view()
                            }}
                        </div>
                    })}
                    {purchasable.then(|| view! {
                        {move || if show_rent.get() {
                            view! { <RentTable tile=rent_tile.clone() prop=prop state=state /> }.into_view()
                        } else {
                            ().into_view()
                        }}
                        <div class="center-link">
                            <button class="linkbtn" on:click=move |_| set_show_rent.update(|v| *v = !*v)>
                                {move || if show_rent.get() { "Ẩn bảng tiền thuê" } else { "Xem bảng tiền thuê" }}
                            </button>
                        </div>
                    })}
                </div>
            </div>
        }
        .into_view()
    };

    // ---------- Hàng nút hành động / trạng thái chờ ----------
    let actions = move || {
        if moving() || !state.with(|s| s.is_game_started) || state.with(|s| s.is_game_over) {
            return ().into_view();
        }
        let cur_main = main.get();
        if matches!(
            cur_main,
            MainCard::Auction | MainCard::IncomingTrade | MainCard::Debt | MainCard::ProposeTrade | MainCard::OutgoingTrade
        ) {
            return ().into_view();
        }
        if is_my_turn() {
            let (in_jail, cards, can_trade) = state.with(|s| {
                let p = my_player_id.get().and_then(|id| s.find_player(id));
                let active_opps = s.players.iter().filter(|op| !op.is_bankrupt && Some(op.id) != my_player_id.get()).count();
                let (j, c) = p.map(|me| (me.in_jail, me.get_out_of_jail_cards)).unwrap_or((false, 0));
                (j, c, active_opps > 0 && s.trade.is_none())
            });
            match phase() {
                TurnPhase::WaitingForRoll => view! {
                    <div class="action-stack">
                        <button class="btn btn-primary btn-wide" on:click=move |_| on_action.call(ClientMessage::RollDice)>"Lắc xúc xắc"</button>
                        <div class="row">
                            {in_jail.then(|| view! {
                                <button class="btn btn-ghost btn-sm" on:click=move |_| on_action.call(ClientMessage::PayJailBail)>"Nộp phạt 500k"</button>
                            })}
                            {(in_jail && cards > 0).then(|| view! {
                                <button class="btn btn-ghost btn-sm" on:click=move |_| on_action.call(ClientMessage::UseJailCard)>"Thẻ ra tù"</button>
                            })}
                            {can_trade.then(|| view! {
                                <button class="btn btn-ghost btn-sm" on:click=move |_| set_trade_open.set(true)>"Giao dịch"</button>
                            })}
                        </div>
                    </div>
                }
                .into_view(),
                TurnPhase::TurnEnded => view! {
                    <div class="action-stack">
                        <button class="btn btn-primary btn-wide" on:click=move |_| on_action.call(ClientMessage::EndTurn)>"Kết thúc lượt"</button>
                        {can_trade.then(|| view! {
                            <div class="row">
                                <button class="btn btn-ghost btn-sm" on:click=move |_| set_trade_open.set(true)>"Giao dịch với người khác"</button>
                            </div>
                        })}
                    </div>
                }
                .into_view(),
                _ => ().into_view(),
            }
        } else if phase() == TurnPhase::Debt {
            let debtor_name = state.with(|s| {
                s.debt.as_ref().and_then(|d| s.find_player(d.debtor)).map(|p| p.name.clone()).unwrap_or_default()
            });
            view! { <div class="waiting"><span>"Đang chờ "<b>{debtor_name}</b>" gom tiền trả nợ…"</span></div> }.into_view()
        } else if phase() != TurnPhase::Auction {
            let name = state.with(|s| s.current_player().map(|p| p.name.clone()).unwrap_or_default());
            view! { <div class="waiting"><span>"Đang chờ "<b>{name}</b>"…"</span></div> }.into_view()
        } else {
            ().into_view()
        }
    };

    view! {
        <div class="stage">
            {turn_pill}
            {dice}
            {ticket}
            {move || match main.get() {
                MainCard::GameOver => game_over().into_view(),
                MainCard::Auction => auction().into_view(),
                MainCard::IncomingTrade => incoming_trade().into_view(),
                MainCard::Debt => debt_card().into_view(),
                MainCard::Decision => decision().into_view(),
                MainCard::ProposeTrade => propose_trade().into_view(),
                MainCard::OutgoingTrade => outgoing_trade().into_view(),
                MainCard::Inspect(i) => inspect(i),
                MainCard::None => ().into_view(),
            }}
            {actions}
        </div>
    }
}

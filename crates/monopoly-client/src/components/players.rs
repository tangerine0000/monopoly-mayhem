//! Cột người chơi: tên + tiền là chính. Dòng trạng thái chỉ hiện khi có chuyện đặc biệt.

use crate::palette::{
    fmt_short, get_district_color, player_color, player_vehicle_svg, ALL_DISTRICTS, ICON_BARS,
    ICON_OFFLINE,
};
use leptos::*;
use monopoly_core::board::get_district_tiles;
use monopoly_core::state::GameState;
use monopoly_proto::PlayerId;

const RING_LEN: f64 = 163.4; // 2π × 26
const TURN_LIMIT_SECS: f64 = 120.0;

#[component]
pub fn PlayersRail(
    state: ReadSignal<GameState>,
    my_player_id: ReadSignal<Option<PlayerId>>,
    timer_seconds: ReadSignal<u32>,
    cash_delta_badge: ReadSignal<Option<(String, bool)>>,
) -> impl IntoView {
    view! {
        <aside class="rail players" aria-label="Người chơi">
            <div class="rail-title">"Người chơi"</div>
            <For
                each=move || state.with(|s| (0..s.players.len()).collect::<Vec<usize>>())
                key=|seat| *seat
                children=move |seat| {
                    let player = move || state.with(|s| s.players.get(seat).cloned());
                    let is_me = move || player().map(|p| Some(p.id) == my_player_id.get()).unwrap_or(false);
                    let is_bot = move || player().map(|p| p.is_bot_controlled).unwrap_or(false);
                    let is_active = move || {
                        state.with(|s| s.current_player().map(|p| p.id) == s.players.get(seat).map(|p| p.id))
                            && !state.with(|s| s.is_game_over)
                    };
                    let class = move || {
                        let mut c = "card player-card".to_string();
                        if is_active() {
                            c.push_str(" active");
                        }
                        if let Some(p) = player() {
                            if p.is_bankrupt {
                                c.push_str(" bankrupt");
                            } else if !p.is_connected {
                                c.push_str(" away");
                            }
                        }
                        c
                    };
                    let pips = move || {
                        let Some(id) = player().map(|p| p.id) else { return Vec::new() };
                        state.with(|s| {
                            ALL_DISTRICTS
                                .iter()
                                .map(|&d| {
                                    let tiles = get_district_tiles(d);
                                    let owned = tiles.iter().filter(|&&t| s.properties[t as usize].owner == Some(id)).count();
                                    if owned == 0 {
                                        String::new()
                                    } else {
                                        let op = if owned == tiles.len() { 1.0 } else { 0.45 };
                                        format!("background: {}; opacity: {op};", get_district_color(d))
                                    }
                                })
                                .collect()
                        })
                    };
                    let p_state = move || {
                        let p = player()?;
                        Some(if p.is_bankrupt {
                            ("Phá sản".to_string(), ICON_BARS)
                        } else if !p.is_connected {
                            ("Mất kết nối · bot đi thay".to_string(), ICON_OFFLINE)
                        } else if p.in_jail {
                            (format!("Tạm giam · lượt {}/3", (p.turns_in_jail + 1).min(3)), ICON_BARS)
                        } else {
                            return None;
                        })
                    };
                    view! {
                        <div class=class>
                            {move || if is_active() {
                                let ratio = (timer_seconds.get() as f64 / TURN_LIMIT_SECS).clamp(0.0, 1.0);
                                let stroke = if timer_seconds.get() <= 10 { "var(--son)" } else { "var(--gold)" };
                                view! {
                                    <div class="timer-ring">
                                        <span class="avatar" style=format!("--c: {};", player_color(seat)) inner_html=player_vehicle_svg(seat)></span>
                                        <svg class="ring" viewBox="0 0 56 56" aria-hidden="true">
                                            <circle cx="28" cy="28" r="26" fill="none" stroke="#e8dcc3" stroke-width="4"/>
                                            <circle cx="28" cy="28" r="26" fill="none" stroke=stroke stroke-width="4" stroke-linecap="round"
                                                stroke-dasharray=RING_LEN stroke-dashoffset=RING_LEN * (1.0 - ratio)/>
                                        </svg>
                                    </div>
                                }.into_view()
                            } else {
                                view! {
                                    <span class="avatar" style=format!("--c: {};", player_color(seat)) inner_html=player_vehicle_svg(seat)></span>
                                }.into_view()
                            }}
                            <div class="p-meta" style="min-width: 0;">
                                <div class="nm">
                                    <span>{move || player().map(|p| p.name).unwrap_or_default()}</span>
                                    {move || is_me().then(|| view! { <span class="you-tag">"Bạn"</span> })}
                                    {move || (!is_me() && is_bot()).then(|| view! { <span class="ai-tag">"AI"</span> })}
                                </div>
                                <div class="cash">
                                    <span class="cash-full">{move || player().map(|p| if p.is_bankrupt { "0 đ".to_string() } else { p.cash.to_string() }).unwrap_or_default()}</span>
                                    <span class="cash-short">{move || player().map(|p| if p.is_bankrupt { "0 đ".to_string() } else { fmt_short(p.cash) }).unwrap_or_default()}</span>
                                    {move || if is_me() {
                                        cash_delta_badge.get().map(|(txt, plus)| view! {
                                            <span class=if plus { "cash-delta plus" } else { "cash-delta minus" }>{txt}</span>
                                        })
                                    } else {
                                        None
                                    }}
                                </div>
                            </div>
                            <div class="pips" aria-hidden="true">
                                {move || pips().into_iter().map(|st| view! { <span style=st></span> }).collect_view()}
                            </div>
                            {move || p_state().map(|(txt, icon)| view! {
                                <div class="p-state">
                                    <span inner_html=icon style="display: contents;"></span>
                                    <span class="p-state-txt">{txt}</span>
                                </div>
                            })}
                        </div>
                    }
                }
            />
        </aside>
    }
}

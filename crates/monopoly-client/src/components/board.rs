//! Bàn cờ dạng lưới HTML 11×11 (góc rộng 1.45 lần), nghiêng nhẹ bằng CSS 3D.
//! Chữ dùng đơn vị `cqw` nên co giãn theo kích thước bàn.

use crate::palette::{
    fmt_short, player_color, player_vehicle_svg, tile_color, tile_icon_svg, tile_short_name,
};
use leptos::*;
use monopoly_core::board::{get_board, Tile, TileKind};
use monopoly_core::money::Money;
use monopoly_core::state::GameState;
use monopoly_proto::PlayerId;

/// Tổng số đơn vị theo mỗi chiều: 2 góc × 1.45 + 9 ô × 1.
const UNITS: f64 = 11.9;
const CORNER: f64 = 1.45;

#[derive(Clone, Debug, PartialEq)]
pub struct BoardFloatingPopup {
    pub id: u64,
    pub tile_index: u8,
    pub text: String,
    pub is_positive: bool,
}

/// Vị trí (cột, hàng) 1-based của ô `index` trong lưới 11×11. Ô 0 ở góc dưới phải.
pub fn tile_cell(index: u8) -> (u8, u8) {
    match index {
        0 => (11, 11),
        1..=9 => (11 - index, 11),
        10 => (1, 11),
        11..=19 => (1, 11 - (index - 10)),
        20 => (1, 1),
        21..=29 => (1 + (index - 20), 1),
        30 => (11, 1),
        _ => (11, 1 + (index - 30)),
    }
}

/// (bắt đầu, độ dài) theo % của một trục cho cột/hàng `k` (1..=11).
fn axis_span(k: u8) -> (f64, f64) {
    let u = 100.0 / UNITS;
    match k {
        1 => (0.0, CORNER * u),
        11 => ((CORNER + 9.0) * u, CORNER * u),
        _ => ((CORNER + (k as f64 - 2.0)) * u, u),
    }
}

/// Hình chữ nhật của ô theo % vùng nội dung bàn cờ: (x, y, w, h).
pub fn tile_rect_pct(index: u8) -> (f64, f64, f64, f64) {
    let (c, r) = tile_cell(index);
    let (x, w) = axis_span(c);
    let (y, h) = axis_span(r);
    (x, y, w, h)
}

/// Chỗ đứng của quân cờ: ở mép trong của ô (phía giữa bàn) để không che tên đường.
/// `slot`/`count` dàn các quân đứng chung một ô.
pub fn token_anchor_pct(index: u8, slot: usize, count: usize) -> (f64, f64) {
    let (x, y, w, h) = tile_rect_pct(index);
    let (c, r) = tile_cell(index);
    let spread = (slot as f64 - (count.saturating_sub(1)) as f64 / 2.0) * 3.0;
    if r == 11 {
        (x + w / 2.0 + spread, y + 0.8)
    } else if r == 1 {
        (x + w / 2.0 + spread, y + h + 4.4)
    } else if c == 1 {
        (x + w + 2.8 + spread.abs(), y + h * 0.5 + spread)
    } else {
        (x - 2.8 - spread.abs(), y + h * 0.5 + spread)
    }
}

const CENTER_ART: &str = r##"<svg viewBox="0 0 900 900" preserveAspectRatio="xMidYMid slice" aria-hidden="true">
<defs><pattern id="gb" width="90" height="90" patternUnits="userSpaceOnUse"><rect width="90" height="90" fill="#efe3c9"/><g fill="none" stroke="#dcc8a2" stroke-width="2"><circle cx="0" cy="0" r="26"/><circle cx="90" cy="0" r="26"/><circle cx="0" cy="90" r="26"/><circle cx="90" cy="90" r="26"/><path d="M45 18 L72 45 L45 72 L18 45Z"/><circle cx="45" cy="45" r="8" fill="#e6d3ad"/></g></pattern></defs>
<rect width="900" height="900" fill="url(#gb)"/>
<path d="M-20 140 C 180 120, 260 300, 420 330 S 700 260, 760 430 S 700 760, 920 820" fill="none" stroke="#2b7c8c" stroke-opacity=".26" stroke-width="70" stroke-linecap="round"/>
<path d="M-20 140 C 180 120, 260 300, 420 330 S 700 260, 760 430 S 700 760, 920 820" fill="none" stroke="#fff" stroke-opacity=".35" stroke-width="3" stroke-dasharray="2 14"/>
</svg>"##;

fn hover_hint_for_tile(s: &GameState, tile: &Tile, idx: u8, my_id: Option<PlayerId>) -> String {
    let prop = s.properties[idx as usize];
    if let Some(owner_id) = prop.owner {
        let owner_name = s
            .find_player(owner_id)
            .map(|p| p.name.as_str())
            .unwrap_or("Chủ");
        if Some(owner_id) == my_id {
            return format!("{} · Của bạn", tile.name);
        }
        if prop.is_mortgaged {
            return format!("{} · {} (đang thế chấp)", tile.name, owner_name);
        }
        let rent_txt = match &tile.kind {
            TileKind::Property(p) => {
                let rent = if prop.house_count > 0 {
                    p.rent_table[prop.house_count as usize]
                } else if s.owns_full_district(owner_id, p.district) {
                    Money(p.rent_table[0].0 * 2)
                } else {
                    p.rent_table[0]
                };
                fmt_short(rent)
            }
            TileKind::Station(_) => {
                let cnt = s.stations_owned_by(owner_id);
                let rent = Money::from_vnd(250_000 * (1i64 << cnt.saturating_sub(1).min(3)));
                fmt_short(rent)
            }
            TileKind::Utility(_) => {
                let mult = if s.utilities_owned_by(owner_id) >= 2 { 10 } else { 4 };
                format!("{mult} × xúc xắc")
            }
            _ => return tile.name.clone(),
        };
        return format!("Nếu dừng ở đây: trả {} {}", owner_name, rent_txt);
    }
    match &tile.kind {
        TileKind::Property(p) => format!("{} · Giá {}", tile.name, fmt_short(p.cost)),
        TileKind::Station(st) => format!("{} · Giá {}", tile.name, fmt_short(st.cost)),
        TileKind::Utility(u) => format!("{} · Giá {}", tile.name, fmt_short(u.cost)),
        TileKind::Tax(t) => format!("{} · Nộp {}", tile.name, fmt_short(t.amount)),
        _ => tile.name.clone(),
    }
}

#[component]
pub fn Board(
    state: ReadSignal<GameState>,
    my_player_id: ReadSignal<Option<PlayerId>>,
    selected_tile: ReadSignal<Option<u8>>,
    visual_positions: ReadSignal<[u8; 4]>,
    hopping_seat: ReadSignal<Option<usize>>,
    step_trail: ReadSignal<Vec<u8>>,
    landing_tile_burst: ReadSignal<Option<u8>>,
    recent_acquired_tile: ReadSignal<Option<u8>>,
    recent_house_tile: ReadSignal<Option<u8>>,
    floating_popups: ReadSignal<Vec<BoardFloatingPopup>>,
    on_tile_click: Callback<u8>,
) -> impl IntoView {
    let tiles = get_board();

    // Chỗ đứng của từng quân: (seat, ô, thứ tự trong ô, số quân trong ô)
    let token_slots = create_memo(move |_| {
        let positions = visual_positions.get();
        state.with(|s| {
            let seats: Vec<(usize, u8)> = s
                .players
                .iter()
                .enumerate()
                .take(4)
                .filter(|(_, p)| !p.is_bankrupt)
                .map(|(i, _)| (i, positions[i]))
                .collect();
            seats
                .iter()
                .map(|&(seat, at)| {
                    let same: Vec<usize> = seats.iter().filter(|(_, t)| *t == at).map(|(s, _)| *s).collect();
                    let slot = same.iter().position(|s| *s == seat).unwrap_or(0);
                    (seat, at, slot, same.len())
                })
                .collect::<Vec<_>>()
        })
    });

    let tile_views = tiles
        .into_iter()
        .map(|tile| {
            let idx = tile.index;
            let (c, r) = tile_cell(idx);
            let is_corner = idx % 10 == 0;
            let side = if is_corner {
                "corner"
            } else if r == 1 {
                "top"
            } else if c == 1 {
                "left"
            } else if c == 11 {
                "right"
            } else {
                "bottom"
            };
            let is_property = matches!(tile.kind, TileKind::Property(_));
            let plate_color = if is_property { Some(tile_color(&tile)) } else { None };
            let icon = tile_icon_svg(&tile);
            let short = tile_short_name(&tile);
            let price = match &tile.kind {
                TileKind::Property(p) => fmt_short(p.cost),
                TileKind::Station(s) => fmt_short(s.cost),
                TileKind::Utility(u) => fmt_short(u.cost),
                TileKind::Tax(t) => format!("−{}", fmt_short(t.amount)),
                TileKind::Go => "+2tr".to_string(),
                _ => String::new(),
            };
            let aria = tile.name.clone();
            let tile_for_hint = tile.clone();
            let title_hint = move || {
                let me = my_player_id.get();
                state.with(|s| hover_hint_for_tile(s, &tile_for_hint, idx, me))
            };

            let prop = move || state.with(|s| s.properties.get(idx as usize).copied().unwrap_or_default());
            let owner_seat = move || {
                let owner = prop().owner?;
                state.with(|s| s.players.iter().position(|p| p.id == owner))
            };

            let class = move || {
                let p = prop();
                let mut cls = format!("tile {side}");
                if owner_seat().is_some() {
                    cls.push_str(" owned");
                }
                if p.is_mortgaged {
                    cls.push_str(" mortgaged");
                }
                if landing_tile_burst.get() == Some(idx) {
                    cls.push_str(" landing");
                } else if step_trail.with(|t| t.contains(&idx)) {
                    cls.push_str(" trail");
                }
                if selected_tile.get() == Some(idx) {
                    cls.push_str(" selected");
                }
                cls
            };
            let style = move || {
                let owner = owner_seat().map(player_color).unwrap_or("transparent");
                format!("grid-column: {c}; grid-row: {r}; --owner: {owner};")
            };

            view! {
                <button class=class style=style title=title_hint aria-label=aria on:click=move |_| on_tile_click.call(idx)>
                    {plate_color.map(|col| view! {
                        <div class="plate" style=format!("background: {col};")>
                            {move || {
                                let houses = prop().house_count;
                                let pop = if recent_house_tile.get() == Some(idx) { " pop" } else { "" };
                                if houses == 5 {
                                    view! { <span class=format!("hotel{pop}")></span> }.into_view()
                                } else {
                                    (0..houses).map(|_| view! { <span class=format!("house{pop}")></span> }).collect_view()
                                }
                            }}
                        </div>
                    })}
                    <div class="body">
                        {icon.map(|svg| view! { <span class="icon" inner_html=svg></span> })}
                        <span class="name">{short}</span>
                        {(!price.is_empty()).then(|| view! { <span class="price">{price}</span> })}
                    </div>
                    {move || owner_seat().map(|seat| {
                        let pop = if recent_acquired_tile.get() == Some(idx) { "flag pop" } else { "flag" };
                        view! { <span class=pop>{seat + 1}</span> }
                    })}
                </button>
            }
        })
        .collect_view();

    view! {
        <div class="board" role="group" aria-label="Bàn cờ">
            <div class="board-center" inner_html=CENTER_ART></div>
            {tile_views}

            <div class="token-layer">
                <For
                    each=move || state.with(|s| (0..s.players.len().min(4)).collect::<Vec<usize>>())
                    key=|seat| *seat
                    children=move |seat| {
                        let slot = move || token_slots.with(|v| v.iter().find(|t| t.0 == seat).copied());
                        let class = move || {
                            let mut cls = "token".to_string();
                            if hopping_seat.get() == Some(seat) {
                                cls.push_str(" hopping");
                            }
                            let active = state.with(|s| {
                                s.current_player().map(|p| p.id) == s.players.get(seat).map(|p| p.id)
                            });
                            if active {
                                cls.push_str(" active");
                            }
                            cls
                        };
                        let style = move || match slot() {
                            Some((_, at, n, count)) => {
                                let (x, y) = token_anchor_pct(at, n, count);
                                format!("left: {x:.2}%; top: {y:.2}%; --c: {};", player_color(seat))
                            }
                            None => "display: none;".to_string(),
                        };
                        view! {
                            <div class=class style=style>
                                <div class="disc" inner_html=player_vehicle_svg(seat)></div>
                            </div>
                        }
                    }
                />

                <For
                    each=move || floating_popups.get()
                    key=|p| p.id
                    children=move |popup| {
                        let (x, y, w, h) = tile_rect_pct(popup.tile_index);
                        let cx = (x + w / 2.0).clamp(12.0, 88.0);
                        let cy = (y + h / 2.0).clamp(8.0, 95.0);
                        let cls = if popup.is_positive { "float-pill plus" } else { "float-pill minus" };
                        view! {
                            <div class=cls style=format!("left: {cx:.2}%; top: {cy:.2}%;")>{popup.text}</div>
                        }
                    }
                />
            </div>
        </div>
    }
}

/// Minimap nhỏ cho màn hình mobile: hiện 40 ô, chủ sở hữu, vị trí quân và khung đỏ vùng đang xem.
#[component]
pub fn MiniMap(
    state: ReadSignal<GameState>,
    visual_positions: ReadSignal<[u8; 4]>,
    cam_center_pct: Signal<(f64, f64)>,
    on_jump_tile: Callback<u8>,
) -> impl IntoView {
    let tiles = get_board();
    view! {
        <div class="mini" aria-label="Bản đồ thu nhỏ">
            <svg viewBox="0 0 100 100">
                <rect x="0" y="0" width="100" height="100" fill="#efe3c9" />
                {tiles
                    .into_iter()
                    .map(|t| {
                        let idx = t.index;
                        let (x, y, w, h) = tile_rect_pct(idx);
                        let base_col = tile_color(&t);
                        let fill = move || {
                            state.with(|s| {
                                s.properties
                                    .get(idx as usize)
                                    .and_then(|p| p.owner)
                                    .and_then(|id| s.players.iter().position(|p| p.id == id))
                                    .map(player_color)
                                    .unwrap_or(base_col)
                            })
                        };
                        view! {
                            <rect
                                x=format!("{x:.2}")
                                y=format!("{y:.2}")
                                width=format!("{:.2}", (w - 0.5).max(1.0))
                                height=format!("{:.2}", (h - 0.5).max(1.0))
                                fill=fill
                                fill-opacity="0.72"
                                stroke="#fffaf0"
                                stroke-width="0.4"
                                style="cursor: pointer;"
                                on:click=move |_| on_jump_tile.call(idx)
                            />
                        }
                    })
                    .collect_view()}
                {move || {
                    let pos = visual_positions.get();
                    state.with(|s| {
                        s.players
                            .iter()
                            .enumerate()
                            .take(4)
                            .filter(|(_, p)| !p.is_bankrupt)
                            .map(|(seat, _)| {
                                let (x, y, w, h) = tile_rect_pct(pos[seat]);
                                let cx = x + w / 2.0;
                                let cy = y + h / 2.0;
                                view! {
                                    <circle
                                        cx=format!("{cx:.2}")
                                        cy=format!("{cy:.2}")
                                        r="3.2"
                                        fill=player_color(seat)
                                        stroke="#fff"
                                        stroke-width="1"
                                        style="pointer-events: none;"
                                    />
                                }
                            })
                            .collect_view()
                    })
                }}
                {move || {
                    let (cx, cy) = cam_center_pct.get();
                    let vx = (cx - 24.0).clamp(0.0, 52.0);
                    let vy = (cy - 22.0).clamp(0.0, 56.0);
                    view! {
                        <rect
                            x=format!("{vx:.2}")
                            y=format!("{vy:.2}")
                            width="48"
                            height="44"
                            rx="3"
                            fill="none"
                            stroke="#c8372d"
                            stroke-width="2.4"
                            style="pointer-events: none;"
                        />
                    }
                }}
            </svg>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tile_has_a_unique_perimeter_cell() {
        let mut seen = std::collections::HashSet::new();
        for i in 0..40u8 {
            let (c, r) = tile_cell(i);
            assert!((1..=11).contains(&c) && (1..=11).contains(&r));
            assert!(c == 1 || c == 11 || r == 1 || r == 11, "ô {i} không nằm trên viền");
            assert!(seen.insert((c, r)), "ô {i} trùng vị trí");
        }
    }

    #[test]
    fn tile_rects_cover_the_board_exactly() {
        let (x, _, w, _) = tile_rect_pct(0);
        assert!((x + w - 100.0).abs() < 1e-9);
        let (x, y, _, _) = tile_rect_pct(20);
        assert_eq!((x, y), (0.0, 0.0));
        for i in 0..40u8 {
            let (x, y, w, h) = tile_rect_pct(i);
            assert!(x >= 0.0 && y >= 0.0 && x + w <= 100.0 + 1e-9 && y + h <= 100.0 + 1e-9);
        }
    }

    #[test]
    fn tokens_stand_inside_the_board() {
        for i in 0..40u8 {
            for count in 1..=4 {
                for slot in 0..count {
                    let (x, y) = token_anchor_pct(i, slot, count);
                    assert!((0.0..=100.0).contains(&x) && (0.0..=100.0).contains(&y), "ô {i}: ({x},{y})");
                }
            }
        }
    }
}

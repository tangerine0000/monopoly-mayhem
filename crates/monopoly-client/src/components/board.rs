use crate::palette::{
    get_district_color, get_district_label, ACCENT_PRIMARY, BG_SECONDARY, BOARD_CENTER_BG,
    BOARD_CENTER_BORDER, BOARD_OUTER_BG, BORDER_DEFAULT, BORDER_SUBTLE, PLAYER_AVATARS,
    PLAYER_COLORS, STATUS_DANGER, STATUS_SUCCESS, STATUS_WARNING, SURFACE_HOVER, SURFACE_PANEL,
    TEXT_MUTED, TEXT_ON_ACCENT, TEXT_PRIMARY, TEXT_SECONDARY, TILE_BG, TILE_CHANCE, TILE_CHEST,
    TILE_CORNER_ACCENT, TILE_CORNER_BG, TILE_SELECTED_STROKE, TILE_STATION, TILE_STROKE, TILE_TAX,
    TILE_UTILITY,
};
use leptos::*;
use monopoly_core::board::{get_board, TileKind};
use monopoly_core::money::Money;
use monopoly_core::state::GameState;

pub const BOARD_WIDTH: f64 = 1280.0;
pub const BOARD_HEIGHT: f64 = 960.0;
pub const CORNER_W: f64 = 145.0;
pub const CORNER_H: f64 = 120.0;
pub const HORIZ_TILE_W: f64 = 110.0; // (1280 - 2 * 145) / 9 = 110
pub const VERT_TILE_H: f64 = 80.0;   // (960 - 2 * 120) / 9 = 80
pub const HEADER_THICKNESS: f64 = 20.0;

#[derive(Clone, Debug, PartialEq)]
pub struct BoardFloatingPopup {
    pub id: u64,
    pub tile_index: u8,
    pub text: String,
    pub is_positive: bool,
}

/// Returns `(rx, ry, w, h)` for any tile `0..40` on the `1280x960` board with zero overlap.
pub fn get_tile_rect(index: u8) -> (f64, f64, f64, f64) {
    match index {
        0 => (BOARD_WIDTH - CORNER_W, BOARD_HEIGHT - CORNER_H, CORNER_W, CORNER_H),
        1..=9 => {
            let i = index as f64;
            let rx = (BOARD_WIDTH - CORNER_W) - i * HORIZ_TILE_W;
            (rx, BOARD_HEIGHT - CORNER_H, HORIZ_TILE_W, CORNER_H)
        }
        10 => (0.0, BOARD_HEIGHT - CORNER_H, CORNER_W, CORNER_H),
        11..=19 => {
            let i = (index - 10) as f64;
            let ry = (BOARD_HEIGHT - CORNER_H) - i * VERT_TILE_H;
            (0.0, ry, CORNER_W, VERT_TILE_H)
        }
        20 => (0.0, 0.0, CORNER_W, CORNER_H),
        21..=29 => {
            let i = (index - 20) as f64;
            let rx = CORNER_W + (i - 1.0) * HORIZ_TILE_W;
            (rx, 0.0, HORIZ_TILE_W, CORNER_H)
        }
        30 => (BOARD_WIDTH - CORNER_W, 0.0, CORNER_W, CORNER_H),
        31..=39 => {
            let i = (index - 30) as f64;
            let ry = CORNER_H + (i - 1.0) * VERT_TILE_H;
            (BOARD_WIDTH - CORNER_W, ry, CORNER_W, VERT_TILE_H)
        }
        _ => (0.0, 0.0, CORNER_W, CORNER_H),
    }
}

pub fn get_tile_center(index: u8) -> (f64, f64) {
    let (rx, ry, w, h) = get_tile_rect(index);
    (rx + w / 2.0, ry + h / 2.0)
}

/// Splits a tile name into compact lines so text never overflows or overlaps neighbors.
fn wrap_tile_name(name: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in name.split_whitespace() {
        let candidate_len = if current.is_empty() {
            word.chars().count()
        } else {
            current.chars().count() + 1 + word.chars().count()
        };

        if !current.is_empty() && candidate_len > max_chars {
            lines.push(current);
            current = word.to_string();
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Returns relative `(dx, dy, is_red)` pip positions centered around `(0, 0)` for a `64x64` SVG die face (`1..=6`).
fn svg_die_pips_centered(value: u8) -> Vec<(f64, f64, bool)> {
    let l = -16.0;
    let c = 0.0;
    let r = 16.0;
    match value {
        1 => vec![(c, c, true)],
        2 => vec![(l, l, false), (r, r, false)],
        3 => vec![(l, l, false), (c, c, false), (r, r, false)],
        4 => vec![(l, l, true), (r, l, true), (l, r, true), (r, r, true)],
        5 => vec![
            (l, l, false),
            (r, l, false),
            (c, c, false),
            (l, r, false),
            (r, r, false),
        ],
        _ => vec![
            (l, l, false),
            (r, l, false),
            (l, c, false),
            (r, c, false),
            (l, r, false),
            (r, r, false),
        ],
    }
}

#[component]
pub fn Board(
    state: ReadSignal<GameState>,
    selected_tile: ReadSignal<Option<u8>>,
    is_rolling: ReadSignal<bool>,
    display_dice: ReadSignal<(u8, u8)>,
    visual_positions: ReadSignal<[u8; 4]>,
    hopping_seat: ReadSignal<Option<usize>>,
    step_trail: ReadSignal<Vec<u8>>,
    landing_tile_burst: ReadSignal<Option<u8>>,
    recent_acquired_tile: ReadSignal<Option<u8>>,
    recent_house_tile: ReadSignal<Option<u8>>,
    floating_popups: ReadSignal<Vec<BoardFloatingPopup>>,
    drawn_card_banner: ReadSignal<Option<(String, String, String)>>,
    #[prop(default = None)] on_tile_click: Option<Callback<u8>>,
) -> impl IntoView {
    let board = get_board();

    view! {
        <div class="board-container">
            <svg class="board-svg" viewBox="0 0 1280 960" preserveAspectRatio="xMidYMid meet">
                <rect x="0" y="0" width="1280" height="960" fill=BOARD_OUTER_BG rx="12" />
                <rect
                    x=CORNER_W
                    y=CORNER_H
                    width=BOARD_WIDTH - 2.0 * CORNER_W
                    height=BOARD_HEIGHT - 2.0 * CORNER_H
                    fill=BOARD_CENTER_BG
                    stroke=BOARD_CENTER_BORDER
                    stroke-width="1"
                    rx="8"
                />

                // Center Well Header
                <text
                    x="640"
                    y="196"
                    font-size="28"
                    font-weight="500"
                    fill=TEXT_PRIMARY
                    text-anchor="middle"
                    font-family="'Be Vietnam Pro', sans-serif"
                    letter-spacing="-0.02em"
                >
                    "Cờ tỷ phú Sài Gòn"
                </text>
                <text
                    x="640"
                    y="220"
                    font-size="13"
                    font-weight="400"
                    fill=TEXT_MUTED
                    text-anchor="middle"
                >
                    "Chọn ô bất kỳ trên bàn cờ để tra cứu khế ước và biểu phí thuê"
                </text>

                // Center-board Tactile Ivory SVG Dice Tray
                {move || {
                    let (d1, d2) = display_dice.get();
                    let rolling = is_rolling.get();
                    let is_doubles = !rolling && d1 == d2;

                    let render_svg_die = |cx: f64, cy: f64, val: u8, cls: &'static str| {
                        let pips = svg_die_pips_centered(val);
                        view! {
                            <g transform=format!("translate({}, {})", cx, cy)>
                                <g class=cls>
                                    <rect
                                        x="-26"
                                        y="-24"
                                        width="56"
                                        height="56"
                                        rx="10"
                                        fill="rgba(0,0,0,0.35)"
                                    />
                                    <rect
                                        x="-28"
                                        y="-28"
                                        width="56"
                                        height="56"
                                        rx="10"
                                        fill="#efe9de"
                                        stroke=if is_doubles { STATUS_WARNING } else { "#c7bfb2" }
                                        stroke-width=if is_doubles { "2" } else { "1" }
                                    />
                                    {pips.into_iter().map(|(px, py, is_red)| {
                                        let fill = if is_red { ACCENT_PRIMARY } else { "#1c1a18" };
                                        let radius = if is_red && val == 1 { "7" } else { "4.5" };
                                        view! {
                                            <circle cx=px * 0.85 cy=py * 0.85 r=radius fill=fill />
                                        }
                                    }).collect_view()}
                                </g>
                            </g>
                        }
                    };

                    let left_cls = if rolling { "svg-die-left-rolling" } else { "svg-die-settled" };
                    let right_cls = if rolling { "svg-die-right-rolling" } else { "svg-die-settled" };

                    view! {
                        <g>
                            <rect
                                x="536"
                                y="244"
                                width="208"
                                height="80"
                                rx="12"
                                fill=BG_SECONDARY
                                stroke=BORDER_DEFAULT
                                stroke-width="1"
                            />
                            {render_svg_die(600.0, 284.0, d1, left_cls)}
                            {render_svg_die(680.0, 284.0, d2, right_cls)}
                            <text
                                x="640"
                                y="344"
                                font-size="12"
                                font-weight="500"
                                font-family="'IBM Plex Mono', monospace"
                                fill=if is_doubles { STATUS_WARNING } else { TEXT_SECONDARY }
                                text-anchor="middle"
                            >
                                {if rolling {
                                    "Đang tung xúc xắc".to_string()
                                } else if is_doubles {
                                    format!("Xúc xắc đôi: {} + {} = {} · Đi thêm lượt", d1, d2, d1 + d2)
                                } else {
                                    format!("Kết quả: {} + {} = {}", d1, d2, d1 + d2)
                                }}
                            </text>
                        </g>
                    }
                }}

                // Drawn Card Banner (on landing at Cơ Hội / Khí Vận)
                {move || {
                    if let Some((deck_name, card_title, card_desc)) = drawn_card_banner.get() {
                        let is_chance = deck_name.contains("Cơ Hội");
                        let dot_col = if is_chance { TILE_CHANCE } else { TILE_CHEST };
                        let deck_label = if is_chance { "Thẻ Cơ hội" } else { "Thẻ Khí vận" };
                        view! {
                            <g class="card-banner-group">
                                <rect
                                    x="0"
                                    y="0"
                                    width="616"
                                    height="40"
                                    rx="8"
                                    fill=SURFACE_PANEL
                                    stroke=BORDER_SUBTLE
                                    stroke-width="1"
                                />
                                <circle cx="18" cy="20" r="4" fill=dot_col />
                                <text
                                    x="32"
                                    y="24"
                                    font-size="13"
                                    font-weight="500"
                                    fill=TEXT_PRIMARY
                                >
                                    {format!("{} · {}: {}", deck_label, card_title, card_desc)}
                                </text>
                            </g>
                        }.into_view()
                    } else {
                        view! {}.into_view()
                    }
                }}

                // Center-board Saigon Cadastral Title Deed Card (Khế Ước Địa Ốc)
                {move || {
                    let sel_idx = selected_tile.get()?;
                    let all_tiles = get_board();
                    let tile = all_tiles.get(sel_idx as usize)?.clone();

                    let (badge_color, category_label) = match &tile.kind {
                        TileKind::Property(p) => (get_district_color(p.district), get_district_label(p.district).to_string()),
                        TileKind::Station(_) => (TILE_STATION, "Bến xe & Nhà ga".to_string()),
                        TileKind::Utility(_) => (TILE_UTILITY, "Công trình công cộng".to_string()),
                        TileKind::Tax(_) => (TILE_TAX, "Thuế nhà nước".to_string()),
                        TileKind::Chance => (TILE_CHANCE, "Ô Cơ hội".to_string()),
                        TileKind::CommunityChest => (TILE_CHEST, "Ô Khí vận".to_string()),
                        TileKind::Go => (STATUS_SUCCESS, "Ô Khởi hành".to_string()),
                        TileKind::Jail => (STATUS_WARNING, "Khu tạm giam".to_string()),
                        TileKind::FreeParking => (TILE_CHEST, "Bãi đỗ xe miễn phí".to_string()),
                        TileKind::GoToJail => (STATUS_DANGER, "Lệnh bắt giữ".to_string()),
                    };

                    let tile_for_owner = tile.clone();
                    let tile_for_table = tile.clone();

                    Some(view! {
                        <g class="deed-card-group" transform="translate(352, 400)">
                            <rect
                                x="0"
                                y="0"
                                width="576"
                                height="384"
                                rx="12"
                                fill=SURFACE_PANEL
                                stroke=BORDER_DEFAULT
                                stroke-width="1"
                            />
                            // Top District Header Band
                            <rect
                                x="8"
                                y="8"
                                width="560"
                                height="64"
                                rx="8"
                                fill=badge_color
                            />
                            <text
                                x="24"
                                y="30"
                                font-size="12"
                                font-weight="400"
                                font-family="'IBM Plex Mono', monospace"
                                fill=TEXT_ON_ACCENT
                                opacity="0.88"
                            >
                                {format!("Ô #{:02} · {}", tile.index, category_label)}
                            </text>
                            <text
                                x="552"
                                y="30"
                                font-size="12"
                                font-weight="400"
                                font-family="'IBM Plex Mono', monospace"
                                fill=TEXT_ON_ACCENT
                                opacity="0.88"
                                text-anchor="end"
                            >
                                "Khế ước địa ốc"
                            </text>
                            <text
                                x="24"
                                y="56"
                                font-size="20"
                                font-weight="600"
                                fill=TEXT_ON_ACCENT
                            >
                                {tile.name.clone()}
                            </text>

                            // Reactive Owner & Landing Price Box
                            {move || {
                                let st = state.get();
                                let prop_state = st.properties.get(sel_idx as usize).copied().unwrap_or_default();

                                let owner_info = if let Some(owner_id) = prop_state.owner {
                                    if let Some((p_idx, p)) = st.players.iter().enumerate().find(|(_, p)| p.id == owner_id) {
                                        let code = PLAYER_AVATARS[p_idx % 4];
                                        (format!("{} · {}", code, p.name), PLAYER_COLORS[p_idx % 4])
                                    } else {
                                        ("Đã có chủ sở hữu".to_string(), STATUS_WARNING)
                                    }
                                } else {
                                    ("Chưa có chủ · Ngân hàng quản lý".to_string(), TEXT_SECONDARY)
                                };

                                let landing_price_desc = match &tile_for_owner.kind {
                                    TileKind::Property(p) => {
                                        if prop_state.is_mortgaged {
                                            "Đang thế chấp · Miễn tiền thuê".to_string()
                                        } else if let Some(owner_id) = prop_state.owner {
                                            let rent = if prop_state.house_count > 0 {
                                                p.rent_table[prop_state.house_count as usize]
                                            } else if st.owns_full_district(owner_id, p.district) {
                                                Money(p.rent_table[0].0 * 2)
                                            } else {
                                                p.rent_table[0]
                                            };
                                            format!("Tiền thuê áp dụng: {}", rent)
                                        } else {
                                            format!("Giá mua: {} · Thuê cơ bản: {}", p.cost, p.rent_table[0])
                                        }
                                    }
                                    TileKind::Station(s) => {
                                        if prop_state.is_mortgaged {
                                            "Đang thế chấp · Miễn tiền thuê".to_string()
                                        } else if let Some(owner_id) = prop_state.owner {
                                            let count = st.stations_owned_by(owner_id);
                                            let mult = 1i64 << count.saturating_sub(1);
                                            let rent = Money::from_vnd(250_000 * mult);
                                            format!("Tiền vé ({} bến): {}", count, rent)
                                        } else {
                                            format!("Giá mua: {} · Thuê từ 250.000 đ đến 2.000.000 đ", s.cost)
                                        }
                                    }
                                    TileKind::Utility(u) => {
                                        if prop_state.is_mortgaged {
                                            "Đang thế chấp · Miễn tiền thuê".to_string()
                                        } else if let Some(owner_id) = prop_state.owner {
                                            let count = st.utilities_owned_by(owner_id);
                                            let mult = if count >= 2 { 10 } else { 4 };
                                            format!("Phí dịch vụ: {} × tổng xúc xắc × 1.000 đ", mult)
                                        } else {
                                            format!("Giá mua: {} · Hệ số 4× hoặc 10× xúc xắc", u.cost)
                                        }
                                    }
                                    TileKind::Tax(t) => format!("Khoản nộp ngân sách: {}", t.amount),
                                    TileKind::Go => "Đi qua hoặc dừng lại nhận +2.000.000 đ".to_string(),
                                    TileKind::Chance => "Rút 1 thẻ Cơ hội ngẫu nhiên".to_string(),
                                    TileKind::CommunityChest => "Rút 1 thẻ Khí vận ngẫu nhiên".to_string(),
                                    TileKind::Jail => "Thăm tù hoặc nộp 500.000 đ để ra tù".to_string(),
                                    TileKind::FreeParking => "Dừng chân miễn phí".to_string(),
                                    TileKind::GoToJail => "Chuyển thẳng tới Khu tạm giam (#10)".to_string(),
                                };

                                view! {
                                    <g transform="translate(16, 84)">
                                        <rect x="0" y="0" width="544" height="64" rx="8" fill=BG_SECONDARY stroke=BORDER_DEFAULT stroke-width="1" />
                                        <text x="16" y="24" font-size="13" font-weight="400" fill=TEXT_MUTED>
                                            "Chủ sở hữu"
                                        </text>
                                        <text x="528" y="24" font-size="13" font-weight="500" fill=owner_info.1 text-anchor="end">
                                            {owner_info.0}
                                        </text>
                                        <text x="16" y="48" font-size="13" font-weight="400" fill=TEXT_MUTED>
                                            "Mức phí khi dừng"
                                        </text>
                                        <text x="528" y="48" font-size="13" font-weight="500" font-family="'IBM Plex Mono', monospace" fill=TEXT_PRIMARY text-anchor="end">
                                            {landing_price_desc}
                                        </text>
                                    </g>
                                }
                            }}

                            // Property Rent Breakdown Table (aligned columns, tabular figures)
                            {move || {
                                let st = state.get();
                                let prop_state = st.properties.get(sel_idx as usize).copied().unwrap_or_default();

                                match &tile_for_table.kind {
                                    TileKind::Property(p) => {
                                        let h_cnt = prop_state.house_count;
                                        let render_tier_row = |y_off: f64, x_off: f64, w_box: f64, label: &str, val: Money, active: bool| {
                                            view! {
                                                <g transform=format!("translate({}, {})", x_off, y_off)>
                                                    {if active {
                                                        view! {
                                                            <rect x="0" y="0" width=w_box height="28" rx="4" fill=SURFACE_HOVER stroke=BORDER_SUBTLE stroke-width="1" />
                                                        }.into_view()
                                                    } else {
                                                        view! {}.into_view()
                                                    }}
                                                    <text
                                                        x="10"
                                                        y="18"
                                                        font-size="12"
                                                        font-weight=if active { "500" } else { "400" }
                                                        fill=if active { TEXT_PRIMARY } else { TEXT_SECONDARY }
                                                    >
                                                        {label.to_string()}
                                                    </text>
                                                    <text
                                                        x=w_box - 10.0
                                                        y="18"
                                                        font-size="12"
                                                        font-weight="500"
                                                        font-family="'IBM Plex Mono', monospace"
                                                        fill=if active { STATUS_WARNING } else { TEXT_PRIMARY }
                                                        text-anchor="end"
                                                    >
                                                        {val.to_string()}
                                                    </text>
                                                </g>
                                            }
                                        };

                                        view! {
                                            <g transform="translate(16, 160)">
                                                <text x="4" y="14" font-size="12" font-weight="400" font-family="'IBM Plex Mono', monospace" fill=TEXT_MUTED>
                                                    {format!("Giá đất: {}   ·   Xây nhà: {}   ·   Thế chấp: {}", p.cost, p.house_cost, p.mortgage_value)}
                                                </text>

                                                <rect x="0" y="24" width="544" height="184" rx="8" fill=BG_SECONDARY stroke=BORDER_DEFAULT stroke-width="1" />
                                                {render_tier_row(32.0, 8.0, 260.0, "Đất trống", p.rent_table[0], h_cnt == 0)}
                                                {render_tier_row(32.0, 276.0, 260.0, "Đủ bộ quận (×2)", Money(p.rent_table[0].0 * 2), false)}
                                                {render_tier_row(68.0, 8.0, 260.0, "1 căn nhà", p.rent_table[1], h_cnt == 1)}
                                                {render_tier_row(68.0, 276.0, 260.0, "2 căn nhà", p.rent_table[2], h_cnt == 2)}
                                                {render_tier_row(104.0, 8.0, 260.0, "3 căn nhà", p.rent_table[3], h_cnt == 3)}
                                                {render_tier_row(104.0, 276.0, 260.0, "4 căn nhà", p.rent_table[4], h_cnt == 4)}
                                                {render_tier_row(144.0, 8.0, 528.0, "Cấp khách sạn (Tối đa)", p.rent_table[5], h_cnt == 5)}
                                            </g>
                                        }.into_view()
                                    }
                                    TileKind::Station(s) => {
                                        view! {
                                            <g transform="translate(16, 160)">
                                                <text x="4" y="14" font-size="12" font-weight="400" font-family="'IBM Plex Mono', monospace" fill=TEXT_MUTED>
                                                    {format!("Giá mua bến: {}   ·   Giá trị thế chấp: {}", s.cost, s.mortgage_value)}
                                                </text>
                                                <rect x="0" y="24" width="544" height="148" rx="8" fill=BG_SECONDARY stroke=BORDER_DEFAULT stroke-width="1" />
                                                <text x="16" y="52" font-size="13" fill=TEXT_SECONDARY>"Sở hữu 1 bến xe hoặc nhà ga"</text>
                                                <text x="528" y="52" font-size="13" font-family="'IBM Plex Mono', monospace" fill=TEXT_PRIMARY text-anchor="end">"250.000 đ"</text>
                                                <text x="16" y="80" font-size="13" fill=TEXT_SECONDARY>"Sở hữu 2 bến xe hoặc nhà ga"</text>
                                                <text x="528" y="80" font-size="13" font-family="'IBM Plex Mono', monospace" fill=TEXT_PRIMARY text-anchor="end">"500.000 đ"</text>
                                                <text x="16" y="108" font-size="13" fill=TEXT_SECONDARY>"Sở hữu 3 bến xe hoặc nhà ga"</text>
                                                <text x="528" y="108" font-size="13" font-family="'IBM Plex Mono', monospace" fill=TEXT_PRIMARY text-anchor="end">"1.000.000 đ"</text>
                                                <text x="16" y="136" font-size="13" font-weight="500" fill=STATUS_WARNING>"Sở hữu trọn bộ 4 bến xe & nhà ga"</text>
                                                <text x="528" y="136" font-size="13" font-weight="500" font-family="'IBM Plex Mono', monospace" fill=STATUS_WARNING text-anchor="end">"2.000.000 đ"</text>
                                            </g>
                                        }.into_view()
                                    }
                                    TileKind::Utility(u) => {
                                        view! {
                                            <g transform="translate(16, 160)">
                                                <text x="4" y="14" font-size="12" font-weight="400" font-family="'IBM Plex Mono', monospace" fill=TEXT_MUTED>
                                                    {format!("Giá mua công trình: {}   ·   Giá trị thế chấp: {}", u.cost, u.mortgage_value)}
                                                </text>
                                                <rect x="0" y="24" width="544" height="108" rx="8" fill=BG_SECONDARY stroke=BORDER_DEFAULT stroke-width="1" />
                                                <text x="16" y="60" font-size="13" fill=TEXT_SECONDARY>"Sở hữu 1 công trình công cộng"</text>
                                                <text x="528" y="60" font-size="13" font-family="'IBM Plex Mono', monospace" fill=TEXT_PRIMARY text-anchor="end">"4 × xúc xắc × 1.000 đ"</text>
                                                <text x="16" y="96" font-size="13" font-weight="500" fill=STATUS_WARNING>"Sở hữu cả 2 công trình công cộng"</text>
                                                <text x="528" y="96" font-size="13" font-weight="500" font-family="'IBM Plex Mono', monospace" fill=STATUS_WARNING text-anchor="end">"10 × xúc xắc × 1.000 đ"</text>
                                            </g>
                                        }.into_view()
                                    }
                                    _ => {
                                        view! {
                                            <g transform="translate(16, 168)">
                                                <rect x="0" y="0" width="544" height="72" rx="8" fill=BG_SECONDARY stroke=BORDER_DEFAULT stroke-width="1" />
                                                <text x="272" y="40" font-size="13" font-weight="400" fill=TEXT_MUTED text-anchor="middle">
                                                    "Ô chức năng đặc biệt trên bàn cờ Sài Gòn, không áp dụng mua bán"
                                                </text>
                                            </g>
                                        }.into_view()
                                    }
                                }
                            }}
                        </g>
                    })
                }}

                // 40 Board Tiles
                {board.into_iter().map(|tile| {
                    let idx = tile.index;
                    let (rx, ry, w, h) = get_tile_rect(idx);
                    let is_corner = idx == 0 || idx == 10 || idx == 20 || idx == 30;

                    let header_color = match &tile.kind {
                        TileKind::Property(p) => Some(get_district_color(p.district)),
                        TileKind::Station(_) => Some(TILE_STATION),
                        TileKind::Utility(_) => Some(TILE_UTILITY),
                        TileKind::Tax(_) => Some(TILE_TAX),
                        TileKind::Chance => Some(TILE_CHANCE),
                        TileKind::CommunityChest => Some(TILE_CHEST),
                        _ => None,
                    };

                    // Content center `(tx, ty)` shifted away from the inner color band
                    let (tx, ty) = if idx > 0 && idx < 10 {
                        (rx + w / 2.0, ry + HEADER_THICKNESS + (h - HEADER_THICKNESS) / 2.0)
                    } else if idx > 10 && idx < 20 {
                        (rx + (w - HEADER_THICKNESS) / 2.0, ry + h / 2.0)
                    } else if idx > 20 && idx < 30 {
                        (rx + w / 2.0, ry + (h - HEADER_THICKNESS) / 2.0)
                    } else if idx > 30 && idx < 40 {
                        (rx + HEADER_THICKNESS + (w - HEADER_THICKNESS) / 2.0, ry + h / 2.0)
                    } else {
                        (rx + w / 2.0, ry + h / 2.0)
                    };

                    let price_text = match &tile.kind {
                        TileKind::Property(p) => p.cost.to_string(),
                        TileKind::Station(s) => s.cost.to_string(),
                        TileKind::Utility(u) => u.cost.to_string(),
                        TileKind::Tax(t) => t.amount.to_string(),
                        _ => String::new(),
                    };

                    let max_chars = if (idx > 10 && idx < 20) || idx > 30 { 14 } else { 13 };
                    let name_lines = wrap_tile_name(&tile.name, max_chars);
                    let line_count = name_lines.len();
                    let line_height = 13.0;
                    let start_y = if price_text.is_empty() {
                        ty - ((line_count.saturating_sub(1) as f64) * line_height) / 2.0 + 3.0
                    } else {
                        ty - ((line_count as f64) * line_height) / 2.0 - 2.0
                    };
                    let price_y = start_y + (line_count as f64) * line_height + 4.0;
                    let status_y = price_y + 13.0;

                    let prop_owner = move || {
                        state.get().properties.get(idx as usize).and_then(|p| p.owner)
                    };

                    let house_count = move || {
                        state.get().properties.get(idx as usize).map(|p| p.house_count).unwrap_or(0)
                    };

                    let is_mortgaged = move || {
                        state.get().properties.get(idx as usize).map(|p| p.is_mortgaged).unwrap_or(false)
                    };

                    let tile_fill = if is_corner { TILE_CORNER_BG } else { TILE_BG };
                    let is_selected = move || selected_tile.get() == Some(idx);
                    let is_stepped_on = move || {
                        if let Some(seat) = hopping_seat.get() {
                            visual_positions.get()[seat % 4] == idx
                        } else {
                            false
                        }
                    };
                    let is_in_trail = move || step_trail.get().contains(&idx);

                    view! {
                        <g class="tile-rect" on:click=move |_| {
                            if let Some(ref cb) = on_tile_click {
                                cb.call(idx);
                            }
                        }>
                            <rect
                                class="tile-base-rect"
                                x=rx
                                y=ry
                                width=w
                                height=h
                                fill=move || {
                                    if is_stepped_on() {
                                        "#2e2b26"
                                    } else if is_in_trail() {
                                        "#25231f"
                                    } else {
                                        tile_fill
                                    }
                                }
                                stroke=move || {
                                    if is_selected() || is_stepped_on() {
                                        TILE_SELECTED_STROKE
                                    } else if is_in_trail() {
                                        BORDER_SUBTLE
                                    } else {
                                        TILE_STROKE
                                    }
                                }
                                stroke-width=move || {
                                    if is_selected() || is_stepped_on() {
                                        "2"
                                    } else {
                                        "1"
                                    }
                                }
                            />

                            {if let Some(h_col) = header_color {
                                let (hx, hy, hw, hh) = if idx < 10 {
                                    (rx, ry, w, HEADER_THICKNESS)
                                } else if idx < 20 {
                                    (rx + w - HEADER_THICKNESS, ry, HEADER_THICKNESS, h)
                                } else if idx < 30 {
                                    (rx, ry + h - HEADER_THICKNESS, w, HEADER_THICKNESS)
                                } else {
                                    (rx, ry, HEADER_THICKNESS, h)
                                };
                                view! { <rect x=hx y=hy width=hw height=hh fill=h_col /> }.into_view()
                            } else {
                                view! {
                                    <rect
                                        x=rx + 4.0
                                        y=ry + 4.0
                                        width=w - 8.0
                                        height=h - 8.0
                                        fill="none"
                                        stroke=TILE_CORNER_ACCENT
                                        stroke-width="1"
                                        rx="4"
                                    />
                                }.into_view()
                            }}

                            {name_lines.into_iter().enumerate().map(|(line_idx, line)| {
                                let ly = start_y + (line_idx as f64) * line_height;
                                view! {
                                    <text
                                        x=tx
                                        y=ly
                                        font-size=if is_corner { "11.5" } else { "10.5" }
                                        font-weight="500"
                                        fill=TEXT_PRIMARY
                                        text-anchor="middle"
                                    >
                                        {line}
                                    </text>
                                }
                            }).collect_view()}

                            {if !price_text.is_empty() {
                                view! {
                                    <text
                                        x=tx
                                        y=price_y
                                        font-size="9.5"
                                        font-weight="400"
                                        font-family="'IBM Plex Mono', monospace"
                                        fill=TEXT_MUTED
                                        text-anchor="middle"
                                    >
                                        {price_text}
                                    </text>
                                }.into_view()
                            } else {
                                view! {}.into_view()
                            }}

                            // Ownership Seat Monogram Stamp Badge (T1..T4, zero emoji)
                            {move || {
                                if let Some(owner_id) = prop_owner() {
                                    let p_idx = state.get().players.iter().position(|p| p.id == owner_id).unwrap_or(0);
                                    let col = PLAYER_COLORS[p_idx % 4];
                                    let code = PLAYER_AVATARS[p_idx % 4];
                                    let just_acquired = recent_acquired_tile.get() == Some(idx);
                                    let badge_cls = if just_acquired { "owner-badge-pop" } else { "" };
                                    view! {
                                        <g class=badge_cls>
                                            <rect x=rx + 4.0 y=ry + 4.0 width="20" height="16" rx="4" fill=col />
                                            <text
                                                x=rx + 14.0
                                                y=ry + 15.0
                                                font-size="9.5"
                                                font-weight="500"
                                                font-family="'IBM Plex Mono', monospace"
                                                fill=TEXT_ON_ACCENT
                                                text-anchor="middle"
                                            >
                                                {code}
                                            </text>
                                        </g>
                                    }.into_view()
                                } else {
                                    view! {}.into_view()
                                }
                            }}

                            // Architectural Vector Houses / Hotel Silhouette & Mortgage Tag
                            {move || {
                                let h_cnt = house_count();
                                let just_built = recent_house_tile.get() == Some(idx);
                                let build_cls = if just_built { "house-icon-pop" } else { "" };

                                if h_cnt > 0 {
                                    let label_str = if h_cnt == 5 { "Khách sạn".to_string() } else { format!("{} nhà", h_cnt) };
                                    let pill_col = if h_cnt == 5 { STATUS_WARNING } else { STATUS_SUCCESS };
                                    view! {
                                        <g class=build_cls>
                                            <rect
                                                x=tx - 32.0
                                                y=status_y - 10.0
                                                width="64"
                                                height="14"
                                                rx="4"
                                                fill=BG_SECONDARY
                                                stroke=pill_col
                                                stroke-width="1"
                                            />
                                            // Vector gabled house/hotel silhouette on left side of badge
                                            <path
                                                d=format!(
                                                    "M {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} Z",
                                                    tx - 26.0, status_y - 3.0,
                                                    tx - 22.5, status_y - 6.5,
                                                    tx - 19.0, status_y - 3.0,
                                                    tx - 19.0, status_y + 1.0,
                                                    tx - 26.0, status_y + 1.0
                                                )
                                                fill=pill_col
                                            />
                                            <text
                                                x=tx + 4.0
                                                y=status_y
                                                font-size="8.5"
                                                font-weight="500"
                                                font-family="'IBM Plex Mono', monospace"
                                                fill=pill_col
                                                text-anchor="middle"
                                            >
                                                {label_str}
                                            </text>
                                        </g>
                                    }.into_view()
                                } else if is_mortgaged() {
                                    view! {
                                        <text
                                            x=tx
                                            y=status_y
                                            font-size="8.5"
                                            font-weight="500"
                                            font-family="'IBM Plex Mono', monospace"
                                            fill=STATUS_DANGER
                                            text-anchor="middle"
                                        >
                                            "Thế chấp"
                                        </text>
                                    }.into_view()
                                } else {
                                    view! {}.into_view()
                                }
                            }}
                        </g>
                    }
                }).collect_view()}

                // Destination Landing Ripple Ring
                {move || {
                    if let Some(burst_idx) = landing_tile_burst.get() {
                        let (cx, cy) = get_tile_center(burst_idx);
                        view! {
                            <g transform=format!("translate({:.1}, {:.1})", cx, cy)>
                                <circle
                                    class="landing-shockwave-ring"
                                    cx="0"
                                    cy="0"
                                    r="32"
                                    fill="none"
                                    stroke=STATUS_WARNING
                                    stroke-width="2"
                                />
                            </g>
                        }.into_view()
                    } else {
                        view! {}.into_view()
                    }
                }}

                // Architectural Pawn Tokens (Monogram Medallions T1..T4, zero emoji)
                {move || {
                    let st = state.get();
                    let cur_active_id = st.current_player().map(|p| p.id);
                    let positions = visual_positions.get();
                    let hopping = hopping_seat.get();

                    st.players.into_iter().enumerate().map(|(p_idx, player)| {
                        if player.is_bankrupt {
                            return view! { <g></g> }.into_view();
                        }
                        let vis_pos = positions[p_idx % 4];
                        let (cx, cy) = get_tile_center(vis_pos);
                        let offset_x = ((p_idx % 2) as f64 - 0.5) * 28.0;
                        let offset_y = ((p_idx / 2) as f64 - 0.5) * 22.0;
                        let px = cx + offset_x;
                        let py = cy + offset_y;
                        let col = PLAYER_COLORS[p_idx % 4];
                        let code = PLAYER_AVATARS[p_idx % 4];
                        let is_active = cur_active_id == Some(player.id);
                        let is_hopping = hopping == Some(p_idx);

                        let body_class = if is_hopping {
                            "pawn-body hopping"
                        } else {
                            "pawn-body"
                        };

                        let shadow_class = if is_hopping {
                            "pawn-shadow hopping"
                        } else {
                            "pawn-shadow"
                        };

                        view! {
                            <g
                                class="pawn-marker"
                                id=format!("pawn_{}", p_idx)
                                style=format!("transform: translate({:.1}px, {:.1}px);", px, py)
                            >
                                // Crisp outer ring for active turn player
                                {if is_active {
                                    view! {
                                        <circle
                                            cx="0"
                                            cy="-2"
                                            r="17"
                                            fill="none"
                                            stroke=TEXT_PRIMARY
                                            stroke-width="1.5"
                                            opacity="0.85"
                                        />
                                    }.into_view()
                                } else {
                                    view! {}.into_view()
                                }}

                                <ellipse
                                    class=shadow_class
                                    cx="0"
                                    cy="14"
                                    rx="12"
                                    ry="4"
                                    fill="rgba(0, 0, 0, 0.45)"
                                />

                                <g class=body_class>
                                    <circle
                                        cx="0"
                                        cy="-2"
                                        r="13"
                                        fill=col
                                        stroke=SURFACE_PANEL
                                        stroke-width="2"
                                    />
                                    <text
                                        x="0"
                                        y="2"
                                        font-size="10.5"
                                        font-weight="500"
                                        font-family="'IBM Plex Mono', monospace"
                                        fill=TEXT_ON_ACCENT
                                        text-anchor="middle"
                                    >
                                        {code}
                                    </text>
                                </g>
                            </g>
                        }.into_view()
                    }).collect_view()
                }}

                // Floating Cash / Event Popups above Tiles
                {move || {
                    floating_popups.get().into_iter().map(|popup| {
                        let (raw_cx, raw_cy) = get_tile_center(popup.tile_index);
                        let cx = raw_cx.clamp(128.0, BOARD_WIDTH - 128.0);
                        let cy = raw_cy.clamp(76.0, BOARD_HEIGHT - 44.0);
                        let border_col = if popup.is_positive { STATUS_SUCCESS } else { STATUS_DANGER };
                        let pill_w = (popup.text.chars().count() as f64 * 7.5).clamp(112.0, 256.0);

                        view! {
                            <g transform=format!("translate({:.1}, {:.1})", cx, cy - 20.0)>
                                <g class="svg-cash-float">
                                    <rect
                                        x=-pill_w / 2.0
                                        y="-14"
                                        width=pill_w
                                        height="28"
                                        rx="6"
                                        fill=SURFACE_PANEL
                                        stroke=border_col
                                        stroke-width="1.5"
                                    />
                                    <text
                                        x="0"
                                        y="4"
                                        font-size="11.5"
                                        font-weight="500"
                                        font-family="'IBM Plex Mono', monospace"
                                        fill=TEXT_PRIMARY
                                        text-anchor="middle"
                                    >
                                        {popup.text}
                                    </text>
                                </g>
                            </g>
                        }
                    }).collect_view()
                }}
            </svg>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_40_tile_rects_fit_within_board_with_zero_overlap() {
        let rects: Vec<(u8, f64, f64, f64, f64)> = (0..40)
            .map(|idx| {
                let (rx, ry, w, h) = get_tile_rect(idx);
                (idx, rx, ry, w, h)
            })
            .collect();

        for &(idx, rx, ry, w, h) in &rects {
            assert!(rx >= 0.0, "Tile {idx} rx < 0");
            assert!(ry >= 0.0, "Tile {idx} ry < 0");
            assert!(w > 0.0 && h > 0.0, "Tile {idx} invalid size");
            assert!(rx + w <= BOARD_WIDTH + 1e-6, "Tile {idx} exceeds BOARD_WIDTH");
            assert!(ry + h <= BOARD_HEIGHT + 1e-6, "Tile {idx} exceeds BOARD_HEIGHT");

            let (cx, cy) = get_tile_center(idx);
            assert!(cx > rx && cx < rx + w, "Tile {idx} center X outside rect");
            assert!(cy > ry && cy < ry + h, "Tile {idx} center Y outside rect");
        }

        for i in 0..rects.len() {
            for j in (i + 1)..rects.len() {
                let (_, x1, y1, w1, h1) = rects[i];
                let (_, x2, y2, w2, h2) = rects[j];
                let overlap_x = (x1 + w1).min(x2 + w2) - x1.max(x2);
                let overlap_y = (y1 + h1).min(y2 + h2) - y1.max(y2);
                assert!(
                    overlap_x <= 1e-6 || overlap_y <= 1e-6,
                    "Tile {i} and Tile {j} overlap by ({overlap_x}, {overlap_y})"
                );
            }
        }
    }

    #[test]
    fn test_svg_die_pips_centered_counts_and_colors() {
        for val in 1..=6u8 {
            let pips = svg_die_pips_centered(val);
            assert_eq!(pips.len(), val as usize);
            for (px, py, is_red) in pips {
                assert!(px >= -16.0 && px <= 16.0);
                assert!(py >= -16.0 && py <= 16.0);
                assert_eq!(is_red, val == 1 || val == 4);
            }
        }
    }

    #[test]
    fn test_wrap_tile_name_splits_all_board_names_cleanly() {
        for tile in get_board() {
            let lines = wrap_tile_name(&tile.name, 13);
            assert!(!lines.is_empty());
            assert!(lines.len() <= 3, "Tile {} wrapped into too many lines: {:?}", tile.name, lines);
        }
    }
}

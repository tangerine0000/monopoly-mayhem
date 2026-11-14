use crate::palette::{
    get_district_color, get_district_label, BOARD_CENTER_BG, BOARD_CENTER_BORDER, BOARD_OUTER_BG,
    BORDER_DEFAULT, PLAYER_COLORS, STATUS_DANGER, STATUS_SUCCESS, STATUS_WARNING, SURFACE_PANEL,
    TEXT_MUTED, TEXT_ON_ACCENT, TEXT_PRIMARY, TEXT_SECONDARY, TILE_BG, TILE_CHANCE, TILE_CHANCE_BG,
    TILE_CHANCE_BORDER, TILE_CHANCE_INNER_BG, TILE_CHEST, TILE_CHEST_BG, TILE_CHEST_BORDER,
    TILE_CHEST_INNER_BG, TILE_CORNER_ACCENT, TILE_CORNER_BG, TILE_SELECTED_STROKE, TILE_STATION,
    TILE_STROKE, TILE_TAX, TILE_TEXT_MUTED, TILE_TEXT_PRIMARY, TILE_UTILITY,
};
use leptos::prelude::*;
use monopoly_core::board::{get_board, get_district_tiles, TileKind};
use monopoly_core::cards::{get_chance_deck, get_community_chest_deck, CardEffect};
use monopoly_core::money::Money;
use monopoly_core::state::{GameState, TurnPhase};
use monopoly_proto::{ClientMessage, PlayerId};

pub const BOARD_WIDTH: f64 = 1280.0;
pub const BOARD_HEIGHT: f64 = 960.0;
pub const CORNER_W: f64 = 145.0;
pub const CORNER_H: f64 = 120.0;
pub const HORIZ_TILE_W: f64 = 110.0; // (1280 - 2 * 145) / 9 = 110
pub const VERT_TILE_H: f64 = 80.0; // (960 - 2 * 120) / 9 = 80
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
        0 => (
            BOARD_WIDTH - CORNER_W,
            BOARD_HEIGHT - CORNER_H,
            CORNER_W,
            CORNER_H,
        ),
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

/// Returns relative `(dx, dy, is_red)` pip positions centered around `(0, 0)` for a `64x64` die face (`1..=6`).
#[allow(dead_code)]
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

/// Returns the CSS 3D `transform` that rotates a 6-sided die cube so `value` (`1..=6`) faces the viewer with a subtle tabletop tilt.
fn die_3d_settled_transform(value: u8) -> &'static str {
    match value {
        1 => "rotateX(-14deg) rotateY(16deg)",
        2 => "rotateX(-14deg) rotateY(-74deg)",
        3 => "rotateX(-104deg) rotateZ(-14deg)",
        4 => "rotateX(76deg) rotateZ(14deg)",
        5 => "rotateX(-14deg) rotateY(106deg)",
        _ => "rotateX(-14deg) rotateY(196deg)",
    }
}

/// Returns which of the 9 cells in a 3x3 grid have a pip for die face `1..=6`.
fn die_face_pip_mask(value: u8) -> [bool; 9] {
    match value {
        1 => [false, false, false, false, true, false, false, false, false],
        2 => [true, false, false, false, false, false, false, false, true],
        3 => [true, false, false, false, true, false, false, false, true],
        4 => [true, false, true, false, false, false, true, false, true],
        5 => [true, false, true, false, true, false, true, false, true],
        _ => [true, false, true, true, false, true, true, false, true],
    }
}

fn render_3d_die_cube(value: u8, rolling: bool, is_left: bool, is_doubles: bool) -> impl IntoView {
    let cube_class = if rolling {
        if is_left {
            "die-cube rolling-left"
        } else {
            "die-cube rolling-right"
        }
    } else if is_doubles {
        "die-cube doubles"
    } else {
        "die-cube"
    };

    let shadow_class = if rolling {
        "die-shadow rolling"
    } else {
        "die-shadow"
    };

    let cube_style = if rolling {
        String::new()
    } else {
        format!("transform: {};", die_3d_settled_transform(value))
    };

    view! {
        <div class="die-scene">
            <div class=shadow_class></div>
            <div class=cube_class style=cube_style>
                {(1..=6u8).map(|face_val| {
                    let face_cls = format!("die-face die-face-{}", face_val);
                    let mask = die_face_pip_mask(face_val);
                    let is_red_face = face_val == 1 || face_val == 4;
                    view! {
                        <div class=face_cls>
                            {mask.into_iter().map(|has_pip| {
                                if has_pip {
                                    let pip_cls = if face_val == 1 {
                                        "die-pip red big-one"
                                    } else if is_red_face {
                                        "die-pip red"
                                    } else {
                                        "die-pip"
                                    };
                                    view! { <span class=pip_cls></span> }.into_any()
                                } else {
                                    view! { <span></span> }.into_any()
                                }
                            }).collect_view()}
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

/// Renders crisp vector Monopoly illustrations for corners and special tiles without overlapping text.
fn render_tile_illustration(
    idx: u8,
    kind: &TileKind,
    rx: f64,
    ry: f64,
    w: f64,
    h: f64,
) -> impl IntoView {
    match kind {
        TileKind::Go => {
            let cx = rx + w / 2.0;
            let by = ry + h - 24.0;
            view! {
                <g>
                    <path
                        d=format!(
                            "M {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} Z",
                            cx - 38.0, by,
                            cx - 18.0, by - 11.0,
                            cx - 18.0, by - 4.0,
                            cx + 36.0, by - 4.0,
                            cx + 36.0, by + 4.0,
                            cx - 18.0, by + 4.0,
                            cx - 18.0, by + 11.0
                        )
                        fill="#d9383a"
                        stroke="#1c1a18"
                        stroke-width="1.2"
                    />
                    <text
                        x=cx
                        y=ry + 24.0
                        font-size="9.5"
                        font-weight="600"
                        font-family="'Roboto Mono', monospace"
                        fill="#2b7a53"
                        text-anchor="middle"
                    >
                        "+2.000.000 đ"
                    </text>
                </g>
            }.into_any()
        }
        TileKind::Jail => {
            let jx = rx + w - 76.0;
            let jy = ry + 6.0;
            view! {
                <g>
                    <rect x=jx y=jy width="70" height="58" rx="4" fill="#d97736" stroke="#1c1a18" stroke-width="1.5" />
                    <rect x=jx + 14.0 y=jy + 8.0 width="42" height="42" rx="2" fill="#f5f0e4" stroke="#1c1a18" stroke-width="1.5" />
                    <line x1=jx + 24.0 y1=jy + 8.0 x2=jx + 24.0 y2=jy + 50.0 stroke="#1c1a18" stroke-width="2" />
                    <line x1=jx + 35.0 y1=jy + 8.0 x2=jx + 35.0 y2=jy + 50.0 stroke="#1c1a18" stroke-width="2" />
                    <line x1=jx + 46.0 y1=jy + 8.0 x2=jx + 46.0 y2=jy + 50.0 stroke="#1c1a18" stroke-width="2" />
                </g>
            }.into_any()
        }
        TileKind::FreeParking => {
            let cx = rx + w / 2.0;
            let cy = ry + 28.0;
            view! {
                <g>
                    <path
                        d=format!(
                            "M {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} Z",
                            cx - 22.0, cy + 4.0,
                            cx - 16.0, cy - 10.0,
                            cx + 12.0, cy - 10.0,
                            cx + 20.0, cy + 4.0,
                            cx + 24.0, cy + 12.0,
                            cx - 24.0, cy + 12.0
                        )
                        fill="#d9383a"
                        stroke="#1c1a18"
                        stroke-width="1.4"
                    />
                    <circle cx=cx - 13.0 cy=cy + 13.0 r="5" fill="#1c1a18" />
                    <circle cx=cx + 13.0 cy=cy + 13.0 r="5" fill="#1c1a18" />
                </g>
            }.into_any()
        }
        TileKind::GoToJail => {
            let cx = rx + w / 2.0;
            let cy = ry + 26.0;
            view! {
                <g>
                    <circle cx=cx cy=cy r="14" fill="#d9383a" stroke="#1c1a18" stroke-width="1.5" />
                    <rect x=cx - 2.0 y=cy - 8.0 width="4" height="9" rx="1.5" fill="#f5f0e4" />
                    <circle cx=cx cy=cy + 5.5 r="2.2" fill="#f5f0e4" />
                </g>
            }
            .into_any()
        }
        TileKind::Chance => {
            let (ix, iy) = if idx < 10 {
                (rx + w / 2.0, ry + 26.0)
            } else if idx < 20 {
                (rx + w - 24.0, ry + h / 2.0 + 10.0)
            } else if idx < 30 {
                (rx + w / 2.0, ry + h - 14.0)
            } else {
                (rx + 24.0, ry + h / 2.0 + 10.0)
            };
            view! {
                <text
                    x=ix
                    y=iy
                    font-size="26"
                    font-weight="700"
                    font-family="'Barlow Condensed', sans-serif"
                    fill="#d9532f"
                    stroke="#1c1a18"
                    stroke-width="0.8"
                    text-anchor="middle"
                >
                    "?"
                </text>
            }
            .into_any()
        }
        TileKind::CommunityChest => {
            let (ix, iy) = if idx < 10 {
                (rx + w / 2.0, ry + 18.0)
            } else if idx < 20 {
                (rx + w - 24.0, ry + h / 2.0)
            } else if idx < 30 {
                (rx + w / 2.0, ry + h - 22.0)
            } else {
                (rx + 24.0, ry + h / 2.0)
            };
            view! {
                <g transform=format!("translate({:.1}, {:.1})", ix, iy)>
                    <rect x="-13" y="-9" width="26" height="18" rx="3" fill="#2b7a78" stroke="#1c1a18" stroke-width="1.4" />
                    <line x1="-13" y1="-2" x2="13" y2="-2" stroke="#1c1a18" stroke-width="1.3" />
                    <rect x="-3" y="-5" width="6" height="6" rx="1" fill="#f2c94c" stroke="#1c1a18" stroke-width="0.8" />
                </g>
            }.into_any()
        }
        TileKind::Station(_) => {
            let (ix, iy) = if idx < 10 {
                (rx + w / 2.0, ry + 18.0)
            } else if idx < 20 {
                (rx + w - 24.0, ry + h / 2.0)
            } else if idx < 30 {
                (rx + w / 2.0, ry + h - 20.0)
            } else {
                (rx + 24.0, ry + h / 2.0)
            };
            view! {
                <g transform=format!("translate({:.1}, {:.1})", ix, iy)>
                    <rect x="-14" y="-6" width="24" height="12" rx="2" fill="#23201c" />
                    <rect x="-10" y="-12" width="10" height="6" rx="1" fill="#23201c" />
                    <rect x="4" y="-11" width="5" height="5" fill="#23201c" />
                    <circle cx="-8" cy="8" r="3.5" fill="#d9383a" stroke="#1c1a18" stroke-width="1" />
                    <circle cx="4" cy="8" r="3.5" fill="#d9383a" stroke="#1c1a18" stroke-width="1" />
                </g>
            }.into_any()
        }
        TileKind::Utility(_) => {
            let (ix, iy) = if idx < 10 {
                (rx + w / 2.0, ry + 18.0)
            } else if idx < 20 {
                (rx + w - 24.0, ry + h / 2.0)
            } else if idx < 30 {
                (rx + w / 2.0, ry + h - 20.0)
            } else {
                (rx + 24.0, ry + h / 2.0)
            };
            if idx == 12 {
                view! {
                    <g transform=format!("translate({:.1}, {:.1})", ix, iy)>
                        <circle cx="0" cy="-3" r="8" fill="#f2c94c" stroke="#1c1a18" stroke-width="1.4" />
                        <rect x="-4" y="5" width="8" height="5" rx="1" fill="#6e6a63" stroke="#1c1a18" stroke-width="1" />
                    </g>
                }.into_any()
            } else {
                view! {
                    <g transform=format!("translate({:.1}, {:.1})", ix, iy)>
                        <path d="M 0 -10 C 6 -2 9 3 9 6 A 9 9 0 1 1 -9 6 C -9 3 -6 -2 0 -10 Z" fill="#4a90b8" stroke="#1c1a18" stroke-width="1.3" />
                    </g>
                }.into_any()
            }
        }
        TileKind::Tax(_) => {
            let (ix, iy) = if idx < 10 {
                (rx + w / 2.0, ry + 18.0)
            } else if idx < 20 {
                (rx + w - 24.0, ry + h / 2.0)
            } else if idx < 30 {
                (rx + w / 2.0, ry + h - 20.0)
            } else {
                (rx + 24.0, ry + h / 2.0)
            };
            view! {
                <g transform=format!("translate({:.1}, {:.1})", ix, iy)>
                    <polygon points="0,-10 10,0 0,10 -10,0" fill="#d49b35" stroke="#1c1a18" stroke-width="1.4" />
                    <circle cx="0" cy="0" r="3" fill="#f5f0e4" />
                </g>
            }.into_any()
        }
        TileKind::Property(_) => ().into_any(),
    }
}

/// Renders an expressive vector character avatar for seat `0..4`, centered at `(0, 0)` within radius `11.5`.
pub fn render_player_avatar_shapes(seat_idx: usize) -> impl IntoView {
    let seat = seat_idx % 4;
    let bg_col = PLAYER_COLORS[seat];

    let character_overlay = match seat {
        0 => view! {
            // Seat 0: Classic Saigon Tycoon (Top hat + Gold band + White mustache + Navy suit)
            <g>
                <path d="M -7 8.5 Q -6 3.5 0 3.5 Q 6 3.5 7 8.5 Z" fill="#18283a" />
                <polygon points="-2,4.2 2,4.2 0,8" fill="#fbf7ee" />
                <circle cx="0" cy="-0.3" r="4.8" fill="#f7d5b5" stroke="#18283a" stroke-width="1" />
                <circle cx="-1.7" cy="-0.8" r="0.7" fill="#18283a" />
                <circle cx="1.7" cy="-0.8" r="0.7" fill="#18283a" />
                <path d="M -2.8 1.5 Q -1.2 0.3 0 1.2 Q 1.2 0.3 2.8 1.5" fill="none" stroke="#fbf7ee" stroke-width="1.3" stroke-linecap="round" />
                <rect x="-6.5" y="-5.4" width="13" height="1.8" rx="0.8" fill="#18283a" />
                <rect x="-4.2" y="-10.6" width="8.4" height="5.4" rx="1.2" fill="#18283a" />
                <rect x="-4.2" y="-6.8" width="8.4" height="1.4" fill="#d4a853" />
            </g>
        }.into_any(),
        1 => view! {
            // Seat 1: Modern Architect (Sleek hair + Round glasses + Ivory turtleneck)
            <g>
                <path d="M -7 8.5 Q -6 3.8 0 3.8 Q 6 3.8 7 8.5 Z" fill="#fbf7ee" stroke="#18283a" stroke-width="0.8" />
                <circle cx="0" cy="-0.3" r="4.8" fill="#f7d5b5" stroke="#18283a" stroke-width="1" />
                <path d="M -5 -1.4 Q -4.6 -6.2 0 -6.2 Q 4.6 -6.2 5 -1.4 Q 2 -4.2 -5 -1.4 Z" fill="#2c2420" />
                <circle cx="-1.8" cy="-0.5" r="1.4" fill="none" stroke="#18283a" stroke-width="0.9" />
                <circle cx="1.8" cy="-0.5" r="1.4" fill="none" stroke="#18283a" stroke-width="0.9" />
                <line x1="-0.4" y1="-0.5" x2="0.4" y2="-0.5" stroke="#18283a" stroke-width="0.9" />
                <path d="M -1.4 2.1 Q 0 3.2 1.4 2.1" fill="none" stroke="#18283a" stroke-width="0.9" stroke-linecap="round" />
            </g>
        }.into_any(),
        2 => view! {
            // Seat 2: Tech Courier (Helmet/Cap with visor + Cheerful smile)
            <g>
                <path d="M -7 8.5 Q -6 3.8 0 3.8 Q 6 3.8 7 8.5 Z" fill="#1b4d38" />
                <circle cx="0" cy="-0.1" r="4.8" fill="#f3c9a2" stroke="#18283a" stroke-width="1" />
                <path d="M -5.1 -1.5 Q -4.8 -6.4 0 -6.4 Q 4.8 -6.4 5.1 -1.5 Z" fill="#1f7a53" stroke="#18283a" stroke-width="1" />
                <rect x="-5.4" y="-2.2" width="10.8" height="1.6" rx="0.8" fill="#fbf7ee" stroke="#18283a" stroke-width="0.8" />
                <circle cx="-1.6" cy="0.2" r="0.7" fill="#18283a" />
                <circle cx="1.6" cy="0.2" r="0.7" fill="#18283a" />
                <path d="M -1.8 2 Q 0 3.4 1.8 2" fill="none" stroke="#18283a" stroke-width="1" stroke-linecap="round" />
            </g>
        }.into_any(),
        _ => view! {
            // Seat 3: Saigon Merchant (Golden Nón Lá conical hat + Crimson collar)
            <g>
                <path d="M -7 8.5 Q -6 3.8 0 3.8 Q 6 3.8 7 8.5 Z" fill="#b82d2f" />
                <circle cx="0" cy="0.4" r="4.7" fill="#f7d5b5" stroke="#18283a" stroke-width="1" />
                <circle cx="-1.6" cy="0.2" r="0.7" fill="#18283a" />
                <circle cx="1.6" cy="0.2" r="0.7" fill="#18283a" />
                <path d="M -1.5 2.3 Q 0 3.4 1.5 2.3" fill="none" stroke="#18283a" stroke-width="0.9" stroke-linecap="round" />
                <polygon points="-8.2,-1.6 0,-9.6 8.2,-1.6" fill="#f3d98b" stroke="#8c5f18" stroke-width="1.1" stroke-linejoin="round" />
                <line x1="-6" y1="-2.2" x2="6" y2="-2.2" stroke="#c89232" stroke-width="0.8" />
            </g>
        }.into_any(),
    };

    view! {
        <g>
            <circle cx="0" cy="0" r="11.5" fill=bg_col stroke="#18283a" stroke-width="1.4" />
            <circle cx="0" cy="0" r="9.6" fill="rgba(255, 255, 255, 0.16)" />
            {character_overlay}
        </g>
    }
}

/// Standalone SVG avatar badge for UI headers and player cards.
#[component]
pub fn PlayerAvatar(seat_idx: usize, #[prop(default = 28)] size: u32) -> impl IntoView {
    view! {
        <svg
            width=size
            height=size
            viewBox="-13 -13 26 26"
            style="display: block; flex-shrink: 0;"
        >
            {render_player_avatar_shapes(seat_idx)}
        </svg>
    }
}

#[component]
pub fn Board(
    state: ReadSignal<GameState>,
    selected_tile: ReadSignal<Option<u8>>,
    is_rolling: ReadSignal<bool>,
    is_animating_move: ReadSignal<bool>,
    display_dice: ReadSignal<(u8, u8)>,
    visual_positions: ReadSignal<[u8; 4]>,
    hopping_seat: ReadSignal<Option<usize>>,
    step_trail: ReadSignal<Vec<u8>>,
    landing_tile_burst: ReadSignal<Option<u8>>,
    recent_acquired_tile: ReadSignal<Option<u8>>,
    recent_house_tile: ReadSignal<Option<u8>>,
    floating_popups: ReadSignal<Vec<BoardFloatingPopup>>,
    drawn_card_banner: ReadSignal<Option<(String, String, String)>>,
    my_player_id: ReadSignal<Option<PlayerId>>,
    timer_seconds: ReadSignal<u32>,
    is_local_mode: ReadSignal<bool>,
    #[prop(default = None)] on_tile_click: Option<Callback<u8>>,
    #[prop(default = None)] on_roll_dice: Option<Callback<()>>,
    #[prop(default = None)] on_action: Option<Callback<ClientMessage>>,
) -> impl IntoView {
    let board = get_board();
    let (camera_mode, set_camera_mode) = signal(0u8);

    view! {
        <div class="board-container">
            <div class="board-camera-hud">
                <button
                    class=move || if camera_mode.get() == 0 { "cam-btn active" } else { "cam-btn" }
                    on:click=move |_| set_camera_mode.set(0)
                >
                    "Góc nhìn 3D"
                </button>
                <button
                    class=move || if camera_mode.get() == 1 { "cam-btn active" } else { "cam-btn" }
                    on:click=move |_| set_camera_mode.set(1)
                >
                    "2D Phẳng"
                </button>
            </div>

            <div class=move || match camera_mode.get() {
                0 => "board-aspect-frame view-3d-tabletop",
                _ => "board-aspect-frame view-2d-flat",
            }>
                // True 3D CSS Cube Dice Overlay aligned to center-board tray
                {move || {
                    let (d1, d2) = display_dice.get();
                    let rolling = is_rolling.get();
                    let is_doubles = !rolling && d1 == d2;
                    view! {
                        <div
                            class="dice-3d-overlay"
                            title="Nhấn để lắc xúc xắc"
                            on:click=move |_| {
                                if let Some(ref cb) = on_roll_dice {
                                    cb.run(());
                                }
                            }
                        >
                            {render_3d_die_cube(d1, rolling, true, is_doubles)}
                            {render_3d_die_cube(d2, rolling, false, is_doubles)}
                        </div>
                    }
                }}

                // Center Board Stage (Drawn Card, Buy Decision, Auction, Waiting Status)
                {move || {
                    let st = state.get();
                    if !st.is_game_started || st.is_game_over {
                        return ().into_any();
                    }

                    let busy = is_rolling.get() || is_animating_move.get();
                    if busy {
                        return ().into_any();
                    }

                    let my_id = my_player_id.get();
                    let is_my_turn = my_id.is_some() && st.current_player().map(|p| p.id) == my_id;
                    let me_opt = my_id.and_then(|id| st.find_player(id));
                    let has_urgent_decision = st.turn_phase == TurnPhase::Auction
                        || (is_my_turn && st.turn_phase == TurnPhase::TileActionPending);

                    if !has_urgent_decision {
                        if let Some((deck_name, card_title, card_desc)) = drawn_card_banner.get() {
                            let is_chance = deck_name.contains("Cơ Hội");
                            let cls = if is_chance { "stage-drawn-card chance" } else { "stage-drawn-card chest" };
                            let deck_label = if is_chance { "Thẻ Cơ hội" } else { "Thẻ Khí vận" };
                            return view! {
                                <div class="board-center-stage">
                                    <div class=cls>
                                        <div style="display: flex; align-items: center; justify-content: space-between; gap: 8px;">
                                            <span class="mono" style="font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; opacity: 0.85;">
                                                {deck_label}
                                            </span>
                                        </div>
                                        <div style="font-size: 16px; line-height: 21px; font-weight: 700;">
                                            {card_title}
                                        </div>
                                        <div style="font-size: 13px; line-height: 18px; font-weight: 500; opacity: 0.92;">
                                            {card_desc}
                                        </div>
                                    </div>
                                </div>
                            }.into_any();
                        }
                    }

                    // Auction Center Stage
                    if st.turn_phase == TurnPhase::Auction {
                        if let Some(auc) = st.auction.as_ref() {
                            let tiles = get_board();
                            if let Some(tile) = tiles.get(auc.tile_index as usize) {
                                let badge_col = match &tile.kind {
                                    TileKind::Property(p) => get_district_color(p.district),
                                    TileKind::Station(_) => TILE_STATION,
                                    TileKind::Utility(_) => TILE_UTILITY,
                                    _ => STATUS_WARNING,
                                };
                                let leader_text = auc.highest_bidder
                                    .and_then(|hid| st.find_player(hid))
                                    .map(|p| format!("{} đang dẫn", p.name))
                                    .unwrap_or_else(|| "Chưa có người trả giá".to_string());
                                let is_active_bidder = me_opt
                                    .map(|me| auc.active_bidders.contains(&me.id))
                                    .unwrap_or(false);
                                let is_leading = me_opt
                                    .map(|me| auc.highest_bidder == Some(me.id))
                                    .unwrap_or(false);
                                let my_cash = me_opt.map(|me| me.cash).unwrap_or(Money::ZERO);
                                let bid_50 = auc.current_bid + Money::from_vnd(50_000);
                                let bid_100 = auc.current_bid + Money::from_vnd(100_000);
                                let bid_500 = auc.current_bid + Money::from_vnd(500_000);

                                let bidders_view = st.players.iter().enumerate().map(|(seat_idx, pl)| {
                                    let is_leading = auc.highest_bidder == Some(pl.id);
                                    let is_in = auc.active_bidders.contains(&pl.id);
                                    let chip_cls = if is_leading {
                                        "stage-bidder-chip leading"
                                    } else if !is_in {
                                        "stage-bidder-chip passed"
                                    } else {
                                        "stage-bidder-chip"
                                    };
                                    view! {
                                        <span class=chip_cls>
                                            <PlayerAvatar seat_idx=seat_idx size=18 />
                                            <span>{pl.name.clone()}</span>
                                        </span>
                                    }
                                }).collect_view();

                                return view! {
                                    <div class="board-center-stage">
                                        <div class="stage-card">
                                            <div class="stage-card-header">
                                                <span class="stage-district-pill">
                                                    <span class="stage-district-dot" style=format!("background: {};", badge_col)></span>
                                                    <span>{format!("Đấu giá · {}", tile.name)}</span>
                                                </span>
                                                <span class="mono" style=format!("font-size: 11px; color: {};", STATUS_WARNING)>
                                                    {leader_text}
                                                </span>
                                            </div>
                                            <div class="stage-price-row">
                                                <span style=format!("font-size: 12px; color: {};", TEXT_SECONDARY)>"Giá cao nhất"</span>
                                                <span class="mono" style=format!("font-size: 22px; font-weight: 700; color: {};", STATUS_WARNING)>
                                                    {auc.current_bid.to_string()}
                                                </span>
                                            </div>
                                            <div class="stage-auction-bidders">
                                                {bidders_view}
                                            </div>
                                            {if is_active_bidder {
                                                let on_act_50 = on_action;
                                                let on_act_100 = on_action;
                                                let on_act_500 = on_action;
                                                let on_act_pass = on_action;
                                                view! {
                                                    <div class="stage-actions">
                                                        <button
                                                            class="btn-hud-pill btn-hud-buy"
                                                            style="height: 34px; padding: 0 11px; font-size: 12px;"
                                                            disabled=move || busy || is_leading || my_cash < bid_50
                                                            on:click=move |_| {
                                                                if let Some(cb) = on_act_50 {
                                                                    cb.run(ClientMessage::PlaceBid { amount: bid_50 });
                                                                }
                                                            }
                                                        >
                                                            "+50k"
                                                        </button>
                                                        <button
                                                            class="btn-hud-pill btn-hud-buy"
                                                            style="height: 34px; padding: 0 11px; font-size: 12px;"
                                                            disabled=move || busy || is_leading || my_cash < bid_100
                                                            on:click=move |_| {
                                                                if let Some(cb) = on_act_100 {
                                                                    cb.run(ClientMessage::PlaceBid { amount: bid_100 });
                                                                }
                                                            }
                                                        >
                                                            "+100k"
                                                        </button>
                                                        <button
                                                            class="btn-hud-pill btn-hud-buy"
                                                            style="height: 34px; padding: 0 11px; font-size: 12px;"
                                                            disabled=move || busy || is_leading || my_cash < bid_500
                                                            on:click=move |_| {
                                                                if let Some(cb) = on_act_500 {
                                                                    cb.run(ClientMessage::PlaceBid { amount: bid_500 });
                                                                }
                                                            }
                                                        >
                                                            "+500k"
                                                        </button>
                                                        <button
                                                            class="btn-hud-pill btn-hud-secondary"
                                                            style="height: 34px; padding: 0 11px; font-size: 12px;"
                                                            disabled=move || busy || is_leading
                                                            on:click=move |_| {
                                                                if let Some(cb) = on_act_pass {
                                                                    cb.run(ClientMessage::PassAuction);
                                                                }
                                                            }
                                                        >
                                                            "Bỏ"
                                                        </button>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <span style=format!("font-size: 12px; color: {};", TEXT_MUTED)>
                                                        "Bạn đã rời phiên đấu giá này"
                                                    </span>
                                                }.into_any()
                                            }}
                                        </div>
                                    </div>
                                }.into_any();
                            }
                        }
                    }

                    // Buy Property Decision Card in Center Stage
                    if is_my_turn && st.turn_phase == TurnPhase::TileActionPending {
                        if let Some(me) = me_opt {
                            let tiles = get_board();
                            let tile_idx = me.position;
                            if let Some(tile) = tiles.get(tile_idx as usize) {
                                let (cost, badge_col, badge_label, reason_text) = match &tile.kind {
                                    TileKind::Property(p) => {
                                        let d_tiles = get_district_tiles(p.district);
                                        let owned_in_district = d_tiles
                                            .iter()
                                            .filter(|&&t| st.properties[t as usize].owner == Some(me.id))
                                            .count();
                                        let total_in_district = d_tiles.len();
                                        let missing_after = total_in_district.saturating_sub(owned_in_district + 1);
                                        let reason = if missing_after == 0 {
                                            format!("Đủ bộ {} · thuê ×2 và được xây nhà!", get_district_label(p.district))
                                        } else {
                                            format!(
                                                "Còn thiếu {} ô để đủ bộ {} · thuê gốc {}",
                                                missing_after,
                                                get_district_label(p.district),
                                                p.rent_table[0]
                                            )
                                        };
                                        (p.cost, get_district_color(p.district), get_district_label(p.district).to_string(), reason)
                                    }
                                    TileKind::Station(s) => {
                                        let owned_st = st.stations_owned_by(me.id);
                                        let next_rent = Money::from_vnd(250_000 * (1i64 << owned_st));
                                        (
                                            s.cost,
                                            TILE_STATION,
                                            "Bến xe & Nhà ga".to_string(),
                                            format!("Sở hữu {} bến · tiền vé {}", owned_st + 1, next_rent),
                                        )
                                    }
                                    TileKind::Utility(u) => {
                                        let owned_ut = st.utilities_owned_by(me.id);
                                        let mult = if owned_ut + 1 >= 2 { 10 } else { 4 };
                                        (
                                            u.cost,
                                            TILE_UTILITY,
                                            "Công trình công cộng".to_string(),
                                            format!("Sở hữu {} công trình · phí {}× xúc xắc", owned_ut + 1, mult),
                                        )
                                    }
                                    _ => (Money::ZERO, STATUS_WARNING, "".to_string(), "".to_string()),
                                };

                                if cost > Money::ZERO {
                                    let can_buy = me.cash >= cost;
                                    let remaining_txt = if can_buy {
                                        format!("Còn lại: {}", Money(me.cash.0 - cost.0))
                                    } else {
                                        format!("Thiếu {}", Money(cost.0 - me.cash.0))
                                    };
                                    let decline_label = if st.config.enable_auction { "Đấu giá" } else { "Bỏ qua" };
                                    let on_buy = on_action;
                                    let on_decline = on_action;
                                    let on_inspect = on_tile_click;

                                    return view! {
                                        <div class="board-center-stage">
                                            <div class="stage-card">
                                                <div class="stage-card-header">
                                                    <span class="stage-district-pill">
                                                        <span class="stage-district-dot" style=format!("background: {};", badge_col)></span>
                                                        <span>{badge_label}</span>
                                                    </span>
                                                    <span class="mono" style=format!(
                                                        "font-size: 11px; color: {};",
                                                        if can_buy { TEXT_SECONDARY } else { STATUS_DANGER }
                                                    )>
                                                        {remaining_txt}
                                                    </span>
                                                </div>
                                                <div class="stage-price-row">
                                                    <span style=format!("font-size: 18px; font-weight: 700; color: {};", TEXT_PRIMARY)>
                                                        {tile.name.clone()}
                                                    </span>
                                                    <span class="mono" style=format!("font-size: 18px; font-weight: 700; color: {};", STATUS_SUCCESS)>
                                                        {cost.to_string()}
                                                    </span>
                                                </div>
                                                <div class="stage-reason">{reason_text}</div>
                                                <button
                                                    class="stage-link-btn"
                                                    on:click=move |_| {
                                                        if let Some(cb) = on_inspect {
                                                            cb.run(tile_idx);
                                                        }
                                                    }
                                                >
                                                    "Xem bảng tiền thuê"
                                                </button>
                                                <div class="stage-actions">
                                                    <button
                                                        class="btn-hud-pill btn-hud-buy"
                                                        style="flex: 1; height: 36px;"
                                                        disabled=move || busy || !can_buy
                                                        on:click=move |_| {
                                                            if let Some(cb) = on_buy {
                                                                cb.run(ClientMessage::BuyProperty);
                                                            }
                                                        }
                                                    >
                                                        {format!("Mua ({})", cost)}
                                                    </button>
                                                    <button
                                                        class="btn-hud-pill btn-hud-secondary"
                                                        style="height: 36px;"
                                                        disabled=move || busy
                                                        on:click=move |_| {
                                                            if let Some(cb) = on_decline {
                                                                cb.run(ClientMessage::DeclineProperty);
                                                            }
                                                        }
                                                    >
                                                        {decline_label}
                                                    </button>
                                                </div>
                                            </div>
                                        </div>
                                    }.into_any();
                                }
                            }
                        }
                    }

                    // Waiting status pill when it's another player's turn
                    if !is_my_turn {
                        if let Some(cur_p) = st.current_player() {
                            let seat_idx = st.players.iter().position(|p| p.id == cur_p.id).unwrap_or(0);
                            let cur_name = cur_p.name.clone();
                            let local = is_local_mode.get();
                            return view! {
                                <div class="board-center-stage">
                                    <div class="stage-waiting-pill">
                                        <PlayerAvatar seat_idx=seat_idx size=20 />
                                        <span>
                                            {if local {
                                                format!("Đang chờ {}…", cur_name).into_any()
                                            } else {
                                                let name_for_timer = cur_name.clone();
                                                (move || {
                                                    let t_sec = timer_seconds.get();
                                                    format!("Đang chờ {}… ({}:{:02})", name_for_timer, t_sec / 60, t_sec % 60)
                                                }).into_any()
                                            }}
                                        </span>
                                    </div>
                                </div>
                            }.into_any();
                        }
                    }

                    ().into_any()
                }}

            <svg class="board-svg" viewBox="0 0 1280 960" preserveAspectRatio="xMidYMid meet">
                <defs>
                    <linearGradient id="boardCenterGrad" x1="0%" y1="0%" x2="100%" y2="100%">
                        <stop offset="0%" stop-color="var(--board-center-grad-start, #e5f4ee)" />
                        <stop offset="55%" stop-color=BOARD_CENTER_BG />
                        <stop offset="100%" stop-color="var(--board-center-grad-end, #cee6de)" />
                    </linearGradient>
                    <linearGradient id="goldFrameGrad" x1="0%" y1="0%" x2="100%" y2="100%">
                        <stop offset="0%" stop-color="#e6c478" />
                        <stop offset="50%" stop-color="#d4a853" />
                        <stop offset="100%" stop-color="#b88b3a" />
                    </linearGradient>
                </defs>

                // Outer Prussian Navy frame + Champagne Gold pinstripe
                <rect x="0" y="0" width="1280" height="960" fill=BOARD_OUTER_BG rx="12" />
                <rect
                    x="4"
                    y="4"
                    width="1272"
                    height="952"
                    rx="10"
                    fill="none"
                    stroke="url(#goldFrameGrad)"
                    stroke-width="2"
                    opacity="0.72"
                />

                // Center Well with Mint-Celadon gradient & double Prussian-Gold cartographic border
                <rect
                    x=CORNER_W
                    y=CORNER_H
                    width=BOARD_WIDTH - 2.0 * CORNER_W
                    height=BOARD_HEIGHT - 2.0 * CORNER_H
                    fill="url(#boardCenterGrad)"
                    stroke=BOARD_CENTER_BORDER
                    stroke-width="2.2"
                />
                <rect
                    x=CORNER_W + 10.0
                    y=CORNER_H + 10.0
                    width=BOARD_WIDTH - 2.0 * CORNER_W - 20.0
                    height=BOARD_HEIGHT - 2.0 * CORNER_H - 20.0
                    fill="none"
                    stroke="#b88b3a"
                    stroke-width="1.2"
                    opacity="0.55"
                />
                <rect
                    x=CORNER_W + 16.0
                    y=CORNER_H + 16.0
                    width=BOARD_WIDTH - 2.0 * CORNER_W - 32.0
                    height=BOARD_HEIGHT - 2.0 * CORNER_H - 32.0
                    fill="none"
                    stroke="rgba(24, 40, 58, 0.22)"
                    stroke-width="1"
                    stroke-dasharray="6 4"
                />

                // Subtle Cartographic Corner Flourishes (4 inner corners of center well)
                <g fill="#d4a853" stroke="#1b334f" stroke-width="1" opacity="0.78">
                    <polygon points="154,132 162,124 170,132 162,140" />
                    <polygon points="1110,132 1118,124 1126,132 1118,140" />
                    <polygon points="154,828 162,820 170,828 162,836" />
                    <polygon points="1110,828 1118,820 1126,828 1118,836" />
                </g>

                // Decorative Compass Medallion & Sunburst Ring behind Skyline
                <g transform="translate(640, 515)" opacity="0.55">
                    <circle cx="0" cy="0" r="192" fill="rgba(232, 190, 98, 0.14)" stroke="#d4a853" stroke-width="1.5" stroke-dasharray="5 5" />
                    <circle cx="0" cy="0" r="168" fill="none" stroke="#1e3654" stroke-width="1" opacity="0.32" />
                </g>

                // Center Well Iconic Saigon Monopoly Artwork (Skyline + Diagonal Banner + Card Decks)
                <g transform="translate(640, 525) rotate(-26)" opacity="0.96">
                    // Saigon Skyline Silhouette in Deep Prussian Navy
                    <g fill="#1e3654" opacity="0.9">
                        <rect x="-210" y="-68" width="26" height="34" />
                        <rect x="-180" y="-84" width="22" height="50" />
                        <rect x="-152" y="-64" width="44" height="30" />
                        <rect x="-136" y="-82" width="12" height="18" />
                        <polygon points="-136,-82 -130,-94 -124,-82" />
                        <rect x="-102" y="-76" width="28" height="42" />
                        <rect x="-68" y="-92" width="24" height="58" />
                        <path d="M -36 -34 L -32 -128 L -10 -122 L -6 -34 Z" />
                        <rect x="-12" y="-114" width="14" height="4" rx="1" />
                        <rect x="4" y="-78" width="28" height="44" />
                        <rect x="38" y="-96" width="24" height="62" />
                        <rect x="70" y="-118" width="22" height="84" />
                        <rect x="75" y="-142" width="12" height="24" />
                        <rect x="79" y="-164" width="4" height="22" />
                        <rect x="98" y="-82" width="26" height="48" />
                        <rect x="130" y="-68" width="32" height="34" />
                        <rect x="168" y="-88" width="24" height="54" />
                        <rect x="196" y="-62" width="18" height="28" />
                    </g>

                    // Bold Diagonal Crimson Red Banner with Warm Ivory & Gold Trim
                    <rect x="-228" y="-32" width="464" height="76" rx="8" fill="#18283a" opacity="0.32" />
                    <rect x="-234" y="-38" width="464" height="76" rx="8" fill="#d9383a" stroke="#fbf7ee" stroke-width="3" />
                    <rect x="-228" y="-32" width="452" height="64" rx="5" fill="none" stroke="#e6c478" stroke-width="1.5" opacity="0.72" />
                    <text
                        x="-2"
                        y="16"
                        font-size="46"
                        font-weight="700"
                        fill="#fbf7ee"
                        text-anchor="middle"
                        font-family="'Barlow Condensed', sans-serif"
                        letter-spacing="0.08em"
                    >
                        "CỜ TỶ PHÚ"
                    </text>
                    <text
                        x="0"
                        y="62"
                        font-size="15"
                        font-weight="700"
                        fill="#1e3654"
                        text-anchor="middle"
                        font-family="'Barlow Condensed', sans-serif"
                        letter-spacing="0.22em"
                    >
                        "PHIÊN BẢN SÀI GÒN · ĐỊA ỐC ĐÔ THỊ"
                    </text>
                </g>

                // Top-Left Card Deck Slot: KHÍ VẬN (3D Stacked Deck)
                <g transform="translate(302, 266) rotate(-26)">
                    <rect x="-71" y="-39" width="154" height="94" rx="6" fill="rgba(24, 40, 58, 0.25)" />
                    <rect x="-74" y="-42" width="154" height="94" rx="6" fill="#d9d0be" stroke="#18283a" stroke-width="1.5" />
                    <rect x="-76" y="-45" width="154" height="94" rx="6" fill="#e8dfc9" stroke="#18283a" stroke-width="1.5" />
                    <rect x="-78" y="-48" width="154" height="94" rx="6" fill="#f2c94c" stroke="#18283a" stroke-width="2" />
                    <rect x="-72" y="-42" width="142" height="82" rx="4" fill="none" stroke="#18283a" stroke-width="1.2" stroke-dasharray="4 3" />
                    <text
                        x="0"
                        y="-10"
                        font-size="18"
                        font-weight="700"
                        fill="#18283a"
                        text-anchor="middle"
                        font-family="'Barlow Condensed', sans-serif"
                        letter-spacing="0.08em"
                    >
                        "KHÍ VẬN"
                    </text>
                    <rect x="-14" y="2" width="28" height="18" rx="3" fill="#2b7a78" stroke="#18283a" stroke-width="1.5" />
                    <line x1="-14" y1="9" x2="14" y2="9" stroke="#18283a" stroke-width="1.5" />
                    <rect x="-3" y="6" width="6" height="6" rx="1" fill="#fbf7ee" />
                </g>

                // Bottom-Right Card Deck Slot: CƠ HỘI (3D Stacked Deck)
                <g transform="translate(978, 706) rotate(-26)">
                    <rect x="-71" y="-39" width="154" height="94" rx="6" fill="rgba(24, 40, 58, 0.25)" />
                    <rect x="-74" y="-42" width="154" height="94" rx="6" fill="#d9d0be" stroke="#18283a" stroke-width="1.5" />
                    <rect x="-76" y="-45" width="154" height="94" rx="6" fill="#e8dfc9" stroke="#18283a" stroke-width="1.5" />
                    <rect x="-78" y="-48" width="154" height="94" rx="6" fill="#e67e35" stroke="#18283a" stroke-width="2" />
                    <rect x="-72" y="-42" width="142" height="82" rx="4" fill="none" stroke="#18283a" stroke-width="1.2" stroke-dasharray="4 3" />
                    <text
                        x="0"
                        y="-12"
                        font-size="18"
                        font-weight="700"
                        fill="#18283a"
                        text-anchor="middle"
                        font-family="'Barlow Condensed', sans-serif"
                        letter-spacing="0.08em"
                    >
                        "CƠ HỘI"
                    </text>
                    <text
                        x="0"
                        y="26"
                        font-size="30"
                        font-weight="700"
                        fill="#fbf7ee"
                        stroke="#18283a"
                        stroke-width="1.2"
                        text-anchor="middle"
                        font-family="'Barlow Condensed', sans-serif"
                    >
                        "?"
                    </text>
                </g>

                <text
                    x="640"
                    y="812"
                    font-size="12.5"
                    font-weight="500"
                    fill="#3b526b"
                    text-anchor="middle"
                    font-family="'Barlow', sans-serif"
                >
                    "Chọn ô bất kỳ trên bàn cờ để tra cứu khế ước và biểu phí thuê"
                </text>

                // Recessed Prussian Navy Dice Tray (paired with HTML 3D Dice Overlay)
                {move || {
                    let (d1, d2) = display_dice.get();
                    let rolling = is_rolling.get();
                    let is_doubles = !rolling && d1 == d2;
                    let can_roll_tray = state.with(|st| {
                        let my_id = my_player_id.get();
                        st.is_game_started
                            && !st.is_game_over
                            && my_id.is_some()
                            && st.current_player().map(|p| p.id) == my_id
                            && st.turn_phase == TurnPhase::WaitingForRoll
                    }) && !rolling
                        && !is_animating_move.get();

                    view! {
                        <g
                            style=if can_roll_tray { "cursor: pointer;" } else { "cursor: default;" }
                            on:click=move |_| {
                                if can_roll_tray {
                                    if let Some(ref cb) = on_roll_dice {
                                        cb.run(());
                                    }
                                }
                            }
                        >
                            <rect
                                x="536"
                                y="206"
                                width="208"
                                height="96"
                                rx="12"
                                fill="#182d46"
                                stroke=if is_doubles { STATUS_WARNING } else { "#d4a853" }
                                stroke-width="2"
                            />
                            <rect
                                x="541"
                                y="211"
                                width="198"
                                height="86"
                                rx="9"
                                fill="none"
                                stroke=if is_doubles { "rgba(212, 155, 53, 0.72)" } else { "rgba(230, 196, 120, 0.35)" }
                                stroke-width="1"
                            />
                            <rect
                                x="512"
                                y="310"
                                width="256"
                                height="24"
                                rx="12"
                                fill="#182d46"
                                stroke="rgba(212, 168, 83, 0.4)"
                                stroke-width="1"
                                opacity="0.94"
                            />
                            <text
                                x="640"
                                y="326"
                                font-size="11.5"
                                font-weight="500"
                                font-family="'Roboto Mono', monospace"
                                fill=if is_doubles { STATUS_WARNING } else { "#fbf7ee" }
                                text-anchor="middle"
                            >
                                {if rolling {
                                    "Đang tung xúc xắc...".to_string()
                                } else if is_doubles {
                                    format!("Xúc xắc đôi: {} + {} = {} · Thêm lượt", d1, d2, d1 + d2)
                                } else {
                                    format!("Kết quả: {} + {} = {}", d1, d2, d1 + d2)
                                }}

                            </text>
                        </g>
                    }
                }}


                // 40 Board Tiles
                {board.into_iter().map(|tile| {
                    let idx = tile.index;
                    let (rx, ry, w, h) = get_tile_rect(idx);
                    let is_corner = idx == 0 || idx == 10 || idx == 20 || idx == 30;
                    let tile_kind = tile.kind.clone();

                    let header_color = match &tile.kind {
                        TileKind::Property(p) => Some(get_district_color(p.district)),
                        _ => None,
                    };

                    // Content center `(tx, ty)` shifted away from the inner color band or corner art
                    let (tx, ty) = if idx == 10 {
                        (rx + w / 2.0 - 12.0, ry + h - 24.0)
                    } else if is_corner {
                        (rx + w / 2.0, ry + h / 2.0 + 6.0)
                    } else if idx > 0 && idx < 10 {
                        (rx + w / 2.0, ry + HEADER_THICKNESS + (h - HEADER_THICKNESS) / 2.0 + 4.0)
                    } else if idx > 10 && idx < 20 {
                        (rx + (w - HEADER_THICKNESS) / 2.0 - 4.0, ry + h / 2.0)
                    } else if idx > 20 && idx < 30 {
                        (rx + w / 2.0, ry + (h - HEADER_THICKNESS) / 2.0 - 4.0)
                    } else {
                        (rx + HEADER_THICKNESS + (w - HEADER_THICKNESS) / 2.0 + 4.0, ry + h / 2.0)
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

                    let tile_fill = match &tile.kind {
                        TileKind::Chance => TILE_CHANCE_BG,
                        TileKind::CommunityChest => TILE_CHEST_BG,
                        _ if is_corner => TILE_CORNER_BG,
                        _ => TILE_BG,
                    };
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
                                cb.run(idx);
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
                                        "#f5e1a4"
                                    } else if is_in_trail() {
                                        "#efe4c6"
                                    } else {
                                        tile_fill
                                    }
                                }
                                stroke=move || {
                                    if is_selected() || is_stepped_on() {
                                        TILE_SELECTED_STROKE
                                    } else if is_in_trail() {
                                        "#a88b4a"
                                    } else {
                                        TILE_STROKE
                                    }
                                }
                                stroke-width=move || {
                                    if is_selected() || is_stepped_on() {
                                        "2.5"
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
                                view! {
                                    <g>
                                        <rect x=hx y=hy width=hw height=hh fill=h_col stroke=TILE_STROKE stroke-width="1" />
                                        <rect x=hx + 1.0 y=hy + 1.0 width=(hw - 2.0).max(1.0) height="3" fill="rgba(255,255,255,0.22)" />
                                    </g>
                                }.into_any()
                            } else {
                                let (inner_fill, inner_stroke, inner_sw) = match &tile_kind {
                                    TileKind::Chance => (TILE_CHANCE_INNER_BG, TILE_CHANCE_BORDER, "1.4"),
                                    TileKind::CommunityChest => (TILE_CHEST_INNER_BG, TILE_CHEST_BORDER, "1.4"),
                                    _ => ("none", TILE_CORNER_ACCENT, "1"),
                                };
                                view! {
                                    <rect
                                        x=rx + 4.0
                                        y=ry + 4.0
                                        width=w - 8.0
                                        height=h - 8.0
                                        fill=inner_fill
                                        stroke=inner_stroke
                                        stroke-width=inner_sw
                                        rx="4"
                                    />
                                }.into_any()
                            }}


                            {render_tile_illustration(idx, &tile_kind, rx, ry, w, h)}

                            {name_lines.into_iter().enumerate().map(|(line_idx, line)| {
                                let ly = start_y + (line_idx as f64) * line_height;
                                view! {
                                    <text
                                        x=tx
                                        y=ly
                                        font-size=if is_corner { "11.5" } else { "10.5" }
                                        font-weight="600"
                                        font-family="'Barlow', sans-serif"
                                        fill=TILE_TEXT_PRIMARY
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
                                        font-weight="500"
                                        font-family="'Roboto Mono', monospace"
                                        fill=TILE_TEXT_MUTED
                                        text-anchor="middle"
                                    >
                                        {price_text}
                                    </text>
                                }.into_any()
                            } else {
                                ().into_any()
                            }}


                            // Ownership Player Avatar Badge
                            {move || {
                                if let Some(owner_id) = prop_owner() {
                                    let p_idx = state.get().players.iter().position(|p| p.id == owner_id).unwrap_or(0);
                                    let just_acquired = recent_acquired_tile.get() == Some(idx);
                                    let badge_cls = if just_acquired { "owner-badge-pop" } else { "" };
                                    view! {
                                        <g
                                            class=badge_cls
                                            transform=format!("translate({}, {}) scale(0.75)", rx + 14.0, ry + 14.0)
                                        >
                                            {render_player_avatar_shapes(p_idx)}
                                        </g>
                                    }.into_any()
                                } else {
                                    ().into_any()
                                }
                            }}


                            // 3D Isometric Miniature Houses / Hotel & Mortgage Tag
                            {move || {
                                let h_cnt = house_count();
                                let just_built = recent_house_tile.get() == Some(idx);
                                let build_cls = if just_built { "house-icon-pop" } else { "" };

                                if h_cnt > 0 {
                                    let label_str = if h_cnt == 5 { "KS".to_string() } else { format!("×{}", h_cnt) };
                                    let pill_col = if h_cnt == 5 { STATUS_WARNING } else { STATUS_SUCCESS };
                                    view! {
                                        <g class=build_cls>
                                            <rect
                                                x=tx - 32.0
                                                y=status_y - 10.0
                                                width="64"
                                                height="16"
                                                rx="4"
                                                fill="#1c1a18"
                                                stroke=pill_col
                                                stroke-width="1"
                                            />
                                            {if h_cnt == 5 {
                                                // 3D Isometric Hotel Block
                                                let hx = tx - 22.0;
                                                let hy = status_y - 2.0;
                                                view! {
                                                    <g>
                                                        <polygon points=format!("{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}", hx - 5.0, hy - 3.0, hx + 2.0, hy - 3.0, hx + 2.0, hy + 4.0, hx - 5.0, hy + 4.0) fill="#d9383a" />
                                                        <polygon points=format!("{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}", hx + 2.0, hy - 3.0, hx + 6.0, hy - 5.0, hx + 6.0, hy + 2.0, hx + 2.0, hy + 4.0) fill="#9e2426" />
                                                        <polygon points=format!("{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}", hx - 5.0, hy - 3.0, hx - 1.0, hy - 5.0, hx + 6.0, hy - 5.0, hx + 2.0, hy - 3.0) fill="#f2c94c" />
                                                    </g>
                                                }.into_any()
                                            } else {
                                                // 3D Isometric House
                                                let hx = tx - 21.0;
                                                let hy = status_y - 1.0;
                                                view! {
                                                    <g>
                                                        <polygon points=format!("{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}", hx - 4.0, hy - 2.0, hx + 1.0, hy - 2.0, hx + 1.0, hy + 3.0, hx - 4.0, hy + 3.0) fill="#3ca370" />
                                                        <polygon points=format!("{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}", hx + 1.0, hy - 2.0, hx + 4.5, hy - 4.0, hx + 4.5, hy + 1.0, hx + 1.0, hy + 3.0) fill="#28754f" />
                                                        <polygon points=format!("{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}", hx - 4.5, hy - 2.0, hx - 1.5, hy - 5.5, hx + 1.5, hy - 2.0) fill="#5ce09b" />
                                                    </g>
                                                }.into_any()
                                            }}

                                            <text
                                                x=tx + 9.0
                                                y=status_y + 1.0
                                                font-size="9"
                                                font-weight="600"
                                                font-family="'Roboto Mono', monospace"
                                                fill=pill_col
                                                text-anchor="middle"
                                            >
                                                {label_str}
                                            </text>
                                        </g>
                                    }.into_any()
                                } else if is_mortgaged() {
                                    view! {
                                        <text
                                            x=tx
                                            y=status_y
                                            font-size="8.5"
                                            font-weight="600"
                                            font-family="'Roboto Mono', monospace"
                                            fill=STATUS_DANGER
                                            text-anchor="middle"
                                        >
                                            "Thế chấp"
                                        </text>
                                    }.into_any()
                                } else {
                                    ().into_any()
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
                        }.into_any()
                    } else {
                        ().into_any()
                    }
                }}


                // 3D Volumetric Standing Pawn Pieces (Monogram T1..T4)
                {move || {
                    let st = state.get();
                    let cur_active_id = st.current_player().map(|p| p.id);
                    let positions = visual_positions.get();
                    let hopping = hopping_seat.get();

                    st.players.into_iter().enumerate().map(|(p_idx, player)| {
                        if player.is_bankrupt {
                            return view! { <g></g> }.into_any();
                        }
                        let vis_pos = positions[p_idx % 4];
                        let (cx, cy) = get_tile_center(vis_pos);
                        let offset_x = ((p_idx % 2) as f64 - 0.5) * 28.0;
                        let offset_y = ((p_idx / 2) as f64 - 0.5) * 22.0;
                        let px = cx + offset_x;
                        let py = cy + offset_y;
                        let col = PLAYER_COLORS[p_idx % 4];
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
                                        <ellipse
                                            cx="0"
                                            cy="10"
                                            rx="17"
                                            ry="7"
                                            fill="none"
                                            stroke="#d9383a"
                                            stroke-width="2"
                                            opacity="0.9"
                                        />
                                    }.into_any()
                                } else {
                                    ().into_any()
                                }}


                                <ellipse
                                    class=shadow_class
                                    cx="2"
                                    cy="14"
                                    rx="14"
                                    ry="5"
                                    fill="rgba(0, 0, 0, 0.45)"
                                />

                                <g class=body_class>
                                    // 3D Pedestal Rim & Base
                                    <ellipse cx="0" cy="11" rx="13" ry="5" fill="#1c1a18" />
                                    <ellipse cx="0" cy="9" rx="12" ry="4.5" fill=col stroke="#1c1a18" stroke-width="1.2" />
                                    // Tapered 3D Column Body
                                    <path d="M -11 9 L -8 -5 L 8 -5 L 11 9 Z" fill=col stroke="#1c1a18" stroke-width="1.2" />
                                    // Left-side Specular Highlight
                                    <path d="M -8 8 L -6 -4 L -2 -4 L -3 8 Z" fill="rgba(255,255,255,0.24)" />
                                    // Top 3D Character Avatar Medallion Bust
                                    <g transform="translate(0, -8) scale(1.1)">
                                        {render_player_avatar_shapes(p_idx)}
                                    </g>
                                </g>
                            </g>
                        }.into_any()
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
                                        font-family="'Roboto Mono', monospace"
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

            // Always-2D Selected Tile / Asset Deed / Chance & Community Chest Detail Card
            {move || {
                let sel_idx = selected_tile.get()?;
                let all_tiles = get_board();
                let tile = all_tiles.get(sel_idx as usize)?.clone();
                let st = state.get();
                let prop_state = st.properties.get(sel_idx as usize).copied().unwrap_or_default();

                let close_card = move || {
                    if let Some(ref cb) = on_tile_click {
                        cb.run(sel_idx);
                    }
                };

                let (badge_color, category_label) = match &tile.kind {
                    TileKind::Property(p) => (get_district_color(p.district), get_district_label(p.district).to_string()),
                    TileKind::Station(_) => (TILE_STATION, "Bến xe & Nhà ga".to_string()),
                    TileKind::Utility(_) => (TILE_UTILITY, "Công trình công cộng".to_string()),
                    TileKind::Tax(_) => (TILE_TAX, "Thuế nhà nước".to_string()),
                    TileKind::Chance => (TILE_CHANCE, "Bộ thẻ Cơ hội".to_string()),
                    TileKind::CommunityChest => (TILE_CHEST, "Bộ thẻ Khí vận".to_string()),
                    TileKind::Go => (STATUS_SUCCESS, "Ô Khởi hành".to_string()),
                    TileKind::Jail => (STATUS_WARNING, "Khu tạm giam".to_string()),
                    TileKind::FreeParking => (TILE_CHEST, "Bãi đỗ xe miễn phí".to_string()),
                    TileKind::GoToJail => (STATUS_DANGER, "Lệnh bắt giữ".to_string()),
                };

                let format_card_effect = |eff: &CardEffect| -> (String, bool) {
                    match eff {
                        CardEffect::CashDelta(m) => {
                            if m.0 >= 0 {
                                (format!("+{}", m), true)
                            } else {
                                (format!("{}", m), false)
                            }
                        }
                        CardEffect::MoveTo(pos) => (format!("Tới ô #{}", pos), true),
                        CardEffect::GoToJail => ("Vào tù".to_string(), false),
                        CardEffect::GetOutOfJailFree => ("Thẻ ra tù".to_string(), true),
                        CardEffect::BuildingRepairs { house, hotel } => {
                            (format!("Sửa {}/{}", house, hotel), false)
                        }
                        CardEffect::CollectFromAll(m) => (format!("Thu +{}", m), true),
                        CardEffect::PayToAll(m) => (format!("Trả -{}", m), false),
                    }
                };

                let body_view = match &tile.kind {
                    TileKind::Chance | TileKind::CommunityChest => {
                        let is_chance = matches!(tile.kind, TileKind::Chance);
                        let deck = if is_chance { get_chance_deck() } else { get_community_chest_deck() };
                        let deck_desc = if is_chance {
                            "Khi dừng tại ô Cơ hội, người chơi rút 1 thẻ ngẫu nhiên từ bộ 16 thẻ và thực hiện hiệu ứng ngay lập tức."
                        } else {
                            "Khi dừng tại ô Khí vận, người chơi rút 1 thẻ ngẫu nhiên từ bộ 16 thẻ và thực hiện hiệu ứng ngay lập tức."
                        };
                        view! {
                            <div class="deed-2d-body">
                                <div class="deed-2d-primary-box" style="flex-direction: column; align-items: flex-start; gap: 4px;">
                                    <span style=format!("font-size: 14px; font-weight: 600; color: {};", TEXT_PRIMARY)>
                                        {if is_chance { "Chi tiết bộ thẻ Cơ hội (16 thẻ)" } else { "Chi tiết bộ thẻ Khí vận (16 thẻ)" }}
                                    </span>
                                    <span style=format!("font-size: 12px; line-height: 17px; color: {};", TEXT_SECONDARY)>
                                        {deck_desc}
                                    </span>
                                </div>

                                <div class="deed-2d-secondary-box" style="max-height: 280px; overflow-y: auto; gap: 8px;">
                                    {deck.into_iter().map(|card| {
                                        let (eff_txt, is_pos) = format_card_effect(&card.effect);
                                        let eff_col = if is_pos { STATUS_SUCCESS } else { STATUS_DANGER };
                                        view! {
                                            <div style=format!("display: flex; align-items: flex-start; justify-content: space-between; gap: 10px; padding-bottom: 6px; border-bottom: 1px solid {};", BORDER_DEFAULT)>
                                                <div style="display: flex; flex-direction: column; gap: 2px; min-width: 0;">
                                                    <span style=format!("font-size: 13px; font-weight: 600; color: {};", TEXT_PRIMARY)>
                                                        {card.title}
                                                    </span>
                                                    <span style=format!("font-size: 11px; line-height: 15px; color: {};", TEXT_MUTED)>
                                                        {card.description}
                                                    </span>
                                                </div>
                                                <span class="mono" style=format!("font-size: 11px; font-weight: 600; color: {}; white-space: nowrap; flex-shrink: 0;", eff_col)>
                                                    {eff_txt}
                                                </span>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            </div>
                        }.into_any()
                    }
                    TileKind::Property(p) => {
                        let owner_view = if let Some(owner_id) = prop_state.owner {
                            if let Some((p_idx, owner)) = st.players.iter().enumerate().find(|(_, pl)| pl.id == owner_id) {
                                view! {
                                    <div style="display: inline-flex; align-items: center; gap: 8px;">
                                        <PlayerAvatar seat_idx=p_idx size=22 />
                                        <span style=format!("font-size: 13px; font-weight: 600; color: {};", TEXT_PRIMARY)>
                                            {owner.name.clone()}
                                        </span>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <span style=format!("font-size: 12px; color: {};", TEXT_MUTED)>"Đã có chủ"</span> }.into_any()
                            }
                        } else {
                            view! { <span style=format!("font-size: 12px; color: {};", TEXT_MUTED)>"Chưa có chủ · Ngân hàng"</span> }.into_any()
                        };

                        let (primary_label, primary_value, primary_col) = if prop_state.is_mortgaged {
                            ("Trạng thái", "Đang thế chấp · 0 đ".to_string(), STATUS_WARNING)
                        } else if let Some(owner_id) = prop_state.owner {
                            let rent = if prop_state.house_count > 0 {
                                p.rent_table[prop_state.house_count as usize]
                            } else if st.owns_full_district(owner_id, p.district) {
                                Money(p.rent_table[0].0 * 2)
                            } else {
                                p.rent_table[0]
                            };
                            ("Tiền thuê hiện tại", rent.to_string(), STATUS_WARNING)
                        } else {
                            ("Giá mua đất", p.cost.to_string(), STATUS_SUCCESS)
                        };

                        let h_cnt = prop_state.house_count;
                        let is_full_set = prop_state
                            .owner
                            .map(|oid| st.owns_full_district(oid, p.district))
                            .unwrap_or(false);

                        view! {
                            <div class="deed-2d-body">
                                // Primary Element: Key Price/Rent + Owner
                                <div class="deed-2d-primary-box">
                                    <div style="display: flex; flex-direction: column; gap: 2px;">
                                        <span style=format!("font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; color: {};", TEXT_MUTED)>
                                            {primary_label}
                                        </span>
                                        <span class="mono" style=format!("font-size: 22px; line-height: 28px; font-weight: 700; color: {};", primary_col)>
                                            {primary_value}
                                        </span>
                                    </div>
                                    <div style="display: flex; flex-direction: column; align-items: flex-end; gap: 3px;">
                                        <span style=format!("font-size: 11px; color: {};", TEXT_MUTED)>"Chủ sở hữu"</span>
                                        {owner_view}
                                    </div>
                                </div>

                                // Secondary Element: Rent Tiers & Building Costs
                                <div class="deed-2d-secondary-box">
                                    <div class={if prop_state.owner.is_some() && h_cnt == 0 && !is_full_set { "deed-2d-tier-row active" } else { "deed-2d-tier-row" }}>
                                        <span>"Đất trống"</span>
                                        <span class="mono">{p.rent_table[0].to_string()}</span>
                                    </div>
                                    <div class={if prop_state.owner.is_some() && h_cnt == 0 && is_full_set { "deed-2d-tier-row active" } else { "deed-2d-tier-row" }}>
                                        <span>"Đủ bộ quận (×2)"</span>
                                        <span class="mono">{Money(p.rent_table[0].0 * 2).to_string()}</span>
                                    </div>
                                    <div class={if h_cnt == 1 { "deed-2d-tier-row active" } else { "deed-2d-tier-row" }}>
                                        <span>"1 căn nhà phố"</span>
                                        <span class="mono">{p.rent_table[1].to_string()}</span>
                                    </div>
                                    <div class={if h_cnt == 2 { "deed-2d-tier-row active" } else { "deed-2d-tier-row" }}>
                                        <span>"2 căn nhà phố"</span>
                                        <span class="mono">{p.rent_table[2].to_string()}</span>
                                    </div>
                                    <div class={if h_cnt == 3 { "deed-2d-tier-row active" } else { "deed-2d-tier-row" }}>
                                        <span>"3 căn nhà phố"</span>
                                        <span class="mono">{p.rent_table[3].to_string()}</span>
                                    </div>
                                    <div class={if h_cnt == 4 { "deed-2d-tier-row active" } else { "deed-2d-tier-row" }}>
                                        <span>"4 căn nhà phố"</span>
                                        <span class="mono">{p.rent_table[4].to_string()}</span>
                                    </div>
                                    <div class={if h_cnt == 5 { "deed-2d-tier-row active" } else { "deed-2d-tier-row" }}>
                                        <span>"Cấp khách sạn"</span>
                                        <span class="mono">{p.rent_table[5].to_string()}</span>
                                    </div>
                                </div>

                                <div style=format!("display: flex; justify-content: space-between; font-size: 11px; color: {}; padding: 0 2px;", TEXT_MUTED) class="mono">
                                    <span>{format!("Xây nhà: {}", p.house_cost)}</span>
                                    <span>{format!("Thế chấp: {}", p.mortgage_value)}</span>
                                </div>
                            </div>
                        }.into_any()
                    }
                    TileKind::Station(s) => {
                        let owner_cnt = prop_state.owner.map(|oid| st.stations_owned_by(oid)).unwrap_or(0);
                        let (primary_label, primary_val, primary_col) = if prop_state.is_mortgaged {
                            ("Trạng thái", "Đang thế chấp · 0 đ".to_string(), STATUS_WARNING)
                        } else if prop_state.owner.is_some() {
                            let mult = 1i64 << owner_cnt.saturating_sub(1);
                            ("Tiền vé hiện tại", Money::from_vnd(250_000 * mult).to_string(), STATUS_WARNING)
                        } else {
                            ("Giá mua bến", s.cost.to_string(), STATUS_SUCCESS)
                        };
                        view! {
                            <div class="deed-2d-body">
                                <div class="deed-2d-primary-box">
                                    <div style="display: flex; flex-direction: column; gap: 2px;">
                                        <span style=format!("font-size: 11px; text-transform: uppercase; color: {};", TEXT_MUTED)>{primary_label}</span>
                                        <span class="mono" style=format!("font-size: 22px; font-weight: 700; color: {};", primary_col)>{primary_val}</span>
                                    </div>
                                    {if let Some(oid) = prop_state.owner {
                                        if let Some((p_idx, owner)) = st.players.iter().enumerate().find(|(_, pl)| pl.id == oid) {
                                            view! {
                                                <div style="display: inline-flex; align-items: center; gap: 8px;">
                                                    <PlayerAvatar seat_idx=p_idx size=22 />
                                                    <span style=format!("font-size: 13px; font-weight: 600; color: {};", TEXT_PRIMARY)>{owner.name.clone()}</span>
                                                </div>
                                            }.into_any()
                                        } else { ().into_any() }
                                    } else {
                                        view! { <span style=format!("font-size: 12px; color: {};", TEXT_MUTED)>"Ngân hàng"</span> }.into_any()
                                    }}

                                </div>
                                <div class="deed-2d-secondary-box">
                                    <div class={if owner_cnt == 1 { "deed-2d-tier-row active" } else { "deed-2d-tier-row" }}><span>"Sở hữu 1 bến"</span><span class="mono">"250.000 đ"</span></div>
                                    <div class={if owner_cnt == 2 { "deed-2d-tier-row active" } else { "deed-2d-tier-row" }}><span>"Sở hữu 2 bến"</span><span class="mono">"500.000 đ"</span></div>
                                    <div class={if owner_cnt == 3 { "deed-2d-tier-row active" } else { "deed-2d-tier-row" }}><span>"Sở hữu 3 bến"</span><span class="mono">"1.000.000 đ"</span></div>
                                    <div class={if owner_cnt == 4 { "deed-2d-tier-row active" } else { "deed-2d-tier-row" }}><span>"Trọn bộ 4 bến"</span><span class="mono">"2.000.000 đ"</span></div>
                                </div>
                                <div style=format!("font-size: 11px; color: {};", TEXT_MUTED) class="mono">
                                    {format!("Giá trị thế chấp: {}", s.mortgage_value)}
                                </div>
                            </div>
                        }.into_any()
                    }
                    TileKind::Utility(u) => {
                        let owner_cnt = prop_state.owner.map(|oid| st.utilities_owned_by(oid)).unwrap_or(0);
                        let (primary_label, primary_val, primary_col) = if prop_state.is_mortgaged {
                            ("Trạng thái", "Đang thế chấp · 0 đ".to_string(), STATUS_WARNING)
                        } else if prop_state.owner.is_some() {
                            let mult = if owner_cnt >= 2 { 10 } else { 4 };
                            ("Hệ số phí", format!("{}× xúc xắc", mult), STATUS_WARNING)
                        } else {
                            ("Giá mua công trình", u.cost.to_string(), STATUS_SUCCESS)
                        };
                        view! {
                            <div class="deed-2d-body">
                                <div class="deed-2d-primary-box">
                                    <div style="display: flex; flex-direction: column; gap: 2px;">
                                        <span style=format!("font-size: 11px; text-transform: uppercase; color: {};", TEXT_MUTED)>{primary_label}</span>
                                        <span class="mono" style=format!("font-size: 22px; font-weight: 700; color: {};", primary_col)>{primary_val}</span>
                                    </div>
                                </div>
                                <div class="deed-2d-secondary-box">
                                    <div class={if owner_cnt == 1 { "deed-2d-tier-row active" } else { "deed-2d-tier-row" }}><span>"Sở hữu 1 công trình"</span><span class="mono">"4 × xúc xắc × 1.000 đ"</span></div>
                                    <div class={if owner_cnt >= 2 { "deed-2d-tier-row active" } else { "deed-2d-tier-row" }}><span>"Sở hữu cả 2 công trình"</span><span class="mono">"10 × xúc xắc × 1.000 đ"</span></div>
                                </div>
                                <div style=format!("font-size: 11px; color: {};", TEXT_MUTED) class="mono">
                                    {format!("Giá trị thế chấp: {}", u.mortgage_value)}
                                </div>
                            </div>
                        }.into_any()
                    }
                    TileKind::Tax(t) => view! {
                        <div class="deed-2d-body">
                            <div class="deed-2d-primary-box">
                                <div style="display: flex; flex-direction: column; gap: 2px;">
                                    <span style=format!("font-size: 11px; color: {};", TEXT_MUTED)>"Khoản nộp ngân sách"</span>
                                    <span class="mono" style=format!("font-size: 22px; font-weight: 700; color: {};", STATUS_DANGER)>{t.amount.to_string()}</span>
                                </div>
                            </div>
                        </div>
                    }.into_any(),
                    _ => {
                        let desc = match &tile.kind {
                            TileKind::Go => "Đi qua hoặc dừng lại ô này được nhận +2.000.000 đ tiền lương.",
                            TileKind::Jail => "Thăm tù miễn phí, hoặc nộp 500.000 đ để bảo lãnh khi đang bị giam.",
                            TileKind::FreeParking => "Dừng chân nghỉ ngơi miễn phí, không phát sinh thu chi.",
                            TileKind::GoToJail => "Di chuyển thẳng tới Khu tạm giam (#10) và không nhận lương qua Khởi hành.",
                            _ => "",
                        };
                        view! {
                            <div class="deed-2d-body">
                                <div class="deed-2d-secondary-box">
                                    <span style=format!("font-size: 13px; line-height: 20px; color: {};", TEXT_SECONDARY)>{desc}</span>
                                </div>
                            </div>
                        }.into_any()
                    }
                };

                let my_id = my_player_id.get();
                let is_my_turn = my_id.is_some() && st.current_player().map(|p| p.id) == my_id;
                let is_owned_by_me = my_id.is_some() && prop_state.owner == my_id;

                let busy = is_rolling.get() || is_animating_move.get();
                let owner_actions_view = if st.is_game_started && !st.is_game_over && is_owned_by_me {
                    if let Some(me) = my_id.and_then(|id| st.find_player(id)) {
                        match &tile.kind {
                            TileKind::Property(p) => {
                                let d_tiles = get_district_tiles(p.district);
                                let full_set = st.owns_full_district(me.id, p.district);
                                let any_mortgaged = d_tiles.iter().any(|&t| st.properties[t as usize].is_mortgaged);
                                let min_h = d_tiles.iter().map(|&t| st.properties[t as usize].house_count).min().unwrap_or(0);
                                let max_h = d_tiles.iter().map(|&t| st.properties[t as usize].house_count).max().unwrap_or(0);
                                let h_cnt = prop_state.house_count;

                                let can_build = !busy
                                    && is_my_turn
                                    && full_set
                                    && !any_mortgaged
                                    && h_cnt < 5
                                    && h_cnt == min_h
                                    && me.cash >= p.house_cost;
                                let can_sell = !busy && is_my_turn && h_cnt > 0 && h_cnt == max_h;
                                let can_mortgage = !busy && is_my_turn && !prop_state.is_mortgaged && max_h == 0;
                                let unmortgage_cost = Money((p.mortgage_value.0 * 11) / 10);
                                let can_unmortgage = !busy && is_my_turn && prop_state.is_mortgaged && me.cash >= unmortgage_cost;

                                let hint = if !is_my_turn {
                                    Some("Chỉ thao tác được trong lượt của bạn".to_string())
                                } else if !full_set {
                                    Some(format!("Cần độc quyền {} để xây nhà", get_district_label(p.district)))
                                } else if any_mortgaged {
                                    Some("Cần chuộc lại các ô thế chấp trong quận trước khi xây".to_string())
                                } else if h_cnt >= 5 {
                                    Some("Đã đạt cấp Khách sạn tối đa".to_string())
                                } else if h_cnt > min_h {
                                    Some("Cần xây đều các ô trong quận".to_string())
                                } else if me.cash < p.house_cost {
                                    Some(format!("Cần {} để xây thêm", p.house_cost))
                                } else {
                                    None
                                };

                                let build_label = if h_cnt == 4 {
                                    format!("Lên Khách sạn ({})", p.house_cost)
                                } else {
                                    format!("Xây nhà ({})", p.house_cost)
                                };
                                let sell_refund = Money(p.house_cost.0 / 2);
                                let on_b = on_action;
                                let on_s = on_action;
                                let on_m = on_action;
                                let on_u = on_action;

                                view! {
                                    <div class="deed-2d-actions" style="padding: 10px 16px 14px;">
                                        <div class="deed-2d-action-row">
                                            <button
                                                class="btn-hud-pill btn-hud-buy"
                                                style="height: 34px; font-size: 12px; flex: 1;"
                                                disabled=move || !can_build
                                                on:click=move |_| {
                                                    if let Some(cb) = on_b {
                                                        cb.run(ClientMessage::BuildHouse { tile_index: sel_idx });
                                                    }
                                                }
                                            >
                                                {build_label}
                                            </button>
                                            {if h_cnt > 0 {
                                                view! {
                                                    <button
                                                        class="btn-hud-pill btn-hud-secondary"
                                                        style="height: 34px; font-size: 12px;"
                                                        disabled=move || !can_sell
                                                        on:click=move |_| {
                                                            if let Some(cb) = on_s {
                                                                cb.run(ClientMessage::SellHouse { tile_index: sel_idx });
                                                            }
                                                        }
                                                    >
                                                        {format!("Bán nhà (+{})", sell_refund)}
                                                    </button>
                                                }.into_any()
                                            } else if prop_state.is_mortgaged {
                                                view! {
                                                    <button
                                                        class="btn-hud-pill btn-hud-secondary"
                                                        style="height: 34px; font-size: 12px;"
                                                        disabled=move || !can_unmortgage
                                                        on:click=move |_| {
                                                            if let Some(cb) = on_u {
                                                                cb.run(ClientMessage::UnmortgageProperty { tile_index: sel_idx });
                                                            }
                                                        }
                                                    >
                                                        {format!("Chuộc ({})", unmortgage_cost)}
                                                    </button>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <button
                                                        class="btn-hud-pill btn-hud-secondary"
                                                        style="height: 34px; font-size: 12px;"
                                                        disabled=move || !can_mortgage
                                                        on:click=move |_| {
                                                            if let Some(cb) = on_m {
                                                                cb.run(ClientMessage::MortgageProperty { tile_index: sel_idx });
                                                            }
                                                        }
                                                    >
                                                        {format!("Thế chấp (+{})", p.mortgage_value)}
                                                    </button>
                                                }.into_any()
                                            }}
                                        </div>
                                        {hint.map(|h| view! { <span class="deed-2d-action-hint">{h}</span> })}
                                    </div>
                                }.into_any()
                            }
                            TileKind::Station(s) => {
                                let m_val = s.mortgage_value;
                                let unm_cost = Money((m_val.0 * 11) / 10);
                                let can_m = !busy && is_my_turn && !prop_state.is_mortgaged;
                                let can_u = !busy && is_my_turn && prop_state.is_mortgaged && me.cash >= unm_cost;
                                let on_m = on_action;
                                let on_u = on_action;
                                view! {
                                    <div class="deed-2d-actions" style="padding: 10px 16px 14px;">
                                        <div class="deed-2d-action-row">
                                            {if prop_state.is_mortgaged {
                                                view! {
                                                    <button
                                                        class="btn-hud-pill btn-hud-secondary"
                                                        style="height: 34px; font-size: 12px; flex: 1;"
                                                        disabled=move || !can_u
                                                        on:click=move |_| {
                                                            if let Some(cb) = on_u {
                                                                cb.run(ClientMessage::UnmortgageProperty { tile_index: sel_idx });
                                                            }
                                                        }
                                                    >
                                                        {format!("Chuộc lại ({})", unm_cost)}
                                                    </button>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <button
                                                        class="btn-hud-pill btn-hud-secondary"
                                                        style="height: 34px; font-size: 12px; flex: 1;"
                                                        disabled=move || !can_m
                                                        on:click=move |_| {
                                                            if let Some(cb) = on_m {
                                                                cb.run(ClientMessage::MortgageProperty { tile_index: sel_idx });
                                                            }
                                                        }
                                                    >
                                                        {format!("Thế chấp (+{})", m_val)}
                                                    </button>
                                                }.into_any()
                                            }}
                                        </div>
                                    </div>
                                }.into_any()
                            }
                            TileKind::Utility(u) => {
                                let m_val = u.mortgage_value;
                                let unm_cost = Money((m_val.0 * 11) / 10);
                                let can_m = !busy && is_my_turn && !prop_state.is_mortgaged;
                                let can_u = !busy && is_my_turn && prop_state.is_mortgaged && me.cash >= unm_cost;
                                let on_m = on_action;
                                let on_u = on_action;
                                view! {
                                    <div class="deed-2d-actions" style="padding: 10px 16px 14px;">
                                        <div class="deed-2d-action-row">
                                            {if prop_state.is_mortgaged {
                                                view! {
                                                    <button
                                                        class="btn-hud-pill btn-hud-secondary"
                                                        style="height: 34px; font-size: 12px; flex: 1;"
                                                        disabled=move || !can_u
                                                        on:click=move |_| {
                                                            if let Some(cb) = on_u {
                                                                cb.run(ClientMessage::UnmortgageProperty { tile_index: sel_idx });
                                                            }
                                                        }
                                                    >
                                                        {format!("Chuộc lại ({})", unm_cost)}
                                                    </button>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <button
                                                        class="btn-hud-pill btn-hud-secondary"
                                                        style="height: 34px; font-size: 12px; flex: 1;"
                                                        disabled=move || !can_m
                                                        on:click=move |_| {
                                                            if let Some(cb) = on_m {
                                                                cb.run(ClientMessage::MortgageProperty { tile_index: sel_idx });
                                                            }
                                                        }
                                                    >
                                                        {format!("Thế chấp (+{})", m_val)}
                                                    </button>
                                                }.into_any()
                                            }}
                                        </div>
                                    </div>
                                }.into_any()
                            }
                            _ => ().into_any(),
                        }
                    } else {
                        ().into_any()
                    }
                } else {
                    ().into_any()
                };

                Some(view! {
                    <div class="deed-2d-backdrop" on:click=move |_| close_card()>
                        <div class="deed-2d-card" on:click=move |ev| ev.stop_propagation()>
                            <div style=format!("background: {}; padding: 14px 16px; display: flex; align-items: flex-start; justify-content: space-between; gap: 12px;", badge_color)>
                                <div style="display: flex; flex-direction: column; gap: 2px;">
                                    <span class="mono" style=format!("font-size: 11px; color: {}; opacity: 0.85;", TEXT_ON_ACCENT)>
                                        {format!("Ô #{:02} · {}", tile.index, category_label)}
                                    </span>
                                    <h3 style=format!("font-size: 20px; line-height: 26px; font-weight: 700; color: {};", TEXT_ON_ACCENT)>
                                        {tile.name.clone()}
                                    </h3>
                                </div>
                                <button
                                    style=format!("background: rgba(0, 0, 0, 0.25); color: {}; border: 1px solid rgba(255, 255, 255, 0.25); border-radius: 6px; padding: 4px 10px; font-size: 12px; font-weight: 500; cursor: pointer;", TEXT_ON_ACCENT)
                                    on:click=move |_| close_card()
                                >
                                    "Đóng"
                                </button>
                            </div>
                            {body_view}
                            {owner_actions_view}
                        </div>
                    </div>
                })
            }}
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
            assert!(
                rx + w <= BOARD_WIDTH + 1e-6,
                "Tile {idx} exceeds BOARD_WIDTH"
            );
            assert!(
                ry + h <= BOARD_HEIGHT + 1e-6,
                "Tile {idx} exceeds BOARD_HEIGHT"
            );

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
                assert!((-16.0..=16.0).contains(&px));
                assert!((-16.0..=16.0).contains(&py));
                assert_eq!(is_red, val == 1 || val == 4);
            }
        }
    }

    #[test]
    fn test_die_3d_settled_transforms_and_pip_masks() {
        for val in 1..=6u8 {
            let mask = die_face_pip_mask(val);
            let pip_count = mask.into_iter().filter(|&b| b).count();
            assert_eq!(
                pip_count, val as usize,
                "Face {val} pip mask count mismatch"
            );
            assert!(!die_3d_settled_transform(val).is_empty());
        }
    }

    #[test]
    fn test_wrap_tile_name_splits_all_board_names_cleanly() {
        for tile in get_board() {
            let lines = wrap_tile_name(&tile.name, 13);
            assert!(!lines.is_empty());
            assert!(
                lines.len() <= 3,
                "Tile {} wrapped into too many lines: {:?}",
                tile.name,
                lines
            );
        }
    }
}

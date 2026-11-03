use leptos::*;
use monopoly_core::board::{get_board, District, TileKind};
use monopoly_core::state::GameState;

pub fn get_tile_center(index: u8) -> (f64, f64) {
    let idx = index as f64;
    if index <= 10 {
        // Bottom Row: 0 (Go) to 10 (Jail)
        (935.0 - (idx * 87.0), 935.0)
    } else if index <= 20 {
        // Left Column: 10 to 20
        (65.0, 935.0 - ((idx - 10.0) * 87.0))
    } else if index <= 30 {
        // Top Row: 20 to 30
        (65.0 + ((idx - 20.0) * 87.0), 65.0)
    } else {
        // Right Column: 30 to 39
        (935.0, 65.0 + ((idx - 30.0) * 87.0))
    }
}

pub fn get_district_color(district: District) -> &'static str {
    match district {
        District::District4 => "#92400e", // Brown
        District::District8 => "#38bdf8", // Light Blue
        District::PhuNhuan => "#f472b6",  // Pink
        District::TanBinh => "#fb923c",   // Orange
        District::District5 => "#ef4444", // Red
        District::District3 => "#facc15", // Yellow
        District::District2 => "#22c55e", // Green
        District::District1 => "#1d4ed8", // Dark Blue
    }
}

pub const PLAYER_COLORS: [&str; 4] = ["#ef4444", "#3b82f6", "#10b981", "#f59e0b"];

#[component]
pub fn Board(
    state: ReadSignal<GameState>,
    #[prop(default = None)] on_tile_click: Option<Callback<u8>>,
) -> impl IntoView {
    let board = get_board();

    view! {
        <div class="board-container">
            <svg class="board-svg" viewBox="0 0 1000 1000">
                <rect x="0" y="0" width="1000" height="1000" fill="#0f172a" rx="16" />
                <rect x="130" y="130" width="740" height="740" fill="#1e293b" rx="8" />

                <text x="500" y="460" font-size="44" font-weight="800" fill="#f8fafc" text-anchor="middle" font-family="'Be Vietnam Pro', sans-serif">
                    "CỜ TỶ PHÚ"
                </text>
                <text x="500" y="520" font-size="36" font-weight="700" fill="#e11d48" text-anchor="middle" font-family="'Be Vietnam Pro', sans-serif">
                    "SÀI GÒN"
                </text>
                <text x="500" y="570" font-size="16" font-weight="500" fill="#94a3b8" text-anchor="middle">
                    "TP. HỒ CHÍ MINH • EDITION"
                </text>

                {board.into_iter().map(|tile| {
                    let idx = tile.index;
                    let (cx, cy) = get_tile_center(idx);
                    let is_corner = idx == 0 || idx == 10 || idx == 20 || idx == 30;

                    let (w, h) = if is_corner {
                        (130.0, 130.0)
                    } else if (idx > 0 && idx < 10) || (idx > 20 && idx < 30) {
                        (87.0, 130.0)
                    } else {
                        (130.0, 87.0)
                    };

                    let rx = cx - (w / 2.0);
                    let ry = cy - (h / 2.0);

                    let header_color = match &tile.kind {
                        TileKind::Property(p) => get_district_color(p.district),
                        TileKind::Station(_) => "#475569",
                        TileKind::Utility(_) => "#7c3aed",
                        TileKind::Tax(_) => "#dc2626",
                        TileKind::Chance => "#ec4899",
                        TileKind::CommunityChest => "#06b6d4",
                        _ => "#334155",
                    };

                    let price_text = match &tile.kind {
                        TileKind::Property(p) => p.cost.to_string(),
                        TileKind::Station(s) => s.cost.to_string(),
                        TileKind::Utility(u) => u.cost.to_string(),
                        TileKind::Tax(t) => t.amount.to_string(),
                        _ => "".to_string(),
                    };

                    let prop_owner = move || {
                        state.get().properties.get(idx as usize).and_then(|p| p.owner)
                    };

                    let house_count = move || {
                        state.get().properties.get(idx as usize).map(|p| p.house_count).unwrap_or(0)
                    };

                    let is_mortgaged = move || {
                        state.get().properties.get(idx as usize).map(|p| p.is_mortgaged).unwrap_or(false)
                    };

                    view! {
                        <g class="tile-rect" on:click=move |_| {
                            if let Some(ref cb) = on_tile_click {
                                cb.call(idx);
                            }
                        }>
                            <rect x=rx y=ry width=w height=h fill="#1e293b" stroke="#334155" stroke-width="1.5" />

                            {if matches!(tile.kind, TileKind::Property(_)) {
                                let (hx, hy, hw, hh) = if idx < 10 {
                                    (rx, ry, w, 22.0)
                                } else if idx < 20 {
                                    (rx + w - 22.0, ry, 22.0, h)
                                } else if idx < 30 {
                                    (rx, ry + h - 22.0, w, 22.0)
                                } else {
                                    (rx, ry, 22.0, h)
                                };
                                view! { <rect x=hx y=hy width=hw height=hh fill=header_color /> }.into_view()
                            } else {
                                view! {}.into_view()
                            }}

                            <text
                                x=cx
                                y=cy - 8.0
                                font-size="10"
                                font-weight="600"
                                fill="#f8fafc"
                                text-anchor="middle"
                            >
                                {tile.name.clone()}
                            </text>

                            {if !price_text.is_empty() {
                                view! {
                                    <text x=cx y=cy + 14.0 font-size="9" font-weight="500" fill="#94a3b8" text-anchor="middle">
                                        {price_text}
                                    </text>
                                }.into_view()
                            } else {
                                view! {}.into_view()
                            }}

                            {move || {
                                if let Some(owner_id) = prop_owner() {
                                    let p_idx = state.get().players.iter().position(|p| p.id == owner_id).unwrap_or(0);
                                    let col = PLAYER_COLORS[p_idx % 4];
                                    view! {
                                        <rect x=rx + 2.0 y=ry + 2.0 width="8" height="8" rx="2" fill=col />
                                    }.into_view()
                                } else {
                                    view! {}.into_view()
                                }
                            }}

                            {move || {
                                let h = house_count();
                                if h > 0 {
                                    let count_str = if h == 5 { "★ Khách Sạn".to_string() } else { format!("{} Nhà", h) };
                                    view! {
                                        <text x=cx y=cy + 26.0 font-size="8" font-weight="700" fill="#fbbf24" text-anchor="middle">
                                            {count_str}
                                        </text>
                                    }.into_view()
                                } else if is_mortgaged() {
                                    view! {
                                        <text x=cx y=cy + 26.0 font-size="8" font-weight="700" fill="#ef4444" text-anchor="middle">
                                            "THẾ CHẤP"
                                        </text>
                                    }.into_view()
                                } else {
                                    view! {}.into_view()
                                }
                            }}
                        </g>
                    }
                }).collect_view()}

                {move || {
                    state.get().players.iter().enumerate().map(|(p_idx, player)| {
                        if player.is_bankrupt {
                            return view! { <g></g> }.into_view();
                        }
                        let (cx, cy) = get_tile_center(player.position);
                        let offset_x = ((p_idx % 2) as f64 - 0.5) * 22.0;
                        let offset_y = ((p_idx / 2) as f64 - 0.5) * 22.0;
                        let px = cx + offset_x;
                        let py = cy + offset_y;
                        let col = PLAYER_COLORS[p_idx % 4];

                        view! {
                            <g class="pawn-marker" id=format!("pawn_{}", p_idx)>
                                <circle cx=px cy=py r="13" fill=col stroke="#ffffff" stroke-width="2.5" filter="drop-shadow(0 2px 4px rgba(0,0,0,0.6))" />
                                <text x=px y=py + 4.5 font-size="11" font-weight="800" fill="#ffffff" text-anchor="middle">
                                    {(p_idx + 1).to_string()}
                                </text>
                            </g>
                        }.into_view()
                    }).collect_view()
                }}
            </svg>
        </div>
    }
}

//! Tài sản của người chơi: mỗi nhóm một dòng (tên, vạch tiến độ, một hành động hoặc gợi ý).

use crate::palette::{get_district_color, get_district_label, ALL_DISTRICTS, TILE_STATION, TILE_UTILITY};
use leptos::*;
use monopoly_core::board::{get_board, get_district_tiles, TileKind};
use monopoly_core::state::GameState;
use monopoly_proto::{ClientMessage, PlayerId};

#[derive(Clone, PartialEq)]
struct Row {
    key: String,
    label: String,
    color: &'static str,
    /// (ô, của mình?)
    slots: Vec<(u8, bool)>,
    /// Ô để xây nhà tiếp theo (đã tính luật xây đều)
    build_on: Option<u8>,
    hint: String,
}

fn rows_for(s: &GameState, me: PlayerId) -> Vec<Row> {
    let board = get_board();
    let my_turn = s.current_player().map(|p| p.id) == Some(me);
    let cash = s.find_player(me).map(|p| p.cash);
    let mut rows = Vec::new();

    for d in ALL_DISTRICTS {
        let tiles = get_district_tiles(d);
        let slots: Vec<(u8, bool)> = tiles.iter().map(|&t| (t, s.properties[t as usize].owner == Some(me))).collect();
        let mine = slots.iter().filter(|x| x.1).count();
        if mine == 0 {
            continue;
        }
        let mut build_on = None;
        let hint;
        if mine < tiles.len() {
            hint = format!("còn {} ô", tiles.len() - mine);
        } else if tiles.iter().any(|&t| s.properties[t as usize].is_mortgaged) {
            hint = "đang thế chấp".to_string();
        } else {
            let target = tiles.iter().copied().min_by_key(|&t| (s.properties[t as usize].house_count, t)).unwrap();
            let houses = s.properties[target as usize].house_count;
            let house_cost = match &board[target as usize].kind {
                TileKind::Property(p) => Some(p.house_cost),
                _ => None,
            };
            if houses >= 5 {
                hint = "tối đa".to_string();
            } else if !my_turn {
                hint = "xây khi tới lượt".to_string();
            } else if house_cost.zip(cash).map(|(c, m)| m < c).unwrap_or(true) {
                hint = "chưa đủ tiền xây".to_string();
            } else {
                build_on = Some(target);
                hint = String::new();
            }
        }
        rows.push(Row {
            key: format!("{d:?}"),
            label: get_district_label(d).to_string(),
            color: get_district_color(d),
            slots,
            build_on,
            hint,
        });
    }

    let group = |pred: fn(&TileKind) -> bool| -> Vec<(u8, bool)> {
        board
            .iter()
            .filter(|t| pred(&t.kind))
            .map(|t| (t.index, s.properties[t.index as usize].owner == Some(me)))
            .collect()
    };
    let stations = group(|k| matches!(k, TileKind::Station(_)));
    let n = stations.iter().filter(|x| x.1).count();
    if n > 0 {
        rows.push(Row { key: "st".into(), label: "Bến xe & ga".into(), color: TILE_STATION, slots: stations, build_on: None, hint: format!("có {n}/4") });
    }
    let utils = group(|k| matches!(k, TileKind::Utility(_)));
    let n = utils.iter().filter(|x| x.1).count();
    if n > 0 {
        rows.push(Row { key: "ut".into(), label: "Điện & nước".into(), color: TILE_UTILITY, slots: utils, build_on: None, hint: format!("có {n}/2") });
    }
    rows
}

#[component]
pub fn Portfolio(
    state: ReadSignal<GameState>,
    my_player_id: ReadSignal<Option<PlayerId>>,
    set_selected_tile: WriteSignal<Option<u8>>,
    on_action: Callback<ClientMessage>,
) -> impl IntoView {
    let rows = create_memo(move |_| match my_player_id.get() {
        Some(me) => state.with(|s| rows_for(s, me)),
        None => Vec::new(),
    });

    view! {
        <div class="card portfolio">
            {move || if rows.with(|r| r.is_empty()) {
                view! { <div class="empty">"Bạn chưa có đất. Dừng ở ô trống để mua."</div> }.into_view()
            } else {
                ().into_view()
            }}
            <For
                each=move || rows.get()
                key=|r| (r.key.clone(), r.slots.clone(), r.build_on, r.hint.clone())
                children=move |row| {
                    let first_mine = row.slots.iter().find(|x| x.1).map(|x| x.0);
                    view! {
                        <div class="pf-row">
                            <div>
                                <button class="pf-name" on:click=move |_| set_selected_tile.set(first_mine)>
                                    <i style=format!("background: {};", row.color)></i>{row.label.clone()}
                                </button>
                                <div class="pf-dots" aria-hidden="true">
                                    {row.slots.iter().map(|(_, mine)| {
                                        let st = if *mine { format!("background: {};", row.color) } else { String::new() };
                                        view! { <i style=st></i> }
                                    }).collect_view()}
                                </div>
                            </div>
                            {match row.build_on {
                                Some(t) => view! {
                                    <button class="btn btn-gold btn-sm" on:click=move |_| on_action.call(ClientMessage::BuildHouse { tile_index: t })>"Xây nhà"</button>
                                }.into_view(),
                                None => view! { <span class="pf-hint">{row.hint.clone()}</span> }.into_view(),
                            }}
                        </div>
                    }
                }
            />
        </div>
    }
}

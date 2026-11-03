use leptos::*;
use monopoly_core::state::GameState;
use monopoly_proto::PlayerId;

pub const PLAYER_COLORS: [&str; 4] = ["#ef4444", "#3b82f6", "#10b981", "#f59e0b"];

#[component]
pub fn Ledger(
    state: ReadSignal<GameState>,
    my_player_id: ReadSignal<Option<PlayerId>>,
) -> impl IntoView {
    view! {
        <div class="panel">
            <h3 style="font-size: 1.1rem; font-weight: 700; color: #f8fafc; border-bottom: 1px solid #334155; padding-bottom: 8px;">
                "Bảng Xếp Hạng & Tài Sản"
            </h3>

            <div style="display: flex; flex-direction: column; gap: 10px; overflow-y: auto;">
                {move || {
                    let st = state.get();
                    let cur_player = st.current_player().map(|p| p.id);

                    st.players.into_iter().enumerate().map(|(idx, p)| {
                        let is_me = my_player_id.get() == Some(p.id);
                        let is_active_turn = cur_player == Some(p.id);
                        let color = PLAYER_COLORS[idx % 4];

                        let border_style = if is_active_turn {
                            "border: 2px solid #fbbf24; background: #1e293b;"
                        } else {
                            "border: 1px solid #334155; background: #0f172a;"
                        };

                        let owned_count = st.properties.iter().filter(|prop| prop.owner == Some(p.id)).count();

                        view! {
                            <div style=format!("padding: 10px; border-radius: 8px; display: flex; flex-direction: column; gap: 6px; {}", border_style)>
                                <div style="display: flex; align-items: center; justify-content: space-between;">
                                    <div style="display: flex; align-items: center; gap: 8px;">
                                        <div style=format!("width: 12px; height: 12px; border-radius: 50%; background: {};", color)></div>
                                        <span style="font-weight: 700; font-size: 0.95rem; color: #f8fafc;">
                                            {p.name}
                                            {if is_me { " (Bạn)" } else { "" }}
                                        </span>
                                    </div>
                                    <span style="font-weight: 800; font-size: 0.95rem; color: #10b981;">
                                        {p.cash.to_string()}
                                    </span>
                                </div>

                                <div style="display: flex; align-items: center; justify-content: space-between; font-size: 0.8rem; color: #94a3b8;">
                                    <span>{format!("Vị trí: Ô {}", p.position)}</span>
                                    <span>{format!("Sở hữu: {} BĐS", owned_count)}</span>
                                </div>

                                <div style="display: flex; gap: 4px; flex-wrap: wrap;">
                                    {if is_active_turn {
                                        view! {
                                            <span style="background: #fbbf24; color: #000; font-size: 0.7rem; font-weight: 700; padding: 2px 6px; border-radius: 4px;">
                                                "LƯỢT ĐI"
                                            </span>
                                        }.into_view()
                                    } else {
                                        view! {}.into_view()
                                    }}

                                    {if p.in_jail {
                                        view! {
                                            <span style="background: #ef4444; color: #fff; font-size: 0.7rem; font-weight: 700; padding: 2px 6px; border-radius: 4px;">
                                                "TRONG TÙ"
                                            </span>
                                        }.into_view()
                                    } else {
                                        view! {}.into_view()
                                    }}

                                    {if !p.is_connected {
                                        view! {
                                            <span style="background: #64748b; color: #fff; font-size: 0.7rem; font-weight: 700; padding: 2px 6px; border-radius: 4px;">
                                                "MẤT KẾT NỐI"
                                            </span>
                                        }.into_view()
                                    } else {
                                        view! {}.into_view()
                                    }}

                                    {if p.is_bankrupt {
                                        view! {
                                            <span style="background: #991b1b; color: #fff; font-size: 0.7rem; font-weight: 700; padding: 2px 6px; border-radius: 4px;">
                                                "PHÁ SẢN"
                                            </span>
                                        }.into_view()
                                    } else {
                                        view! {}.into_view()
                                    }}
                                </div>
                            </div>
                        }
                    }).collect_view()
                }}
            </div>
        </div>
    }
}

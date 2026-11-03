use leptos::*;
use monopoly_core::state::GameState;

pub const PLAYER_COLORS: [&str; 4] = ["#ef4444", "#3b82f6", "#10b981", "#f59e0b"];

#[component]
pub fn Minimap(state: ReadSignal<GameState>, on_select_quadrant: Callback<u8>) -> impl IntoView {
    view! {
        <div style="display: flex; flex-direction: column; align-items: center; gap: 8px;">
            <span style="font-size: 0.8rem; font-weight: 600; color: #94a3b8;">"Bản Đồ Thu Nhỏ (Minimap)"</span>
            <svg width="160" height="160" viewBox="0 0 160 160" style="background: #0f172a; border-radius: 8px; border: 1px solid #334155;">
                <rect x="25" y="135" width="110" height="20" fill="#334155" opacity="0.6" style="cursor: pointer;" on:click=move |_| on_select_quadrant.call(0) />
                <rect x="5" y="25" width="20" height="110" fill="#334155" opacity="0.6" style="cursor: pointer;" on:click=move |_| on_select_quadrant.call(1) />
                <rect x="25" y="5" width="110" height="20" fill="#334155" opacity="0.6" style="cursor: pointer;" on:click=move |_| on_select_quadrant.call(2) />
                <rect x="135" y="25" width="20" height="110" fill="#334155" opacity="0.6" style="cursor: pointer;" on:click=move |_| on_select_quadrant.call(3) />

                <text x="80" y="85" font-size="10" font-weight="700" fill="#64748b" text-anchor="middle">
                    "SÀI GÒN"
                </text>

                {move || {
                    state.get().players.into_iter().enumerate().map(|(idx, p)| {
                        if p.is_bankrupt {
                            return view! { <circle cx="0" cy="0" r="0" /> }.into_view();
                        }
                        let pos = p.position;
                        let (mx, my) = if pos <= 10 {
                            (145.0 - (pos as f64 * 13.0), 145.0)
                        } else if pos <= 20 {
                            (15.0, 145.0 - ((pos as f64 - 10.0) * 13.0))
                        } else if pos <= 30 {
                            (15.0 + ((pos as f64 - 20.0) * 13.0), 15.0)
                        } else {
                            (145.0, 15.0 + ((pos as f64 - 30.0) * 13.0))
                        };

                        let col = PLAYER_COLORS[idx % 4];
                        view! {
                            <circle cx=mx cy=my r="4" fill=col stroke="#ffffff" stroke-width="1" />
                        }.into_view()
                    }).collect_view()
                }}
            </svg>
        </div>
    }
}

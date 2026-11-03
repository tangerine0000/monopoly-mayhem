use leptos::*;

#[component]
pub fn LogFeed(logs: ReadSignal<Vec<String>>) -> impl IntoView {
    view! {
        <div class="panel" style="height: 250px;">
            <h3 style="font-size: 1.1rem; font-weight: 700; color: #f8fafc; border-bottom: 1px solid #334155; padding-bottom: 8px;">
                "Nhật Ký Trận Đấu"
            </h3>
            <div style="display: flex; flex-direction: column; gap: 6px; overflow-y: auto; height: 100%; font-size: 0.85rem;">
                {move || {
                    let items = logs.get();
                    if items.is_empty() {
                        view! {
                            <span style="color: #64748b; font-style: italic;">
                                "Chưa có sự kiện nào..."
                            </span>
                        }.into_view()
                    } else {
                        items.into_iter().rev().map(|line| {
                            view! {
                                <div style="padding: 4px 6px; border-radius: 4px; background: #0f172a; color: #e2e8f0; line-height: 1.4;">
                                    {line}
                                </div>
                            }
                        }).collect_view().into_view()
                    }
                }}
            </div>
        </div>
    }
}

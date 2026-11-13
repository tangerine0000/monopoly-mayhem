use crate::palette::{
    BG_SECONDARY, BORDER_DEFAULT, BORDER_SUBTLE, SURFACE_HOVER, TEXT_MUTED, TEXT_PRIMARY,
    TEXT_SECONDARY,
};
use leptos::*;

#[component]
pub fn LogFeed(logs: ReadSignal<Vec<String>>) -> impl IntoView {
    view! {
        <div class="panel" style="flex: 1; min-height: 160px;">
            <div style=format!(
                "display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid {}; padding-bottom: 8px;",
                BORDER_DEFAULT
            )>
                <span style=format!(
                    "font-size: 14px; line-height: 20px; font-weight: 500; color: {};",
                    TEXT_PRIMARY
                )>
                    "Nhật ký ván đấu"
                </span>
                <span class="mono" style=format!("font-size: 12px; line-height: 16px; color: {};", TEXT_MUTED)>
                    {move || format!("{} mục", logs.get().len())}
                </span>
            </div>
            <div style="display: flex; flex-direction: column; gap: 4px; overflow-y: auto; flex: 1; font-size: 13px; line-height: 20px;">
                {move || {
                    let items = logs.get();
                    let total = items.len();
                    if items.is_empty() {
                        view! {
                            <span style=format!("color: {};", TEXT_MUTED)>
                                "Chưa có sự kiện nào"
                            </span>
                        }.into_view()
                    } else {
                        items.into_iter().rev().enumerate().map(|(rev_idx, line)| {
                            let is_newest = rev_idx == 0;
                            let step_num = total.saturating_sub(rev_idx);
                            let cls = if is_newest { "log-item-enter" } else { "" };
                            let row_bg = if is_newest { SURFACE_HOVER } else { BG_SECONDARY };
                            let row_border = if is_newest { BORDER_SUBTLE } else { BORDER_DEFAULT };
                            let text_col = if is_newest { TEXT_PRIMARY } else { TEXT_SECONDARY };
                            view! {
                                <div
                                    class=cls
                                    style=format!(
                                        "padding: 8px 12px; border-radius: 8px; background: {}; color: {}; border: 1px solid {}; display: flex; align-items: flex-start; gap: 8px;",
                                        row_bg, text_col, row_border
                                    )
                                >
                                    <span class="mono" style=format!("font-size: 11px; line-height: 20px; color: {}; flex-shrink: 0;", TEXT_MUTED)>
                                        {format!("#{:02}", step_num)}
                                    </span>
                                    <span>{line}</span>
                                </div>
                            }
                        }).collect_view().into_view()
                    }
                }}
            </div>
        </div>
    }
}

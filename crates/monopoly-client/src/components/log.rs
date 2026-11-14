//! Bản tin: mỗi sự kiện một dòng, chỉ hiện vài dòng mới nhất; mở rộng khi cần.

use leptos::prelude::*;

const COLLAPSED_COUNT: usize = 5;

#[component]
pub fn LogFeed(logs: ReadSignal<Vec<String>>) -> impl IntoView {
    let (expanded, set_expanded) = create_signal(false);

    view! {
        <div class="card feed" aria-live="polite">
            <div class="feed-list">
                {move || {
                    let open = expanded.get();
                    logs.with(|items| {
                        let shown = if open { items.len() } else { COLLAPSED_COUNT };
                        items
                            .iter()
                            .rev()
                            .take(shown)
                            .enumerate()
                            .map(|(i, line)| {
                                let mut cls = String::from("ev");
                                if !open {
                                    cls.push_str(" collapsed");
                                }
                                if i == 0 {
                                    cls.push_str(" new");
                                }
                                view! { <div class=cls title=line.clone()><span>{line.clone()}</span></div> }
                            })
                            .collect_view()
                    })
                }}
            </div>
            {move || (logs.with(|l| l.len()) > COLLAPSED_COUNT).then(|| view! {
                <div class="more-row">
                    <button class="linkbtn" on:click=move |_| set_expanded.update(|v| *v = !*v)>
                        {move || if expanded.get() {
                            "Thu gọn".to_string()
                        } else {
                            format!("Xem toàn bộ {} sự kiện", logs.with(|l| l.len()))
                        }}
                    </button>
                </div>
            })}
        </div>
    }
}

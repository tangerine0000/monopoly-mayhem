mod app;
mod components;
pub mod i18n;
pub mod palette;
pub mod store;
mod ws;

use app::App;
use leptos::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App /> });
}

//! Papicons, the only icon set used by Recoder.
//!
//! The SVGs are vendored from the Papicons repository (MIT) and inlined at
//! compile time so the app stays fully offline. They are recoloured through
//! `currentColor`, which keeps them consistent with the design tokens.

use dioxus::prelude::*;

pub fn source(name: &str) -> &'static str {
    match name {
        "play" => include_str!("../../assets/papicons/play.svg"),
        "pause" => include_str!("../../assets/papicons/pause.svg"),
        "trash" => include_str!("../../assets/papicons/trash.svg"),
        "check" => include_str!("../../assets/papicons/check.svg"),
        "list" => include_str!("../../assets/papicons/list.svg"),
        "pie" => include_str!("../../assets/papicons/pie.svg"),
        "plus" => include_str!("../../assets/papicons/plus.svg"),
        "minus" => include_str!("../../assets/papicons/minus.svg"),
        "cross" => include_str!("../../assets/papicons/cross.svg"),
        _ => include_str!("../../assets/papicons/info.svg"),
    }
}

#[component]
pub fn Papicon(name: String, #[props(default = 22.0)] size: f64) -> Element {
    rsx! {
        span {
            class: "papicon",
            style: "width:{size}px;height:{size}px",
            dangerous_inner_html: "{source(&name)}",
        }
    }
}

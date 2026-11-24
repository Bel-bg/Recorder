use dioxus::prelude::*;

use crate::state::{AppContext, Screen};
use crate::ui::{MicGlyph, Papicon};

#[component]
pub fn NavBar() -> Element {
    let context = use_context::<AppContext>();
    let current = context.screen.cloned();
    let practice = context.clone();
    let history = context.clone();
    let stats = context.clone();

    rsx! {
        nav { class: "nav", "aria-label": "Navigation principale",
            div { class: "nav-inner",
                button {
                    class: if current == Screen::Record { "nav-item is-active" } else { "nav-item" },
                    onclick: move |_| practice.go_to(Screen::Record),
                    MicGlyph { size: 17.0 }
                    span { class: "nav-label", "Pratique" }
                }
                button {
                    class: if current == Screen::History { "nav-item is-active" } else { "nav-item" },
                    onclick: move |_| history.go_to(Screen::History),
                    Papicon { name: "list", size: 18.0 }
                    span { class: "nav-label", "Historique" }
                }
                button {
                    class: if current == Screen::Stats { "nav-item is-active" } else { "nav-item" },
                    onclick: move |_| stats.go_to(Screen::Stats),
                    Papicon { name: "pie", size: 18.0 }
                    span { class: "nav-label", "Stats" }
                }
            }
        }
    }
}

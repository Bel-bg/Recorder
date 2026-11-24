use dioxus::prelude::*;
use recoder_core::format as text;

use crate::state::AppContext;
use crate::ui::Papicon;

#[component]
pub fn PlayerPanel() -> Element {
    let context = use_context::<AppContext>();
    let playback = context.playback;
    if !playback.open.cloned() {
        return rsx! {};
    }

    let duration = playback.duration_ms.cloned();
    let position = playback.position_ms.cloned().min(duration.max(1));
    let playing = playback.playing.cloned();
    let percent = if duration > 0 {
        (position as f64 / duration as f64 * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    let toggle = context.clone();
    let seek = context.clone();
    let stop = context.clone();
    let close = context.clone();

    rsx! {
        section { class: "player", "aria-label": "Lecteur",
            div { class: "player-head",
                div { class: "player-times",
                    span { class: "player-now", "{text::clock(position as i64)}" }
                    span { class: "player-total", "{text::clock(duration as i64)}" }
                }
                button {
                    class: "player-close",
                    "aria-label": "Fermer le lecteur",
                    onclick: move |_| close.close_player(),
                    Papicon { name: "cross", size: 15.0 }
                }
            }
            input {
                r#type: "range",
                class: "seek",
                min: "0",
                max: "{duration.max(1)}",
                value: "{position}",
                style: "--p:{percent}%",
                "aria-label": "Position de lecture",
                oninput: move |event| {
                    if let Ok(value) = event.value().parse::<u64>() {
                        seek.seek_playback(value);
                    }
                },
            }
            div { class: "player-controls",
                button {
                    class: "player-play",
                    "aria-label": if playing { "Pause" } else { "Lire" },
                    onclick: move |_| toggle.toggle_playback(),
                    Papicon { name: (if playing { "pause" } else { "play" }).to_string(), size: 22.0 }
                }
                button {
                    class: "player-stop",
                    "aria-label": "Arrêter",
                    onclick: move |_| stop.stop_playback(),
                    span { class: "stop-square" }
                }
            }
        }
    }
}

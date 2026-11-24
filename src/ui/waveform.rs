use dioxus::prelude::*;

/// Amplitude bars fed by the real microphone level, oldest on the left.
#[component]
pub fn Waveform(levels: Vec<f32>, live: bool) -> Element {
    rsx! {
        div { class: if live { "wave is-live" } else { "wave" },
            for (index, level) in levels.iter().enumerate() {
                span {
                    key: "{index}",
                    class: "wave-bar",
                    style: "transform: scaleY({bar_scale(*level)})",
                }
            }
        }
    }
}

fn bar_scale(level: f32) -> String {
    let shaped = level.clamp(0.0, 1.0).sqrt();
    format!("{:.3}", 0.06 + shaped * 0.94)
}

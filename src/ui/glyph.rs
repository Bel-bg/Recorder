use dioxus::prelude::*;

/// Microphone drawn in CSS: Papicons has no microphone glyph, and adding a
/// second icon set was out of scope.
#[component]
pub fn MicGlyph(#[props(default = 24.0)] size: f64) -> Element {
    let scale = size / 24.0;
    rsx! {
        span {
            class: "mic",
            style: "width:{size}px;height:{size}px;--mic-scale:{scale}",
            span { class: "mic-shape",
                span { class: "mic-body" }
                span { class: "mic-arc" }
                span { class: "mic-stem" }
                span { class: "mic-base" }
            }
        }
    }
}

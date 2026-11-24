//! Application shell: providers, gestures, screen transitions and navigation.

use std::sync::Arc;
use std::time::Duration;

use dioxus::prelude::*;
use recoder_core::Database;

use crate::audio::{AudioEngine, Take};
use crate::pages::{HistoryPage, RecordPage, StatsPage};
use crate::state::{AppContext, Capture, Playback, Screen, Studio, BAR_COUNT};
use crate::storage::AppPaths;
use crate::ui::{NavBar, PlayerPanel};

type Bootstrap = Result<(AppPaths, Arc<Database>), String>;

#[component]
pub fn App() -> Element {
    let storage = use_hook(bootstrap);
    let _ = use_context_provider(|| storage.clone());
    match storage {
        Ok(_) => rsx! { Shell {} },
        Err(message) => rsx! {
            style { dangerous_inner_html: include_str!("../assets/main.css") }
            div { class: "app",
                div { class: "fatal",
                    p { class: "fatal-title", "Recoder" }
                    p { class: "fatal-text", "Le stockage local n'est pas accessible." }
                    p { class: "fatal-detail", "{message}" }
                }
            }
        },
    }
}

fn bootstrap() -> Bootstrap {
    let paths = AppPaths::detect().map_err(|err| err.to_string())?;
    let database = Database::open(&paths.database).map_err(|err| err.to_string())?;
    Ok((paths, Arc::new(database)))
}

#[component]
fn Shell() -> Element {
    let storage = use_context::<Bootstrap>();
    let Ok((paths, db)) = storage else {
        return rsx! {};
    };
    let studio = Studio {
        state: use_signal(|| Capture::Idle),
        take: use_signal(|| None::<Take>),
        error: use_signal(|| None::<String>),
        elapsed_ms: use_signal(|| 0u64),
        bars: use_signal(|| vec![0.0f32; BAR_COUNT]),
    };
    let playback = Playback {
        recording_id: use_signal(|| None::<String>),
        path: use_signal(|| None::<String>),
        duration_ms: use_signal(|| 0u64),
        position_ms: use_signal(|| 0u64),
        playing: use_signal(|| false),
        open: use_signal(|| false),
    };
    let screen = use_signal(|| Screen::Record);
    let direction = use_signal(|| 1i8);
    let revision = use_signal(|| 0u64);
    let audio = use_hook(|| Arc::new(AudioEngine::new()));

    let context = AppContext {
        paths,
        db,
        audio,
        studio,
        playback,
        screen,
        direction,
        revision,
    };
    use_context_provider(|| context.clone());

    let tick_context = context.clone();
    use_hook(move || {
        spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(90)).await;
                tick_context.tick();
            }
        });
    });

    let gesture = use_signal(|| None::<(f64, f64)>);
    let current = screen();
    let entering = if direction.cloned() >= 0 {
        "stage enter-right"
    } else {
        "stage enter-left"
    };
    let screen_key = match current {
        Screen::Record => "record",
        Screen::History => "history",
        Screen::Stats => "stats",
    };

    let swipe_context = context.clone();

    rsx! {
        style { dangerous_inner_html: include_str!("../assets/main.css") }
        div {
            class: "app",
            onpointerdown: move |event: Event<PointerData>| {
                let start = event.data().client_coordinates();
                let mut gesture = gesture;
                gesture.set(Some((start.x, start.y)));
            },
            onpointerup: move |event: Event<PointerData>| {
                let mut gesture = gesture;
                let Some((start_x, start_y)) = gesture.cloned() else { return };
                gesture.set(None);
                let end = event.data().client_coordinates();
                let dx = end.x - start_x;
                let dy = end.y - start_y;
                if dx.abs() < 64.0 || dx.abs() < dy.abs() * 1.5 {
                    return;
                }
                let goal = match (swipe_context.screen.cloned(), dx < 0.0) {
                    (Screen::Record, true) => Some(Screen::History),
                    (Screen::Record, false) => Some(Screen::Stats),
                    (Screen::History, false) => Some(Screen::Record),
                    (Screen::Stats, true) => Some(Screen::Record),
                    _ => None,
                };
                if let Some(goal) = goal {
                    swipe_context.go_to(goal);
                }
            },
            main { class: "{entering}", key: "{screen_key}",
                match current {
                    Screen::Record => rsx! { RecordPage {} },
                    Screen::History => rsx! { HistoryPage {} },
                    Screen::Stats => rsx! { StatsPage {} },
                }
            }
            PlayerPanel {}
            NavBar {}
        }
    }
}

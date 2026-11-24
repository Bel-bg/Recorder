use dioxus::prelude::*;
use recoder_core::format as text;
use recoder_core::stats;
use recoder_core::Recording;

use crate::state::{AppContext, Screen};
use crate::ui::Papicon;

#[component]
pub fn HistoryPage() -> Element {
    let context = use_context::<AppContext>();
    let mut rows = use_signal(Vec::<Recording>::new);
    let pending = use_signal(|| None::<String>);
    let revision = context.revision;
    let loader = context.clone();
    use_effect(move || {
        let _ = revision.cloned();
        rows.set(loader.recordings());
    });

    let items = rows.cloned();
    let today = text::today();
    let groups = stats::group_by_day(&items);
    let total_ms: i64 = items.iter().map(|item| item.duration_ms).sum();
    let count_label = match items.len() {
        1 => "1 prise".to_string(),
        count => format!("{count} prises"),
    };
    let subtitle = format!("{count_label} · {}", text::practice_time(total_ms));
    let start = context.clone();

    rsx! {
        section { class: "page history-page",
            header { class: "page-head",
                h1 { class: "page-title", "Historique" }
                p { class: "page-sub", "{subtitle}" }
            }

            if items.is_empty() {
                div { class: "empty",
                    span { class: "empty-icon", Papicon { name: "list", size: 24.0 } }
                    p { class: "empty-title", "Aucune prise" }
                    p { class: "empty-text", "Vos enregistrements apparaîtront ici, jour après jour." }
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| start.go_to(Screen::Record),
                        "Commencer"
                    }
                }
            } else {
                for (day, day_items) in groups {
                    div { class: "day-group", key: "{day}",
                        p { class: "day-label", "{text::day_label(day, today)}" }
                        for recording in day_items {
                            HistoryRow {
                                key: "{recording.id}",
                                recording: recording.clone(),
                                waiting: pending.cloned() == Some(recording.id.clone()),
                                pending,
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn HistoryRow(recording: Recording, waiting: bool, pending: Signal<Option<String>>) -> Element {
    let context = use_context::<AppContext>();
    let play_context = context.clone();
    let delete_context = context.clone();

    let play_main = recording.clone();
    let play_button = recording.clone();
    let ask_delete = recording.clone();
    let cancel_delete = recording.clone();

    rsx! {
        div { class: if waiting { "row is-waiting" } else { "row" },
            button {
                class: "row-main",
                onclick: move |_| play_context.open_recording(&play_main),
                span { class: "row-time", "{text::time_of_day(recording.created_at)}" }
                span { class: "row-duration", "{text::duration(recording.duration_ms)}" }
            }
            if waiting {
                div { class: "row-actions",
                    button {
                        class: "chip chip-danger",
                        onclick: move |_| {
                            let mut target = pending;
                            delete_context.delete_recording(&ask_delete);
                            target.set(None);
                        },
                        "Supprimer"
                    }
                    button {
                        class: "chip",
                        onclick: move |_| {
                            let mut target = pending;
                            target.set(None);
                        },
                        "Annuler"
                    }
                }
            } else {
                div { class: "row-actions",
                    button {
                        class: "icon-btn accent",
                        "aria-label": "Lire la prise",
                        onclick: move |_| {
                            let player = context.clone();
                            player.open_recording(&play_button);
                        },
                        Papicon { name: "play", size: 16.0 }
                    }
                    button {
                        class: "icon-btn",
                        "aria-label": "Supprimer la prise",
                        onclick: move |_| {
                            let mut target = pending;
                            target.set(Some(cancel_delete.id.clone()));
                        },
                        Papicon { name: "trash", size: 16.0 }
                    }
                }
            }
        }
    }
}

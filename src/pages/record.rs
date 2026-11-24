use dioxus::prelude::*;
use recoder_core::format as text;
use recoder_core::stats;

use crate::state::{AppContext, Capture};
use crate::ui::{MicGlyph, Papicon, Waveform};

#[component]
pub fn RecordPage() -> Element {
    let context = use_context::<AppContext>();
    let studio = context.studio;
    let capture = studio.state.cloned();
    let recording = capture == Capture::Recording;

    let mut stats_today = use_signal(|| (0u64, 0i64, 10u32));
    let revision = context.revision;
    let loader = context.clone();
    use_effect(move || {
        let _ = revision.cloned();
        let summary = stats::summarize(&loader.recordings(), text::today());
        stats_today.set((
            summary.today_count,
            summary.today_ms,
            loader.daily_goal_minutes(),
        ));
    });

    let (count, today_ms, goal_minutes) = stats_today.cloned();
    let activity = match count {
        0 => "Aucune prise aujourd'hui".to_string(),
        1 => "1 prise aujourd'hui".to_string(),
        other => format!("{other} prises aujourd'hui"),
    };
    let goal_ms = goal_minutes as f64 * 60_000.0;
    let progress = (today_ms as f64 / goal_ms.max(1.0)).clamp(0.0, 1.0);

    let elapsed = studio.elapsed_ms.cloned() as i64;
    let status = match capture {
        Capture::Idle => "Prêt à parler",
        Capture::Permission | Capture::Arming => "Microphone",
        Capture::Recording => "Enregistrement en cours",
        Capture::Review => "Prise terminée",
        Capture::Saved => "Prise enregistrée",
    };
    let hint = if recording {
        "Appuyez pour arrêter"
    } else {
        "Appuyez pour enregistrer"
    };
    let status_class = match capture {
        Capture::Recording => "status is-live",
        Capture::Saved => "status is-saved",
        _ => "status",
    };

    let action = context.clone();
    let error = studio.error.cloned();

    rsx! {
        section { class: "page record-page",
            header { class: "topbar",
                span { class: "brand", "Recoder" }
                span { class: "today", "{activity}" }
            }

            div { class: "goal-strip",
                div { class: "goal-line",
                    span { class: "goal-label", "Objectif du jour" },
                    span { class: "goal-value", "{text::practice_time(today_ms)} / {goal_minutes} min" },
                }
                div { class: "progress",
                    div { class: "progress-fill", style: "transform: scaleX({progress})" }
                }
            }

            div { class: "recorder",
                div { class: "timer-block",
                    span { class: if recording { "timer is-live" } else { "timer" }, "{text::clock(elapsed)}" }
                    span { class: status_class, "{status}" }
                }

                Waveform { levels: studio.bars.cloned(), live: recording }

                if capture == Capture::Review {
                    TakeCard {}
                } else if capture == Capture::Permission || capture == Capture::Arming {
                    PermissionCard { arming: capture == Capture::Arming }
                } else {
                    div { class: "rec-wrap",
                        button {
                            class: if recording { "rec is-live" } else { "rec" },
                            "aria-label": if recording { "Arrêter l'enregistrement" } else { "Démarrer l'enregistrement" },
                            onclick: move |_| action.handle_record_button(),
                            span { class: "rec-ring" }
                            span { class: "rec-ring second" }
                            if recording {
                                span { class: "stop-glyph" }
                            } else {
                                MicGlyph { size: 58.0 }
                            }
                        }
                        span { class: "hint", "{hint}" }
                    }
                }
            }

            if let Some(message) = error {
                div { class: "banner",
                    Papicon { name: "info", size: 16.0 }
                    span { class: "banner-text", "{message}" }
                }
            }
        }
    }
}

#[component]
fn TakeCard() -> Element {
    let context = use_context::<AppContext>();
    let studio = context.studio;
    let playback = context.playback;
    let Some(take) = studio.take.cloned() else {
        return rsx! {};
    };

    let path = take.path.to_string_lossy().to_string();
    let current = playback.path.cloned() == Some(path);
    let playing = current && playback.playing.cloned();
    let position = if current { playback.position_ms.cloned() } else { 0 };
    let total = if current { playback.duration_ms.cloned() } else { 0 };
    let ratio = if total > 0 {
        (position as f64 / total as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let listen = context.clone();
    let keep = context.clone();
    let drop_take = context.clone();

    rsx! {
        div { class: "take",
            div { class: "take-head",
                div { class: "take-meta",
                    span { class: "take-label", "Prise" },
                    span { class: "take-duration", "{text::duration(take.duration_ms)}" },
                }
                button {
                    class: "take-play",
                    "aria-label": if playing { "Pause" } else { "Écouter la prise" },
                    onclick: move |_| listen.toggle_take_playback(),
                    Papicon { name: (if playing { "pause" } else { "play" }).to_string(), size: 22.0 }
                }
            }
            div { class: "progress thin",
                div { class: "progress-fill", style: "transform: scaleX({ratio})" }
            }
            div { class: "take-actions",
                button {
                    class: "btn btn-primary",
                    onclick: move |_| keep.save_take(),
                    Papicon { name: "check", size: 18.0 }
                    span { "Garder" }
                }
                button {
                    class: "btn btn-ghost",
                    onclick: move |_| drop_take.discard_take(),
                    Papicon { name: "trash", size: 18.0 }
                    span { "Supprimer" }
                }
            }
        }
    }
}

#[component]
fn PermissionCard(arming: bool) -> Element {
    let context = use_context::<AppContext>();
    let authorize = context.clone();
    rsx! {
        div { class: "take",
            div { class: "take-head",
                div { class: "take-meta",
                    span { class: "take-label", "Microphone" },
                    span { class: "take-duration", if arming { "Autorisation en cours" } else { "Requis pour enregistrer" } },
                }
                Papicon { name: "info", size: 20.0 }
            }
            p { class: "take-text",
                "Votre voix est enregistrée dans l'espace privé de l'application. Aucun fichier ne quitte ce téléphone."
            }
            button {
                class: "btn btn-primary",
                onclick: move |_| authorize.authorize_microphone(),
                "Autoriser le microphone"
            }
        }
    }
}

use dioxus::prelude::*;
use recoder_core::format as text;
use recoder_core::stats;
use recoder_core::Recording;

use crate::state::AppContext;
use crate::ui::{Papicon, Waveform};

#[component]
pub fn StatsPage() -> Element {
    let context = use_context::<AppContext>();
    let mut rows = use_signal(Vec::<Recording>::new);
    let goal = use_signal(|| context.daily_goal_minutes());

    let revision = context.revision;
    let loader = context.clone();
    use_effect(move || {
        let _ = revision.cloned();
        rows.set(loader.recordings());
    });

    let today = text::today();
    let items = rows.cloned();
    let summary = stats::summarize(&items, today);
    let week = stats::week_activity(&items, today);
    let goal_minutes = goal.cloned();
    let goal_ms = (goal_minutes as f64 * 60_000.0).max(1.0);
    let ratio = (summary.today_ms as f64 / goal_ms).clamp(0.0, 1.0);
    let reached = summary.today_ms as f64 >= goal_ms;
    let peak = week.iter().map(|day| day.ms).max().unwrap_or(0).max(1);

    let mut decrease = goal;
    let mut increase = goal;
    let save_minus = context.clone();
    let save_plus = context.clone();

    rsx! {
        section { class: "page stats-page",
            header { class: "page-head",
                h1 { class: "page-title", "Statistiques" }
                p { class: "page-sub", "Votre pratique, calculée sur vos prises" }
            }

            div { class: "card goal-card",
                div { class: "goal-head",
                    div {
                        p { class: "card-label", "Objectif du jour" }
                        p { class: "goal-big",
                            span { class: if reached { "goal-done" } else { "" }, "{text::practice_time(summary.today_ms)}" }
                            span { class: "goal-target", " / {goal_minutes} min" }
                        }
                    }
                    div { class: "stepper",
                        button {
                            class: "icon-btn",
                            "aria-label": "Réduire l'objectif",
                            onclick: move |_| {
                                let next = goal_minutes.saturating_sub(5).max(5);
                                decrease.set(next);
                                save_minus.set_daily_goal_minutes(next);
                            },
                            Papicon { name: "minus", size: 16.0 }
                        }
                        button {
                            class: "icon-btn",
                            "aria-label": "Augmenter l'objectif",
                            onclick: move |_| {
                                let next = (goal_minutes + 5).min(180);
                                increase.set(next);
                                save_plus.set_daily_goal_minutes(next);
                            },
                            Papicon { name: "plus", size: 16.0 }
                        }
                    }
                }
                div { class: "progress",
                    div { class: "progress-fill", style: "transform: scaleX({ratio})" }
                }
                p { class: "goal-caption",
                    if summary.today_count == 0 {
                        "Aucune prise aujourd'hui."
                    } else if summary.today_count == 1 {
                        "1 prise aujourd'hui."
                    } else {
                        "{summary.today_count} prises aujourd'hui."
                    }
                }
            }

            div { class: "stat-grid",
                div { class: "card stat",
                    p { class: "stat-value", "{summary.total_count}" }
                    p { class: "stat-label", "Prises" }
                }
                div { class: "card stat",
                    p { class: "stat-value", "{text::practice_time(summary.total_ms)}" }
                    p { class: "stat-label", "Temps de parole" }
                }
                div { class: "card stat",
                    p { class: "stat-value", "{text::duration(summary.average_ms)}" }
                    p { class: "stat-label", "Durée moyenne" }
                }
                div { class: "card stat",
                    p { class: "stat-value", "{summary.active_days}" }
                    p { class: "stat-label", "Jours actifs" }
                }
            }

            div { class: "card week-card",
                div { class: "card-head",
                    p { class: "card-label", "Sept derniers jours" }
                    p { class: "card-hint", "{text::practice_time(week.iter().map(|day| day.ms).sum::<i64>())}" }
                }
                div { class: "week",
                    for day in week {
                        div { class: "week-day", key: "{day.date}",
                            div { class: "week-track",
                                div {
                                    class: if day.ms > 0 { "week-bar is-active" } else { "week-bar" },
                                    style: "transform: scaleY({bar_scale(day.ms, peak)})",
                                }
                            }
                            span { class: "week-label", "{text::weekday_initial(day.date)}" }
                        }
                    }
                }
            }

            if items.is_empty() {
                div { class: "empty small",
                    Waveform { levels: vec![0.02; 18], live: false }
                    p { class: "empty-text", "Enregistrez une première prise pour lancer vos statistiques." }
                }
            }
        }
    }
}

fn bar_scale(ms: i64, peak: i64) -> String {
    let ratio = (ms as f64 / peak as f64).clamp(0.0, 1.0);
    format!("{:.3}", (ratio * 0.94 + 0.06).min(1.0))
}

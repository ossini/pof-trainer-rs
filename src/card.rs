//! One question: header with markers, text (look-alike parts highlighted),
//! shuffled answers, explanation and – in block modes – a "Weiter" button.

use crate::icons::{pick, Icon};
use crate::model::question;
use crate::util::{applog::log, shuffle_answers};
use crate::{use_app, AppState};
use dioxus::prelude::*;
use serde_json::json;

const CHEERS: [&str; 7] = [
    "Volltreffer!",
    "Butterweich gelandet!",
    "Clear for take-off!",
    "Sauber getrimmt!",
    "Wie ein Profi!",
    "Perfekter Anflug!",
    "Check – sitzt!",
];

const LETTERS: [&str; 4] = ["A", "B", "C", "D"];

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Badge {
    Test,
    Retry,
    Surprise,
}

pub fn toggle_favorite(mut app: Signal<AppState>, id: u32) {
    app.write().p.toggle_favorite(id);
    log("favorite.toggle", json!({ "id": id, "caa": question(id).map(|q| q.caa_id.clone()) }));
}

pub fn toggle_ultra(mut app: Signal<AppState>, id: u32) {
    app.write().p.toggle_ultra(id);
    log("ultraHard.toggle", json!({ "id": id, "caa": question(id).map(|q| q.caa_id.clone()) }));
}

#[component]
pub fn QuestionCard(
    qid: u32,
    /// varies the answer order, e.g. for repeats in a block
    salt: u32,
    number: usize,
    total: usize,
    /// earlier result shown in the list view (cannot be answered again there)
    #[props(!optional, default)]
    preanswered: Option<bool>,
    on_answer: EventHandler<bool>,
    #[props(!optional, default)]
    badge: Option<Badge>,
    #[props(default)] surprise: bool,
    /// no look-alike hint / highlights (tests)
    #[props(default)] hide_similar: bool,
    /// when set: stay on the question after answering and show "Weiter"
    #[props(!optional, default)]
    on_next: Option<EventHandler<()>>,
    #[props(default)] next_label: String,
) -> Element {
    let app = use_app();
    let shuffled = use_hook(move || question(qid).map(|q| shuffle_answers(q, salt)));
    let mut selected = use_signal(|| None::<usize>);
    let mut show_explain = use_signal(|| false);
    // only answers from before this card opened count as "earlier" (the
    // answer given here updates the progress right away)
    let preanswered = use_hook(move || preanswered);

    let (Some(q), Some(sh)) = (question(qid), shuffled) else {
        return rsx! {};
    };

    let (is_fav, is_ultra) = {
        let a = app.read();
        (a.p.favorites.contains(&qid), a.p.ultra_hard.contains(&qid))
    };
    let picked = selected();
    let reveal = picked.is_some() || preanswered.is_some();
    let similar = if hide_similar { None } else { q.similar.clone() };
    let correct_now = picked.map(|i| i == sh.correct);

    let answer_class = move |i: usize| -> &'static str {
        if !reveal {
            "ans"
        } else if i == sh.correct {
            "ans is-correct"
        } else if picked == Some(i) {
            "ans is-wrong"
        } else {
            "ans is-dim"
        }
    };

    let card_class = if surprise { "qcard surprise" } else { "qcard" };
    let segments = similar.as_ref().and_then(|s| s.segments.clone());
    let qtext = q.question.clone();
    let caa_id = q.caa_id.clone();

    rsx! {
        div { class: "{card_class}",
            div { class: "qhead",
                div { class: "qmeta",
                span { class: "chip mono", {caa_id} }
                span { class: "qnum", "{number} / {total}" }
                {match badge {
                    Some(Badge::Surprise) => rsx! { span { class: "badge b-surprise", Icon { name: "sparkles" } "Überraschung" } },
                    Some(Badge::Test) => rsx! { span { class: "badge b-test", Icon { name: "test" } "Test" } },
                    Some(Badge::Retry) => rsx! { span { class: "badge b-retry", Icon { name: "retry" } "Wiederholung" } },
                    None => rsx! {},
                }}
                }
                div { class: "qactions",
                // explanation behind an icon (only once answered, so it never spoils)
                if reveal && q.explanation.is_some() {
                    button {
                        class: "iconbtn explain-btn",
                        title: "KI-Erklärung",
                        "aria-label": "KI-Erklärung anzeigen",
                        onclick: move |_| show_explain.set(true),
                        Icon { name: "bulb" }
                    }
                }
                button {
                    class: if is_ultra { "iconbtn ultra on" } else { "iconbtn ultra" },
                    title: "ULTRA SCHWER",
                    "aria-label": if is_ultra { "ULTRA SCHWER entfernen" } else { "Als ULTRA SCHWER markieren" },
                    onclick: move |_| toggle_ultra(app, qid),
                    Icon { name: pick(is_ultra, "gem-fill", "gem") }
                }
                button {
                    class: if is_fav { "iconbtn fav on" } else { "iconbtn fav" },
                    "aria-label": if is_fav { "Markierung entfernen" } else { "Frage markieren" },
                    onclick: move |_| toggle_favorite(app, qid),
                    Icon { name: pick(is_fav, "bookmark-fill", "bookmark") }
                }
                }
            }

            div { class: "qtext",
                if let Some(sim) = similar.as_ref() {
                    p { class: "simhint",
                        Icon { name: "compare" }
                        if sim.same_text {
                            "Gleiche Frage gibt es mehrfach – lies die Antworten genau!"
                        } else {
                            "Ähnliche Frage – achte auf das Markierte!"
                        }
                    }
                }
                p { class: "qbody",
                    {match segments {
                        Some(segs) => rsx! {
                            for seg in segs {
                                if seg.m {
                                    mark { class: "diff", {seg.t} }
                                } else {
                                    span { {seg.t} }
                                }
                            }
                        },
                        None => rsx! { {qtext} },
                    }}
                }
            }

            if let Some(prev) = preanswered {
                div { class: if prev { "prev ok" } else { "prev bad" },
                    Icon { name: pick(prev, "check", "x") }
                    if prev { "Schon richtig beantwortet" } else { "Zuletzt falsch beantwortet" }
                }
            }

            div { class: "answers",
                for (i, text) in sh.answers.iter().cloned().enumerate() {
                    button {
                        class: answer_class(i),
                        disabled: reveal,
                        onclick: move |_| {
                            if selected.peek().is_some() || preanswered.is_some() {
                                return;
                            }
                            selected.set(Some(i));
                            on_answer.call(i == sh.correct);
                        },
                        span { class: "letter", {LETTERS[i]} }
                        span { class: "atext", {text} }
                    }
                }
            }

            if show_explain() {
                ExplainSheet { qid, correct: correct_now == Some(true), on_close: move |_| show_explain.set(false) }
            }

            if let Some(next) = on_next {
                if picked.is_some() {
                    button {
                        class: if correct_now == Some(true) { "btn primary big next ok" } else { "btn primary big next" },
                        onmounted: move |e: MountedEvent| async move {
                            let _ = e.data().set_focus(true).await;
                        },
                        onkeydown: move |e: KeyboardEvent| {
                            if e.key() == Key::ArrowRight {
                                next.call(());
                            }
                        },
                        onclick: move |_| next.call(()),
                        "{next_label}"
                        Icon { name: "next" }
                    }
                }
            }
        }
    }
}

/// KI-Erklärung as a sheet over the question (no scrolling of the page)
#[component]
pub fn ExplainSheet(qid: u32, correct: bool, on_close: EventHandler<()>) -> Element {
    let Some(ex) = question(qid).and_then(|q| q.explanation.clone()) else {
        return rsx! {};
    };
    let cheer = CHEERS[qid as usize % CHEERS.len()];
    let title_class = if correct { "ex-title ok" } else { "ex-title" };
    rsx! {
        div { class: "sheet-backdrop top", onclick: move |_| on_close.call(()),
            div { class: "sheet explain-sheet", onclick: move |e| e.stop_propagation(),
                div { class: "sheet-head",
                    h2 { class: title_class,
                        Icon { name: "bulb" }
                        if correct { "{cheer}" } else { "KI-Erklärung" }
                    }
                    button { class: "iconbtn", "aria-label": "Schließen", onclick: move |_| on_close.call(()), Icon { name: "close" } }
                }
                p { class: "ex-why", {ex.why} }
                if let Some(tip) = ex.tip {
                    div { class: "ex-box",
                        h3 { "Merkhilfe" }
                        p { {tip} }
                    }
                }
                if let Some(twin) = ex.twin {
                    div { class: "ex-box",
                        h3 { "Verwechslungsgefahr" }
                        p { {twin} }
                    }
                }
                button { class: "btn wide", onclick: move |_| on_close.call(()), "Schließen" }
            }
        }
    }
}

//! "Test": practice exam with 12 or 16 random questions, no time limit,
//! passed from 75 %. No feedback until it is handed in; answers can be
//! changed until then. The running exam is part of the saved progress, so it
//! survives closing the app.
//!
//! Effect on the learning progress: wrong / unanswered questions count as a
//! miss (they may come back as surprise questions); the right/wrong state of
//! the other modes is not changed.

use crate::card::ExplainSheet;
use crate::icons::{pick, Icon};
use crate::model::{question, questions, ExamResult, ExamState};
use crate::transfer::ago;
use crate::util::{applog::log, iso_from_ms, now_ms, random, shuffle, shuffle_answers_seeded, Shuffled};
use crate::{use_app, AppState};
use dioxus::prelude::*;
use serde_json::json;

pub const SIZES: [u32; 2] = [12, 16];
pub const PASS_PERCENT: u32 = 75;
const LETTERS: [&str; 4] = ["A", "B", "C", "D"];

/// correct answers needed to pass (12 → 9, 16 → 12)
pub fn needed(size: u32) -> u32 {
    (size * PASS_PERCENT).div_ceil(100)
}

pub fn passed(correct: u32, size: u32) -> bool {
    correct >= needed(size)
}

pub fn new_exam(size: u32) -> ExamState {
    let mut ids: Vec<u32> = questions().iter().map(|q| q.id).collect();
    shuffle(&mut ids);
    ids.truncate(size as usize);
    ExamState {
        size: ids.len() as u32,
        answers: vec![None; ids.len()],
        ids,
        seed: (random() * 1_000_000_000.0) as u32,
        current: 0,
        started: iso_from_ms(now_ms()),
        finished: false,
    }
}

fn shuffled(e: &ExamState, i: usize) -> Option<Shuffled> {
    let q = question(*e.ids.get(i)?)?;
    Some(shuffle_answers_seeded(q, e.seed as i64 + i as i64 * 7919))
}

/// Whether question `i` was answered correctly
fn is_right(e: &ExamState, i: usize) -> bool {
    match (shuffled(e, i), e.answers.get(i).copied().flatten()) {
        (Some(sh), Some(a)) => a as usize == sh.correct,
        _ => false,
    }
}

pub fn count_correct(e: &ExamState) -> u32 {
    (0..e.ids.len()).filter(|&i| is_right(e, i)).count() as u32
}

fn edit_exam(mut app: Signal<AppState>, f: impl FnOnce(&mut ExamState)) {
    if let Some(e) = app.write().p.exam.as_mut() {
        f(e);
    }
}

fn scroll_strip(i: usize) {
    document::eval(&format!(
        "requestAnimationFrame(() => document.getElementById('en-{i}')?.scrollIntoView({{block:'nearest',inline:'center',behavior:'smooth'}}))"
    ));
}

fn submit(mut app: Signal<AppState>) {
    let mut a = app.write();
    let Some(e) = a.p.exam.clone() else { return };
    let correct = count_correct(&e);
    for (i, &id) in e.ids.iter().enumerate() {
        if !is_right(&e, i) {
            a.p.record_miss(id);
        }
    }
    a.p.exam_history.push(ExamResult { date: iso_from_ms(now_ms()), size: e.size, correct });
    if let Some(x) = a.p.exam.as_mut() {
        x.finished = true;
    }
    log(
        "exam.submit",
        json!({ "size": e.size, "correct": correct, "passed": passed(correct, e.size), "unanswered": e.answers.iter().filter(|a| a.is_none()).count() }),
    );
}

// --------------------------------------------------------------------- view

#[component]
pub fn ExamView() -> Element {
    let app = use_app();
    let exam = app.read().p.exam.clone();
    match exam {
        None => rsx! { ExamSetup {} },
        Some(e) if e.finished => rsx! { ExamReview { exam: e } },
        Some(e) => rsx! { ExamRun { exam: e } },
    }
}

#[component]
fn ExamSetup() -> Element {
    let mut app = use_app();
    let mut size = use_signal(|| 12u32);
    // (passed, title, sub) per earlier exam, newest first
    let history: Vec<(bool, String, String)> = app
        .read()
        .p
        .exam_history
        .iter()
        .rev()
        .take(8)
        .map(|r| {
            let ok = passed(r.correct, r.size);
            let title = format!("{} von {} · {} %", r.correct, r.size, r.correct * 100 / r.size.max(1));
            let sub = format!(
                "{} · {}",
                ago(&r.date).unwrap_or_default(),
                if ok { "bestanden" } else { "nicht bestanden" }
            );
            (ok, title, sub)
        })
        .collect();
    let total = questions().len();

    rsx! {
        div { class: "page",
            section { class: "card exam-intro",
                h2 { "Prüfung simulieren" }
                p { class: "hint", "{total} Fragen im Topf, zufällig gezogen · ohne Zeitlimit · bestanden ab {PASS_PERCENT} %. Die Auflösung gibt es erst nach dem Abgeben." }
                div { class: "choice",
                    for (n, need_label) in SIZES.iter().map(|&n| (n, format!("bestanden ab {} richtig", needed(n)))) {
                        button {
                            class: if size() == n { "choice-card on" } else { "choice-card" },
                            onclick: move |_| size.set(n),
                            span { class: "choice-n", "{n}" }
                            span { class: "choice-l", "Fragen" }
                            span { class: "choice-s", {need_label} }
                        }
                    }
                }
                button {
                    class: "btn primary big",
                    onclick: move |_| {
                        let e = new_exam(size());
                        log("exam.start", json!({ "size": e.size }));
                        app.write().p.exam = Some(e);
                    },
                    "Test starten"
                    Icon { name: "next" }
                }
            }

            if !history.is_empty() {
                h3 { class: "list-title", "Letzte Tests" }
                div { class: "list",
                    for (ok, title, sub) in history {
                        div { class: "list-row static",
                            span { class: if ok { "row-icon ok" } else { "row-icon bad" },
                                Icon { name: pick(ok, "check", "x") }
                            }
                            span { class: "row-text",
                                span { class: "row-title", {title} }
                                span { class: "row-sub", {sub} }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ExamRun(exam: ExamState) -> Element {
    let app = use_app();
    let mut confirm = use_signal(|| None::<&'static str>); // "submit" | "cancel"

    let size = exam.ids.len();
    let i = (exam.current as usize).min(size.saturating_sub(1));
    let Some(q) = exam.ids.get(i).and_then(|&id| question(id)) else {
        return rsx! {};
    };
    let Some(sh) = shuffled(&exam, i) else { return rsx! {} };
    let chosen = exam.answers.get(i).copied().flatten().map(|a| a as usize);
    let answered = exam.answers.iter().filter(|a| a.is_some()).count();
    let open = size - answered;
    let pct = answered * 100 / size.max(1);
    let pos = i + 1;
    let last = i + 1 >= size;
    let qtext = q.question.clone();
    let caa_id = q.caa_id.clone();
    let flags: Vec<bool> = exam.answers.iter().map(|a| a.is_some()).collect();

    let confirm_text = match confirm() {
        Some("cancel") => "Test abbrechen? Die Antworten werden verworfen und nicht gewertet.".to_string(),
        _ if open > 0 => format!("Noch {open} Fragen unbeantwortet – die zählen als falsch. Trotzdem abgeben?"),
        _ => "Alle Fragen beantwortet. Jetzt abgeben?".to_string(),
    };

    let go = move |n: usize| {
        edit_exam(app, |e| e.current = n as u32);
        scroll_strip(n);
    };

    rsx! {
        div { class: "page learn",
            div { class: "blockbar",
                div { class: "blockbar-top",
                    span { class: "blockbar-title", "Test · {size} Fragen" }
                    span { class: "mono muted", "{answered} / {size} beantwortet" }
                }
                div { class: "bar", div { class: "bar-fill accent", style: "width: {pct}%" } }
            }

            if let Some(what) = confirm() {
                div { class: "confirm-box",
                    p { {confirm_text} }
                    div { class: "row",
                        button { class: "btn", onclick: move |_| confirm.set(None), "Zurück" }
                        if what == "cancel" {
                            button { class: "btn danger", onclick: move |_| {
                                log("exam.cancel", json!({}));
                                let mut app = app;
                                app.write().p.exam = None;
                            }, "Abbrechen" }
                        } else {
                            button { class: "btn primary", onclick: move |_| { confirm.set(None); submit(app); }, "Abgeben" }
                        }
                    }
                }
            }

            div { class: "qcard",
                div { class: "qhead",
                    div { class: "qmeta",
                        span { class: "chip mono", {caa_id} }
                        span { class: "qnum", "Frage {pos} / {size}" }
                    }
                }
                div { class: "qtext", p { class: "qbody", {qtext} } }
                div { class: "answers",
                    for (k, text) in sh.answers.iter().cloned().enumerate() {
                        button {
                            class: if chosen == Some(k) { "ans is-sel" } else { "ans" },
                            onclick: move |_| {
                                edit_exam(app, |e| {
                                    if let Some(slot) = e.answers.get_mut(i) {
                                        *slot = Some(k as u8);
                                    }
                                });
                            },
                            span { class: "letter", {LETTERS[k]} }
                            span { class: "atext", {text} }
                        }
                    }
                }
                div { class: "exam-actions",
                    button { class: "btn ghost small", onclick: move |_| confirm.set(Some("cancel")), "Test abbrechen" }
                    if !last {
                        button { class: "btn ghost small", onclick: move |_| confirm.set(Some("submit")), "Abgeben" }
                    }
                }
            }
        }

        footer { class: "strip-bar",
            div { class: "strip-nav",
                button { class: "btn ghost", disabled: i == 0, onclick: move |_| go(i - 1), Icon { name: "back" } "Zurück" }
                span { class: "mono pos", "{pos} / {size}" }
                if last {
                    button { class: "btn primary", onclick: move |_| confirm.set(Some("submit")), "Abgeben" }
                } else {
                    button { class: "btn ghost", onclick: move |_| go(i + 1), "Weiter" Icon { name: "next" } }
                }
            }
            div { class: "strip",
                for (n, cls, label) in flags.iter().copied().enumerate().map(|(n, done)| {
                    let cls: &'static str = if n == i { "qn cur" } else if done { "qn answered" } else { "qn" };
                    (n, cls, n + 1)
                }) {
                    button {
                        key: "{n}",
                        id: "en-{n}",
                        class: cls,
                        onclick: move |_| go(n),
                        "{label}"
                    }
                }
            }
        }
    }
}

#[component]
fn ExamReview(exam: ExamState) -> Element {
    let mut app = use_app();
    let mut open = use_signal(|| None::<usize>);
    let mut explain = use_signal(|| None::<(u32, bool)>);
    let mut marked = use_signal(|| false);

    let size = exam.ids.len() as u32;
    let correct = count_correct(&exam);
    let ok = passed(correct, size);
    let pct = correct * 100 / size.max(1);
    let need = needed(size);
    let wrong_ids: Vec<u32> = exam
        .ids
        .iter()
        .enumerate()
        .filter(|&(i, _)| !is_right(&exam, i))
        .map(|(_, &id)| id)
        .collect();
    let wrong_count = wrong_ids.len();

    struct Row {
        i: usize,
        id: u32,
        title: String,
        text: String,
        right: bool,
        answers: Vec<(String, &'static str)>,
        unanswered: bool,
        has_explain: bool,
    }
    let rows: Vec<Row> = (0..exam.ids.len())
        .filter_map(|i| {
            let id = exam.ids[i];
            let q = question(id)?;
            let sh = shuffled(&exam, i)?;
            let chosen = exam.answers[i].map(|a| a as usize);
            let answers = sh
                .answers
                .iter()
                .enumerate()
                .map(|(k, t)| {
                    let class = if k == sh.correct {
                        "ans is-correct"
                    } else if chosen == Some(k) {
                        "ans is-wrong"
                    } else {
                        "ans is-dim"
                    };
                    (t.clone(), class)
                })
                .collect();
            Some(Row {
                i,
                id,
                title: format!("Frage {} · {}", i + 1, q.caa_id),
                text: q.question.clone(),
                right: is_right(&exam, i),
                answers,
                unanswered: chosen.is_none(),
                has_explain: q.explanation.is_some(),
            })
        })
        .collect();

    rsx! {
        div { class: "page",
            section { class: if ok { "card result pass" } else { "card result fail" },
                div { class: "verdict",
                    Icon { name: pick(ok, "check", "x") }
                    if ok { "Bestanden" } else { "Nicht bestanden" }
                }
                div { class: "big", "{pct} %" }
                p { class: "muted", "{correct} von {size} richtig · nötig: {need} ({PASS_PERCENT} %)" }
            }

            div { class: "row result-actions",
                if wrong_count > 0 {
                    button {
                        class: "btn",
                        disabled: marked(),
                        onclick: move |_| {
                            let mut a = app.write();
                            for &id in &wrong_ids {
                                if !a.p.favorites.contains(&id) {
                                    a.p.favorites.push(id);
                                }
                            }
                            marked.set(true);
                            log("exam.markWrong", json!({ "count": wrong_ids.len() }));
                        },
                        Icon { name: "bookmark" }
                        if marked() { "Markiert" } else { "Falsche markieren ({wrong_count})" }
                    }
                }
                button { class: "btn primary", onclick: move |_| app.write().p.exam = None, Icon { name: "repeat" } "Neuer Test" }
            }

            h3 { class: "list-title", "Auswertung" }
            div { class: "list",
                for row in rows {
                    div { class: "review-item",
                        button {
                            class: "list-row",
                            onclick: move |_| open.set(if open() == Some(row.i) { None } else { Some(row.i) }),
                            span { class: if row.right { "row-icon ok" } else { "row-icon bad" },
                                Icon { name: pick(row.right, "check", "x") }
                            }
                            span { class: "row-text",
                                span { class: "row-title", {row.title} }
                                span { class: "row-sub clamp", {row.text.clone()} }
                            }
                            span { class: "chev", Icon { name: pick(open() == Some(row.i), "down", "next") } }
                        }
                        if open() == Some(row.i) {
                            div { class: "review-body",
                                p { class: "qbody", {row.text.clone()} }
                                if row.unanswered {
                                    p { class: "prev bad", Icon { name: "x" } "Nicht beantwortet" }
                                }
                                div { class: "answers",
                                    for (k, (t, class)) in row.answers.into_iter().enumerate() {
                                        div { class: class,
                                            span { class: "letter", {LETTERS[k]} }
                                            span { class: "atext", {t} }
                                        }
                                    }
                                }
                                if row.has_explain {
                                    button {
                                        class: "btn wide explain-open",
                                        onclick: move |_| explain.set(Some((row.id, row.right))),
                                        Icon { name: "bulb" }
                                        "KI-Erklärung"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some((qid, right)) = explain() {
            ExplainSheet { qid, correct: right, on_close: move |_| explain.set(None) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_mark() {
        assert_eq!(needed(12), 9);
        assert_eq!(needed(16), 12);
        assert!(passed(9, 12) && !passed(8, 12));
        assert!(passed(12, 16) && !passed(11, 16));
    }

    #[test]
    fn exam_scoring() {
        let mut e = new_exam(16);
        assert_eq!(e.ids.len(), 16);
        let mut uniq = e.ids.clone();
        uniq.sort();
        uniq.dedup();
        assert_eq!(uniq.len(), 16, "no question twice");
        // answer everything right
        for i in 0..16 {
            e.answers[i] = Some(shuffled(&e, i).unwrap().correct as u8);
        }
        assert_eq!(count_correct(&e), 16);
        // same order after a "restart" (same seed)
        let again = shuffled(&e, 3).unwrap();
        assert_eq!(again.correct as u8, e.answers[3].unwrap());
        e.answers[0] = None;
        assert_eq!(count_correct(&e), 15);
    }
}

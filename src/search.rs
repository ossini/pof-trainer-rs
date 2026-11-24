//! "Suche": find questions by a word in the question or in one of its
//! answers (case-insensitive). Every hit shows the question with all answers,
//! the correct one in green – for looking things up, nothing is saved.

use crate::card::{toggle_favorite, ExplainSheet};
use crate::icons::{pick, Icon};
use crate::model::{question, questions, Question};
use crate::use_app;
use dioxus::prelude::*;

/// hits rendered at once; more on request (keeps typing snappy)
const PAGE: usize = 30;
const LETTERS: [&str; 4] = ["A", "B", "C", "D"];

/// Lowercased chars of `text`, each with the byte range of the original char
/// it came from (some chars lowercase to more than one char).
fn folded(text: &str) -> Vec<(char, usize, usize)> {
    text.char_indices()
        .flat_map(|(i, c)| {
            let end = i + c.len_utf8();
            c.to_lowercase().map(move |l| (l, i, end))
        })
        .collect()
}

/// Byte ranges in `text` where `needle` (already lowercased) occurs,
/// ignoring case; non-overlapping.
pub fn find_ranges(text: &str, needle: &[char]) -> Vec<(usize, usize)> {
    if needle.is_empty() {
        return vec![];
    }
    let hay = folded(text);
    let mut out: Vec<(usize, usize)> = vec![];
    let mut i = 0;
    while i + needle.len() <= hay.len() {
        if hay[i..i + needle.len()].iter().map(|h| h.0).eq(needle.iter().copied()) {
            let (start, end) = (hay[i].1, hay[i + needle.len() - 1].2);
            match out.last_mut() {
                // two hits inside the same original char: merge
                Some(last) if start < last.1 => last.1 = last.1.max(end),
                _ => out.push((start, end)),
            }
            i += needle.len();
        } else {
            i += 1;
        }
    }
    out
}

/// `text` split into (part, is_hit)
fn split_hits(text: &str, needle: &[char]) -> Vec<(String, bool)> {
    let mut parts = vec![];
    let mut pos = 0;
    for (s, e) in find_ranges(text, needle) {
        if s > pos {
            parts.push((text[pos..s].to_string(), false));
        }
        parts.push((text[s..e].to_string(), true));
        pos = e;
    }
    if pos < text.len() {
        parts.push((text[pos..].to_string(), false));
    }
    parts
}

fn matches(q: &Question, needle: &[char]) -> bool {
    let hit = |t: &str| !find_ranges(t, needle).is_empty();
    hit(&q.question) || q.answers.iter().any(|a| hit(a)) || hit(&q.caa_id)
}

pub fn search(term: &str) -> Vec<u32> {
    let needle: Vec<char> = term.trim().to_lowercase().chars().collect();
    if needle.is_empty() {
        return vec![];
    }
    questions().iter().filter(|q| matches(q, &needle)).map(|q| q.id).collect()
}

#[component]
fn Highlighted(text: String, needle: Vec<char>) -> Element {
    rsx! {
        for (part, hit) in split_hits(&text, &needle) {
            if hit {
                mark { class: "hit", {part} }
            } else {
                span { {part} }
            }
        }
    }
}

#[component]
pub fn SearchView() -> Element {
    let mut term = use_signal(String::new);
    let mut shown = use_signal(|| PAGE);
    let hits = use_memo(move || search(&term()));

    let t = term();
    let needle: Vec<char> = t.trim().to_lowercase().chars().collect();
    let all = hits();
    let count = all.len();
    let limit = shown().min(count);

    rsx! {
        div { class: "page search",
            div { class: "searchbox",
                Icon { name: "search" }
                input {
                    r#type: "search",
                    placeholder: "Frage oder Antwort suchen …",
                    "aria-label": "Fragen und Antworten durchsuchen",
                    autocomplete: "off",
                    autocapitalize: "off",
                    spellcheck: "false",
                    value: "{t}",
                    onmounted: move |e: MountedEvent| async move {
                        let _ = e.data().set_focus(true).await;
                    },
                    oninput: move |e| {
                        term.set(e.value());
                        shown.set(PAGE);
                    },
                }
                if !t.is_empty() {
                    button {
                        class: "iconbtn",
                        "aria-label": "Suche leeren",
                        onclick: move |_| {
                            term.set(String::new());
                            shown.set(PAGE);
                        },
                        Icon { name: "close" }
                    }
                }
            }

            if needle.is_empty() {
                p { class: "hint search-hint", "Durchsucht alle {questions().len()} Fragen und ihre Antworten. Groß- und Kleinschreibung ist egal; die richtige Antwort ist grün." }
            } else if count == 0 {
                p { class: "hint search-hint", "Keine Treffer für „{t.trim()}“." }
            } else {
                p { class: "hint search-hint",
                    if count == 1 { "1 Treffer" } else { "{count} Treffer" }
                }
                for id in all[..limit].iter().copied() {
                    SearchHit { key: "{id}", qid: id, needle: needle.clone() }
                }
                if limit < count {
                    button {
                        class: "btn wide more",
                        onclick: move |_| shown.set(shown() + PAGE),
                        "Weitere {(count - limit).min(PAGE)} anzeigen (noch {count - limit})"
                    }
                }
            }
        }
    }
}

#[component]
fn SearchHit(qid: u32, needle: Vec<char>) -> Element {
    let app = use_app();
    let mut show_explain = use_signal(|| false);
    let Some(q) = question(qid) else {
        return rsx! {};
    };
    let is_fav = app.read().p.favorites.contains(&qid);
    let has_explain = q.explanation.is_some();

    rsx! {
        div { class: "card hit-card",
            div { class: "qhead",
                div { class: "qmeta",
                    span { class: "chip mono", Highlighted { text: q.caa_id.clone(), needle: needle.clone() } }
                }
                div { class: "qactions",
                    if has_explain {
                        button {
                            class: "iconbtn explain-btn",
                            title: "KI-Erklärung",
                            "aria-label": "KI-Erklärung anzeigen",
                            onclick: move |_| show_explain.set(true),
                            Icon { name: "bulb" }
                        }
                    }
                    button {
                        class: if is_fav { "iconbtn fav on" } else { "iconbtn fav" },
                        "aria-label": if is_fav { "Markierung entfernen" } else { "Frage markieren" },
                        onclick: move |_| toggle_favorite(app, qid),
                        Icon { name: pick(is_fav, "bookmark-fill", "bookmark") }
                    }
                }
            }
            p { class: "qbody", Highlighted { text: q.question.clone(), needle: needle.clone() } }
            div { class: "answers",
                // answers[0] is the correct one
                for (i, a) in q.answers.iter().cloned().enumerate() {
                    div { class: if i == 0 { "ans is-correct" } else { "ans" },
                        if i == 0 {
                            span { class: "letter", Icon { name: "check" } }
                        } else {
                            span { class: "letter", {LETTERS[i]} }
                        }
                        span { class: "atext", Highlighted { text: a, needle: needle.clone() } }
                    }
                }
            }
            if show_explain() {
                ExplainSheet { qid, correct: false, on_close: move |_| show_explain.set(false) }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(s: &str) -> Vec<char> {
        s.to_lowercase().chars().collect()
    }

    #[test]
    fn ignores_case() {
        assert_eq!(find_ranges("Der Auftrieb steigt", &n("AUFTRIEB")), vec![(4, 12)]);
        assert_eq!(find_ranges("LIFT and lift", &n("lift")), vec![(0, 4), (9, 13)]);
    }

    #[test]
    fn umlauts_keep_byte_ranges() {
        let t = "Größe ÄNDERT";
        let r = find_ranges(t, &n("änd"));
        assert_eq!(&t[r[0].0..r[0].1], "ÄND");
    }

    #[test]
    fn splits_text() {
        let parts = split_hits("abc ABC", &n("b"));
        let joined: String = parts.iter().map(|p| p.0.as_str()).collect();
        assert_eq!(joined, "abc ABC");
        assert_eq!(parts.iter().filter(|p| p.1).count(), 2);
    }

    #[test]
    fn finds_in_answers() {
        let q = &questions()[0];
        let word: String = q.answers[2].chars().take(6).collect();
        assert!(search(&word.to_uppercase()).contains(&q.id));
    }
}

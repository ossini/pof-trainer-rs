//! "Propeller": endless practice of a fixed set of questions
//! (PL080-0408 … PL080-0416, fixed pitch propeller). The set comes in rounds,
//! freshly shuffled each time; a wrong answer comes back later in the same
//! round. Never ends – stop whenever you like.
//!
//! Effect on the learning progress: like "Markiert lernen" – a wrong answer
//! counts as a miss, the right/wrong state of the other modes is not changed.

use crate::card::QuestionCard;
use crate::icons::Icon;
use crate::model::{question, questions};
use crate::use_app;
use crate::util::{applog::log, shuffle};
use dioxus::prelude::*;
use serde_json::json;

pub const TITLE: &str = "Propeller";
pub const RANGE_LABEL: &str = "PL080-0408 bis 0416";
const FIRST: &str = "PL080-0408";
const LAST: &str = "PL080-0416";

/// ids of the set, in catalogue order
pub fn focus_ids() -> Vec<u32> {
    questions()
        .iter()
        .filter(|q| q.caa_id.as_str() >= FIRST && q.caa_id.as_str() <= LAST)
        .map(|q| q.id)
        .collect()
}

/// New shuffled round; `last` (the question just asked) never comes first
/// again, so nothing repeats back to back across rounds.
fn new_round(ids: &[u32], last: Option<u32>) -> Vec<u32> {
    let mut v = ids.to_vec();
    shuffle(&mut v);
    if v.len() > 1 && v.first().copied() == last {
        let mid = 1 + (v.len() - 1) / 2;
        v.swap(0, mid);
    }
    v
}

#[derive(Clone, PartialEq)]
struct Session {
    round: u32,
    /// remaining questions of this round, [0] = current
    queue: Vec<u32>,
    /// answered correctly in this round
    done: usize,
    step: u32,
    right: u32,
    wrong: u32,
}

#[component]
pub fn FocusView() -> Element {
    let mut app = use_app();
    let ids = use_hook(focus_ids);
    let set_ids = ids.clone();
    let mut s = use_signal(move || Session {
        round: 1,
        queue: new_round(&set_ids, None),
        done: 0,
        step: 0,
        right: 0,
        wrong: 0,
    });
    let mut result = use_signal(|| None::<bool>);

    let Some(qid) = s.read().queue.first().copied() else {
        return rsx! {
            div { class: "page empty",
                div { class: "empty-icon", Icon { name: "alert" } }
                h2 { "Keine Fragen gefunden" }
                p { "Die Fragen {RANGE_LABEL} sind nicht im Katalog." }
            }
        };
    };
    let total = ids.len();
    let Session { round, done, step, right, wrong, .. } = s.read().clone();
    let pct = done * 100 / total.max(1);
    let in_round = s.read().queue.len();

    let on_answer = move |correct: bool| {
        result.set(Some(correct));
        log("focus.answer", json!({ "id": qid, "caa": question(qid).map(|q| q.caa_id.clone()), "correct": correct }));
        if !correct {
            app.write().p.record_miss(qid);
        }
    };

    let next_ids = ids.clone();
    let on_next = move |_: ()| {
        let Some(ok) = *result.peek() else { return };
        s.with_mut(|s| {
            let cur = s.queue.remove(0);
            s.step += 1;
            if ok {
                s.done += 1;
                s.right += 1;
            } else {
                s.wrong += 1;
                // comes back at the end of this round
                s.queue.push(cur);
            }
            if s.queue.is_empty() {
                s.round += 1;
                s.done = 0;
                s.queue = new_round(&next_ids, Some(cur));
            }
        });
        result.set(None);
    };

    let next_label = match result() {
        Some(true) if in_round == 1 => "Richtig – nächste Runde".to_string(),
        Some(true) => "Richtig – weiter".to_string(),
        Some(false) => "Falsch – kommt nochmal".to_string(),
        None => "Weiter".to_string(),
    };
    let on_answer = EventHandler::new(on_answer);
    let on_next = EventHandler::new(on_next);
    let card_key = format!("focus-{qid}-{step}");

    rsx! {
        div { class: "page learn",
            div { class: "blockbar",
                div { class: "blockbar-top",
                    span { class: "blockbar-title", "{TITLE} · Runde {round}" }
                    span { class: "mono muted", "{done} / {total} · ✓ {right} · ✗ {wrong}" }
                }
                div { class: "bar", div { class: "bar-fill", style: "width: {pct}%" } }
            }
            for k in std::iter::once(card_key) {
                QuestionCard {
                    key: "{k}",
                    qid,
                    salt: step,
                    number: done + 1,
                    total,
                    on_answer,
                    on_next: Some(on_next),
                    next_label: next_label.clone(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_is_0408_to_0416() {
        let caa: Vec<String> = focus_ids().iter().map(|&id| question(id).unwrap().caa_id.clone()).collect();
        assert_eq!(caa.len(), 9);
        assert_eq!(caa.first().unwrap(), "PL080-0408");
        assert_eq!(caa.last().unwrap(), "PL080-0416");
    }

    #[test]
    fn round_has_every_question_once_and_no_repeat() {
        let ids = focus_ids();
        for _ in 0..50 {
            let last = ids[3];
            let mut r = new_round(&ids, Some(last));
            assert_ne!(r[0], last);
            r.sort();
            assert_eq!(r, ids);
        }
    }
}

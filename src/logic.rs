//! Learning in blocks, ported 1:1 from the web app (DrillView.tsx).
//!
//! - A block takes the next BLOCK_SIZE open questions.
//! - A wrong answer sends the question to the end of the block, so it comes
//!   back until it is answered correctly.
//! - Questions learned in a block come back as a test, mixed into the next
//!   block. Only a passed test marks the question as correct; a failed test
//!   marks it wrong and it has to be learned again.
//! - Favorites variant ("Markiert lernen"): same on the marked questions, but
//!   never removes them; a passed test only finishes the question for the
//!   current round. Once all are finished, a new round starts.
//! - Surprise questions: answered wrong at least SURPRISE_MIN_MISSES times,
//!   mixed into new blocks (purple), never repeated inside the block, no
//!   effect on the saved result. Due again after an interval that grows with
//!   every correct answer in a row and starts over after a wrong one.

use crate::model::{questions, question, DrillItem, DrillState, SubjectProgress, Variant};
use crate::util::{applog::log, ms_from_iso, now_ms, random, shuffle};
use serde_json::json;
use std::collections::HashSet;

pub const BLOCK_SIZE: usize = 10;
const SURPRISE_PER_BLOCK: usize = 2;
const SURPRISE_MIN_MISSES: u32 = 2;
/// Minutes until a surprise question is due again, by correct answers in a
/// row (0, 1, 2, ...); the last value doubles from then on.
const SURPRISE_INTERVALS_MIN: [f64; 4] = [30.0, 120.0, 480.0, 1440.0];

pub fn surprise_due(p: &SubjectProgress, id: u32, now: f64) -> bool {
    let Some(r) = p.surprises.get(&id) else { return true };
    let steps = &SURPRISE_INTERVALS_MIN;
    let s = r.streak as usize;
    let minutes = if s < steps.len() {
        steps[s]
    } else {
        steps[steps.len() - 1] * 2f64.powi((s - steps.len() + 1) as i32)
    };
    now - ms_from_iso(&r.last).unwrap_or(0.0) >= minutes * 60_000.0
}

/// Whether a question belongs to the variant's pool at all
pub fn in_pool(v: Variant, p: &SubjectProgress, id: u32) -> bool {
    match v {
        Variant::Favorites => p.favorites.contains(&id),
        Variant::All => p.answered.get(&id) != Some(&true),
    }
}

/// Whether a question still has to be learned in this round
fn is_open(v: Variant, p: &SubjectProgress, completed: &[u32], id: u32) -> bool {
    in_pool(v, p, id) && !completed.contains(&id)
}

fn caa(id: u32) -> String {
    question(id).map_or_else(|| id.to_string(), |q| q.caa_id.clone())
}

/// Next block: tests for the questions learned in the previous block plus
/// the next batch of still-open questions, plus surprise questions.
pub fn build_block(
    v: Variant,
    p: &SubjectProgress,
    to_test: Vec<u32>,
    block: u32,
    step: u32,
    completed: Vec<u32>,
    round: u32,
) -> DrillState {
    let test_set: HashSet<u32> = to_test.iter().copied().collect();
    let mut queue: Vec<DrillItem> = questions()
        .iter()
        .filter(|q| is_open(v, p, &completed, q.id) && !test_set.contains(&q.id))
        .take(BLOCK_SIZE)
        .map(|q| DrillItem { id: q.id, test: false, retry: false, surprise: false })
        .chain(to_test.iter().map(|&id| DrillItem { id, test: true, retry: false, surprise: false }))
        .collect();
    shuffle(&mut queue);

    // Surprise questions: due ones among the often-missed questions that are
    // not part of this block anyway, preferring the most-missed.
    if !queue.is_empty() {
        let in_block: HashSet<u32> = queue.iter().map(|i| i.id).collect();
        let misses = |id: u32| p.wrong_count.get(&id).copied().unwrap_or(0);
        let now = now_ms();
        let hard: Vec<u32> = questions()
            .iter()
            .map(|q| q.id)
            .filter(|&id| misses(id) >= SURPRISE_MIN_MISSES && !in_block.contains(&id))
            .collect();
        let mut due: Vec<u32> = hard.iter().copied().filter(|&id| surprise_due(p, id, now)).collect();
        let due_count = due.len();
        shuffle(&mut due);
        due.sort_by(|a, b| misses(*b).cmp(&misses(*a)));
        due.truncate(SURPRISE_PER_BLOCK * 4);
        shuffle(&mut due);
        due.truncate(SURPRISE_PER_BLOCK);
        log(
            "drill.surprisePick",
            json!({ "hard": hard.len(), "due": due_count, "picked": due.iter().map(|&id| caa(id)).collect::<Vec<_>>() }),
        );
        for id in due {
            // never as the first question of a block
            let pos = (1 + (random() * queue.len() as f64) as usize).min(queue.len());
            queue.insert(pos, DrillItem { id, test: false, retry: false, surprise: true });
        }
    }

    let state = DrillState {
        block,
        total: queue.len() as u32,
        queue,
        learned: vec![],
        done: 0,
        step,
        completed,
        round,
    };
    log_block(v, &state);
    state
}

fn log_block(v: Variant, s: &DrillState) {
    let ids = |f: fn(&DrillItem) -> bool| s.queue.iter().filter(|i| f(i)).map(|i| caa(i.id)).collect::<Vec<_>>();
    log(
        "drill.block",
        json!({
            "variant": format!("{v:?}"),
            "round": s.round,
            "block": s.block,
            "learn": ids(|i| !i.test && !i.surprise),
            "tests": ids(|i| i.test),
            "surprises": ids(|i| i.surprise),
        }),
    );
}

/// Resume a saved session, or start a new one.
pub fn restore(v: Variant, p: &SubjectProgress, stored: Option<DrillState>) -> DrillState {
    let Some(s) = stored else {
        log("drill.start", json!({ "variant": format!("{v:?}") }));
        return build_block(v, p, vec![], 1, 0, vec![], 1);
    };
    // Drop questions that no longer exist or left the pool elsewhere since
    // the last session (answered correctly in another mode, or unmarked).
    let valid = |id: u32| question(id).is_some() && in_pool(v, p, id);
    let queue: Vec<DrillItem> = s
        .queue
        .iter()
        .filter(|i| if i.surprise { question(i.id).is_some() } else { valid(i.id) })
        .cloned()
        .collect();
    let learned: Vec<u32> = s.learned.iter().copied().filter(|&id| valid(id)).collect();
    let dropped = s.queue.len() - queue.len() + s.learned.len() - learned.len();
    log(
        "drill.resume",
        json!({ "variant": format!("{v:?}"), "block": s.block, "queue": queue.len(), "learned": learned.len(), "dropped": dropped }),
    );
    if !queue.is_empty() {
        let total = s.total.max(s.done + queue.len() as u32);
        return DrillState { queue, learned, total, ..s };
    }
    let next = s.block + if s.total > 0 { 1 } else { 0 };
    build_block(v, p, learned, next, s.step, s.completed, s.round)
}

/// Moves on after the current item was answered.
pub fn advance(s: &mut DrillState, correct: bool) {
    if s.queue.is_empty() {
        return;
    }
    let item = s.queue.remove(0);
    s.step += 1;
    if item.surprise {
        // never repeated within the block
        s.done += 1;
    } else if correct {
        // learned items get tested in the next block, tests are done
        s.done += 1;
        if item.test {
            s.completed.push(item.id);
        } else {
            s.learned.push(item.id);
        }
    } else {
        // back to the end of the block (a failed test is learned again)
        s.queue.push(DrillItem { id: item.id, test: false, retry: true, surprise: false });
    }
}

/// Open questions not yet part of the current block or its pending tests
pub fn remaining_in_pool(v: Variant, p: &SubjectProgress, s: &DrillState) -> usize {
    let in_block: HashSet<u32> = s.queue.iter().map(|i| i.id).chain(s.learned.iter().copied()).collect();
    questions()
        .iter()
        .filter(|q| is_open(v, p, &s.completed, q.id) && !in_block.contains(&q.id))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_cycle() {
        let mut p = SubjectProgress::default();
        let mut s = restore(Variant::All, &p, None);
        assert!(s.queue.len() >= BLOCK_SIZE);
        // answer everything correctly
        while let Some(item) = s.queue.first().cloned() {
            if item.test {
                p.mark_answer(item.id, true);
            }
            advance(&mut s, true);
        }
        assert_eq!(s.learned.len(), BLOCK_SIZE);
        let next = build_block(Variant::All, &p, s.learned.clone(), 2, s.step, vec![], 1);
        assert_eq!(next.queue.iter().filter(|i| i.test).count(), BLOCK_SIZE);
    }

    #[test]
    fn wrong_goes_to_end() {
        let p = SubjectProgress::default();
        let mut s = build_block(Variant::All, &p, vec![], 1, 0, vec![], 1);
        let first = s.queue[0].id;
        advance(&mut s, false);
        assert_eq!(s.queue.last().unwrap().id, first);
        assert!(s.queue.last().unwrap().retry);
    }

    #[test]
    fn seed_progress_parses() {
        let ap: crate::model::AppProgress =
            serde_json::from_str(include_str!("../data/seed-progress.json")).unwrap();
        let sp = &ap.subjects[crate::model::SUBJECT];
        let back = serde_json::to_string(&ap).unwrap();
        let again: crate::model::AppProgress = serde_json::from_str(&back).unwrap();
        assert_eq!(&again.subjects[crate::model::SUBJECT], sp);
    }
}

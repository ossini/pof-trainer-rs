//! Questions (embedded at compile time) and the saved progress.
//!
//! The progress format is the same as the export of the old web app
//! (`{"subjects": {"Principles of Flight": {...}}, "theme": "dark"}`), so
//! files can be moved between the two.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

pub const SUBJECT: &str = "Principles of Flight";

// ---------------------------------------------------------------- questions

#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct Explanation {
    /// why the correct answer is correct
    pub why: String,
    /// mnemonic / fun fact
    #[serde(default)]
    pub tip: Option<String>,
    /// what tells this question apart from its look-alikes
    #[serde(default)]
    pub twin: Option<String>,
}

/// Part of a question text; `m` = differs from look-alike questions
#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct Segment {
    pub t: String,
    pub m: bool,
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct Similar {
    #[allow(dead_code)]
    pub twins: Vec<u32>,
    /// a twin has the exact same text (only the answers differ)
    pub same_text: bool,
    #[serde(default)]
    pub segments: Option<Vec<Segment>>,
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct Question {
    pub id: u32,
    pub caa_id: String,
    pub question: String,
    /// answers[0] is always the correct one
    pub answers: [String; 4],
    #[serde(default)]
    pub explanation: Option<Explanation>,
    #[serde(default)]
    pub similar: Option<Similar>,
}

static QUESTIONS: LazyLock<Vec<Question>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../data/pof.json")).expect("data/pof.json ist ungültig")
});

static INDEX: LazyLock<HashMap<u32, usize>> =
    LazyLock::new(|| QUESTIONS.iter().enumerate().map(|(i, q)| (q.id, i)).collect());

pub fn questions() -> &'static [Question] {
    &QUESTIONS
}

pub fn question(id: u32) -> Option<&'static Question> {
    INDEX.get(&id).map(|&i| &QUESTIONS[i])
}

// ----------------------------------------------------------------- progress

fn is_false(b: &bool) -> bool {
    !*b
}
fn one() -> u32 {
    1
}
fn dark() -> String {
    "dark".into()
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct DrillItem {
    pub id: u32,
    /// test of a question learned in the previous block
    #[serde(default)]
    pub test: bool,
    /// came back after a wrong answer
    #[serde(default)]
    pub retry: bool,
    /// often-missed question mixed in; never repeated in the block
    #[serde(default, skip_serializing_if = "is_false")]
    pub surprise: bool,
}

/// Position in a "10er-Blöcke" / "Markiert lernen" session
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct DrillState {
    pub block: u32,
    /// remaining items of the current block, [0] = current
    pub queue: Vec<DrillItem>,
    /// learned in this block -> tested in the next block
    #[serde(default)]
    pub learned: Vec<u32>,
    pub total: u32,
    pub done: u32,
    /// increments per answer, used to re-shuffle repeats
    #[serde(default)]
    pub step: u32,
    /// favorites variant: finished in the current round
    #[serde(default)]
    pub completed: Vec<u32>,
    #[serde(default = "one")]
    pub round: u32,
}

/// When a surprise question was last asked and how often in a row it was
/// answered correctly; decides when it is due again.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct SurpriseRecord {
    pub last: String,
    pub streak: u32,
}

/// Practice exam: random questions, answers chosen freely (and changeable)
/// until it is handed in; no feedback before that.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ExamState {
    pub size: u32,
    pub ids: Vec<u32>,
    /// chosen answer per question, index into its shuffled answers
    pub answers: Vec<Option<u8>>,
    /// fixes the answer order for the whole exam (also after a restart)
    pub seed: u32,
    #[serde(default)]
    pub current: u32,
    pub started: String,
    #[serde(default)]
    pub finished: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ExamResult {
    pub date: String,
    pub size: u32,
    pub correct: u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct SubjectProgress {
    /// questionId -> wasCorrect
    #[serde(default)]
    pub answered: BTreeMap<u32, bool>,
    #[serde(default)]
    pub favorites: Vec<u32>,
    #[serde(default)]
    pub ultra_hard: Vec<u32>,
    #[serde(default)]
    pub current_index: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favorites_drill: Option<DrillState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub all_drill: Option<DrillState>,
    /// questionId -> how often answered wrong
    #[serde(default)]
    pub wrong_count: BTreeMap<u32, u32>,
    #[serde(default)]
    pub surprises: BTreeMap<u32, SurpriseRecord>,
    /// running or finished practice exam ("Test")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exam: Option<ExamState>,
    /// results of finished exams, newest last
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exam_history: Vec<ExamResult>,
    /// old "Lernen" mode of the web app, kept untouched for round-trips
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drill: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct AppProgress {
    #[serde(default)]
    pub subjects: BTreeMap<String, SubjectProgress>,
    #[serde(default = "dark")]
    pub theme: String,
    /// set on export, shown when importing on another device
    #[serde(default, rename = "exportedAt", skip_serializing_if = "Option::is_none")]
    pub exported_at: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Variant {
    /// whole subject, every question not answered correctly yet
    All,
    /// marked questions, in rounds
    Favorites,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Blocks,
    List,
    Errors,
    Favorites,
    FavoritesDrill,
    UltraHard,
    Exam,
    Search,
}

impl SubjectProgress {
    fn count_miss(&mut self, id: u32) {
        *self.wrong_count.entry(id).or_insert(0) += 1;
    }

    /// Saves the result; a wrong answer also counts as a miss.
    pub fn mark_answer(&mut self, id: u32, correct: bool) {
        self.answered.insert(id, correct);
        if !correct {
            self.count_miss(id);
        }
    }

    /// Wrong answer that should not change the saved result, only the counter.
    pub fn record_miss(&mut self, id: u32) {
        self.count_miss(id);
    }

    /// Surprise question answered: extend (correct) or reset (wrong) the
    /// interval until it is due again.
    pub fn record_surprise(&mut self, id: u32, correct: bool) {
        let streak = if correct {
            self.surprises.get(&id).map_or(0, |r| r.streak) + 1
        } else {
            0
        };
        self.surprises.insert(
            id,
            SurpriseRecord { last: crate::util::iso_from_ms(crate::util::now_ms()), streak },
        );
        if !correct {
            self.count_miss(id);
        }
    }

    pub fn toggle_favorite(&mut self, id: u32) {
        toggle(&mut self.favorites, id);
    }

    pub fn toggle_ultra(&mut self, id: u32) {
        toggle(&mut self.ultra_hard, id);
    }

    pub fn drill(&self, v: Variant) -> Option<&DrillState> {
        match v {
            Variant::All => self.all_drill.as_ref(),
            Variant::Favorites => self.favorites_drill.as_ref(),
        }
    }

    pub fn set_drill(&mut self, v: Variant, s: DrillState) {
        match v {
            Variant::All => self.all_drill = Some(s),
            Variant::Favorites => self.favorites_drill = Some(s),
        }
    }

    pub fn correct_count(&self) -> usize {
        self.answered.values().filter(|&&c| c).count()
    }

    pub fn wrong_answered(&self) -> usize {
        self.answered.values().filter(|&&c| !c).count()
    }
}

fn toggle(list: &mut Vec<u32>, id: u32) {
    if let Some(pos) = list.iter().position(|&x| x == id) {
        list.remove(pos);
    } else {
        list.push(id);
    }
}

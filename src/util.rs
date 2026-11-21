//! Time, randomness and storage helpers that work on the web (WASM) and on
//! iOS / desktop alike, without extra crates.

use crate::model::Question;
use std::cell::Cell;

// --------------------------------------------------------------------- time

pub fn now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as f64)
            .unwrap_or(0.0)
    }
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `2026-11-21T16:30:00.000Z`
pub fn iso_from_ms(ms: f64) -> String {
    let ms = ms as i64;
    let days = ms.div_euclid(86_400_000);
    let rest = ms.rem_euclid(86_400_000);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z",
        rest / 3_600_000,
        rest / 60_000 % 60,
        rest / 1000 % 60,
        rest % 1000
    )
}

/// Parses the ISO timestamps written by this app and the old web app.
pub fn ms_from_iso(s: &str) -> Option<f64> {
    let num = |a: usize, b: usize| s.get(a..b)?.parse::<i64>().ok();
    let (y, mo, d) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    let (h, mi, se) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    let frac = if s.get(19..20) == Some(".") {
        let digits: String = s[20..].chars().take_while(|c| c.is_ascii_digit()).take(3).collect();
        format!("{digits:0<3}").parse::<i64>().unwrap_or(0)
    } else {
        0
    };
    let days = days_from_civil(y, mo, d);
    Some((((days * 24 + h) * 60 + mi) * 60 + se) as f64 * 1000.0 + frac as f64)
}

pub fn today() -> String {
    iso_from_ms(now_ms())[..10].to_string()
}

pub async fn sleep_ms(ms: u32) {
    #[cfg(target_arch = "wasm32")]
    {
        gloo_timers::future::TimeoutFuture::new(ms).await;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = dioxus::prelude::document::eval(&format!(
            "await new Promise(r => setTimeout(r, {ms})); return true;"
        ))
        .await;
    }
}

// --------------------------------------------------------------- randomness

thread_local! {
    static RNG: Cell<u64> = const { Cell::new(0) };
}

/// xorshift64, seeded from the clock; 0.0 <= x < 1.0
pub fn random() -> f64 {
    RNG.with(|r| {
        let mut x = r.get();
        if x == 0 {
            x = (now_ms() as u64) ^ 0x9E37_79B9_7F4A_7C15;
        }
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        r.set(x);
        (x >> 11) as f64 / (1u64 << 53) as f64
    })
}

pub fn shuffle<T>(v: &mut [T]) {
    for i in (1..v.len()).rev() {
        let j = ((random() * (i + 1) as f64) as usize).min(i);
        v.swap(i, j);
    }
}

#[derive(Clone, PartialEq)]
pub struct Shuffled {
    pub answers: Vec<String>,
    pub correct: usize,
}

/// Answer order: stable per question for an hour; `salt` varies it, e.g.
/// when the same question is repeated in a block.
pub fn shuffle_answers(q: &Question, salt: u32) -> Shuffled {
    let hour = (now_ms() / 3_600_000.0).floor() as i64;
    shuffle_answers_seeded(q, hour + salt as i64 * 7919)
}

/// Same order for the same question and seed, every time.
pub fn shuffle_answers_seeded(q: &Question, seed: i64) -> Shuffled {
    let mut seed: i64 = q.id as i64 + seed;
    let mut order = [0usize, 1, 2, 3];
    let mut i = order.len();
    while i > 0 {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345) & 0x7fff_ffff;
        let r = seed as f64 / 0x7fff_ffff as f64;
        let j = ((r * i as f64).floor() as usize).min(i - 1);
        i -= 1;
        order.swap(i, j);
    }
    Shuffled {
        answers: order.iter().map(|&k| q.answers[k].clone()).collect(),
        correct: order.iter().position(|&k| k == 0).unwrap_or(0),
    }
}

// ------------------------------------------------------------------ storage
//
// Everything stays on the device: localStorage in the browser / home-screen
// app, a file in the app's Documents folder on iOS.

#[cfg(target_arch = "wasm32")]
pub mod store {
    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }
    pub fn get(key: &str) -> Option<String> {
        storage()?.get_item(key).ok()?
    }
    pub fn set(key: &str, value: &str) {
        if let Some(s) = storage() {
            let _ = s.set_item(key, value);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod store {
    use std::path::PathBuf;

    fn dir() -> PathBuf {
        let home = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()));
        if cfg!(target_os = "ios") {
            home.join("Documents")
        } else {
            home.join(".pof-trainer")
        }
    }
    pub fn get(key: &str) -> Option<String> {
        std::fs::read_to_string(dir().join(format!("{key}.json"))).ok()
    }
    pub fn set(key: &str, value: &str) {
        let dir = dir();
        let _ = std::fs::create_dir_all(&dir);
        let tmp = dir.join(format!("{key}.json.tmp"));
        if std::fs::write(&tmp, value).is_ok() {
            let _ = std::fs::rename(&tmp, dir.join(format!("{key}.json")));
        }
    }
}

// ---------------------------------------------------------------------- log
//
// Small event log (like logs/app.log of the web app), exportable in the
// settings when something goes wrong.

pub mod applog {
    use super::{iso_from_ms, now_ms, store};
    use std::cell::RefCell;

    const KEY: &str = "pof-trainer-log";
    const MAX: usize = 1500;

    thread_local! {
        static LINES: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
    }

    fn with<R>(f: impl FnOnce(&mut Vec<String>) -> R) -> R {
        LINES.with(|l| {
            let mut l = l.borrow_mut();
            let lines = l.get_or_insert_with(|| {
                store::get(KEY)
                    .map(|s| s.lines().map(str::to_string).collect())
                    .unwrap_or_default()
            });
            f(lines)
        })
    }

    pub fn log(event: &str, data: serde_json::Value) {
        let line = serde_json::json!({ "t": iso_from_ms(now_ms()), "event": event, "data": data }).to_string();
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&line.clone().into());
        #[cfg(not(target_arch = "wasm32"))]
        println!("{line}");
        with(|lines| {
            lines.push(line);
            if lines.len() > MAX {
                let cut = lines.len() - MAX;
                lines.drain(..cut);
            }
            store::set(KEY, &lines.join("\n"));
        });
    }

    pub fn export() -> String {
        with(|lines| lines.join("\n"))
    }

    pub fn clear() {
        with(|lines| lines.clear());
        store::set(KEY, "");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_roundtrip() {
        let s = "2026-11-09T20:37:12.562Z";
        let ms = ms_from_iso(s).unwrap();
        assert_eq!(iso_from_ms(ms), s);
        assert_eq!(ms_from_iso("1970-01-01T00:00:00Z"), Some(0.0));
    }

    #[test]
    fn shuffle_keeps_correct() {
        for q in crate::model::questions().iter().take(50) {
            let s = shuffle_answers(q, 3);
            assert_eq!(s.answers[s.correct], q.answers[0]);
        }
    }
}

//! PoF Trainer – PPL Principles of Flight, offline on the phone.

mod card;
mod icons;
mod logic;
mod model;
mod util;

use card::{Badge, QuestionCard};
use dioxus::prelude::*;
use icons::Icon;
use model::{question, questions, AppProgress, DrillState, Mode, SubjectProgress, Variant, SUBJECT};
use serde_json::json;
use util::{applog, applog::log, store};

const CSS: Asset = asset!("/assets/style.css");
const STORAGE_KEY: &str = "pof-trainer-progress";
const SEED: &str = include_str!("../data/seed-progress.json");

fn main() {
    dioxus::launch(App);
}

// -------------------------------------------------------------------- state

#[derive(Clone, PartialEq)]
pub struct AppState {
    pub p: SubjectProgress,
    pub theme: String,
    /// bumped on import / reset so open sessions restart from the new state
    pub gen: u32,
}

pub fn use_app() -> Signal<AppState> {
    use_context::<Signal<AppState>>()
}

fn summary(p: &SubjectProgress) -> serde_json::Value {
    json!(format!(
        "{}/{} richtig, {} markiert, {} ultra",
        p.correct_count(),
        p.answered.len(),
        p.favorites.len(),
        p.ultra_hard.len()
    ))
}

fn from_progress(ap: AppProgress) -> Option<AppState> {
    let theme = if ap.theme == "light" { "light" } else { "dark" }.to_string();
    let p = ap.subjects.get(SUBJECT)?.clone();
    Some(AppState { p, theme, gen: 0 })
}

/// Saved state on this device, or the snapshot taken from the web app on
/// first start.
fn load_initial() -> AppState {
    if let Some(text) = store::get(STORAGE_KEY) {
        match serde_json::from_str::<AppProgress>(&text).ok().and_then(from_progress) {
            Some(s) => {
                log("progress.load", json!({ "source": "device", "summary": summary(&s.p) }));
                return s;
            }
            None => log("progress.load.failed", json!({ "source": "device" })),
        }
    }
    let s = serde_json::from_str::<AppProgress>(SEED)
        .ok()
        .and_then(from_progress)
        .unwrap_or(AppState { p: SubjectProgress::default(), theme: "dark".into(), gen: 0 });
    log("progress.load", json!({ "source": "seed", "summary": summary(&s.p) }));
    s
}

/// Same format as the export of the old web app
fn to_progress(s: &AppState) -> AppProgress {
    AppProgress {
        subjects: [(SUBJECT.to_string(), s.p.clone())].into_iter().collect(),
        theme: s.theme.clone(),
    }
}

fn import(mut app: Signal<AppState>, text: &str) -> Result<String, String> {
    let ap: AppProgress = serde_json::from_str(text).map_err(|e| format!("Datei unlesbar: {e}"))?;
    let theme = ap.theme.clone();
    let p = ap
        .subjects
        .get(SUBJECT)
        .cloned()
        .ok_or_else(|| "In der Datei ist kein Principles-of-Flight-Fortschritt.".to_string())?;
    log("progress.import", json!({ "summary": summary(&p) }));
    let msg = format!("Importiert: {} beantwortet, {} markiert.", p.answered.len(), p.favorites.len());
    let mut a = app.write();
    a.p = p;
    a.theme = if theme == "light" { "light".into() } else { "dark".into() };
    a.gen += 1;
    Ok(msg)
}

/// JS string literal
fn js(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

/// Share sheet on the iPhone (→ "In Dateien sichern"), download elsewhere.
fn save_file(name: &str, mime: &str, content: &str) {
    document::eval(&format!(
        r#"const name = {n}, type = {t}, data = {c};
        const file = new File([data], name, {{ type }});
        if (navigator.canShare && navigator.canShare({{ files: [file] }})) {{
          navigator.share({{ files: [file] }}).catch(() => {{}});
        }} else {{
          const a = document.createElement('a');
          a.href = URL.createObjectURL(file);
          a.download = name;
          document.body.appendChild(a);
          a.click();
          setTimeout(() => {{ URL.revokeObjectURL(a.href); a.remove(); }}, 2000);
        }}"#,
        n = js(name),
        t = js(mime),
        c = js(content)
    ));
}

fn copy_text(content: &str) {
    document::eval(&format!("navigator.clipboard && navigator.clipboard.writeText({})", js(content)));
}

// ---------------------------------------------------------------------- app

#[component]
fn App() -> Element {
    let app = use_context_provider(|| Signal::new(load_initial()));
    let mut screen = use_signal(|| None::<Mode>); // None = start screen
    let mut settings = use_signal(|| false);

    // Save every change on the device
    use_effect(move || {
        let text = serde_json::to_string(&to_progress(&app.read())).unwrap_or_default();
        store::set(STORAGE_KEY, &text);
    });

    // Ask the browser not to evict the saved progress
    use_hook(|| {
        document::eval("navigator.storage && navigator.storage.persist && navigator.storage.persist()");
    });

    let theme = app.read().theme.clone();

    rsx! {
        document::Stylesheet { href: CSS }
        document::Title { "PoF Trainer" }
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1, viewport-fit=cover" }
        document::Meta { name: "theme-color", content: if theme == "light" { "#f3efe6" } else { "#070b14" } }
        div { class: "app", "data-theme": "{theme}",
            {match screen() {
                None => rsx! {
                    Home {
                        on_open: move |m| {
                            log("nav.mode", json!({ "mode": format!("{m:?}") }));
                            screen.set(Some(m));
                        },
                        on_settings: move |_| settings.set(true),
                    }
                },
                Some(mode) => rsx! {
                    Learn {
                        mode,
                        on_mode: move |m| {
                            log("nav.mode", json!({ "mode": format!("{m:?}") }));
                            screen.set(Some(m));
                        },
                        on_back: move |_| screen.set(None),
                        on_settings: move |_| settings.set(true),
                    }
                },
            }}
            if settings() {
                Settings { on_close: move |_| settings.set(false) }
            }
        }
    }
}

// ------------------------------------------------------------- start screen

fn ring_svg(ok: f64, bad: f64) -> String {
    let c = 2.0 * std::f64::consts::PI * 52.0;
    let (lo, lb) = (c * ok, c * bad);
    // tick marks like an instrument dial
    let ticks: String = (0..60)
        .map(|i| {
            let a = i as f64 * 6.0_f64.to_radians();
            let (r1, r2) = (64.0, if i % 5 == 0 { 58.0 } else { 61.0 });
            format!(
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}"/>"#,
                70.0 + r1 * a.sin(),
                70.0 - r1 * a.cos(),
                70.0 + r2 * a.sin(),
                70.0 - r2 * a.cos()
            )
        })
        .collect();
    format!(
        r#"<svg viewBox="0 0 140 140" aria-hidden="true">
        <g class="ticks">{ticks}</g>
        <circle class="track" cx="70" cy="70" r="52"/>
        <circle class="arc-ok" cx="70" cy="70" r="52" stroke-dasharray="{lo:.2} {c:.2}" transform="rotate(-90 70 70)"/>
        <circle class="arc-bad" cx="70" cy="70" r="52" stroke-dasharray="{lb:.2} {c:.2}" stroke-dashoffset="{:.2}" transform="rotate(-90 70 70)"/>
        </svg>"#,
        -lo
    )
}

#[component]
fn Home(on_open: EventHandler<Mode>, on_settings: EventHandler<()>) -> Element {
    let app = use_app();
    let a = app.read();
    let p = &a.p;
    let total = questions().len();
    let correct = p.correct_count();
    let wrong = p.wrong_answered();
    let answered = p.answered.len();
    let fav = p.favorites.len();
    let ultra = p.ultra_hard.len();
    let pct = if total > 0 { correct * 100 / total } else { 0 };
    let rate = if answered > 0 { correct * 100 / answered } else { 0 };
    let open = total - correct;
    let ring = ring_svg(correct as f64 / total.max(1) as f64, wrong as f64 / total.max(1) as f64);
    let block_info = match &p.all_drill {
        Some(d) if !d.queue.is_empty() => format!("Block {} · noch {} Fragen im Block", d.block, d.queue.len()),
        Some(d) => format!("Block {} ist fertig – weiter geht's", d.block),
        None => "Starte mit dem ersten 10er-Block".to_string(),
    };

    let tiles: [(Mode, &'static str, &'static str, &'static str, String); 5] = [
        (Mode::List, "list", "Liste", "Alle Fragen der Reihe nach", format!("{answered}/{total}")),
        (Mode::Errors, "alert", "Fehler", "Falsch beantwortete Fragen", wrong.to_string()),
        (Mode::Favorites, "bookmark", "Markiert", "Deine Lesezeichen", fav.to_string()),
        (Mode::FavoritesDrill, "repeat", "Markiert lernen", "Lesezeichen in Blöcken", String::new()),
        (Mode::UltraHard, "gem", "Ultra schwer", "Die richtig fiesen", ultra.to_string()),
    ];

    rsx! {
        header { class: "topbar",
            div { class: "brand",
                span { class: "logo", Icon { name: "plane" } }
                div {
                    div { class: "brand-title", "PoF Trainer" }
                    div { class: "brand-sub", "PPL(A) · Principles of Flight" }
                }
            }
            button { class: "iconbtn", "aria-label": "Einstellungen", onclick: move |_| on_settings.call(()), Icon { name: "settings" } }
        }

        main { class: "page home",
            section { class: "panel hero",
                div { class: "dial",
                    div { class: "dial-svg", dangerous_inner_html: ring }
                    div { class: "dial-center",
                        div { class: "dial-value", "{pct}" span { class: "unit", "%" } }
                        div { class: "dial-label", "sitzt" }
                    }
                }
                div { class: "stats",
                    div { class: "stat", div { class: "v", "{answered}" } div { class: "l", "beantwortet" } }
                    div { class: "stat ok", div { class: "v", "{correct}" } div { class: "l", "richtig" } }
                    div { class: "stat bad", div { class: "v", "{wrong}" } div { class: "l", "falsch" } }
                    div { class: "stat", div { class: "v", "{rate}%" } div { class: "l", "Trefferquote" } }
                }
            }

            button { class: "cta", onclick: move |_| on_open.call(Mode::Blocks),
                span { class: "cta-icon", Icon { name: "layers" } }
                span { class: "cta-text",
                    span { class: "cta-title", "Weiterlernen" }
                    span { class: "cta-sub", "{block_info}" }
                }
                span { class: "cta-meta", "{open} offen" }
                Icon { name: "next" }
            }

            div { class: "tiles",
                for (mode, icon, title, sub, count) in tiles {
                    button {
                        class: if mode == Mode::UltraHard { "tile ultra" } else { "tile" },
                        onclick: move |_| on_open.call(mode),
                        span { class: "tile-icon", Icon { name: icon } }
                        span { class: "tile-title", "{title}" }
                        span { class: "tile-sub", "{sub}" }
                        if !count.is_empty() {
                            span { class: "tile-count mono", "{count}" }
                        }
                    }
                }
            }

            p { class: "foot", "{total} Fragen · alles bleibt auf diesem Gerät · Viel Erfolg bei der Prüfung ✈" }
        }
    }
}

// ----------------------------------------------------------- learning views

#[component]
fn Learn(mode: Mode, on_mode: EventHandler<Mode>, on_back: EventHandler<()>, on_settings: EventHandler<()>) -> Element {
    let app = use_app();
    let (errors, fav, ultra, gen) = {
        let a = app.read();
        (a.p.wrong_answered(), a.p.favorites.len(), a.p.ultra_hard.len(), a.gen)
    };
    let qcount = questions().len();
    let active = if mode == Mode::List { Mode::Blocks } else { mode };
    let tabs: [(Mode, &'static str, &'static str, usize); 5] = [
        (Mode::Blocks, "layers", "Alle", 0),
        (Mode::Errors, "alert", "Fehler", errors),
        (Mode::Favorites, "bookmark", "Markiert", fav),
        (Mode::FavoritesDrill, "repeat", "Markiert lernen", 0),
        (Mode::UltraHard, "gem", "Ultra", ultra),
    ];

    rsx! {
        header { class: "topbar",
            button { class: "iconbtn", "aria-label": "Zurück", onclick: move |_| on_back.call(()), Icon { name: "back" } }
            div { class: "top-title",
                div { class: "brand-title", "Principles of Flight" }
                div { class: "brand-sub", "{qcount} Fragen" }
            }
            button { class: "iconbtn", "aria-label": "Einstellungen", onclick: move |_| on_settings.call(()), Icon { name: "settings" } }
        }
        nav { class: "tabs",
            for (m, icon, label, count) in tabs {
                button {
                    class: if m == active { "tab on" } else { "tab" },
                    onclick: move |_| on_mode.call(m),
                    Icon { name: icon }
                    span { "{label}" }
                    if count > 0 {
                        span { class: if m == Mode::UltraHard { "count ultra" } else if m == Mode::Errors { "count bad" } else { "count" }, "{count}" }
                    }
                }
            }
        }
        if mode == Mode::Blocks || mode == Mode::List {
            div { class: "seg",
                button { class: if mode == Mode::Blocks { "on" } else { "" }, onclick: move |_| on_mode.call(Mode::Blocks), Icon { name: "layers" } "10er-Blöcke" }
                button { class: if mode == Mode::List { "on" } else { "" }, onclick: move |_| on_mode.call(Mode::List), Icon { name: "list" } "Liste" }
            }
        }
        {match mode {
            Mode::Blocks => rsx! { DrillView { key: "all-{gen}", variant: Variant::All } },
            Mode::FavoritesDrill => rsx! { DrillView { key: "fav-{gen}", variant: Variant::Favorites } },
            m => rsx! { ListView { key: "{m:?}-{gen}", mode: m } },
        }}
    }
}

fn list_ids(mode: Mode, p: &SubjectProgress) -> Vec<u32> {
    questions()
        .iter()
        .map(|q| q.id)
        .filter(|id| match mode {
            Mode::Errors => p.answered.get(id) == Some(&false),
            Mode::Favorites => p.favorites.contains(id),
            Mode::UltraHard => p.ultra_hard.contains(id),
            _ => true,
        })
        .collect()
}

fn set_index(mut app: Signal<AppState>, mut local: Signal<usize>, mode: Mode, i: usize) {
    if mode == Mode::List {
        app.write().p.current_index = i;
    } else {
        local.set(i);
    }
}

/// "Liste", "Fehler", "Markiert", "Ultra schwer": one question at a time
/// with a number strip to jump around. The filtered lists are taken when the
/// view opens, so a question answered right in "Fehler" turns green instead
/// of vanishing under your finger.
#[component]
fn ListView(mode: Mode) -> Element {
    let mut app = use_app();
    let ids = use_hook(|| list_ids(mode, &app.peek().p));
    let local = use_signal(|| 0usize);
    let len = ids.len();

    let idx = use_memo(move || {
        let i = if mode == Mode::List { app.read().p.current_index } else { local() };
        i.min(len.saturating_sub(1))
    });

    // keep the current number visible in the strip
    use_effect(move || {
        let i = idx();
        document::eval(&format!(
            "requestAnimationFrame(() => document.getElementById('qn-{i}')?.scrollIntoView({{block:'nearest',inline:'center',behavior:'smooth'}}))"
        ));
    });

    if len == 0 {
        let (icon, title, text) = match mode {
            Mode::Errors => ("check", "Keine Fehler!", "Du hast alle beantworteten Fragen richtig. Weiter so!"),
            Mode::Favorites => ("bookmark", "Keine Markierungen", "Markiere Fragen mit dem Lesezeichen, um sie hier zu sammeln."),
            Mode::UltraHard => ("gem", "Noch nichts ULTRA SCHWER", "Markiere die richtig fiesen Fragen mit dem Diamanten, um sie hier zu sammeln."),
            _ => ("alert", "Keine Fragen", "Dieses Fach enthält keine Fragen."),
        };
        return rsx! { Empty { icon, title, text } };
    }

    let i = idx();
    let qid = ids[i];
    let pos = i + 1;
    let answered = app.read().p.answered.clone();
    let preanswered = if mode == Mode::List { answered.get(&qid).copied() } else { None };

    let on_answer = move |correct: bool| {
        log("answer", json!({ "mode": format!("{mode:?}"), "id": qid, "caa": question(qid).map(|q| q.caa_id.clone()), "correct": correct }));
        app.write().p.mark_answer(qid, correct);
        if correct && i + 1 < len {
            spawn(async move {
                util::sleep_ms(400).await;
                if *idx.peek() == i {
                    set_index(app, local, mode, i + 1);
                }
            });
        }
    };

    rsx! {
        div { class: "page learn",
            tabindex: "-1",
            onkeydown: move |e: KeyboardEvent| {
                if e.key() == Key::ArrowLeft && i > 0 {
                    set_index(app, local, mode, i - 1);
                } else if e.key() == Key::ArrowRight && i + 1 < len {
                    set_index(app, local, mode, i + 1);
                }
            },
            QuestionCard {
                key: "{mode:?}-{qid}-{i}",
                qid,
                salt: 0,
                number: i + 1,
                total: len,
                preanswered,
                on_answer,
            }
        }
        footer { class: "strip-bar",
            div { class: "strip-nav",
                button { class: "btn ghost", disabled: i == 0, onclick: move |_| set_index(app, local, mode, i - 1), Icon { name: "back" } "Zurück" }
                span { class: "mono pos", "{pos} / {len}" }
                button { class: "btn ghost", disabled: i + 1 >= len, onclick: move |_| set_index(app, local, mode, i + 1), "Weiter" Icon { name: "next" } }
            }
            div { class: "strip",
                for (n, id, label) in ids.iter().copied().enumerate().map(|(n, id)| (n, id, n + 1)) {
                    button {
                        key: "{id}",
                        id: "qn-{n}",
                        class: if n == i { "qn cur" } else if answered.get(&id) == Some(&true) { "qn ok" } else if mode == Mode::List && answered.get(&id) == Some(&false) { "qn bad" } else { "qn" },
                        onclick: move |_| set_index(app, local, mode, n),
                        "{label}"
                    }
                }
            }
        }
    }
}

#[component]
fn Empty(icon: &'static str, title: &'static str, text: &'static str) -> Element {
    rsx! {
        div { class: "page empty",
            div { class: "empty-icon", Icon { name: icon } }
            h2 { "{title}" }
            p { "{text}" }
        }
    }
}

/// "10er-Blöcke" (whole subject) and "Markiert lernen" (marked questions)
#[component]
fn DrillView(variant: Variant) -> Element {
    let mut app = use_app();
    let mut state = use_signal(|| {
        let p = app.peek().p.clone();
        let stored = p.drill(variant).cloned();
        logic::restore(variant, &p, stored)
    });
    let mut result = use_signal(|| None::<bool>);

    // The session is part of the saved progress (and of the export)
    use_effect(move || {
        let s: DrillState = state.read().clone();
        app.write().p.set_drill(variant, s);
    });

    let p = app.read().p.clone();
    let s = state.read().clone();
    let remaining = logic::remaining_in_pool(variant, &p, &s);
    let current = s.queue.first().cloned().filter(|it| question(it.id).is_some());

    let next_block = move |_: MouseEvent| {
        log("drill.nextBlock", json!({ "variant": format!("{variant:?}") }));
        let p = app.peek().p.clone();
        let next = {
            let s = state.peek();
            logic::build_block(variant, &p, s.learned.clone(), s.block + 1, s.step, s.completed.clone(), s.round)
        };
        state.set(next);
        result.set(None);
    };
    let next_round = move |_: MouseEvent| {
        log("drill.nextRound", json!({ "variant": format!("{variant:?}") }));
        let p = app.peek().p.clone();
        let next = {
            let s = state.peek();
            logic::build_block(variant, &p, vec![], 1, s.step, vec![], s.round + 1)
        };
        state.set(next);
        result.set(None);
    };

    let Some(item) = current else {
        // Block finished
        let has_more = !s.learned.is_empty() || remaining > 0;
        let (round, block) = (s.round, s.block);
        if !has_more && variant == Variant::Favorites {
            if !p.favorites.iter().any(|&id| question(id).is_some()) {
                return rsx! { Empty { icon: "bookmark", title: "Keine markierten Fragen", text: "Markiere Fragen mit dem Lesezeichen, um sie hier zu lernen." } };
            }
            return rsx! {
                div { class: "page center",
                    div { class: "panel done",
                        div { class: "empty-icon ok", Icon { name: "trophy" } }
                        h2 { "Durchlauf {round} geschafft!" }
                        p { "Du hast alle markierten Fragen durchgearbeitet. Sie bleiben markiert – starte einfach eine neue Runde." }
                        button { class: "btn primary big", onclick: next_round, Icon { name: "repeat" } "Von vorne starten" }
                    }
                }
            };
        }
        if !has_more {
            return rsx! { Empty { icon: "trophy", title: "Alles gelernt!", text: "Du hast alle Fragen dieses Fachs richtig beantwortet. Stark!" } };
        }
        let learned_text = if s.learned.is_empty() {
            String::new()
        } else {
            format!("Die {} gelernten Fragen kommen im nächsten Block als Test wieder. ", s.learned.len())
        };
        let rest_text = if remaining > 0 {
            let kind = if variant == Variant::Favorites { "markierte" } else { "offene" };
            format!("Noch {remaining} {kind} Fragen offen.")
        } else {
            "Danach sind alle Fragen durch.".to_string()
        };
        return rsx! {
            div { class: "page center",
                div { class: "panel done",
                    div { class: "empty-icon ok", Icon { name: "check" } }
                    h2 { "Block {block} geschafft!" }
                    p { "{learned_text}{rest_text}" }
                    button {
                        class: "btn primary big",
                        onmounted: move |e: MountedEvent| async move { let _ = e.data().set_focus(true).await; },
                        onclick: next_block,
                        "Nächster Block"
                        Icon { name: "next" }
                    }
                }
            }
        };
    };

    let res = result();
    let next_label = match res {
        None => "Weiter",
        Some(ok) if item.surprise => if ok { "Stark! Weiter" } else { "Weiter – die kommt irgendwann wieder" },
        Some(ok) if item.test => if ok { "Test bestanden – weiter" } else { "Nicht bestanden – kommt nochmal" },
        Some(ok) => if ok { "Richtig – weiter" } else { "Falsch – kommt nochmal" },
    }
    .to_string();
    let badge = if item.surprise {
        Some(Badge::Surprise)
    } else if item.test {
        Some(Badge::Test)
    } else if item.retry {
        Some(Badge::Retry)
    } else {
        None
    };
    let pct = if s.total > 0 { s.done * 100 / s.total } else { 0 };
    let round_label = if variant == Variant::Favorites { format!("Runde {} · ", s.round) } else { String::new() };
    let (block, done, total, step, item_id) = (s.block, s.done, s.total, s.step, item.id);
    let number = (done + 1) as usize;
    let rest_label = if remaining > 0 { format!(" · noch {remaining} offen") } else { String::new() };

    let answer_item = item.clone();
    let on_answer = move |correct: bool| {
        let it = &answer_item;
        result.set(Some(correct));
        let kind = if it.surprise { "surprise" } else if it.test { "test" } else if it.retry { "retry" } else { "learn" };
        log(
            "drill.answer",
            json!({ "variant": format!("{variant:?}"), "block": state.peek().block, "id": it.id, "caa": question(it.id).map(|q| q.caa_id.clone()), "kind": kind, "correct": correct }),
        );
        let mut a = app.write();
        if it.surprise {
            // only reschedules it (and counts a miss when wrong)
            a.p.record_surprise(it.id, correct);
        } else if it.test {
            // only tests change the saved result ...
            a.p.mark_answer(it.id, correct);
        } else if !correct {
            // ... except that a wrong answer in "Alle" counts as wrong
            if variant == Variant::All {
                a.p.mark_answer(it.id, false);
            } else {
                a.p.record_miss(it.id);
            }
        }
    };
    let on_next = move |_: ()| {
        // copy out first: a read guard held across `result.set` would panic
        let answered = *result.peek();
        if let Some(ok) = answered {
            state.with_mut(|s| logic::advance(s, ok));
            result.set(None);
        }
    };

    rsx! {
        div { class: "page learn",
            div { class: "blockbar",
                div { class: "blockbar-top",
                    span { class: "blockbar-title", "{round_label}Block {block}" }
                    span { class: "mono muted", "{done} / {total} geschafft{rest_label}" }
                }
                div { class: "bar", div { class: "bar-fill", style: "width: {pct}%" } }
            }
            QuestionCard {
                key: "drill-{item_id}-{step}",
                qid: item_id,
                salt: step,
                number,
                total: total as usize,
                on_answer,
                badge,
                surprise: item.surprise,
                hide_similar: item.test,
                on_next: Some(EventHandler::new(on_next)),
                next_label,
            }
        }
    }
}

// ----------------------------------------------------------------- settings

/// Web: asks the service worker whether every file is cached, so you know
/// before take-off that the app works in flight mode.
#[cfg(target_arch = "wasm32")]
fn use_offline_status() -> (bool, String) {
    const JS: &str = r#"
        const sw = navigator.serviceWorker;
        if (!sw || !sw.controller) return { ready: false, reason: 'nosw' };
        return await new Promise((resolve) => {
          const timer = setTimeout(() => resolve({ ready: false, reason: 'timeout' }), 4000);
          sw.addEventListener('message', function handler(e) {
            if (e.data && e.data.type === 'offline-status') {
              clearTimeout(timer);
              sw.removeEventListener('message', handler);
              resolve(e.data);
            }
          });
          sw.controller.postMessage('offline-status');
        });"#;
    let status = use_resource(|| async { document::eval(JS).await.ok() });
    let value = status.read().clone();
    match value {
        None => (false, "Prüfe Offline-Speicher …".into()),
        Some(None) => (false, "Offline-Status unbekannt.".into()),
        Some(Some(v)) => {
            if v["ready"].as_bool() == Some(true) {
                let n = v["files"].as_u64().unwrap_or(0);
                (true, format!("Offline bereit – alle {n} Dateien sind auf dem Gerät. Funktioniert im Flugmodus."))
            } else if v["reason"].as_str() == Some("nosw") {
                (false, "Noch nicht offline bereit. Öffne die App einmal mit Internet, schließe sie und starte sie neu.".into())
            } else if let Some(m) = v["missing"].as_u64() {
                (false, format!("Lädt noch – {m} Dateien fehlen. Kurz mit Internet offen lassen."))
            } else {
                (false, "Offline-Status unbekannt – App mit Internet neu starten.".into())
            }
        }
    }
}

/// Native app: everything is built in.
#[cfg(not(target_arch = "wasm32"))]
fn use_offline_status() -> (bool, String) {
    (true, "Alles ist in der App enthalten – läuft komplett offline.".into())
}

#[component]
fn Settings(on_close: EventHandler<()>) -> Element {
    let mut app = use_app();
    let mut status = use_signal(|| None::<(bool, String)>);
    let mut confirm_reset = use_signal(|| false);
    let mut paste = use_signal(String::new);
    let theme = app.read().theme.clone();

    let (offline_ok, offline_text) = use_offline_status();

    let export_text = move || serde_json::to_string_pretty(&to_progress(&app.peek())).unwrap_or_default();

    rsx! {
        div { class: "sheet-backdrop", onclick: move |_| on_close.call(()),
            div { class: "sheet", onclick: move |e| e.stop_propagation(),
                div { class: "sheet-head",
                    h2 { "Einstellungen" }
                    button { class: "iconbtn", "aria-label": "Schließen", onclick: move |_| on_close.call(()), Icon { name: "close" } }
                }

                if let Some((ok, msg)) = status() {
                    div { class: if ok { "toast ok" } else { "toast bad" }, "{msg}" }
                }

                section {
                    h3 { "Darstellung" }
                    div { class: "seg wide",
                        button { class: if theme == "dark" { "on" } else { "" }, onclick: move |_| app.write().theme = "dark".into(), Icon { name: "moon" } "Nachtflug" }
                        button { class: if theme == "light" { "on" } else { "" }, onclick: move |_| app.write().theme = "light".into(), Icon { name: "sun" } "Tag" }
                    }
                }

                section {
                    h3 { "Daten übertragen" }
                    p { class: "hint", "Der Fortschritt liegt nur auf diesem Gerät. Exportiere ihn als Sicherung oder um ihn auf ein anderes Gerät zu holen (gleiches Format wie die Web-App)." }
                    div { class: "row",
                        button { class: "btn", onclick: move |_| {
                            save_file(&format!("pof-progress-{}.json", util::today()), "application/json", &export_text());
                            log("progress.export", json!({}));
                            status.set(Some((true, "Export erstellt.".into())));
                        }, Icon { name: "download" } "Exportieren" }
                        button { class: "btn", onclick: move |_| {
                            copy_text(&export_text());
                            status.set(Some((true, "In die Zwischenablage kopiert.".into())));
                        }, Icon { name: "copy" } "Kopieren" }
                    }
                    label { class: "btn file",
                        Icon { name: "upload" }
                        "Datei importieren"
                        input {
                            r#type: "file",
                            accept: ".json,application/json",
                            onchange: move |evt: FormEvent| async move {
                                let Some(files) = evt.files() else { return };
                                for name in files.files() {
                                    match files.read_file_to_string(&name).await {
                                        Some(text) => status.set(Some(match import(app, &text) {
                                            Ok(m) => (true, m),
                                            Err(e) => (false, e),
                                        })),
                                        None => status.set(Some((false, "Die Datei konnte nicht gelesen werden.".into()))),
                                    }
                                }
                            },
                        }
                    }
                    details { class: "paste",
                        summary { "…oder JSON einfügen" }
                        textarea {
                            placeholder: "{{\"subjects\": …}}",
                            value: "{paste}",
                            oninput: move |e| paste.set(e.value()),
                        }
                        button { class: "btn", disabled: paste.read().trim().is_empty(), onclick: move |_| {
                            let text = paste.peek().clone();
                            status.set(Some(match import(app, &text) {
                                Ok(m) => { paste.set(String::new()); (true, m) }
                                Err(e) => (false, e),
                            }));
                        }, Icon { name: "upload" } "Einfügen & importieren" }
                    }
                }

                section {
                    h3 { "Fehlersuche" }
                    p { class: "hint", "Die App protokolliert, was passiert. Wenn etwas hakt: Log exportieren und mitschicken." }
                    div { class: "row",
                        button { class: "btn", onclick: move |_| {
                            save_file(&format!("pof-log-{}.jsonl", util::today()), "text/plain", &applog::export());
                        }, Icon { name: "file" } "Log exportieren" }
                        button { class: "btn", onclick: move |_| {
                            applog::clear();
                            status.set(Some((true, "Log geleert.".into())));
                        }, Icon { name: "trash" } "Log leeren" }
                    }
                }

                section {
                    h3 { "Offline" }
                    div { class: if offline_ok { "offline ok" } else { "offline" },
                        Icon { name: if offline_ok { "check" } else { "alert" } }
                        span { "{offline_text}" }
                    }
                }

                section {
                    h3 { "Aufs iPhone" }
                    p { class: "hint",
                        "Web-Version: in Safari öffnen → Teilen → „Zum Home-Bildschirm“. Danach läuft sie offline wie eine App. "
                        "Native App: siehe README (auf dem Mac mit „dx bundle --platform ios“ bauen)."
                    }
                }

                section { class: "danger",
                    h3 { "Gefahrenzone" }
                    if confirm_reset() {
                        p { class: "hint", "Wirklich alles löschen? Antworten, Markierungen, Ultra schwer und Lernblöcke sind dann weg." }
                        div { class: "row",
                            button { class: "btn", onclick: move |_| confirm_reset.set(false), "Abbrechen" }
                            button { class: "btn danger", onclick: move |_| {
                                log("progress.reset", json!({}));
                                {
                                    let mut a = app.write();
                                    a.p = SubjectProgress::default();
                                    a.gen += 1;
                                }
                                confirm_reset.set(false);
                                status.set(Some((true, "Fortschritt gelöscht.".into())));
                            }, Icon { name: "trash" } "Ja, löschen" }
                        }
                    } else {
                        button { class: "btn danger", onclick: move |_| confirm_reset.set(true), Icon { name: "retry" } "Fortschritt zurücksetzen" }
                    }
                }
            }
        }
    }
}

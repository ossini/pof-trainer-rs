//! Move the progress between devices without any server: a compact code
//! (`POF1-…`, deflate + base64url) to copy / share / paste, a link that
//! imports on open (web), and the JSON file of the web app. Every import is
//! shown side by side with the current state before it replaces anything.

use crate::icons::Icon;
use crate::model::{AppProgress, SubjectProgress, SUBJECT};
use crate::util::{applog::log, iso_from_ms, ms_from_iso, now_ms, today};
use crate::{copy_text, js, save_file, summary, to_progress, use_app, AppState};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use dioxus::prelude::*;
use serde_json::json;

pub const PREFIX: &str = "POF1-";

/// Pending import waiting for confirmation (shared via context)
pub type Pending = Signal<Option<AppProgress>>;

pub fn export_progress(s: &AppState) -> AppProgress {
    let mut ap = to_progress(s);
    ap.exported_at = Some(iso_from_ms(now_ms()));
    ap
}

pub fn encode(ap: &AppProgress) -> String {
    let json = serde_json::to_vec(ap).unwrap_or_default();
    let packed = miniz_oxide::deflate::compress_to_vec(&json, 9);
    format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(packed))
}

/// Accepts a code, a link containing a code, a whole shared message, or the
/// JSON file content.
pub fn decode(input: &str) -> Result<AppProgress, String> {
    let s = input.trim();
    if s.is_empty() {
        return Err("Nichts eingefügt.".into());
    }
    let ap: AppProgress = if s.starts_with('{') {
        serde_json::from_str(s).map_err(|e| format!("Die Datei ist unlesbar: {e}"))?
    } else {
        let start = s.find(PREFIX).ok_or("Kein Code gefunden. Er beginnt mit „POF1-“.")?;
        let code: String = s[start + PREFIX.len()..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        let broken = "Der Code ist unvollständig oder beschädigt. Am besten nochmal kopieren.";
        let bytes = URL_SAFE_NO_PAD.decode(code.as_bytes()).map_err(|_| broken)?;
        let json = miniz_oxide::inflate::decompress_to_vec_with_limit(&bytes, 20_000_000).map_err(|_| broken)?;
        serde_json::from_slice(&json).map_err(|_| broken)?
    };
    if !ap.subjects.contains_key(SUBJECT) {
        return Err("Darin ist kein Principles-of-Flight-Stand.".into());
    }
    Ok(ap)
}

/// `#stand=POF1-…` in the address (web): picked up once at start, then removed
/// from the address bar so a reload does not ask again.
pub fn take_link_code() -> Option<AppProgress> {
    #[cfg(target_arch = "wasm32")]
    {
        let hash = web_sys::window()?.location().hash().ok()?;
        let code = hash.strip_prefix("#stand=")?.to_string();
        document::eval("history.replaceState(null, '', location.pathname + location.search)");
        match decode(&code) {
            Ok(ap) => {
                log("transfer.link", json!({ "summary": summary(&ap.subjects[SUBJECT]) }));
                Some(ap)
            }
            Err(e) => {
                log("transfer.link.failed", json!({ "error": e }));
                None
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

pub fn ago(iso: &str) -> Option<String> {
    let t = ms_from_iso(iso)?;
    let m = ((now_ms() - t) / 60_000.0).max(0.0) as u64;
    Some(match m {
        0 => "gerade eben".into(),
        1..=59 => format!("vor {m} Min."),
        60..=1439 => format!("vor {} Std.", m / 60),
        _ if m / 1440 == 1 => "vor 1 Tag".into(),
        _ => format!("vor {} Tagen", m / 1440),
    })
}

fn try_import(mut pending: Pending, mut status: Signal<Option<(bool, String)>>, input: &str) {
    match decode(input) {
        Ok(ap) => {
            status.set(None);
            pending.set(Some(ap));
        }
        Err(e) => status.set(Some((false, e))),
    }
}

// ------------------------------------------------------------------ sheet

#[component]
pub fn Transfer(on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let pending = use_context::<Pending>();
    let mut status = use_signal(|| None::<(bool, String)>);
    let mut text = use_signal(String::new);

    let (answered, fav) = {
        let a = app.read();
        (a.p.answered.len(), a.p.favorites.len())
    };

    let share = move |_: MouseEvent| {
        let ap = export_progress(&app.peek());
        let code = encode(&ap);
        let msg = format!(
            "Mein PoF-Lernstand ({answered} beantwortet, {fav} markiert). Link öffnen – oder in der App unter „Stand übertragen“ einfügen."
        );
        document::eval(&format!(
            r#"const code = {c}, text = {t};
            const web = location.protocol.startsWith('http');
            const url = web ? location.origin + location.pathname + '#stand=' + code : null;
            const data = url ? {{ title: 'PoF Trainer – Lernstand', text, url }} : {{ title: 'PoF Trainer – Lernstand', text: text + '\n\n' + code }};
            if (navigator.share) navigator.share(data).catch(() => {{}});
            else if (navigator.clipboard) navigator.clipboard.writeText(url || code);"#,
            c = js(&code),
            t = js(&msg)
        ));
        log("transfer.share", json!({ "chars": code.len() }));
    };

    let copy = move |_: MouseEvent| {
        let code = encode(&export_progress(&app.peek()));
        copy_text(&code);
        log("transfer.copy", json!({ "chars": code.len() }));
        status.set(Some((true, "Code kopiert. Auf dem anderen Gerät unter „Stand übertragen“ → „Einfügen“.".into())));
    };

    let save = move |_: MouseEvent| {
        let ap = export_progress(&app.peek());
        save_file(
            &format!("pof-progress-{}.json", today()),
            "application/json",
            &serde_json::to_string_pretty(&ap).unwrap_or_default(),
        );
        log("progress.export", json!({}));
    };

    let paste_clipboard = move |_: MouseEvent| {
        // started inside the tap, so Safari allows reading the clipboard
        let read = document::eval("return navigator.clipboard ? await navigator.clipboard.readText() : '';");
        spawn(async move {
            match read.await.ok().and_then(|v| v.as_str().map(str::to_string)) {
                Some(s) if !s.trim().is_empty() => try_import(pending, status, &s),
                _ => status.set(Some((false, "Zwischenablage ist leer oder gesperrt – füge den Code unten ins Feld ein.".into()))),
            }
        });
    };

    rsx! {
        div { class: "sheet-backdrop", onclick: move |_| on_close.call(()),
            div { class: "sheet", onclick: move |e| e.stop_propagation(),
                div { class: "sheet-head",
                    h2 { "Stand übertragen" }
                    button { class: "iconbtn", "aria-label": "Schließen", onclick: move |_| on_close.call(()), Icon { name: "close" } }
                }
                p { class: "hint", "Dein Fortschritt liegt nur auf diesem Gerät. So kommt er auf ein anderes – ganz ohne Server." }

                if let Some((ok, msg)) = status() {
                    div { class: if ok { "toast ok" } else { "toast bad" }, "{msg}" }
                }

                section {
                    h3 { "Von diesem Gerät senden" }
                    div { class: "stack",
                        button { class: "btn primary", onclick: share, Icon { name: "upload" } "Teilen …" }
                        div { class: "row",
                            button { class: "btn", onclick: copy, Icon { name: "copy" } "Kopieren" }
                            button { class: "btn", onclick: save, Icon { name: "download" } "Als Datei" }
                        }
                    }
                }

                section {
                    h3 { "Auf diesem Gerät empfangen" }
                    div { class: "stack",
                        button { class: "btn primary", onclick: paste_clipboard, Icon { name: "copy" } "Einfügen aus Zwischenablage" }
                        textarea {
                            placeholder: "…oder Code / Link hier einfügen",
                            rows: "3",
                            value: "{text}",
                            oninput: move |e| text.set(e.value()),
                        }
                        if !text.read().trim().is_empty() {
                            button { class: "btn", onclick: move |_| {
                                let t = text.peek().clone();
                                try_import(pending, status, &t);
                                text.set(String::new());
                            }, Icon { name: "check" } "Code prüfen" }
                        }
                        label { class: "btn file",
                            Icon { name: "file" }
                            "Datei wählen"
                            input {
                                r#type: "file",
                                accept: ".json,application/json,text/plain",
                                onchange: move |evt: FormEvent| async move {
                                    let Some(files) = evt.files() else { return };
                                    for name in files.files() {
                                        match files.read_file_to_string(&name).await {
                                            Some(t) => try_import(pending, status, &t),
                                            None => status.set(Some((false, "Die Datei konnte nicht gelesen werden.".into()))),
                                        }
                                    }
                                },
                            }
                        }
                    }
                    p { class: "hint small",
                        "Tipp iPhone: Safari und die App auf dem Home-Bildschirm speichern getrennt. "
                        "Für die Home-Bildschirm-App den Code dort unter „Stand übertragen“ einfügen."
                    }
                }
            }
        }
    }
}

// ----------------------------------------------------------- confirmation

fn stats(p: &SubjectProgress) -> [usize; 4] {
    [p.answered.len(), p.correct_count(), p.favorites.len(), p.ultra_hard.len()]
}

#[component]
pub fn ImportConfirm(on_done: EventHandler<String>) -> Element {
    let mut app = use_app();
    let mut pending = use_context::<Pending>();
    let Some(ap) = pending() else { return rsx! {} };

    let incoming = ap.subjects.get(SUBJECT).cloned().unwrap_or_default();
    let new = stats(&incoming);
    let cur = stats(&app.read().p);
    let when = ap
        .exported_at
        .as_deref()
        .and_then(ago)
        .map(|s| format!("Erstellt {s}"))
        .unwrap_or_else(|| "Ohne Zeitangabe".into());
    let fewer = new[0] < cur[0];
    let labels = ["Beantwortet", "Richtig", "Markiert", "Ultra schwer"];
    let rows: Vec<(&'static str, usize, usize)> = (0..4).map(|i| (labels[i], new[i], cur[i])).collect();

    let accept = move |_: MouseEvent| {
        let Some(ap) = pending.peek().clone() else { return };
        let p = ap.subjects.get(SUBJECT).cloned().unwrap_or_default();
        log("progress.import", json!({ "summary": summary(&p) }));
        {
            let mut a = app.write();
            a.p = p;
            a.theme = if ap.theme == "light" { "light".into() } else { "dark".into() };
            a.gen += 1;
        }
        pending.set(None);
        on_done.call("Stand übernommen.".into());
    };

    rsx! {
        div { class: "sheet-backdrop top",
            div { class: "sheet confirm",
                div { class: "sheet-head", h2 { "Stand übernehmen?" } }
                p { class: "hint", "{when}. Der Stand auf diesem Gerät wird dabei ersetzt." }
                table { class: "compare",
                    thead { tr { th {} th { "Neu" } th { "Dieses Gerät" } } }
                    tbody {
                        for (label, n, c) in rows {
                            tr {
                                td { "{label}" }
                                td { class: "num", "{n}" }
                                td { class: "num muted", "{c}" }
                            }
                        }
                    }
                }
                if fewer {
                    div { class: "toast bad", "Achtung: Der neue Stand hat weniger beantwortete Fragen als dieses Gerät." }
                }
                div { class: "row",
                    button { class: "btn", onclick: move |_| pending.set(None), "Abbrechen" }
                    button { class: "btn primary", onclick: accept, "Übernehmen" }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_roundtrip() {
        let ap: AppProgress = serde_json::from_str(include_str!("../data/seed-progress.json")).unwrap();
        let code = encode(&ap);
        assert!(code.starts_with(PREFIX));
        assert_eq!(decode(&code).unwrap(), ap);
        // inside a shared message / link
        let msg = format!("Mein Stand: https://example.org/app/#stand={code}\n");
        assert_eq!(decode(&msg).unwrap(), ap);
        // JSON file
        assert_eq!(decode(&serde_json::to_string(&ap).unwrap()).unwrap(), ap);
        assert!(decode("POF1-abc").is_err());
        assert!(decode("hallo").is_err());
    }
}

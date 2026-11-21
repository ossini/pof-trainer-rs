//! Inline SVG icons (lucide style), no icon font or network needed.

use dioxus::prelude::*;

fn paths(name: &str) -> &'static str {
    match name {
        "back" => r#"<path d="m15 18-6-6 6-6"/>"#,
        "next" => r#"<path d="m9 18 6-6-6-6"/>"#,
        "down" => r#"<path d="m6 9 6 6 6-6"/>"#,
        "settings" => r#"<path d="M4 6h10M18 6h2M4 12h4M12 12h8M4 18h12M20 18h0"/><circle cx="16" cy="6" r="2"/><circle cx="10" cy="12" r="2"/><circle cx="18" cy="18" r="2"/>"#,
        "bookmark" => r#"<path d="m19 21-7-4-7 4V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v16z"/>"#,
        "bookmark-fill" => r#"<path fill="currentColor" d="m19 21-7-4-7 4V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v16z"/>"#,
        "sparkles" => r#"<path d="M12 3l1.9 5.1L19 10l-5.1 1.9L12 17l-1.9-5.1L5 10l5.1-1.9z"/><path d="M5 3v4M3 5h4M19 17v4M17 19h4"/>"#,
        "layers" => r#"<path d="m12 2 10 5-10 5L2 7z"/><path d="m2 17 10 5 10-5"/><path d="m2 12 10 5 10-5"/>"#,
        "list" => r#"<path d="M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01"/>"#,
        "alert" => r#"<circle cx="12" cy="12" r="10"/><path d="M12 8v4M12 16h.01"/>"#,
        "repeat" => r#"<path d="m17 2 4 4-4 4"/><path d="M3 11v-1a4 4 0 0 1 4-4h14"/><path d="m7 22-4-4 4-4"/><path d="M21 13v1a4 4 0 0 1-4 4H3"/>"#,
        "trophy" => r#"<path d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6M18 9h1.5a2.5 2.5 0 0 0 0-5H18M4 22h16M10 14.7V17c0 .6-.5 1-1 1.2C7.9 18.8 7 20.2 7 22M14 14.7V17c0 .6.5 1 1 1.2 1.1.6 2 2 2 3.8M18 2H6v7a6 6 0 0 0 12 0z"/>"#,
        "check" => r#"<path d="M20 6 9 17l-5-5"/>"#,
        "x" => r#"<path d="M18 6 6 18M6 6l12 12"/>"#,
        "bulb" => r#"<path d="M9 18h6M10 22h4M12 2a7 7 0 0 0-4 12.7c.6.5 1 1.3 1 2.3h6c0-1 .4-1.8 1-2.3A7 7 0 0 0 12 2z"/>"#,
        "compare" => r#"<circle cx="18" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><path d="M13 6h3a2 2 0 0 1 2 2v7M11 18H8a2 2 0 0 1-2-2V9"/>"#,
        "test" => r#"<rect x="8" y="2" width="8" height="4" rx="1"/><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/><path d="m9 14 2 2 4-4"/>"#,
        "retry" => r#"<path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/>"#,
        "sun" => r#"<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>"#,
        "moon" => r#"<path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9z"/>"#,
        "download" => r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M7 10l5 5 5-5M12 15V3"/>"#,
        "upload" => r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M17 8l-5-5-5 5M12 3v12"/>"#,
        "copy" => r#"<rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>"#,
        "file" => r#"<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6M16 13H8M16 17H8M10 9H8"/>"#,
        "trash" => r#"<path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>"#,
        "plane" => r#"<path d="M17.8 19.2 16 11l3.5-3.5C21 6 21.5 4 21 3c-1-.5-3 0-4.5 1.5L13 8 4.8 6.2c-.5-.1-.9.1-1.1.5l-.3.5c-.2.5-.1 1 .3 1.3L9 12l-2 3H4l-1 1 3 2 2 3 1-1v-3l3-2 3.5 5.3c.3.4.8.5 1.3.3l.5-.2c.4-.3.6-.7.5-1.2z"/>"#,
        "gem" => r#"<path d="M6 3h12l4 6-10 13L2 9z"/><path d="M11 3 8 9l4 13 4-13-3-6M2 9h20"/>"#,
        "gem-fill" => r#"<path fill="currentColor" fill-opacity=".25" d="M6 3h12l4 6-10 13L2 9z"/><path d="M11 3 8 9l4 13 4-13-3-6M2 9h20"/>"#,
        "phone" => r#"<rect x="5" y="2" width="14" height="20" rx="2"/><path d="M12 18h.01"/>"#,
        "close" => r#"<path d="M18 6 6 18M6 6l12 12"/>"#,
        _ => "",
    }
}

/// `if c { a } else { b }` for static strings. Inside `rsx!`, an inline `if`
/// turns string literals into `String`s, which `&'static str` props reject.
pub fn pick(c: bool, a: &'static str, b: &'static str) -> &'static str {
    if c { a } else { b }
}

pub fn svg(name: &str) -> String {
    format!(
        r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{}</svg>"#,
        paths(name)
    )
}

#[component]
pub fn Icon(name: &'static str) -> Element {
    rsx! { span { class: "ic", dangerous_inner_html: svg(name) } }
}

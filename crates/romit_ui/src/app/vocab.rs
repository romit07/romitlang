//! The words Romit apps understand, with their SwiftUI counterparts.

/// Every built-in element. (SwiftUI name in comments.)
pub const ELEMENTS: &[&str] = &[
    // layout
    "column",     // VStack
    "row",        // HStack
    "stack",      // ZStack
    "grid",       // LazyVGrid / Grid
    "wrap",       // flow layout
    "scroll",     // ScrollView
    "spacer",     // Spacer
    "divider",    // Divider
    "group",      // Group
    "list",       // List
    "section",    // Section
    "form",       // Form
    "box",        // GroupBox
    "card",       // styled container
    "disclosure", // DisclosureGroup
    // text & media
    "text",       // Text
    "title",      // Text .largeTitle
    "heading",    // Text .title2
    "subheading", // Text .headline
    "caption",    // Text .caption
    "label",      // Label
    "image",      // Image / AsyncImage
    "icon",       // Image(systemName:)
    "video",      // VideoPlayer
    "circle",     // Circle
    "rectangle",  // Rectangle
    "capsule",    // Capsule
    "rounded",    // RoundedRectangle
    "ellipse",    // Ellipse
    // controls
    "button",      // Button
    "link",        // Link / NavigationLink
    "toggle",      // Toggle
    "field",       // TextField
    "secure",      // SecureField
    "editor",      // TextEditor
    "search",      // .searchable
    "slider",      // Slider
    "stepper",     // Stepper
    "picker",      // Picker
    "date",        // DatePicker
    "colorpicker", // ColorPicker
    "menu",        // Menu
    "share",       // ShareLink
    "progress",    // ProgressView
    "spinner",     // ProgressView()
    "gauge",       // Gauge
    "empty",       // ContentUnavailableView
];

pub fn is_element(w: &str) -> bool {
    ELEMENTS.contains(&w)
}

/// Every modifier. Aliases map onto these in [`canonical_modifier`].
pub const MODIFIERS: &[&str] = &[
    "padding",
    "gap",
    "width",
    "height",
    "size",
    "maxwidth",
    "minheight",
    "stretch",
    "color",
    "background",
    "gradient",
    "tint",
    "fill",
    "stroke",
    "corner",
    "border",
    "shadow",
    "opacity",
    "blur",
    "rotate",
    "scale",
    "offset",
    "z",
    "hidden",
    "disabled",
    "bold",
    "italic",
    "underline",
    "strike",
    "mono",
    "serif",
    "uppercase",
    "lowercase",
    "font",
    "weight",
    "lines",
    "align",
    "center",
    "leading",
    "trailing",
    "top",
    "bottom",
    "selectable",
    "help",
    "clip",
    "aspect",
    "transition",
    "animated",
    "badge",
    "focused",
    "describe",
    "style",
    "role",
    "keyboard",
    "grow",
    "sticky",
];

const MODIFIER_ALIASES: &[(&str, &str)] = &[
    ("spacing", "gap"),
    ("foreground", "color"),
    ("textcolor", "color"),
    ("bg", "background"),
    ("corners", "corner"),
    ("radius", "corner"),
    ("cornerradius", "corner"),
    ("full", "stretch"),
    ("wide", "stretch"),
    ("strikethrough", "strike"),
    ("monospaced", "mono"),
    ("tooltip", "help"),
    ("faded", "opacity"),
    ("alpha", "opacity"),
    ("rotation", "rotate"),
    ("zindex", "z"),
    ("linelimit", "lines"),
    ("accessibility", "describe"),
    ("autofocus", "focused"),
    ("animation", "animated"),
];

pub fn canonical_modifier(w: &str) -> &str {
    MODIFIER_ALIASES
        .iter()
        .find(|(a, _)| *a == w)
        .map(|(_, c)| *c)
        .unwrap_or(w)
}

pub fn is_modifier(w: &str) -> bool {
    MODIFIERS.contains(&canonical_modifier(w))
}

/// (min, max) number of values a modifier takes.
pub fn modifier_arity(m: &str) -> (usize, usize) {
    match m {
        "bold" | "italic" | "underline" | "strike" | "mono" | "serif" | "uppercase"
        | "lowercase" | "center" | "leading" | "trailing" | "top" | "bottom" | "stretch"
        | "selectable" | "hidden" | "disabled" | "focused" | "grow" | "sticky" => (0, 0),
        "padding" => (0, 2),
        "shadow" | "corner" | "blur" | "gap" | "transition" | "animated" | "clip" | "aspect" => {
            (0, 1)
        }
        "border" | "offset" | "gradient" => (1, 2),
        _ => (1, 1),
    }
}

/// Words that are enum values for a modifier (so `padding horizontal` is not parsed as two modifiers).
pub fn modifier_takes_word(m: &str, w: &str) -> bool {
    matches!(
        (m, w),
        ("padding", "top" | "bottom" | "leading" | "trailing")
            | (
                "align",
                "center" | "leading" | "trailing" | "top" | "bottom"
            )
            | ("clip", "center")
    )
}

pub fn modifier_help(m: &str) -> String {
    match m {
        "color" | "background" | "tint" | "fill" | "stroke" => format!("e.g. `{m} blue` or `{m} \"#7c3aed\"`"),
        "font" => "fonts: title, heading, subheading, body, caption, small, big, huge, mono, serif, rounded".into(),
        "weight" => "weights: thin, light, regular, medium, semibold, bold, heavy, black".into(),
        "style" => "styles: prominent, bordered, plain, link, glass, segmented, menu, wheel, switch, checkbox, inset, grouped".into(),
        "role" => "roles: destructive, cancel".into(),
        "align" => "align: leading, center, trailing, top, bottom".into(),
        "transition" => "transitions: fade, slide, scale, move".into(),
        "keyboard" => "keyboards: text, number, decimal, email, phone, url".into(),
        _ => format!("e.g. `{m} 16`"),
    }
}

/// Keyword arguments elements accept: `toggle "Dark" bind dark`, `slider bind v from 0 to 10`.
pub const KWARGS: &[&str] = &[
    "bind",
    "from",
    "to",
    "step",
    "options",
    "icon",
    "of",
    "message",
    "footer",
    "with",
    "placeholder",
    "header",
    "columns",
    "min",
];

/// Which keyword arguments each element takes.
pub fn element_kwargs(kind: &str) -> &'static [&'static str] {
    match kind {
        "toggle" | "field" | "secure" | "editor" | "search" | "date" | "colorpicker"
        | "disclosure" => &["bind", "placeholder"],
        "slider" | "stepper" => &["bind", "from", "to", "step"],
        "picker" => &["bind", "options"],
        "button" | "link" => &["to", "with", "icon"],
        "label" | "menu" => &["icon"],
        "progress" => &["of"],
        "gauge" => &["from", "to", "of"],
        "empty" => &["icon", "message"],
        "section" => &["header", "footer"],
        "grid" => &["columns", "min"],
        k if k.chars().next().is_some_and(|c| c.is_uppercase()) => &["with"],
        _ => &[],
    }
}

pub fn is_kwarg(w: &str) -> bool {
    KWARGS.contains(&canonical_kwarg(w))
}

pub fn canonical_kwarg(w: &str) -> &str {
    match w {
        "binds" | "bound" => "bind",
        "choices" => "options",
        "through" => "to",
        "prompt" => "placeholder",
        other => other,
    }
}

pub const THEME_KEYS: &[&str] = &[
    "accent",
    "background",
    "surface",
    "text",
    "muted",
    "separator",
    "corner",
    "font",
    "appearance",
    "dark",
    "spacing",
    "danger",
    "success",
    "warning",
    "titlefont",
    "width",
];

/// Reasons a word can't name a state value or parameter.
pub fn bad_value_name(w: &str) -> Option<&'static str> {
    if is_modifier(w) {
        return Some("it is a modifier word");
    }
    if w == "bind" || w == "with" || w == "options" {
        return Some("it is a keyword");
    }
    if super::expr::is_reserved_word(w) || matches!(w, "index" | "it" | "now" | "today" | "screen")
    {
        return Some("it is a reserved word");
    }
    None
}

pub fn did_you_mean(word: &str, options: &[&str]) -> String {
    let mut best: Option<(&str, usize)> = None;
    for o in options {
        let d = levenshtein(word, o);
        if d <= 2.max(word.len() / 3) && best.is_none_or(|(_, bd)| d < bd) {
            best = Some((o, d));
        }
    }
    best.map(|(o, _)| format!(" did you mean `{o}`?"))
        .unwrap_or_default()
}

pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Named colors (Apple system palette).
pub fn named_color(w: &str) -> Option<&'static str> {
    Some(match w {
        "red" => "#ff3b30",
        "orange" => "#ff9500",
        "yellow" => "#ffcc00",
        "green" => "#34c759",
        "mint" => "#00c7be",
        "teal" => "#30b0c7",
        "cyan" => "#32ade6",
        "blue" => "#007aff",
        "indigo" => "#5856d6",
        "purple" => "#af52de",
        "pink" => "#ff2d55",
        "brown" => "#a2845e",
        "gray" | "grey" => "#8e8e93",
        "black" => "#000000",
        "white" => "#ffffff",
        "clear" | "transparent" => "transparent",
        "primary" => "var(--text)",
        "secondary" | "muted" => "var(--muted)",
        "accent" => "var(--accent)",
        "surface" => "var(--surface)",
        "danger" => "var(--danger)",
        "success" => "var(--success)",
        "warning" => "var(--warning)",
        _ => return None,
    })
}

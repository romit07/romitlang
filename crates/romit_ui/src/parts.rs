pub const VALID_PARTS: &[&str] = &[
    "page", "header", "brand", "nav", "hero", "title", "text", "section", "card", "button",
    "image", "link", "footer", "theme",
];

pub fn is_valid_part(word: &str) -> bool {
    VALID_PARTS.contains(&word)
}

pub fn valid_parts_list() -> String {
    VALID_PARTS.join(", ")
}

/// Whether `child` may appear indented under `parent`.
pub fn allowed_child(parent: &str, child: &str) -> bool {
    match parent {
        "page" => matches!(child, "header" | "hero" | "section" | "footer" | "theme"),
        "header" => matches!(child, "brand" | "nav" | "button" | "link" | "image"),
        "hero" => matches!(child, "title" | "text" | "button" | "link" | "image"),
        "section" => matches!(
            child,
            "card" | "title" | "text" | "button" | "link" | "image"
        ),
        "card" => matches!(child, "title" | "text" | "button" | "link" | "image"),
        "footer" => matches!(child, "text" | "link" | "button"),
        "theme" => false, // theme uses token lines, not nested parts
        _ => false,
    }
}

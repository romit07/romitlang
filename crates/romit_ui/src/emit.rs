use crate::parse::{attrs_map, UiNode};
use crate::template::{named_color, TokenSet};

pub struct BuildOutput {
    pub html: String,
    pub css: String,
}

pub fn render_page(page: &UiNode, tokens: &TokenSet, page_title: &str) -> BuildOutput {
    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("  <meta charset=\"utf-8\" />\n");
    html.push_str("  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n");
    html.push_str(&format!("  <title>{}</title>\n", escape_html(page_title)));
    html.push_str("  <link rel=\"stylesheet\" href=\"style.css\" />\n");
    html.push_str("</head>\n<body>\n");
    html.push_str("  <div class=\"page\">\n");

    for child in &page.children {
        if child.kind == "theme" {
            continue;
        }
        render_part(&mut html, child, tokens, tokens);
    }

    html.push_str("  </div>\n</body>\n</html>\n");

    let css = stylesheet(tokens);
    BuildOutput { html, css }
}

fn render_part(html: &mut String, node: &UiNode, base_tokens: &TokenSet, _inherited: &TokenSet) {
    let local = base_tokens.apply_element_attr(base_tokens, &node.attrs);
    match node.kind.as_str() {
        "header" => {
            html.push_str("    <header class=\"site-header\">\n");
            html.push_str("      <div class=\"header-inner\">\n");
            for c in &node.children {
                render_part(html, c, &local, &local);
            }
            html.push_str("      </div>\n    </header>\n");
        }
        "brand" => {
            let label = node.strings.first().map(String::as_str).unwrap_or("Brand");
            html.push_str(&format!(
                "        <span class=\"brand\">{}</span>\n",
                escape_html(label)
            ));
        }
        "nav" => {
            html.push_str("        <nav class=\"nav\" aria-label=\"Primary\">\n");
            for label in &node.strings {
                html.push_str(&format!(
                    "          <a class=\"nav-link\" href=\"#\">{}</a>\n",
                    escape_html(label)
                ));
            }
            html.push_str("        </nav>\n");
        }
        "hero" => {
            html.push_str("    <section class=\"hero\">\n");
            for c in &node.children {
                render_part(html, c, &local, &local);
            }
            html.push_str("    </section>\n");
        }
        "title" => {
            let text = node.strings.first().map(String::as_str).unwrap_or("");
            html.push_str(&format!(
                "      <h1 class=\"hero-title\">{}</h1>\n",
                escape_html(text)
            ));
        }
        "text" => {
            let text = node.strings.first().map(String::as_str).unwrap_or("");
            html.push_str(&format!(
                "      <p class=\"text\">{}</p>\n",
                escape_html(text)
            ));
        }
        "section" => {
            let heading = node.strings.first().map(String::as_str).unwrap_or("");
            html.push_str("    <section class=\"content-section\">\n");
            if !heading.is_empty() {
                html.push_str(&format!(
                    "      <h2 class=\"section-title\">{}</h2>\n",
                    escape_html(heading)
                ));
            }
            html.push_str("      <div class=\"card-grid\">\n");
            for c in &node.children {
                render_part(html, c, &local, &local);
            }
            html.push_str("      </div>\n    </section>\n");
        }
        "card" => {
            let title = node.strings.first().map(String::as_str).unwrap_or("");
            let body = node.strings.get(1).map(String::as_str).unwrap_or("");
            html.push_str("        <article class=\"card\">\n");
            if !title.is_empty() {
                html.push_str(&format!(
                    "          <h3 class=\"card-title\">{}</h3>\n",
                    escape_html(title)
                ));
            }
            if !body.is_empty() {
                html.push_str(&format!(
                    "          <p class=\"card-body\">{}</p>\n",
                    escape_html(body)
                ));
            }
            for c in &node.children {
                render_part(html, c, &local, &local);
            }
            html.push_str("        </article>\n");
        }
        "button" => {
            let label = node.strings.first().map(String::as_str).unwrap_or("Button");
            let attrs = attrs_map(&node.attrs);
            let mut style = String::new();
            if let Some(c) = attrs.get("color") {
                let resolved = named_color(c).unwrap_or_else(|| c.clone());
                style.push_str(&format!(" style=\"--btn-bg:{resolved};\""));
            }
            html.push_str(&format!(
                "      <button type=\"button\" class=\"btn\"{}>{}</button>\n",
                style,
                escape_html(label)
            ));
        }
        "image" => {
            let alt = node.strings.first().map(String::as_str).unwrap_or("");
            let src = node.strings.get(1).map(String::as_str).unwrap_or("");
            html.push_str(&format!(
                "      <img class=\"img\" src=\"{}\" alt=\"{}\" />\n",
                escape_html(src),
                escape_html(alt)
            ));
        }
        "link" => {
            let label = node.strings.first().map(String::as_str).unwrap_or("");
            let href = node.strings.get(1).map(String::as_str).unwrap_or("#");
            html.push_str(&format!(
                "      <a class=\"link\" href=\"{}\">{}</a>\n",
                escape_html(href),
                escape_html(label)
            ));
        }
        "footer" => {
            html.push_str("    <footer class=\"site-footer\">\n");
            for c in &node.children {
                render_part(html, c, &local, &local);
            }
            html.push_str("    </footer>\n");
        }
        _ => {}
    }
}

pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn stylesheet(tokens: &TokenSet) -> String {
    let v = |k: &str| tokens.values.get(k).map(String::as_str).unwrap_or("");
    format!(
        r#":root {{
  --color-primary: {cp};
  --color-text: {ct};
  --color-muted: {cm};
  --color-background: {cb};
  --color-accent: {ca};
  --color-button: {cbtn};
  --color-button-text: {cbt};
  --space-unit: {su};
  --space-section: {ss};
  --space-gap: {sg};
  --font-family: {ff};
  --font-size-base: {fb};
  --font-size-title: {ft};
  --font-size-hero: {fh};
  --radius-button: {rb};
  --radius-card: {rc};
  --max-width: {mw};
}}

* {{
  box-sizing: border-box;
}}

body {{
  margin: 0;
  font-family: var(--font-family);
  font-size: var(--font-size-base);
  color: var(--color-text);
  background: var(--color-background);
  line-height: 1.5;
}}

.page {{
  max-width: var(--max-width);
  margin: 0 auto;
  padding: var(--space-unit) calc(var(--space-unit) * 2);
}}

.site-header {{
  padding: calc(var(--space-unit) * 2) 0;
  border-bottom: 1px solid color-mix(in srgb, var(--color-text) 12%, transparent);
}}

.header-inner {{
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-gap);
  justify-content: space-between;
}}

.brand {{
  font-weight: 700;
  font-size: var(--font-size-title);
  color: var(--color-primary);
}}

.nav {{
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-gap);
}}

.nav-link {{
  color: var(--color-text);
  text-decoration: none;
}}

.nav-link:hover {{
  color: var(--color-accent);
}}

.hero {{
  padding: var(--space-section) 0;
  text-align: center;
}}

.hero-title {{
  font-size: var(--font-size-hero);
  margin: 0 0 var(--space-gap);
}}

.text {{
  color: var(--color-muted);
  margin: 0 0 var(--space-gap);
}}

.content-section {{
  padding: var(--space-section) 0;
}}

.section-title {{
  font-size: var(--font-size-title);
  margin: 0 0 var(--space-gap);
}}

.card-grid {{
  display: grid;
  gap: var(--space-gap);
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
}}

.card {{
  padding: calc(var(--space-unit) * 2);
  border-radius: var(--radius-card);
  border: 1px solid color-mix(in srgb, var(--color-text) 15%, transparent);
  background: color-mix(in srgb, var(--color-background) 92%, var(--color-primary));
}}

.card-title {{
  margin: 0 0 var(--space-unit);
  font-size: 1.125rem;
}}

.card-body {{
  margin: 0;
  color: var(--color-muted);
}}

.btn {{
  display: inline-block;
  margin: var(--space-unit) 0;
  padding: calc(var(--space-unit) * 1.25) calc(var(--space-unit) * 2.5);
  border: none;
  border-radius: var(--radius-button);
  background: var(--btn-bg, var(--color-button));
  color: var(--color-button-text);
  font: inherit;
  cursor: pointer;
}}

.site-footer {{
  padding: var(--space-section) 0 calc(var(--space-unit) * 2);
  border-top: 1px solid color-mix(in srgb, var(--color-text) 12%, transparent);
  color: var(--color-muted);
  text-align: center;
}}

.link {{
  color: var(--color-accent);
}}

.img {{
  max-width: 100%;
  height: auto;
}}

@media (max-width: 640px) {{
  .header-inner {{
    flex-direction: column;
    align-items: flex-start;
  }}
  .hero-title {{
    font-size: calc(var(--font-size-hero) * 0.75);
  }}
}}
"#,
        cp = v("color-primary"),
        ct = v("color-text"),
        cm = v("color-muted"),
        cb = v("color-background"),
        ca = v("color-accent"),
        cbtn = v("color-button"),
        cbt = v("color-button-text"),
        su = v("space-unit"),
        ss = v("space-section"),
        sg = v("space-gap"),
        ff = v("font-family"),
        fb = v("font-size-base"),
        ft = v("font-size-title"),
        fh = v("font-size-hero"),
        rb = v("radius-button"),
        rc = v("radius-card"),
        mw = v("max-width"),
    )
}

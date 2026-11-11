mod diag;
pub mod emit;
mod parse;
mod parts;
pub mod preview;
mod template;

pub use diag::{format_diagnostics, Diagnostic};
pub use parse::{is_ui_source, parse, UiFile, UiNode};
pub use preview::serve_preview;
pub use template::{valid_tokens_list, TokenSet, VALID_TOKEN_KEYS};

use std::fs;
use std::path::{Path, PathBuf};

pub fn build_ui(source: &str, file: &str, out_dir: &Path) -> Result<(), Vec<Diagnostic>> {
    let ui = parse(source, file)?;
    let page = ui
        .page
        .as_ref()
        .ok_or_else(|| vec![Diagnostic::error("U003", "missing page", file, 1, 1, "")])?;

    let base_dir = Path::new(file).parent();
    let template_name = ui.template_name.as_deref().unwrap_or("default");
    let tokens = template::resolve_for_page(template_name, base_dir, page, source, file)?;

    let page_name = page
        .strings
        .first()
        .cloned()
        .unwrap_or_else(|| "Page".to_string());

    let out = emit::render_page(page, &tokens, &page_name);

    fs::create_dir_all(out_dir).map_err(|e| {
        vec![Diagnostic::error(
            "U008",
            format!("cannot create output dir: {e}"),
            file,
            1,
            1,
            "",
        )]
    })?;

    fs::write(out_dir.join("index.html"), &out.html).map_err(|e| io_err(file, e))?;
    fs::write(out_dir.join("style.css"), &out.css).map_err(|e| io_err(file, e))?;

    Ok(())
}

fn io_err(file: &str, e: std::io::Error) -> Vec<Diagnostic> {
    vec![Diagnostic::error(
        "U008",
        format!("write error: {e}"),
        file,
        1,
        1,
        "",
    )]
}

pub fn default_out_dir(source_path: &Path) -> PathBuf {
    source_path
        .parent()
        .map(|p| p.join("dist"))
        .unwrap_or_else(|| PathBuf::from("dist"))
}

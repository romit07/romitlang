//! Romit apps (`use ui` + `app`/`screen`): parsing, checking, and code generation.

use romit_ui::app::{build_app_html, compile_js, is_app_source};

fn ok(src: &str) -> String {
    match compile_js(src, "t.rmt") {
        Ok(js) => js,
        Err(d) => panic!("{}", romit_ui::format_diagnostics(&d)),
    }
}

fn err(src: &str, code: &str) -> String {
    let d = compile_js(src, "t.rmt").expect_err("expected an error");
    let text = romit_ui::format_diagnostics(&d);
    assert!(
        d.iter().any(|x| x.code == code),
        "expected {code}, got:\n{text}"
    );
    text
}

#[test]
fn examples_compile() {
    for (name, src) in [
        (
            "counter",
            include_str!("../../../examples/apps/counter.rmt"),
        ),
        ("todos", include_str!("../../../examples/apps/todos.rmt")),
        (
            "showcase",
            include_str!("../../../examples/apps/showcase.rmt"),
        ),
        ("notes", include_str!("../../../examples/apps/notes.rmt")),
        ("mail", include_str!("../../../examples/apps/mail.rmt")),
    ] {
        assert!(is_app_source(src), "{name} should be an app");
        let html = build_app_html(src, name)
            .unwrap_or_else(|d| panic!("{name}:\n{}", romit_ui::format_diagnostics(&d)));
        assert!(html.contains("R.start()"));
        assert!(!html.contains("</script>\n</script>"));
    }
}

#[test]
fn landing_page_is_not_an_app() {
    assert!(!is_app_source(include_str!(
        "../../../examples/ui/landing.rmt"
    )));
}

#[test]
fn minimal_app_without_app_line() {
    let js = ok("use ui\nscreen Home\n  text \"Hi\"\n");
    assert!(js.contains("R.screens[\"Home\"]"));
}

#[test]
fn state_and_actions() {
    let js = ok("use ui\napp A\n  state\n    count is 0\n  screen Home\n    button \"+\" do count += 1\n    text \"{count}\" bold\n");
    assert!(js.contains("S.count = R.add(S.count, 1);"));
    assert!(js.contains("[\"bold\"]"));
}

#[test]
fn conditional_modifiers_and_if_otherwise() {
    let js = ok("use ui\nstate\n  n is 0\nscreen Home\n  if n is 0\n    text \"zero\"\n  otherwise if n is above 0\n    text \"pos\"\n  otherwise\n    text \"neg\"\n  text \"x\" color red if n is below 0\n");
    assert!(js.contains("? [\"color\", \"#ff3b30\"] : null"));
}

#[test]
fn bindings_and_controls() {
    let js = ok("use ui\nstate\n  name is \"\"\n  lit is false\n  v is 3\n  pick is \"A\"\nscreen Home\n  field \"Name\" bind name\n  toggle \"On\" bind lit\n  slider bind v from 0 to 10 step 1\n  picker \"P\" bind pick options \"A\", \"B\"\n");
    assert!(js.contains("set: (v) => { S.name = v; }"));
    assert!(js.contains("R.opts([\"A\", \"B\"])"));
}

#[test]
fn navigation_with_params() {
    let js = ok("use ui\nstate\n  items is list \"a\"\nscreen Home\n  for each item in items\n    link item to Detail with item\nscreen Detail with item\n  text item\n");
    assert!(js.contains("nav: {screen: \"Detail\", args: [L_item]}"));
}

#[test]
fn where_uses_item_fields() {
    let js = ok("use ui\nstate\n  todos is list\n    title \"a\", done true\ncomputed\n  left is count of todos where not done\nscreen Home\n  text \"{left}\"\n");
    assert!(js.contains("R.field(it, \"done\")"));
}

#[test]
fn unknown_name_suggests() {
    let text = err(
        "use ui\nstate\n  count is 0\nscreen Home\n  text \"{cuont}\"\n",
        "U101",
    );
    assert!(text.contains("did you mean `count`"), "{text}");
}

#[test]
fn unknown_element_suggests() {
    let text = err("use ui\nscreen Home\n  buton \"x\"\n", "U001");
    assert!(text.contains("did you mean `button`"), "{text}");
}

#[test]
fn modifier_word_cannot_be_state() {
    err(
        "use ui\nstate\n  size is 1\nscreen Home\n  text \"x\"\n",
        "U110",
    );
}

#[test]
fn controls_need_bind() {
    err("use ui\nscreen Home\n  toggle \"Dark\"\n", "U116");
}

#[test]
fn unknown_screen() {
    let text = err(
        "use ui\nscreen Home\n  button \"Go\" do go to Setings\nscreen Settings\n  text \"s\"\n",
        "U113",
    );
    assert!(text.contains("did you mean `Settings`"), "{text}");
}

#[test]
fn computed_is_read_only() {
    err(
        "use ui\nstate\n  a is 1\ncomputed\n  b is a * 2\nscreen Home\n  button \"x\" do b = 3\n",
        "U112",
    );
}

#[test]
fn no_braces_outside_text() {
    err("use ui\nscreen Home\n  text (1)\n", "P013");
}

#[test]
fn needs_a_screen() {
    err("use ui\napp A\n  state\n    n is 0\n", "U003");
}

#[test]
fn html_escapes_script_close() {
    let html = build_app_html("use ui\nscreen Home\n  text \"</script><b>\"\n", "t.rmt").unwrap();
    assert!(!html.contains("\"</script>"));
}

// ---------- app structure ----------

#[test]
fn split_view_toolbars_subtitle_settings() {
    let js = ok("use ui\nstate\n  folders is list \"A\"\nscreen Home \"Mail\" subtitle \"Hi\"\n  toolbar leading\n    button \"L\" do toast \"l\"\n  toolbar bottom\n    button \"B\" do toast \"b\"\n  toolbar title\n    text \"T\"\n  sidebar\n    for each f in folders\n      link f to Folder with f\n  text \"Pick one\"\nscreen Folder with f titled f inline\n  text f\nscreen Prefs \"Settings\" settings\n  text \"s\"\n");
    assert!(js.contains("{place: \"leading\""));
    assert!(js.contains("{place: \"bottom\""));
    assert!(js.contains("{place: \"title\""));
    assert!(js.contains("sidebar: () =>"));
    assert!(js.contains("settings: true"));
    assert!(js.contains("inline: true"));
    assert!(js.contains("subtitle: () => \"Hi\""));
}

#[test]
fn presentation_styles() {
    let js = ok("use ui\nstate\n  n is 1\nscreen Home\n  button \"a\" do show S as half\n  button \"b\" do show S as popover\n  button \"c\" do show S as inspector locked\n  button \"d\" do show S as full\n  button \"e\" do cover S\n  button \"f\" do show S\nscreen S\n  text \"s\"\n");
    for want in [
        "R.sheet(\"S\", [], \"half\", false)",
        "R.sheet(\"S\", [], \"popover\", false)",
        "R.sheet(\"S\", [], \"inspector\", true)",
        "R.sheet(\"S\", [], \"full\", false)",
        "R.sheet(\"S\", [], \"sheet\", false)",
    ] {
        assert!(js.contains(want), "missing {want}");
    }
    err(
        "use ui\nscreen Home\n  button \"a\" do show S as sideways\nscreen S\n  text \"s\"\n",
        "U109",
    );
}

#[test]
fn windows_files_documents() {
    let js = ok("use ui\nstate\n  notes is list\n  title is \"\"\ndocument notes, title named \"n.json\"\nscreen Home\n  button \"w\" do open window Home\n  button \"i\" do open file into notes as json\n  button \"s\" do save notes to file \"notes.json\"\n  button \"d\" do save document\n");
    assert!(js.contains("R.openWindow(\"Home\", [])"));
    assert!(js.contains("S.notes = await R.openFile(\"json\", S.notes);"));
    assert!(js.contains("R.saveFile(S.notes, \"notes.json\");"));
    assert!(js.contains("R.document = {names: [\"notes\", \"title\"]"));
    assert!(js.contains("await R.doc.save();"));
    err(
        "use ui\nscreen Home\n  button \"d\" do save document\n",
        "U118",
    );
    err(
        "use ui\nstate\n  a is 1\ndocument missing\nscreen Home\n  text \"x\"\n",
        "U118",
    );
}

#[test]
fn user_action_named_save_still_runs() {
    let js = ok(
        "use ui\nstate\n  n is 0\naction save\n  n = 1\nscreen Home\n  button \"Save\" do save\n",
    );
    assert!(js.contains("await A.save();"));
}

#[test]
fn scene_phase_and_deep_links() {
    let js = ok("use ui\nstate\n  last is \"\"\non background\n  last = \"bg\"\non foreground\n  toast \"back\"\non url\n  last = url\nscreen Home\n  text last\n");
    assert!(js.contains("R.phases.push({kind: \"background\""));
    assert!(js.contains("R.phases.push({kind: \"foreground\""));
    assert!(js.contains("R.onUrl = (L_url) =>"));
}

#[test]
fn search_suggestions_and_scopes() {
    let js = ok("use ui\nstate\n  q is \"\"\n  sc is \"All\"\nscreen Home\n  search \"Find\" bind q suggest \"a\", \"b\" scopes \"All\", \"Mine\" scope sc\n");
    assert!(js.contains("\"suggest\": R.opts([\"a\", \"b\"])"));
    assert!(js.contains("\"scopes\": R.opts([\"All\", \"Mine\"])"));
    assert!(js.contains("\"scope\": {get: () => S.sc"));
}

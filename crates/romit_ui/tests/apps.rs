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

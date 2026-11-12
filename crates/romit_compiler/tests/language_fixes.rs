//! Regression tests for core-language fixes: text joining, decimals, lists, comments, interpolation.

use romit_compiler::run_source;

fn run(src: &str) -> String {
    match run_source(src, "t.rmt", 100_000) {
        Ok(o) => o.stdout,
        Err(d) => panic!("{}", romit_compiler::format_diagnostics(&d)),
    }
}

fn fails(src: &str, code: &str) {
    let err = run_source(src, "t.rmt", 100_000).expect_err("expected an error");
    assert!(
        err.iter().any(|d| d.code == code),
        "expected {code}, got {:?}",
        err.iter().map(|d| &d.code).collect::<Vec<_>>()
    );
}

#[test]
fn text_joins_with_plus() {
    assert_eq!(run("say \"a\" plus \"b\"\nsay \"n=\" + 3\n"), "ab\nn=3\n");
}

#[test]
fn decimals_and_division() {
    assert_eq!(
        run("say 1.5\nsay 7 / 2\nsay 6 / 2\nsay 0.1 + 0.2\nsay 2.5 * 2\n"),
        "1.5\n3.5\n3\n0.3\n5\n"
    );
}

#[test]
fn interpolation() {
    assert_eq!(
        run("name = \"Ada\"\nsay \"Hi {name}, {2 + 3}!\"\n"),
        "Hi Ada, 5!\n"
    );
    assert_eq!(run("say \"{{literal}}\"\n"), "{literal}\n");
    fails("say \"{nope}\"\n", "E108");
}

#[test]
fn comments() {
    assert_eq!(
        run("# top comment\nnote also a comment\nsay 1 # trailing\nsay 2 // trailing\n"),
        "1\n2\n"
    );
}

#[test]
fn compound_assignment() {
    assert_eq!(
        run("x = 1\nx += 2\nx -= 1\nincrease x by 10\ndecrease x\nsay x\n"),
        "11\n"
    );
}

#[test]
fn lists() {
    let out = run(
        "fruits = list \"apple\", \"pear\"\nadd \"kiwi\" to fruits\nsay fruits\nsay count of fruits\nremove \"pear\" from fruits\nsay item 2 of fruits\nsay first of fruits\nif fruits contains \"kiwi\"\n  say \"yes\"\nsay join fruits, \" & \"\nfor each f in fruits\n  say upper f\n",
    );
    assert_eq!(
        out,
        "apple, pear, kiwi\n3\nkiwi\napple\nyes\napple & kiwi\nAPPLE\nKIWI\n"
    );
}

#[test]
fn list_math() {
    assert_eq!(run("n = list 3, 1, 2\nsay sum of n\nsay sort n\nsay max of n\nsay average of n\nsay round 3.14159, 2\n"), "6\n1, 2, 3\n3\n2\n3.14\n");
    assert_eq!(
        run("e = list\nsay count of e\nadd 1 to e\nsay e\nsay 2 in e\n"),
        "0\n1\nfalse\n"
    );
}

#[test]
fn stop_and_skip() {
    assert_eq!(
        run("for each i in 1 to 10\n  if i is 2\n    skip\n  if i is 4\n    stop\n  say i\n"),
        "1\n3\n"
    );
}

#[test]
fn text_comparison() {
    assert_eq!(run("say \"apple\" is below \"banana\"\n"), "true\n");
}

// ---------- Kotlin-inspired features ----------

#[test]
fn recursion_with_if_then_give() {
    // used to fail with E113 ("can finish without give")
    let src = "fib: n\n  if n is below 2\n    give n\n  a = fib n minus 1\n  b = fib n minus 2\n  give a plus b\nsay fib 15\n";
    assert_eq!(run(src), "610\n");
    fails("f: n\n  if n is 1\n    give 1\nsay f 2\n", "E113");
}

#[test]
fn records_and_maps() {
    let src = "ada = with name \"Ada\", age 36\nada.age += 1\nset ada.city to \"London\"\nsay \"{ada.name} {ada.age} {ada.city}\"\nprices = map \"tea\" to 3, \"green tea\" to 4\nset item \"cake\" of prices to 5\nsay item \"green tea\" of prices\nsay keys of prices\nsay count of prices\nsay prices contains \"cake\"\n";
    assert_eq!(
        run(src),
        "Ada 37 London\n4\ntea, green tea, cake\n3\ntrue\n"
    );
}

#[test]
fn nothing_and_or_else() {
    assert_eq!(run("x = nothing\nsay x\nsay \"Hi {x or else \"friend\"}\"\nif x is nothing\n  say \"empty\"\n"), "nothing\nHi friend\nempty\n");
}

#[test]
fn match_values_comparisons_and_no_subject() {
    let src = "for each t in list -5, 12, 30\n  match t\n    below 0\n      say \"cold\"\n    12, 13\n      say \"twelve-ish\"\n    otherwise\n      say \"hot\"\nmatch\n  1 is 2\n    say \"no\"\n  otherwise\n    say \"yes\"\n";
    assert_eq!(run(src), "cold\ntwelve-ish\nhot\nyes\n");
    fails("match 1\n  otherwise\n    say 1\n  2\n    say 2\n", "P024");
}

#[test]
fn try_fail_and_errors() {
    let src = "check: n\n  if n is below 0\n    fail \"negative: {n}\"\n  give n\ntry\n  say check 5\n  say check -2\nif it fails\n  say \"caught {error}\"\n";
    assert_eq!(run(src), "5\ncaught negative: -2\n");
    fails("fail \"boom\"\n", "E901");
}

#[test]
fn list_transforms() {
    let src = "people = list with name \"Ada\", age 36, with name \"Alan\", age 41, with name \"Grace\", age 30\nsay people where age is above 32 each name\nsay people sorted by age descending each name\nsay first 1 of people sorted by age descending each name\nteams = people grouped by age\nsay keys of teams\nnums = list 5, 2, 9\nsay nums each it times 10\nsay items 2 to 3 of nums\nsay last 2 of nums\nage = 100\nsay count of people where age is above 32\n";
    assert_eq!(
        run(src),
        "Ada, Alan\nAlan, Ada, Grace\nAlan\n36, 41, 30\n50, 20, 90\n2, 9\n2, 9\n2\n"
    );
}

#[test]
fn text_helpers_and_multiline() {
    let src = "say replace \"cat\" with \"dog\" in \"the cat\"\nsay \"Ada\" starts with \"A\"\nsay \"Ada\" ends with \"x\"\nsay letters 1 to 3 of \"Romit\"\nsay \"a\\tb \\\"q\\\"\"\npoem = \"\"\"\n  one\n    two\n  \"\"\"\nsay poem\n";
    assert_eq!(
        run(src),
        "the dog\ntrue\nfalse\nRom\na\tb \"q\"\none\n  two\n"
    );
}

#[test]
fn enums_dates_and_ranges() {
    let src = "enum Size small, medium, large\nsay Size\nsay medium is in Size\nsay weekday of \"2024-02-29\"\nsay days from \"2024-01-01\" to \"2024-12-25\"\nfor each i in 10 to 1 by 3\n  say i\n";
    assert_eq!(
        run(src),
        "small, medium, large\ntrue\nThursday\n359\n10\n7\n4\n1\n"
    );
}

#[test]
fn ask_reads_answers() {
    use romit_compiler::interp::set_ask_source;
    let mut answers = vec!["36".to_string(), "Ada".to_string(), "007".to_string()];
    set_ask_source(Some(Box::new(move |_q: &str| answers.pop())));
    let out = run("a = ask \"Zip?\"\nb = ask \"Name?\"\nc = ask \"Age?\"\nsay \"{a} {b} {c + 1}\"\nd = ask \"More?\"\nsay d or else \"none\"\n");
    set_ask_source(None);
    assert_eq!(out, "007 Ada 37\nnone\n");
}

#[test]
fn named_required_argument() {
    assert_eq!(
        run("greet: name\n  give \"Hi \" + name\nsay greet with name \"Bo\"\n"),
        "Hi Bo\n"
    );
}

#[test]
fn functions_read_top_level_values() {
    assert_eq!(
        run("rate = 2\nfn double n\n  give n times rate\nsay double 4\n"),
        "8\n"
    );
}

#[test]
fn use_other_files() {
    let dir = std::env::temp_dir().join(format!("romit_use_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("helpers.rmt"),
        "rate = 0.5\nfn half n\n  give n times rate\n",
    )
    .unwrap();
    let main = dir.join("main.rmt");
    let src = "use \"helpers.rmt\"\nsay half 10\n";
    let out = run_source(src, main.to_str().unwrap(), 100_000).expect("runs");
    assert_eq!(out.stdout, "5\n");
    let missing = run_source("use \"nope.rmt\"\nsay 1\n", main.to_str().unwrap(), 100_000)
        .expect_err("missing file");
    assert!(missing.iter().any(|d| d.code == "E120"));
    std::fs::remove_dir_all(&dir).ok();
}

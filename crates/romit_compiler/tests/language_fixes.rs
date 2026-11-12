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

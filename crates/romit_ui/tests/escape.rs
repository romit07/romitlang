use romit_ui::emit::escape_html;

#[test]
fn escapes_injection() {
    assert_eq!(
        escape_html("<script>alert('x')</script>"),
        "&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt;"
    );
    assert_eq!(escape_html("a & b"), "a &amp; b");
}

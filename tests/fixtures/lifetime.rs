fn foo<'a>(x: &'a str) -> &'a str {
    let c = 'a';
    let d = '\n';
    let s: &'static str = "hello";
    x
}

use wrenlet::Wren;

fn main() {
    let mut wren = Wren::new();

    let source = r#"
        var a = 1 + 2

        System.print(a)
    "#;

    wren.interpret("main", source).unwrap();

    dbg!(wren.has_variable("main", "ab"));
}

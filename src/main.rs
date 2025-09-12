use wrenlet::{Wren, value::Value};

fn main() {
    let mut wren = Wren::new();

    let source = r#"
        var a = 1 + 2

        System.print(a)
    "#;

    wren.interpret("main", source).unwrap();

    dbg!(wren.get_variable::<Value<'_>>("main", "a").unwrap());
}

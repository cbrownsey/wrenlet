use wrenlet::Wren;

fn main() {
    let mut wren = Wren::new();

    let source = r#"
        foreign class Test {
            construct new() {}
        }

        Test.new()
    "#;

    wren.interpret("main", source).unwrap();
}

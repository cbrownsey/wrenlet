use wrenlet::{Wren, value::Handle};

fn main() {
    let mut wren = Wren::new();

    let source = r#"
        class Wren {
            static flyTo(city) {
                System.print("Flying to %(city)")
            }
        }

        var a = 1 + 2

        var helloworld = "Hello, World!"

        System.printAll(["He", "ll", "o, Wo", "rld!"])
    "#;

    wren.interpret("main", source).unwrap();

    let a = wren.get_variable::<&str>("main", "helloworld");

    dbg!(a);

    let fly_to = wren.make_call_handle("static flyTo(_)");

    let class = wren.get_variable::<Handle>("main", "Wren").unwrap();

    wren.call::<()>(fly_to, (class,)).unwrap();
}

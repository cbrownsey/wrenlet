use unsynn::{Parse, ToTokenIter};

#[proc_macro_attribute]
pub fn foreign_method(
    arg: proc_macro::TokenStream,
    tokens: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    if !arg.is_empty() {
        return quote::quote! { compile_error!("`wrenlet::foreign_method` takes no arguments."); }
            .into();
    }

    foreign_method_impl(tokens.into()).into()
}

fn foreign_method_impl(tokens: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let x = Function::parse(&mut tokens.to_token_iter());

    todo!()
}

unsynn::unsynn! {
    keyword Const = "const";
    keyword Async = "async";
    keyword Safe = "safe";
    keyword Unsafe = "unsafe";
    keyword Extern = "extern";
    keyword FnKw = "fn";

    operator Arrow = "->";

    #[derive(Clone, Default)]
    struct FunctionQualifiers {
        constness: Option<Const>,
        asyncness: Option<Async>,
        safety: Option<unsynn::Either<Safe, Unsafe>>,
    }

    struct Function {
        qualifiers: FunctionQualifiers,
        fn_kw: FnKw,
        ident: unsynn::Ident,
        remainder: unsynn::TokenStream,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use unsynn::{Parse, ToTokenIter};

    #[test]
    fn function_parser() {
        let tokens = unsynn::quote! {
            fn testing() {}
        };

        let func = Function::parse(&mut tokens.to_token_iter()).unwrap();

        assert_eq!(func.ident.to_string(), "testing");
    }
}

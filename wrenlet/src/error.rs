error_set::error_set! {
    InterpretError := {
        Runtime,
        Compiletime,
    }
}

#[derive(Debug)]
pub struct IntoWrenError;

#[derive(Debug)]
pub struct FromWrenError;

pub struct Builder<U, M, W> {
    user_data: U,
    module_loader: M,
    writer: W,
}

impl Builder<U, M, W> {
    pub fn user_data<T>(self, user_data: T) -> Builder<T, M, W> {
        todo!()
    }

    pub fn module_loader<T>(self, module_loader: T) -> Builder<U, T, W> {
        todo!()
    }

    pub fn writer<T>(self, writer: T) -> Builder<U, M, T> {
        todo!()
    }
}

use aquila_core::prelude::*;

#[derive(Clone)]
pub struct AppState<R: AquilaServices> {
    pub services: R,
}

impl<S> AquilaServices for AppState<S>
where
    S: AquilaServices,
{
    type Storage = S::Storage;
    type Auth = S::Auth;
    type Compute = S::Compute;
    type Jwt = S::Jwt;

    fn storage(&self) -> &Self::Storage {
        &self.services.storage()
    }
    fn auth(&self) -> &Self::Auth {
        &self.services.auth()
    }
    fn compute(&self) -> &Self::Compute {
        &self.services.compute()
    }
    fn jwt(&self) -> &Self::Jwt {
        &self.services.jwt()
    }
}

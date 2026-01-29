use aquila_core::prelude::*;

#[derive(Clone)]
pub struct CoreServices<S, A, C, J> {
    pub storage: S,
    pub auth: A,
    pub compute: C,
    pub jwt: J,
}

impl<S, A, C, J> AquilaServices for CoreServices<S, A, C, J>
where
    S: StorageBackend,
    A: AuthProvider,
    C: ComputeBackend,
    J: JwtBackend,
{
    type Storage = S;
    type Auth = A;
    type Compute = C;
    type Jwt = J;

    fn storage(&self) -> &S {
        &self.storage
    }
    fn auth(&self) -> &A {
        &self.auth
    }
    fn compute(&self) -> &C {
        &self.compute
    }
    fn jwt(&self) -> &J {
        &self.jwt
    }
}

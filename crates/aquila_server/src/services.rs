use aquila_core::prelude::*;

#[derive(Clone)]
pub struct CoreServices<S, A, C, J, P> {
    pub storage: S,
    pub auth: A,
    pub compute: C,
    pub jwt: J,
    pub permissions: P,
}

impl<S, A, C, J, P> AquilaServices for CoreServices<S, A, C, J, P>
where
    S: StorageBackend,
    A: AuthProvider,
    C: ComputeBackend,
    J: JwtBackend,
    P: PermissionService,
{
    type Storage = S;
    type Auth = A;
    type Compute = C;
    type Jwt = J;

    type Permission = P;

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

    fn permissions(&self) -> &P {
        &self.permissions
    }
}

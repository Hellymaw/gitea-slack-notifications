use sqlx::PgPool;
use std::sync::Arc;

use crate::gitea::GiteaResourcePool;

#[derive(Debug)]
struct AppStateInner {
    gitea: GiteaResourcePool,
    database: PgPool,
}

#[derive(Debug)]
pub struct AppState(Arc<AppStateInner>);

impl AppState {
    pub fn new(gitea: GiteaResourcePool, database: PgPool) -> Self {
        Self(Arc::new(AppStateInner { gitea, database }))
    }

    pub fn gitea(&self) -> &GiteaResourcePool {
        &self.0.gitea
    }

    pub fn database(&self) -> &PgPool {
        &self.0.database
    }
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

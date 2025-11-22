use redis::aio::ConnectionManager;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::config::Config;
use crate::jobs::JobQueue;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub redis: ConnectionManager,
    pub config: Arc<Config>,
    pub job_queue: JobQueue,
}

impl AppState {
    pub fn new(
        db: DatabaseConnection,
        redis: ConnectionManager,
        config: Config,
        job_queue: JobQueue,
    ) -> Self {
        Self {
            db,
            redis,
            config: Arc::new(config),
            job_queue,
        }
    }
}

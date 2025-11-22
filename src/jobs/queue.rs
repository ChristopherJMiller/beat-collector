use anyhow::Result;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::db::entities::job::JobType;

/// Message sent to the job queue
#[derive(Debug, Clone)]
pub struct JobMessage {
    pub job_id: Uuid,
    pub job_type: JobType,
    pub entity_id: Option<Uuid>,
}

/// Job queue for async background task processing
#[derive(Clone)]
pub struct JobQueue {
    sender: mpsc::UnboundedSender<JobMessage>,
}

impl JobQueue {
    /// Create a new job queue and return (queue, receiver)
    pub fn new() -> (Self, mpsc::UnboundedReceiver<JobMessage>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (Self { sender }, receiver)
    }

    /// Submit a job to the queue
    pub fn submit(&self, message: JobMessage) -> Result<()> {
        self.sender
            .send(message)
            .map_err(|e| anyhow::anyhow!("Failed to submit job: {}", e))?;

        tracing::info!(
            "Job {} ({:?}) submitted to queue",
            message.job_id,
            message.job_type
        );

        Ok(())
    }
}

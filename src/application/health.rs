use std::sync::Arc;

use crate::infrastructure::ReadinessState;

#[derive(Clone)]
pub struct HealthService {
    readiness: Arc<ReadinessState>,
}

impl HealthService {
    pub fn new(readiness: Arc<ReadinessState>) -> Self {
        Self { readiness }
    }

    pub fn readiness(&self) -> &ReadinessState {
        self.readiness.as_ref()
    }
}

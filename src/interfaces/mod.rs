pub mod controller;
pub mod dto;
pub mod error;
pub mod middleware;

#[derive(Clone, Default)]
pub struct SharedState {}

impl SharedState {
    pub fn new() -> Self {
        Self {}
    }
}

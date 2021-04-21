use tokio::sync::Mutex;

use crate::{wl_split_timer::WlSplitTimer, TimerDisplay};
use async_trait::async_trait;

use std::{error::Error, sync::Arc};
pub struct App {
    timer: Arc<Mutex<WlSplitTimer>>,
}
impl App {
    pub fn new(timer: WlSplitTimer) -> Self {
        Self {
            timer: Arc::new(Mutex::new(timer)),
        }
    }
}

#[async_trait]
impl TimerDisplay for App {
    async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn timer(&self) -> &Arc<Mutex<WlSplitTimer>> {
        &self.timer
    }
}

use crate::{wl_split_timer::WlSplitTimer, TimerDisplay};

use std::{
    error::Error,
    sync::{Arc, Mutex},
};
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

impl TimerDisplay for App {
    fn run(&mut self) -> Result<bool, Box<dyn Error>> {
        let timer = self.timer.lock().unwrap();
        if timer.exit {
            return Ok(true);
        }
        Ok(false)
    }

    fn timer(&self) -> &Arc<Mutex<WlSplitTimer>> {
        &self.timer
    }
}

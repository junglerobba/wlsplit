use livesplit_core::{Run, Segment, Time, TimeSpan, Timer, TimerPhase};

const MSEC_HOUR: u128 = 3600000;
const MSEC_MINUTE: u128 = 60000;
const MSEC_SECOND: u128 = 1000;

pub struct WlSplitTimer {
    pub timer: Timer,
    file: String,
}

pub struct TimeFormat {
    pub hours: usize,
    pub minutes: usize,
    pub seconds: usize,
    pub msecs: usize,
}

impl Default for TimeFormat {
    fn default() -> Self {
        Self {
            hours: 2,
            minutes: 2,
            seconds: 2,
            msecs: 3,
        }
    }
}

impl WlSplitTimer {
    pub fn new(file: String) -> Self {
        let mut run = Run::new();
        let game_name = "test".to_string();
        let category_name = "any%".to_string();
        let mut segments: Vec<Segment> = Vec::new();
        segments.push(Segment::new("seg 01"));
        segments.push(Segment::new("seg 02"));
        run.set_game_name(&game_name);
        run.set_category_name(&category_name);

        for segment in segments {
            run.push_segment(segment);
        }

        let mut timer = Timer::new(run).expect("At least one segment expected");

        Self { timer, file }
    }

    pub fn run(&mut self) -> &Run {
        self.timer.run()
    }

    pub fn game_name(&mut self) -> &str {
        self.timer.run().game_name()
    }

    pub fn category_name(&mut self) -> &str {
        self.timer.run().category_name()
    }

    pub fn start(&mut self) {
        self.timer.start();
    }

    pub fn pause(&mut self) {
        self.timer.toggle_pause_or_start();
    }

    pub fn split(&mut self) {
        self.timer.split();
    }

    pub fn reset(&mut self) {
        let paused = self.timer.current_phase() == TimerPhase::Paused;
        self.timer.reset(true);
        self.timer.start();
        if paused {
            self.timer.pause();
        }
    }

    pub fn time(&mut self) -> Option<TimeSpan> {
        self.timer.current_time().real_time
    }

    pub fn segments(&mut self) -> &[Segment] {
        self.timer.run().segments()
    }

    pub fn current_segment(&mut self) -> Option<&Segment> {
        self.timer.current_split()
    }

    pub fn current_segment_index(&mut self) -> Option<usize> {
        self.timer.current_split_index()
    }

    pub fn segment_time(&mut self, index: usize) -> Time {
        self.timer.run().segment(index).split_time()
    }

    pub fn segment_best_time(&mut self, index: usize) -> Time {
        self.timer.run().segment(index).best_segment_time()
    }

    pub fn format_time(time: u128, format: TimeFormat, negative: bool) -> String {
        let prefix = if negative { "-" } else { "" };
        let mut time = time;
        let hours = time / MSEC_HOUR;
        time = time - (hours * MSEC_HOUR);
        let minutes = time / MSEC_MINUTE;
        time = time - (minutes * MSEC_MINUTE);
        let seconds = time / MSEC_SECOND;
        time = time - (seconds * MSEC_SECOND);

        format!(
            "{}{}:{}:{}.{}",
            prefix,
            pad_zeroes(hours, format.hours),
            pad_zeroes(minutes, format.minutes),
            pad_zeroes(seconds, format.seconds),
            pad_zeroes(time, format.msecs),
        )
    }
}

fn pad_zeroes(time: u128, length: usize) -> String {
    let str_length = time.to_string().chars().count();
    if str_length >= length {
        return format!("{}", time);
    }
    let count = length - str_length;
    let zeroes = "0".repeat(count);
    format!("{}{}", zeroes, time)
}

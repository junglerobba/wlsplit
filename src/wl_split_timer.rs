use std::error::Error;

use crate::file::{self, Run as RunFile};
use livesplit_core::{Attempt, Run, Segment, Time, TimeSpan, Timer, TimerPhase};

const MSEC_HOUR: u128 = 3600000;
const MSEC_MINUTE: u128 = 60000;
const MSEC_SECOND: u128 = 1000;

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

pub struct WlSplitTimer {
    pub timer: Timer,
    file: String,
}

impl WlSplitTimer {
    pub fn new(file: String, create_run: bool) -> Self {
        let mut run = Run::new();

        if create_run {
            let _run = RunFile::default();
            file_to_run(_run, &mut run)
        } else {
            read_file(&file, &mut run).expect("Unable to parse file");
        }

        let mut timer = Timer::new(run).expect("At least one segment expected");

        Self { timer, file }
    }

    pub fn run(&self) -> &Run {
        self.timer.run()
    }

    pub fn game_name(&self) -> &str {
        self.timer.run().game_name()
    }

    pub fn category_name(&self) -> &str {
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
        let end_of_run = self.timer.current_phase() == TimerPhase::Ended;

        if end_of_run {
            self.reset(true);
            self.write_file();
        }
    }

    pub fn reset(&mut self, update_splits: bool) {
        self.timer.reset(update_splits);
        self.timer.start();
        self.timer.pause();
    }

    pub fn write_file(&self) -> Result<(), Box<dyn Error>> {
        write_file(&self.file, &self.timer.run())
    }

    pub fn time(&self) -> Option<TimeSpan> {
        self.timer.current_time().real_time
    }

    pub fn segments(&self) -> &[Segment] {
        self.timer.run().segments()
    }

    pub fn current_segment(&self) -> Option<&Segment> {
        self.timer.current_split()
    }

    pub fn current_segment_index(&self) -> Option<usize> {
        self.timer.current_split_index()
    }

    pub fn segment_time(&self, index: usize) -> Time {
        self.timer.run().segment(index).split_time()
    }

    pub fn segment_best_time(&self, index: usize) -> Time {
        self.timer.run().segment(index).best_segment_time()
    }

    pub fn sum_of_best_segments(&self) -> usize {
        let mut sum: usize = 0;
        for segment in self.timer.run().segments() {
            if let Some(time) = segment.best_segment_time().real_time {
                sum += time.total_milliseconds() as usize;
            }
        }
        sum
    }

    pub fn best_possible_time(&self) -> usize {
        let index = self.current_segment_index().unwrap_or(0);

        if index == 0 {
            return self.sum_of_best_segments();
        }

        let mut time: usize = self
            .run()
            .segment(index - 1)
            .split_time()
            .real_time
            .unwrap_or_default()
            .total_milliseconds() as usize;

        for segment in self.run().segments().into_iter().skip(index) {
            let segment = segment
                .best_segment_time()
                .real_time
                .unwrap_or_default()
                .total_milliseconds() as usize;
            time += segment;
        }

        time
    }

    pub fn format_time(time: u128, format: TimeFormat, negative: bool) -> String {
        let prefix = if negative { "-" } else { "" };
        let mut time = time;
        let hours = time / MSEC_HOUR;
        time -= hours * MSEC_HOUR;
        let minutes = time / MSEC_MINUTE;
        time -= minutes * MSEC_MINUTE;
        let seconds = time / MSEC_SECOND;
        time -= seconds * MSEC_SECOND;

        format!(
            "{}{}:{}:{}.{}",
            prefix,
            pad_zeroes(hours, format.hours),
            pad_zeroes(minutes, format.minutes),
            pad_zeroes(seconds, format.seconds),
            pad_zeroes(time, format.msecs),
        )
    }

    pub fn parse_time_string(time: String) -> u128 {
        let split = time.split(":");
        let mut time: u128 = 0;
        let vec: Vec<&str> = split.collect();

        time += MSEC_HOUR * vec[0].parse::<u128>().unwrap_or(0);
        time += MSEC_MINUTE * vec[1].parse::<u128>().unwrap_or(0);

        let split = vec[2].split(".");
        let vec: Vec<&str> = split.collect();

        time += MSEC_SECOND * vec[0].parse::<u128>().unwrap_or(0);
        let msecs: String = vec[1].chars().take(3).collect();
        time += msecs.parse::<u128>().unwrap_or(0);

        time
    }

    pub fn string_to_time(string: String) -> Time {
        let time = WlSplitTimer::parse_time_string(string) as f64;
        let time_span = TimeSpan::from_milliseconds(time);

        let mut time: Time = Time::new();
        time.with_real_time(Some(time_span))
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

fn read_file(file: &String, run: &mut Run) -> Result<(), ()> {
    match file::read_json::<RunFile>(file) {
        Ok(_run) => {
            file_to_run(_run, run);
        }
        Err(_) => {
            return Err(());
        }
    }

    Ok(())
}

fn file_to_run(_run: RunFile, run: &mut Run) {
    run.set_game_name(_run.game_name);
    run.set_category_name(_run.category_name);
    run.set_attempt_count(_run.attempt_count as u32);

    for attempt in _run.attempt_history {
        let time = match attempt.time {
            Some(t) => t,
            _ => continue,
        };
        let time = WlSplitTimer::string_to_time(time);
        run.add_attempt_with_index(time, attempt.id, None, None, None);
    }

    for segment in _run.segments {
        let mut _segment = Segment::new(segment.name);
        if let Some(split_time) = segment.best_segment_time {
            if let Some(time) = split_time.time {
                _segment.set_best_segment_time(WlSplitTimer::string_to_time(time));
            }
        }

        for split in segment.split_times {
            if let (Some(time), Some(name)) = (split.time, split.name) {
                if name == "Personal Best" {
                    _segment.set_personal_best_split_time(WlSplitTimer::string_to_time(time));
                }
            }
        }

        for split in segment.segment_history {
            if let (Some(time), Some(id)) = (split.time, split.id) {
                _segment
                    .segment_history_mut()
                    .insert(id, WlSplitTimer::string_to_time(time.to_string()));
            }
        }

        run.push_segment(_segment);
    }
}

fn write_file(file: &String, run: &Run) -> Result<(), Box<dyn Error>> {
    let run = RunFile::new(&run);
    file::write_json(file, run)
}

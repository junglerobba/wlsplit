use std::error::Error;

use crate::file::{self, Run as RunFile};
use chrono::{DateTime, Utc};
use livesplit_core::{AtomicDateTime, Run, Segment, Time, TimeSpan, Timer, TimerPhase};

const MSEC_HOUR: u128 = 3600000;
const MSEC_MINUTE: u128 = 60000;
const MSEC_SECOND: u128 = 1000;

pub struct RunMetadata<'a> {
    pub game_name: Option<&'a str>,
    pub category_name: Option<&'a str>,
    pub splits: Option<Vec<&'a str>>,
}
pub struct WlSplitTimer {
    timer: Timer,
    file: String,
    pub exit: bool,
}

impl WlSplitTimer {
    pub fn new(file: String, metadata: RunMetadata) -> Self {
        let mut run = Run::new();

        let mut generated = RunFile::default();
        if let Some(game_name) = metadata.game_name {
            generated = generated.with_game_name(game_name);
        }
        if let Some(category_name) = metadata.category_name {
            generated = generated.with_category_name(category_name);
        }
        if let Some(splits) = metadata.splits {
            generated = generated.with_splits(splits);
        }
        file_to_run(generated, &mut run);
        write_file(&file, &run).expect("Could not write file");
        let timer = Timer::new(run).unwrap();

        Self {
            timer,
            file,
            exit: false,
        }
    }

    pub fn from_file(file: String) -> Self {
        let mut run = Run::new();
        read_file(&file, &mut run).expect("Unable to parse file");
        let timer = Timer::new(run).expect("At least one segment expected");

        Self {
            timer,
            file,
            exit: false,
        }
    }

    pub fn timer(&self) -> &Timer {
        &self.timer
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
            self.write_file().ok();
        }
    }

    pub fn skip(&mut self) {
        self.timer.skip_split();
    }

    pub fn reset(&mut self, update_splits: bool) {
        self.timer.reset(update_splits);
        if update_splits {
            self.write_file().ok();
        }
    }

    pub fn quit(&mut self) {
        self.exit = true;
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

        for segment in self.run().segments().iter().skip(index) {
            let segment = segment
                .best_segment_time()
                .real_time
                .unwrap_or_default()
                .total_milliseconds() as usize;
            time += segment;
        }

        time
    }

    pub fn parse_time_string(time: String) -> Result<u128, Box<dyn Error>> {
        let split: Vec<&str> = time.split(':').collect();
        let mut time: u128 = 0;
        time += MSEC_HOUR * split.get(0).ok_or("")?.parse::<u128>()?;
        time += MSEC_MINUTE * split.get(1).ok_or("")?.parse::<u128>()?;

        let split: Vec<&str> = split.get(2).ok_or("")?.split('.').collect();

        time += MSEC_SECOND * split.get(0).ok_or("")?.parse::<u128>()?;
        time += split
            .get(1)
            .ok_or("")?
            .chars()
            .take(3)
            .collect::<String>()
            .parse::<u128>()?;

        Ok(time)
    }

    pub fn string_to_time(string: String) -> Time {
        let time = WlSplitTimer::parse_time_string(string)
            .map(|time| TimeSpan::from_milliseconds(time as f64))
            .expect("Unable to parse time");

        Time::new().with_real_time(Some(time))
    }

    pub fn get_segment_time(&self, index: usize) -> Option<usize> {
        let current_time = self
            .segments()
            .get(index)
            .and_then(|segment| segment.split_time().real_time);
        if index == 0 {
            return current_time.map(|time| time.to_duration().num_milliseconds() as usize);
        }
        let time = self
            .segments()
            .get(index - 1)
            .and_then(|segment| segment.split_time().real_time);
        if let (Some(current_time), Some(time)) = (current_time, time) {
            Some(
                (current_time.to_duration().num_milliseconds()
                    - time.to_duration().num_milliseconds()) as usize,
            )
        } else {
            None
        }
    }
}

fn read_file(file: &str, run: &mut Run) -> Result<(), Box<dyn Error>> {
    file::read_json::<RunFile>(file).map(|json| file_to_run(json, run))
}

fn file_to_run(file: RunFile, run: &mut Run) {
    run.set_game_name(file.game_name);
    run.set_category_name(file.category_name);
    run.set_attempt_count(file.attempt_count as u32);

    for attempt in file.attempt_history {
        let time = match attempt.time {
            Some(t) => t,
            _ => continue,
        };
        let time = WlSplitTimer::string_to_time(time);
        let started = attempt.started.and_then(|t| {
            DateTime::parse_from_rfc3339(&t)
                .map(|t| AtomicDateTime::new(t.with_timezone(&Utc), false))
                .ok()
        });
        let ended = attempt.ended.and_then(|t| {
            DateTime::parse_from_rfc3339(&t)
                .map(|t| AtomicDateTime::new(t.with_timezone(&Utc), false))
                .ok()
        });
        let pause_time = attempt.pause_time.and_then(|time| {
            WlSplitTimer::parse_time_string(time)
                .map(|ms| TimeSpan::from_milliseconds(ms as f64))
                .ok()
        });
        run.add_attempt_with_index(time, attempt.id, started, ended, pause_time);
    }

    for segment in file.segments {
        let best_segment_time = segment
            .best_segment_time
            .map_or(Time::new(), WlSplitTimer::string_to_time);
        let personal_best_split_time = segment
            .personal_best_split_time
            .map_or(Time::new(), WlSplitTimer::string_to_time);

        let mut segment_new = Segment::new(segment.name);
        segment_new.set_best_segment_time(best_segment_time);
        segment_new.set_personal_best_split_time(personal_best_split_time);

        for split in segment.segment_history {
            if let Some(id) = split.id {
                segment_new.segment_history_mut().insert(
                    id,
                    split.time.map_or(Time::new(), WlSplitTimer::string_to_time),
                )
            }
        }

        run.push_segment(segment_new);
    }
}

fn write_file(file: &str, run: &Run) -> Result<(), Box<dyn Error>> {
    let run = RunFile::new(&run);
    file::write_json(file, run)
}

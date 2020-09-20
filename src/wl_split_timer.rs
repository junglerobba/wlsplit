use std::convert::TryInto;

use crate::file::{self, read, Run as RunXML};
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

        read_file(&file, &mut run);

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

    pub fn reset(&mut self, update_splits: bool) {
        let paused = self.timer.current_phase() == TimerPhase::Paused;
        self.timer.reset(update_splits);
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

    pub fn parse_time_string(time: String) -> u128 {
        let split = time.split(":");
        let mut time: u128 = 0;
        let vec: Vec<&str> = split.collect();

        time = time + (MSEC_HOUR * vec[0].parse::<u128>().unwrap());
        time = time + (MSEC_MINUTE * vec[1].parse::<u128>().unwrap());

        let split = vec[2].split(".");
        let vec: Vec<&str> = split.collect();

        time = time + (MSEC_SECOND * vec[0].parse::<u128>().unwrap());
        let msecs: String = vec[1].chars().take(3).collect();
        time = time + msecs.parse::<u128>().unwrap();

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

fn read_file(file: &String, run: &mut Run) {
    if let Ok(_run) = file::read::<RunXML>(file) {
        run.set_game_name(_run.GameName);
        run.set_category_name(_run.CategoryName);
        run.set_attempt_count(_run.AttemptCount.try_into().unwrap());

        for segment in _run.Segments.Segment {
            let mut _segment = Segment::new(segment.Name);
            if let Some(real_time) = segment.BestSegmentTime.RealTime {
                _segment.set_best_segment_time(WlSplitTimer::string_to_time(real_time));
            }

            for split in segment.SplitTimes.SplitTime {
                if let (Some(time), Some(name)) = (split.RealTime, split.name) {
                    if name == "Personal Best" {
                        _segment.set_personal_best_split_time(WlSplitTimer::string_to_time(time));
                    }
                }
            }

            for i in 0..segment.SegmentHistory.len() {
                if let Some(time) = &segment.SegmentHistory[i].RealTime {
                    _segment
                        .segment_history_mut()
                        .insert(i as i32, WlSplitTimer::string_to_time(time.to_string()));
                }
            }

            run.push_segment(_segment);
        }
    }
}

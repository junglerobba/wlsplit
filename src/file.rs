use std::{error::Error, fs::File, io::Read, io::Write};

use livesplit_core::Run as LivesplitRun;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::wl_split_timer::{TimeFormat, WlSplitTimer};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Run {
    pub game_name: String,
    pub category_name: String,
    pub attempt_count: usize,
    pub attempt_history: Vec<Attempt>,
    pub segments: Vec<Segment>,
}

impl Default for Run {
    fn default() -> Self {
        let segments = vec![Segment {
            name: "Example Segment".to_string(),
            icon: None,
            segment_history: Vec::new(),
            split_times: Vec::new(),
            best_segment_time: None,
        }];

        Self {
            game_name: "Example Splits".to_string(),
            category_name: "Any%".to_string(),
            attempt_count: 0,
            attempt_history: Vec::new(),
            segments: segments,
        }
    }
}

impl Run {
    pub fn new(run: &LivesplitRun) -> Self {
        let mut attempt_history: Vec<Attempt> = Vec::new();

        for attempt in run.attempt_history() {
            let msecs = match attempt.time().real_time {
                Some(time) => time.total_milliseconds() as u128,
                None => 0,
            };
            let _attempt = Attempt {
                time: Some(WlSplitTimer::format_time(
                    msecs,
                    TimeFormat::default(),
                    false,
                )),
                id: attempt.index() as usize,
                started: None,
                ended: None,
            };
            attempt_history.push(_attempt);
        }

        let mut segments: Vec<Segment> = Vec::new();
        for segment in run.segments() {
            let msecs = match segment.best_segment_time().real_time {
                Some(time) => time.total_milliseconds() as u128,
                None => 0,
            };
            let best_time = SplitTime {
                id: None,
                name: None,
                time: Some(WlSplitTimer::format_time(
                    msecs,
                    TimeFormat::default(),
                    false,
                )),
            };
            let mut _segment = Segment {
                name: segment.name().to_string(),
                icon: None,
                segment_history: Vec::new(),
                split_times: vec![SplitTime {
                    id: None,
                    name: Some("Personal Best".to_string()),
                    time: Some(WlSplitTimer::format_time(
                        msecs,
                        TimeFormat::default(),
                        false,
                    )),
                }],
                best_segment_time: Some(best_time),
            };

            for history in segment.segment_history() {
                let msecs = match history.1.real_time {
                    Some(time) => time.total_milliseconds() as u128,
                    None => 0,
                };
                _segment.segment_history.push(SplitTime {
                    id: Some(history.0 as usize),
                    name: None,
                    time: Some(WlSplitTimer::format_time(
                        msecs,
                        TimeFormat::default(),
                        false,
                    )),
                });
            }

            segments.push(_segment);
        }

        Self {
            game_name: run.game_name().to_string(),
            category_name: run.category_name().to_string(),
            attempt_count: run.attempt_count() as usize,
            attempt_history: attempt_history,
            segments: segments,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Attempt {
    pub id: usize,
    pub started: Option<String>,
    pub ended: Option<String>,
    pub time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SplitTime {
    pub name: Option<String>,
    pub time: Option<String>,
    pub id: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Segment {
    pub name: String,
    pub icon: Option<String>,
    pub split_times: Vec<SplitTime>,
    pub best_segment_time: Option<SplitTime>,
    pub segment_history: Vec<SplitTime>,
}

pub fn read<T: DeserializeOwned>(path: &String) -> Result<T, ()> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(_) => {
            return Err(());
        }
    };
    let mut content = String::new();
    if let Err(_) = file.read_to_string(&mut content) {
        return Err(());
    }

    let result: T = match serde_json::from_str(&content) {
        Ok(content) => content,
        Err(err) => {
            println!("{}", err);
            return Err(());
        }
    };

    Ok(result)
}

pub fn write(path: &String, run: Run) -> Result<(), Box<dyn Error>> {
    let serialized = serde_json::to_string_pretty(&run)?;
    let path = format!("{}", path);
    let mut file = File::create(path)?;
    file.write_all(serialized.as_bytes())?;

    Ok(())
}

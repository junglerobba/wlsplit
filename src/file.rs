use std::{error::Error, fs::File, io::Read};

use quick_xml::de::{from_str, DeError};
use serde::{Deserialize, de::DeserializeOwned};

#[derive(Debug, Deserialize, PartialEq)]
pub struct Run {
    pub GameName: String,
    pub CategoryName: String,
    pub AttemptCount: usize,
    pub AttemptHistory: AttemptHistory,
    pub Segments: Segments,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct AttemptHistory {
    pub Attempt: Vec<Attempt>,
}
#[derive(Debug, Deserialize, PartialEq)]
pub struct Segments {
    pub Segment: Vec<Segment>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Attempt {
    pub id: usize,
    pub started: String,
    pub ended: String,
    pub RealTime: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct SplitTime {
    pub name: Option<String>,
    pub RealTime: Option<String>,
    pub id: Option<usize>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct SplitTimes {
    pub SplitTime: Vec<SplitTime>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Segment {
    pub Name: String,
    pub Icon: String,
    pub SplitTimes: SplitTimes,
    pub BestSegmentTime: SplitTime,
    pub SegmentHistory: Vec<SplitTime>,
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

    let result: T = match from_str(&content) {
        Ok(content) => content,
        Err(err) => {
            println!("{}", err);
            return Err(());
        }
    };

    Ok(result)
}

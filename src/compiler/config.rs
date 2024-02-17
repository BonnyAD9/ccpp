use std::{ops::RangeBounds, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum Optimization {
    None,
    All,
    Level(i32),
}

impl Optimization {
    pub fn in_range<R>(&self, range: R) -> bool
    where
        R: RangeBounds<i32>,
    {
        matches!(self, Self::Level(l) if range.contains(l))
            || matches!(self, Self::All | Self::None)
    }
}

impl ToString for Optimization {
    fn to_string(&self) -> String {
        match self {
            Self::None => "None".to_owned(),
            Self::All => "All".to_owned(),
            Self::Level(n) => n.to_string(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Std {
    Number(i32),
    Name(String),
}

impl Std {
    pub fn is_c_num(&self) -> bool {
        matches!(self, Self::Number(99 | 11 | 17))
    }

    pub fn is_cpp_num(&self) -> bool {
        matches!(self, Self::Number(98 | 3 | 11 | 14 | 17 | 20))
    }
}

impl From<String> for Std {
    fn from(value: String) -> Self {
        Self::Name(value)
    }
}

impl From<i32> for Std {
    fn from(value: i32) -> Self {
        Self::Number(value)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub bin_root: PathBuf,
    pub src_root: PathBuf,
    pub optimization: Optimization,
    pub asan: bool,
    pub dbg_symbols: bool,
    pub c_std: Std,
    pub cpp_std: Std,
    pub defines: Vec<(String, Option<String>)>,
    pub warn: Vec<String>,
    pub no_warn: Vec<String>,
    pub args: Vec<String>,
}

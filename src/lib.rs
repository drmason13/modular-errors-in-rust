//! This crate provides types for UCD’s `Blocks.txt`.
//!
//! It was written by Sabrina Jewson in her article [Modular Errors in Rust](https://sabrinajewson.org/blog/errors)
//!
//! I have simply iterated on the usage of the existing error types
//! by adding convenience methods that return closures that can be passed into `Result::map_err`.
//!
//! This hopes to alleviate the "unavoidable cost" she mentions in [Constructing the error types](https://sabrinajewson.org/blog/errors#constructing-the-error-types)

pub struct Blocks {
    ranges: Vec<(RangeInclusive<u32>, String)>,
}

impl Blocks {
    pub fn block_of(&self, c: char) -> &str {
        self.ranges
            .binary_search_by(|(range, _)| {
                if *range.end() < u32::from(c) {
                    cmp::Ordering::Less
                } else if u32::from(c) < *range.start() {
                    cmp::Ordering::Greater
                } else {
                    cmp::Ordering::Equal
                }
            })
            .map(|i| &*self.ranges[i].1)
            .unwrap_or("No_Block")
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, FromFileError> {
        let path = path.as_ref();
        let data = fs::read_to_string(path).map_err(FromFileError::read_file(path))?;

        Self::from_str(&data).map_err(FromFileError::parse(path))
    }

    pub fn download(agent: &ureq::Agent) -> Result<Self, DownloadError> {
        let response = agent
            .get(LATEST_URL)
            .call()
            .map_err(DownloadError::request())?;

        Self::from_str(&response.into_string().map_err(DownloadError::read_body())?)
            .map_err(DownloadError::parse())
    }
}

impl FromStr for Blocks {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ranges = s
            .lines()
            .enumerate()
            .map(|(i, line)| {
                (
                    i,
                    line.split_once('#').map(|(line, _)| line).unwrap_or(line),
                )
            })
            .filter(|(_, line)| !line.is_empty())
            .map(|(i, line)| {
                let (range, name) = line
                    .split_once(';')
                    .ok_or(ParseError::new(i, ParseErrorKind::NoSemicolon))?;
                let (range, name) = (range.trim(), name.trim());

                let (start, end) = range
                    .split_once("..")
                    .ok_or(ParseError::new(i, ParseErrorKind::NoDotDot))?;

                let start = u32::from_str_radix(start, 16).map_err(ParseError::parse_int(i))?;
                let end = u32::from_str_radix(end, 16).map_err(ParseError::parse_int(i))?;

                Ok((start..=end, name.to_owned()))
            })
            .collect::<Result<Vec<_>, ParseError>>()?;
        Ok(Self { ranges })
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub struct DownloadError {
    pub kind: DownloadErrorKind,
}

#[derive(Debug)]
pub enum DownloadErrorKind {
    Request(Box<ureq::Error>),
    ReadBody(io::Error),
    Parse(ParseError),
}

impl DownloadError {
    pub fn request() -> impl FnOnce(ureq::Error) -> Self {
        move |error: ureq::Error| DownloadError {
            kind: DownloadErrorKind::Request(Box::new(error)),
        }
    }
    pub fn read_body() -> impl FnOnce(io::Error) -> Self {
        move |error: io::Error| DownloadError {
            kind: DownloadErrorKind::ReadBody(error),
        }
    }
    pub fn parse() -> impl FnOnce(ParseError) -> Self {
        move |error: ParseError| DownloadError {
            kind: DownloadErrorKind::Parse(error),
        }
    }
}

impl Display for DownloadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "failed to download Blocks.txt from the Unicode website")
    }
}

impl Error for DownloadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            DownloadErrorKind::Request(e) => Some(e),
            DownloadErrorKind::ReadBody(e) => Some(e),
            DownloadErrorKind::Parse(e) => Some(e),
        }
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub struct FromFileError {
    pub path: Box<Path>,
    pub kind: FromFileErrorKind,
}

#[derive(Debug)]
pub enum FromFileErrorKind {
    ReadFile(io::Error),
    Parse(ParseError),
}

impl FromFileError {
    pub fn read_file<P>(path: P) -> impl FnOnce(io::Error) -> FromFileError
    where
        P: Into<Box<Path>>,
    {
        move |error: io::Error| FromFileError {
            path: path.into(),
            kind: FromFileErrorKind::ReadFile(error),
        }
    }

    pub fn parse<P>(path: P) -> impl FnOnce(ParseError) -> FromFileError
    where
        P: Into<Box<Path>>,
    {
        move |error: ParseError| FromFileError {
            path: path.into(),
            kind: FromFileErrorKind::Parse(error),
        }
    }
}

impl Display for FromFileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "error reading `{}`", self.path.display())
    }
}

impl Error for FromFileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            FromFileErrorKind::ReadFile(e) => Some(e),
            FromFileErrorKind::Parse(e) => Some(e),
        }
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub struct ParseError {
    pub line: usize,
    pub kind: ParseErrorKind,
}

#[derive(Debug)]
pub enum ParseErrorKind {
    #[non_exhaustive]
    NoSemicolon,
    #[non_exhaustive]
    NoDotDot,
    #[non_exhaustive]
    ParseInt(ParseIntError),
}

impl ParseError {
    pub fn new(line: usize, kind: ParseErrorKind) -> Self {
        ParseError { line, kind }
    }

    pub fn parse_int(line: usize) -> impl FnOnce(ParseIntError) -> Self {
        move |error: ParseIntError| ParseError {
            line,
            kind: ParseErrorKind::ParseInt(error),
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "invalid Blocks.txt data on line {}", self.line + 1)
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.kind)
    }
}

impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NoSemicolon => f.write_str("no semicolon"),
            Self::NoDotDot => f.write_str("no `..` in range"),
            Self::ParseInt { .. } => {
                write!(f, "one end of range is not a valid hexadecimal integer")
            }
        }
    }
}

impl Error for ParseErrorKind {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ParseInt(source) => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn real_unicode() {
        let data = include_str!("../Blocks.txt").parse::<Blocks>().unwrap();
        assert_eq!(data.block_of('\u{0080}'), "Latin-1 Supplement");
        assert_eq!(data.block_of('½'), "Latin-1 Supplement");
        assert_eq!(data.block_of('\u{00FF}'), "Latin-1 Supplement");
        assert_eq!(data.block_of('\u{EFFFF}'), "No_Block");
    }

    use crate::Blocks;
}

pub const LATEST_URL: &str = "https://www.unicode.org/Public/UCD/latest/ucd/Blocks.txt";

use std::cmp;
use std::error::Error;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fs;
use std::io;
use std::num::ParseIntError;
use std::ops::RangeInclusive;
use std::path::Path;
use std::str::FromStr;

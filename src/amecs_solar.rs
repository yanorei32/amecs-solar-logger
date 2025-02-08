use std::io::{BufRead, BufReader, Read};
use std::time::Duration;

use chrono::{DateTime, NaiveDateTime, Utc};
use clap::Parser;
use itertools::Itertools;
use ordered_float::NotNan;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Parser)]
pub struct Coord {
    #[clap(long, env)]
    lat: f32,

    #[clap(long, env)]
    lon: f32,
}

impl std::fmt::Display for Coord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{},{}", self.lat, self.lon)
    }

}

impl Coord {
    fn distance(&self, otherhand: Self) -> f32 {
        // Approximating Earth as a plane.
        // No sign reversal cases in Japan.
        // Sufficient accuracy for data resolution.
        let lat_diff = self.lat - otherhand.lat;
        let lon_diff = self.lon - otherhand.lon;

        (lat_diff * lat_diff + lon_diff * lon_diff).sqrt()
    }
}

#[derive(Debug, Clone)]
pub struct DataPoint {
    pub power: f32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SolarData {
    data: Vec<(Coord, Vec<DataPoint>)>,
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("HeaderLine is not found")]
    HeaderLineIsNotFound,

    #[error("Invalid HeaderLine was supplied")]
    HeaderLineIsNotValid,

    #[error("Non UTC time was supplied, Currently, it's not supported")]
    NonUtcZoneSupplied,

    #[error("Invalid Base Time")]
    InvalidBaseTime(chrono::format::ParseError),

    #[error("Invalid value in data")]
    InvalidValueInData(std::num::ParseFloatError),

    #[error("Invalid line was supplied")]
    InvalidLineWasSupplied,

    #[error("I/O Error")]
    IO(#[from] std::io::Error),
}

impl SolarData {
    pub fn nearest_series_data(&self, coord: Coord) -> (Coord, &[DataPoint]) {
        let (c, v) = self
            .data
            .iter()
            .min_by_key(|(c, _)| NotNan::new(c.distance(coord)).unwrap())
            .unwrap();

        (c.to_owned(), v.as_ref())
    }

    pub fn try_new<R: Read>(r: R) -> Result<Self, ParseError> {
        let r = BufReader::new(r);

        let mut lines = r.lines();

        let first_line = lines
            .next()
            .ok_or(ParseError::HeaderLineIsNotFound)?
            .map_err(ParseError::IO)?;

        let first_line = first_line.trim_end();

        let (naive_time, timezone) = first_line
            .split_once(',')
            .ok_or(ParseError::HeaderLineIsNotValid)?;

        if timezone != "UTC" {
            return Err(ParseError::NonUtcZoneSupplied);
        }

        let base_time = NaiveDateTime::parse_from_str(naive_time, "%Y/%m/%d %H:%M:%S")
            .map_err(ParseError::InvalidBaseTime)?
            .and_utc();

        let mut s = SolarData { data: vec![] };

        for line in lines {
            let values: Result<Vec<f32>, ParseError> = line
                .map_err(ParseError::IO)?
                .split(',')
                .map(|s| s.parse::<f32>().map_err(ParseError::InvalidValueInData))
                .collect();

            let mut values = values?.into_iter();

            let (lat, lon) = values
                .next_tuple()
                .ok_or(ParseError::InvalidLineWasSupplied)?;

            let timeseries: Vec<_> = values
                .enumerate()
                .map(|(offset, power)| {
                    let timestamp = base_time + Duration::from_secs(60 * 60 * offset as u64);

                    DataPoint { power, timestamp }
                })
                .collect();

            s.data.push((Coord { lat, lon }, timeseries));
        }

        Ok(s)
    }
}

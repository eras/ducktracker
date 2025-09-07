use rand::{Rng, distributions::Alphanumeric};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

#[derive(Debug)]
pub struct CommaSeparatedVec<T>(pub Vec<T>);

impl<T> CommaSeparatedVec<T> {
    pub fn new() -> Self {
        CommaSeparatedVec(Vec::new())
    }
}

impl<'de, T> Deserialize<'de> for CommaSeparatedVec<T>
where
    T: Clone + std::str::FromStr + std::fmt::Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s.is_empty() {
            return Ok(CommaSeparatedVec(Vec::new()));
        }
        let vec = s
            .split(',')
            .map(|s| s.trim().parse::<T>())
            .collect::<Result<Vec<T>, _>>()
            .map_err(|_err| serde::de::Error::custom("Failed to parse comma separated list"))?;
        Ok(CommaSeparatedVec(vec))
    }
}

impl<T> std::fmt::Display for CommaSeparatedVec<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for field in self.0.iter() {
            if !first {
                write!(f, ",")?;
            }
            first = false;
            write!(f, "{}", field)?;
        }
        Ok(())
    }
}

/// Reads a file where each line contains a key-value pair separated by a colon, e.g., "key:value".
///
/// Returns a `HashMap<String, String>` where keys and values are trimmed.
/// Lines without a colon or empty lines are skipped.
///
/// # Arguments
/// * `path` - The path to the file to read.
///
/// # Errors
/// Returns an `io::Error` if the file cannot be opened or read.
pub fn read_colon_separated_file<P: AsRef<Path>>(path: P) -> io::Result<HashMap<String, String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut map = HashMap::new();

    for line_result in reader.lines() {
        let line = line_result?; // Propagate file read errors

        let trimmed_line = line.trim();

        // Skip empty lines
        if trimmed_line.is_empty() {
            continue;
        }

        // Split the line at the first colon
        if let Some((key, value)) = trimmed_line.split_once(':') {
            let trimmed_key = key.trim().to_string();
            let trimmed_value = value.trim().to_string();
            map.insert(trimmed_key, trimmed_value);
        } else {
            // Optionally, handle malformed lines (e.g., log a warning, skip, or return an error).
            // For this implementation, we skip them silently to be permissive.
            eprintln!(
                "Warning: Skipping malformed line (no colon found): '{}'",
                line
            );
        }
    }

    Ok(map)
}

pub fn generate_id() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}

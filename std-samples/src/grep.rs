use std::{env, fmt, fs};

#[derive(Debug)]
pub enum GrepError {
    Usage,
    Io(std::io::Error),
}

impl fmt::Display for GrepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GrepError::Usage => write!(f, "usage: minigrep <query> <path>"),
            GrepError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}
impl std::error::Error for GrepError {}
impl From<std::io::Error> for GrepError {
    fn from(e: std::io::Error) -> Self { GrepError::Io(e) }
}

pub struct Config {
    pub query: String,
    pub path: String,
    pub ignore_case: bool,
}

impl Config {
    pub fn parse(mut args: impl Iterator<Item = String>) -> Result<Config, GrepError> {
        args.next(); // program name
        let query = args.next().ok_or(GrepError::Usage)?;
        let path = args.next().ok_or(GrepError::Usage)?;
        Ok(Config { query, path, ignore_case: env::var("IGNORE_CASE").is_ok() })
    }
}

/// The returned lines borrow from `contents`, which is what the lifetime says.
pub fn search<'a>(query: &str, contents: &'a str) -> Vec<&'a str> {
    contents.lines().filter(|l| l.contains(query)).collect()
}

pub fn search_insensitive<'a>(query: &str, contents: &'a str) -> Vec<&'a str> {
    let q = query.to_lowercase();
    contents.lines().filter(|l| l.to_lowercase().contains(&q)).collect()
}

pub fn run(cfg: &Config) -> Result<(), GrepError> {
    let contents = fs::read_to_string(&cfg.path)?;
    let hits = if cfg.ignore_case {
        search_insensitive(&cfg.query, &contents)
    } else {
        search(&cfg.query, &contents)
    };
    for line in hits { println!("{line}"); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finds_matching_lines() {
        let text = "Rust:\nsafe, fast.\nPick three.";
        assert_eq!(search("fast", text), vec!["safe, fast."]);
    }
    #[test]
    fn case_insensitive() {
        assert_eq!(search_insensitive("RUST", "Rust:\ntrust me"), vec!["Rust:", "trust me"]);
    }
}

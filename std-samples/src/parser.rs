/// A record that borrows from the input buffer: no copies, no allocation.
#[derive(Debug, PartialEq)]
pub struct Record<'a> {
    pub level: &'a str,
    pub ts: u64,
    pub msg: &'a str,
}

pub fn parse_line(line: &str) -> Option<Record<'_>> {
    let mut it = line.splitn(3, ' ');
    let level = it.next()?;
    let ts = it.next()?.parse().ok()?;
    let msg = it.next()?;
    Some(Record { level, ts, msg })
}

pub fn parse_all(input: &str) -> impl Iterator<Item = Record<'_>> {
    input.lines().filter_map(parse_line)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses() {
        let r = parse_line("WARN 1700000000 disk almost full").unwrap();
        assert_eq!(r, Record { level: "WARN", ts: 1700000000, msg: "disk almost full" });
        assert!(parse_line("garbage").is_none());
    }
}

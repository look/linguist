/// Custom wrapper around a RegexSet that defaults to using multi-line mode and
/// allows capturing the fact that some linguist regexes are not supported by
/// the Rust regex library because they use Ruby specific character classes,
/// different escaping rules, look-around, etc.
#[derive(Debug)]
pub enum Regex {
    Native(regex::RegexSet),
    Ruby,
}

impl Regex {
    pub fn new<I, S>(patterns: I) -> Self
    where
        S: AsRef<str>,
        I: IntoIterator<Item = S>,
    {
        match regex::RegexSetBuilder::new(patterns)
            .multi_line(true)
            .build()
        {
            Ok(re) => Self::Native(re),
            Err(_) => {
                // NB: There are a number of ruby regexes that don't compile,
                // here's how to see exactly which ones aren't compatible:
                // eprintln!("failed to compile regex: {e:?}");
                Self::Ruby
            }
        }
    }

    pub fn is_match(&self, text: &str) -> bool {
        match &self {
            Self::Native(re) => re.is_match(text),
            // NOTE: We experimented with using the Ruby regex engine
            // (Oniguruma), but have decided that we don't want to take on that
            // c library as a dependency due to the associated safety, security,
            // and performance concerns.
            //
            // Instead, we just always return false for regexes that aren't
            // supported by the Rust regex library.
            Self::Ruby => false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn regex() {
        let re = Regex::new([r"^\s*;"]);
        assert!(re.is_match(r" ;"));
    }
}

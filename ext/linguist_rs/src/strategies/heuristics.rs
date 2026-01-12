use std::collections::HashMap;
use std::sync::LazyLock;

use serde_yaml::Value;

use super::lowercase_ext;
use crate::regex::Regex;
use crate::{Answer, Language, get_language_by_name};

/// Detects the language of a file using content heuristics. Returns the first
/// matching language or Answer::None.
pub(crate) fn by_content_heuristics(path: &str, content: &str) -> Answer {
    if let Some(ext) = lowercase_ext(path) {
        // Limit the amount of content we process to ~50KB.
        const MAX_BYTES_TO_SCAN: usize = 50 * 1024;
        let content = if content.len() > MAX_BYTES_TO_SCAN {
            let idx = ceil_char_boundary(content, MAX_BYTES_TO_SCAN);
            &content[..idx]
        } else {
            content
        };

        for rule in HEURISTICS.rules(&ext) {
            if rule.pattern.is_match(&HEURISTICS.named_patterns, content) {
                // TODO: This is how linguist is written (return first result)
                // but we could return multiple and intersect them with the
                // prior steps...
                return Answer::Only(rule.language);
            }
        }
    }
    Answer::Unknown
}

pub(crate) fn init() {
    let _ = LazyLock::force(&HEURISTICS);
}

/// Finds the closest `x` not below `index` where `is_char_boundary(x)` is `true`.
///
/// Borrowed from implementation of unstable ceil_char_boundary API:
/// https://github.com/rust-lang/rust/pull/86497.
fn ceil_char_boundary(content: &str, index: usize) -> usize {
    let upper_bound = Ord::min(index + 4, content.len());
    content.as_bytes()[index..upper_bound]
        .iter()
        .position(|b| (*b as i8) >= -0x40) // NB: b < 128 || b >= 192
        .map_or(upper_bound, |pos| index + pos)
}

#[derive(Debug)]
struct ContentHeuristics {
    named_patterns: HashMap<String, Regex>,
    rules: Vec<Rule>,
    rules_by_extension: HashMap<String, Vec<usize>>,
}

impl ContentHeuristics {
    fn rules(&self, ext: &str) -> Vec<&Rule> {
        self.rules_by_extension
            .get(ext)
            .map(|xs| xs.iter().map(|x| &self.rules[*x]).collect())
            .unwrap_or_default()
    }
}

#[derive(Debug)]
struct Rule {
    language: &'static Language,
    pattern: RulePattern,
}

#[derive(Debug)]
enum RulePattern {
    And(Vec<RulePattern>),
    Pattern(Regex),
    NegativePattern(Regex),
    NamedPattern(String),
    AlwaysMatch,
}

impl RulePattern {
    fn parse_regex(rule: &Value, key: &str) -> Option<Regex> {
        rule.get(key).map(|pattern| {
            let patterns = pattern
                .as_sequence()
                .map(|xs| {
                    xs.iter()
                        .map(|x| x.as_str().unwrap_or_else(|| panic!("invalid {key}")))
                        .collect()
                })
                .unwrap_or_else(|| {
                    vec![pattern.as_str().unwrap_or_else(|| panic!("invalid {key}"))]
                });
            Regex::new(patterns)
        })
    }

    fn from_value(rule: &Value) -> Self {
        if let Some(rules) = rule.get("and").and_then(|x| x.as_sequence()) {
            Self::And(rules.iter().map(Self::from_value).collect())
        } else if let Some(pattern) = Self::parse_regex(rule, "pattern") {
            Self::Pattern(pattern)
        } else if let Some(pattern) = Self::parse_regex(rule, "negative_pattern") {
            Self::NegativePattern(pattern)
        } else if let Some(pattern) = rule
            .get("named_pattern")
            .map(|x| x.as_str().expect("invalid named_pattern").to_owned())
        {
            Self::NamedPattern(pattern)
        } else {
            Self::AlwaysMatch
        }
    }

    fn is_match(&self, named_patterns: &HashMap<String, Regex>, content: &str) -> bool {
        match &self {
            RulePattern::And(patterns) => {
                patterns.iter().all(|p| p.is_match(named_patterns, content))
            }
            RulePattern::Pattern(regex) => regex.is_match(content),
            RulePattern::NegativePattern(regex) => !regex.is_match(content),
            RulePattern::NamedPattern(name) => {
                if let Some(re) = named_patterns.get(name) {
                    re.is_match(content)
                } else {
                    panic!("named pattern {name} not found, invalid heuristics.yml");
                }
            }
            RulePattern::AlwaysMatch => true,
        }
    }
}

static HEURISTICS: LazyLock<ContentHeuristics> = LazyLock::new(|| {
    let mut named_patterns = HashMap::new();
    let mut rules = Vec::new();
    let mut rules_by_extension = HashMap::new();

    let yml = include_bytes!("../linguist/heuristics.yml");
    let value: serde_yaml::Value =
        serde_yaml::from_slice(&yml[..]).expect("unable to parse heuristics.yml!");
    let data = value
        .as_mapping()
        .expect("invalid mapping in heuristics.yml!")
        .to_owned();

    for (k, v) in data
        .get("named_patterns")
        .expect("named_patterns not found")
        .as_mapping()
        .expect("invalid mapping in heuristics.yml!")
    {
        let name = k.as_str().expect("invalid named_pattern key").to_owned();
        let patterns = v
            .as_sequence()
            .map(|xs| {
                xs.iter()
                    .map(|x| x.as_str().expect("invalid pattern in named_pattern"))
                    .collect()
            })
            .unwrap_or_else(|| vec![v.as_str().expect("invalid pattern in named_pattern")]);
        if named_patterns.contains_key(&name) {
            panic!("duplicate named_pattern {name} found");
        }
        named_patterns.insert(name, Regex::new(&patterns));
    }

    for disambiguation in data
        .get("disambiguations")
        .expect("disambiguations not found")
        .as_sequence()
        .expect("invalid mapping in heuristics.yml!")
    {
        let exts = disambiguation
            .get("extensions")
            .expect("extensions not found")
            .as_sequence()
            .expect("invalid sequence");
        for rule in disambiguation
            .get("rules")
            .expect("rules not found")
            .as_sequence()
            .expect("invalid sequence")
        {
            if let Some(language) = rule
                .get("language")
                .expect("language not found")
                .as_str()
                .map(|x| x.to_owned())
            {
                let language = get_language_by_name(&language).expect("language not found");
                let pattern = RulePattern::from_value(rule);
                let idx = rules.len();
                rules.push(Rule { language, pattern });
                for ext in exts {
                    let ext = ext
                        .as_str()
                        .expect("invalid ext")
                        .strip_prefix('.')
                        .expect("extensions to have dot prefix")
                        .to_owned();
                    rules_by_extension.entry(ext).or_insert(vec![]).push(idx);
                }
            }
        }
    }

    ContentHeuristics {
        named_patterns,
        rules,
        rules_by_extension,
    }
});

#[cfg(test)]
mod tests {
    use std::env::current_dir;
    use std::fs::{read_dir, read_to_string};

    use super::by_content_heuristics;
    use crate::fixtures::SKIPS_WRONG_LANGUAGE;
    use crate::get_language_by_name;
    use crate::strategies::Answer;

    #[test]
    fn test_heuristics_generic() {
        let pwd = current_dir().unwrap();
        let generic_fixtures = pwd.join("test/fixtures/Generic");
        for entry in read_dir(generic_fixtures.clone()).unwrap() {
            let entry = entry.unwrap();
            let extension_name = entry.path();

            if !extension_name.is_dir() {
                continue;
            }

            for entry in read_dir(extension_name).unwrap() {
                let entry = entry.unwrap();
                let language_name = entry.path();

                if !language_name.is_dir() {
                    continue;
                }

                let language =
                    get_language_by_name(language_name.file_name().unwrap().to_str().unwrap());

                for entry in read_dir(language_name).unwrap() {
                    let entry = entry.unwrap();
                    let example = entry.path();

                    if !example.is_file() {
                        continue;
                    }

                    let content = read_to_string(example.clone()).unwrap();
                    let answer = by_content_heuristics(example.to_str().unwrap(), &content);

                    if SKIPS_WRONG_LANGUAGE.contains(
                        example
                            .strip_prefix(&generic_fixtures)
                            .unwrap()
                            .to_str()
                            .unwrap(),
                    ) {
                        assert_eq!(
                            answer,
                            Answer::Unknown,
                            "for {example:?}, expected unknown language due to incompatible regex"
                        );
                        continue;
                    }

                    if let Some(lang) = language {
                        assert_eq!(
                            answer,
                            Answer::Only(lang),
                            "for {example:?}, expected {lang:?}, got {answer:?}",
                        );
                    } else {
                        // examples in the `nil` directories can't be detected by heuristics
                        assert_eq!(
                            answer,
                            Answer::Unknown,
                            "for {example:?}, expected an unknown language",
                        );
                    }
                }
            }
        }
    }
}

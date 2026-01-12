use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Instant;

use serde_yaml::Value;
use strategies::ShouldRun;

pub use crate::generated::is_generated;
pub use crate::indexable::IngestFilter;
use crate::strategies::Answer;
pub use crate::vendored::is_vendored;

pub mod fixtures;
pub mod generated;
pub mod indexable;
pub mod popular;
pub mod regex;
mod strategies;
pub mod vendored;

#[cfg(feature = "c-bindings")]
pub mod linguist_c;

pub mod languages {
    include!(concat!(env!("OUT_DIR"), "/languages.rs"));
}

pub use languages::LanguageId;

/// Detect the programming language of the given path and content. Uses the
/// following strategies (in this order) as ported from linguist:
///   - Modeline,
///   - Filename,
///   - Shebang,
///   - Extension,
///   - XML,
///   - Manpage,
///   - ContentHeuristics,
///
/// NOTE: This version of detect does not use a bayesian classifier. The
/// classifier is slow and not overly correct, but this does mean that there are
/// some files that Ruby linguist will properly detect that cannot be detected
/// by this Rust port.
///
/// The detection strategy is a bit nuanced in the ordering of the strategies
/// and how potential results are passed between each step. See
/// strategies::default() for details.
///
/// Be aware that linguist's approach here is not sound. It is possible for a
/// strategy to return candidate languages and then have the next strategy
/// return a different set which would cause all candidates to cancel out due to
/// the intersection of results. Not running the second strategy at all many
/// have returning a list of potential results: instead you'd get nothing.
///
/// For now, we opt to keep this behavior, but there is the potential to provide
/// superior results in the future.
pub fn detect(path: &str, content: &str) -> Option<&'static Language> {
    if content.is_empty() {
        return None;
    }

    let start = Instant::now();
    let (answer, strategy) =
        strategies::default()
            .iter()
            .fold((Answer::Unknown, ""), |(answer, name), strategy| {
                if matches!(answer, Answer::Only(_)) {
                    // we've already got a single answer: pass it along and skip
                    // evaluating the rest of the strategies.
                    return (answer, name);
                }

                let run = match strategy.should_run {
                    ShouldRun::Always => true,
                    ShouldRun::OnlyIfNoAnswer => answer == Answer::Unknown,
                };

                if run {
                    let start_step = Instant::now();
                    let res = (strategy.f)(path, content);
                    metrics::histogram!("linguist.detect.strategy_duration", "language" => res.label(), "strategy" => strategy.name)
                        .record(start_step.elapsed());
                    match (res, answer) {
                        (Answer::Unknown, a) => (a, name), // keep current answer instead
                        (a @ Answer::Many(_), b @ Answer::Many(_)) => {
                            (a.intersect(b), strategy.name) // use the intersection as our answer
                        }
                        (a, _) => (a, strategy.name), // keep this as our new answer
                    }
                } else {
                    (answer, name)
                }
            });
    metrics::histogram!("linguist.detect_duration", "language" => answer.label(), "strategy" => strategy)
        .record(start.elapsed());
    answer.into()
}

/// Eagerly initialize all lazy statics, compiling all dynamic regexes in the
/// linguist crate.
///
/// TODO: Maybe eagerly load test, doc, generated, and indexable rules too?
pub fn init() {
    let _ = LazyLock::force(&LINGUIST_DATA);
    popular::init();
    strategies::extension::init();
    strategies::heuristics::init();
    strategies::manpage::init();
    strategies::modeline::init();
    strategies::shebang::init();
    strategies::xml::init();
}

regex_set_from_path!(TEST_PATH_RULES, "tests.yml");
regex_set_from_path!(DOCUMENTATION_PATH_RULES, "documentation.yml");
regex_set_from_path!(DEPENDENCY_MANAGEMENT_RULES, "dependency_management.yml");

/// Detects if the path is a test file.
pub fn is_test(path: &str) -> bool {
    let start = Instant::now();
    let is_test = TEST_PATH_RULES.is_match(path);
    metrics::histogram!("linguist.check.is_test_duration", "is_test" => is_test.to_string())
        .record(start.elapsed());
    is_test
}

/// Detects if the path is a documentation file.
pub fn is_documentation(path: &str) -> bool {
    let start = Instant::now();
    let is_documentation = DOCUMENTATION_PATH_RULES.is_match(path);
    metrics::histogram!("linguist.check.is_documentation_duration", "is_documentation" => is_documentation.to_string())
        .record(start.elapsed());
    is_documentation
}

pub fn is_dependency_management(path: &str) -> bool {
    let start = Instant::now();
    let is_dependency_management = DEPENDENCY_MANAGEMENT_RULES.is_match(path);
    metrics::histogram!("linguist.check.is_dependency_management", "is_dependency_management" => is_dependency_management.to_string())
        .record(start.elapsed());
    is_dependency_management
}

/// Get a language by its canonical Linguist id.
pub fn get_language_by_id(id: LanguageId) -> Option<&'static Language> {
    LINGUIST_DATA
        .by_id
        .get(&id)
        .map(|x| &LINGUIST_DATA.languages[*x])
}

/// Get a language by its canonical Linguist name.
pub fn get_language_by_name(name: &str) -> Option<&'static Language> {
    LINGUIST_DATA
        .by_name
        .get(name)
        .map(|x| &LINGUIST_DATA.languages[*x])
}

/// Get a language by its alias (see languages.yml).
pub fn get_language_by_alias(alias: &str) -> Option<&'static Language> {
    LINGUIST_DATA
        .by_alias
        .get(&alias.to_lowercase())
        .map(|x| &LINGUIST_DATA.languages[*x])
}

/// Get all languages for the given filename (see languages.yml).
pub(crate) fn languages_by_filename(filename: &str) -> Vec<&'static Language> {
    LINGUIST_DATA
        .by_filename
        .get(filename)
        .map(|xs| xs.iter().map(|x| &LINGUIST_DATA.languages[*x]).collect())
        .unwrap_or_default()
}

/// Get all languages for the given interpreter (see languages.yml).
pub(crate) fn languages_by_interpreter(interpreter: &str) -> Vec<&'static Language> {
    LINGUIST_DATA
        .by_interpreter
        .get(interpreter)
        .map(|xs| xs.iter().map(|x| &LINGUIST_DATA.languages[*x]).collect())
        .unwrap_or_default()
}

/// Get all languages for the given extension (see languages.yml).
pub fn languages_by_extension(ext: &str) -> Vec<&'static Language> {
    LINGUIST_DATA
        .by_extension
        .get(ext)
        .map(|xs| xs.iter().map(|x| &LINGUIST_DATA.languages[*x]).collect())
        .unwrap_or_default()
}

struct LinguistData {
    languages: Vec<Language>,
    by_id: HashMap<LanguageId, usize>,
    by_name: HashMap<String, usize>,
    by_filename: HashMap<String, Vec<usize>>,
    by_alias: HashMap<String, usize>,
    by_interpreter: HashMap<String, Vec<usize>>,
    by_extension: HashMap<String, Vec<usize>>,
}

static LINGUIST_DATA: LazyLock<LinguistData> = LazyLock::new(|| {
    let mut languages = Vec::new();
    let mut by_id = HashMap::new();
    let mut by_name = HashMap::new();
    let mut by_filename = HashMap::new();
    let mut by_alias = HashMap::new();
    let mut by_interpreter = HashMap::new();
    let mut by_extension = HashMap::new();

    let yml = include_bytes!("linguist/languages.yml");
    let value: serde_yaml::Value =
        serde_yaml::from_slice(&yml[..]).expect("unable to parse languages.yml!");
    let data = value
        .as_mapping()
        .expect("invalid mapping in languages.yml!")
        .to_owned();

    for (key, language) in data.into_iter() {
        let language_id: u32 = language
            .get("language_id")
            .expect("languages.yml: each language must have an id")
            .as_u64()
            .expect("language_id must be a u64")
            .try_into()
            .expect("language_id must fit in a u32");
        let name = key
            .as_str()
            .expect("languages.yml: each language must have a name")
            .to_owned();
        let color = language
            .get("color")
            .map(|x| x.as_str().expect("color must be a string").to_owned());
        let language_type = match language
            .get("type")
            .map(|x| x.as_str().expect("type must be a string"))
        {
            Some("programming") => LanguageType::Programming,
            Some("data") => LanguageType::Data,
            Some("markup") => LanguageType::Markup,
            Some("prose") => LanguageType::Prose,
            _ => LanguageType::Unknown,
        };
        let tm_scope = language
            .get("tm_scope")
            .map(|x| x.as_str().expect("tm_scope must be a string").to_owned())
            .expect("languages must have tm_scope");

        let idx = languages.len();
        languages.push(Language {
            language_id,
            name: name.clone(),
            color,
            language_type,
            tm_scope,
        });
        by_id.insert(language_id, idx);
        by_name.insert(name.clone(), idx);
        for filename in from_str_array(&language, "filenames") {
            by_filename.entry(filename).or_insert(vec![]).push(idx);
        }
        by_alias.insert(name.to_lowercase(), idx);
        for alias in from_str_array(&language, "aliases") {
            by_alias.insert(alias.to_lowercase(), idx);
        }
        for interpreter in from_str_array(&language, "interpreters") {
            by_interpreter
                .entry(interpreter)
                .or_insert(vec![])
                .push(idx);
        }
        for ext in from_str_array(&language, "extensions") {
            by_extension
                .entry(
                    ext.strip_prefix('.')
                        .expect("invalid extension")
                        .to_lowercase()
                        .to_string(),
                )
                .or_insert(vec![])
                .push(idx);
        }
    }
    LinguistData {
        languages,
        by_id,
        by_name,
        by_filename,
        by_alias,
        by_interpreter,
        by_extension,
    }
});

fn from_str_array(data: &Value, key: &str) -> Vec<String> {
    data.get(key)
        .and_then(|x| x.as_sequence())
        .map(|x| {
            x.iter()
                .map(|f| f.as_str().expect("must be a string").to_owned())
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Language {
    pub language_id: LanguageId,
    pub name: String,
    pub color: Option<String>,
    pub language_type: LanguageType,
    pub tm_scope: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum LanguageType {
    Programming,
    Data,
    Markup,
    Prose,
    Unknown,
}

macro_rules! regex_set_from_path {
    ($x:ident, $path:expr) => {
        static $x: ::std::sync::LazyLock<::regex::RegexSet> = ::std::sync::LazyLock::new(|| {
            let file = include_bytes!(concat!("linguist/", $path));
            let value: serde_yaml::Value =
                serde_yaml::from_slice(&file[..]).expect("unabled to parse yml file");
            ::regex::RegexSet::new(
                value
                    .as_sequence()
                    .expect("invalid sequence in yml!")
                    .to_owned()
                    .iter()
                    .map(|r| {
                        r.as_str()
                            .expect("invalid entry in yml")
                            .replace(r"\/", "/")
                    }),
            )
            .expect("invalid regex in yml!")
        });
    };
}
pub(crate) use regex_set_from_path;

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::fs::read;
    use std::path::PathBuf;
    use std::str::from_utf8;
    use std::vec;

    use super::*;
    use crate::fixtures::{SKIPS_WRONG_LANGUAGE, Sample, samples, samples_dir};

    #[test]
    fn language_by_name() {
        let rust = get_language_by_name("Rust").unwrap();
        assert_eq!(rust.name, "Rust");
        assert_eq!(rust.language_id, 327);
    }

    #[test]
    fn language_by_id() {
        let rust = get_language_by_id(327).unwrap();
        assert_eq!(rust.name, "Rust");
        assert_eq!(rust.language_id, 327);
    }

    #[test]
    fn language_by_alias() {
        ["csharp", "c#", "C#"].iter().for_each(|n| {
            let l = get_language_by_alias(n).unwrap();
            assert_eq!(l.name, "C#");
            assert_eq!(l.language_id, 42);
        });

        assert_eq!(
            get_language_by_alias("protocol buffer")
                .unwrap()
                .language_id,
            297
        );
        assert_eq!(
            get_language_by_alias("Protocol Buffers")
                .unwrap()
                .language_id,
            297
        );
    }

    #[test]
    fn language_by_filename() {
        assert_eq!(
            vec!["Browserslist"],
            languages_by_filename("browserslist")
                .iter()
                .map(|x| &x.name)
                .collect::<Vec<_>>()
        )
    }

    #[test]
    fn is_test_file() {
        crate::fixtures::run("a test", include_str!("../test/tests.txt"), |path, _| {
            is_test(path)
        });
    }

    #[test]
    fn is_documentation_file() {
        crate::fixtures::run(
            "documentation",
            include_str!("../test/documentation.txt"),
            |path, _| is_documentation(path),
        );
    }

    #[test]
    fn test_dependency_management() {
        crate::fixtures::run(
            "dependency management",
            include_str!("../test/dependency_management.txt"),
            |path, _| is_dependency_management(path),
        )
    }

    #[test]
    fn constants() {
        assert_eq!(crate::languages::PYTHON, 303);
        assert_eq!(crate::languages::RUBY, 326);
    }

    #[test]
    fn detect_all_samples() {
        let mut res = CheckResult::default();
        for sample in samples() {
            res.add(check(sample));
        }

        assert_eq!(res.error, 0);
        assert_eq!(
            res.not_found.len(),
            0,
            "unable to detect language: {:?}",
            res.not_found
        );

        let dir = samples_dir();
        let wrong_language = res
            .wrong_language
            .iter()
            .map(|x| x.strip_prefix(&dir).unwrap().to_str().unwrap())
            .filter(|x| !SKIPS_WRONG_LANGUAGE.contains(x))
            .collect::<HashSet<_>>();
        assert_eq!(
            wrong_language.len(),
            0,
            "language detection wrong: {wrong_language:?}"
        );
        assert_eq!(res.ok, 2714);
    }

    #[test]
    fn detect_one_sample() {
        // NB: This test is useful for debugging single examples.
        let samples_dir = samples_dir();
        let res = check(Sample {
            language: "Rust".to_string(),
            path: samples_dir.join("Rust/base64url"),
        });
        assert_eq!(res.ok, 1);
    }

    fn check(sample: Sample) -> CheckResult {
        let dir = samples_dir();
        let path = &sample.path;
        let content = &read(path).unwrap();
        if let Ok(content) = from_utf8(content) {
            let languages = detect(path.as_path().to_str().unwrap(), content);
            if let Some(l) = languages {
                let name = if l.name == "F*" {
                    "Fstar"
                } else {
                    l.name.as_str()
                };
                if sample.language == name {
                    return CheckResult::ok();
                } else {
                    let on_skip_list = SKIPS_WRONG_LANGUAGE
                        .contains(path.strip_prefix(&dir).unwrap().to_str().unwrap());
                    if !on_skip_list {
                        eprintln!(
                            "𝙭 {}\texpected {}, got {}. known_skip?={}",
                            path.display(),
                            sample.language,
                            l.name,
                            on_skip_list
                        );
                    }
                    return CheckResult::wrong_language(path.to_path_buf());
                }
            } else {
                eprintln!("𝙭 {}\texpected {}", path.display(), sample.language);
                return CheckResult::not_found(path.to_path_buf());
            }
        } else {
            eprintln!("{}: invalid utf8", path.display())
        }
        CheckResult::error()
    }

    #[derive(Default, Debug)]
    struct CheckResult {
        ok: usize,
        not_found: Vec<PathBuf>,
        wrong_language: Vec<PathBuf>,
        error: usize,
    }

    impl CheckResult {
        fn ok() -> Self {
            Self {
                ok: 1,
                ..Default::default()
            }
        }

        fn not_found(path: PathBuf) -> Self {
            Self {
                not_found: vec![path],
                ..Default::default()
            }
        }

        fn wrong_language(path: PathBuf) -> Self {
            Self {
                wrong_language: vec![path],
                ..Default::default()
            }
        }

        fn error() -> Self {
            Self {
                error: 1,
                ..Default::default()
            }
        }

        fn add(&mut self, other: Self) {
            self.ok += other.ok;
            self.not_found.extend(other.not_found);
            self.wrong_language.extend(other.wrong_language);
            self.error += other.error;
        }
    }
}

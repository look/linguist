pub(crate) mod extension;
pub(crate) mod filename;
pub(crate) mod heuristics;
pub(crate) mod manpage;
pub(crate) mod modeline;
pub(crate) mod shebang;
pub(crate) mod xml;

use std::path::Path;

use crate::Language;
use crate::popular::sort_by_popularity;
use crate::strategies::extension::by_extension;
use crate::strategies::filename::by_filename;
use crate::strategies::heuristics::by_content_heuristics;
use crate::strategies::manpage::by_manpage;
use crate::strategies::modeline::{by_emacs_modeline, by_vim_modeline};
use crate::strategies::shebang::by_shebang;
use crate::strategies::xml::by_xml;

/// A strategy for determining the language of a file.
pub struct Strategy {
    pub name: &'static str,
    pub f: DetectFn,
    pub should_run: ShouldRun,
}

/// A detection fn takes a path and contents and returns an `Answer`.
pub(crate) type DetectFn = fn(&str, &str) -> Answer;

/// A predicate to determine if a strategy should be run or not.
pub enum ShouldRun {
    Always,
    OnlyIfNoAnswer,
}

/// The result of running a strategy which can be zero or more detected
/// languages.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Answer {
    Unknown,
    Only(&'static Language),
    Many(Vec<&'static Language>),
}

// The default linguist strategies to run in order to detect the language for a
// particular file.
pub(crate) fn default() -> Vec<Strategy> {
    vec![
        Strategy {
            name: "emacs_modeline", // returns zero or one languages
            f: by_emacs_modeline,
            should_run: ShouldRun::Always,
        },
        Strategy {
            name: "vim_modeline", // returns zero or one languages
            f: by_vim_modeline,
            should_run: ShouldRun::Always,
        },
        Strategy {
            name: "filename", // -> returns zero or *more* languages
            f: by_filename,
            should_run: ShouldRun::Always,
        },
        Strategy {
            name: "shebang", // -> returns zero or *more* languages
            f: by_shebang,
            should_run: ShouldRun::Always,
        },
        Strategy {
            name: "extension", // -> early returns zero results if blob.generic?, returns zero or *more* languages
            f: by_extension,
            should_run: ShouldRun::Always,
        },
        Strategy {
            name: "xml", // early return candidates if any, otherwise returns *just* languages detected by this strategy
            f: by_xml,
            should_run: ShouldRun::OnlyIfNoAnswer,
        },
        Strategy {
            // TODO: Consider running this one before xml b/c it only needs the filename.
            // TODO: This could be refactored in concert with by_extension
            // .1, etc are too common as extensions, so we make sure not to use naive by_extension strategy and instead force content heuristics.
            name: "manpage", // early return candidates if any, otherwise returns *just* languages detected by this strategy
            f: by_manpage,
            should_run: ShouldRun::OnlyIfNoAnswer,
        },
        Strategy {
            // TODO: There might be some benefit to returning multiple results
            // here and intersecting them with the candidates from prior steps.
            name: "content_heuristics", // returns zero or one languages
            f: by_content_heuristics,
            should_run: ShouldRun::Always,
        },
    ]
}

/// Returns the lowercase extension of a path if it has one.
pub(crate) fn lowercase_ext(path: &str) -> Option<String> {
    Path::new(path)
        .extension()?
        .to_str()
        .map(|x| x.to_lowercase())
}

// Slice out up to n lines from the beginning of `content`.
pub(crate) fn take_n_lines(content: &str, n: usize) -> &str {
    let len = content.len();
    let header_end = content
        .match_indices('\n')
        .nth(n)
        .map(|(i, _)| i)
        .unwrap_or(len);
    content.get(0..header_end).unwrap_or("")
}

impl Answer {
    pub fn intersect(self, other: Self) -> Self {
        let mut a: Vec<&'static Language> = self.into();
        let b: Vec<&'static Language> = other.into();
        a.retain(|x| b.contains(x)); // NB: b should be very small (fewer than 10 items) so just iterate the vec.
        a.into()
    }
}

impl Answer {
    pub(crate) fn label(&self) -> &'static str {
        match &self {
            Answer::Unknown => "unknown",
            Answer::Only(x) => &x.name,
            Answer::Many(xs) => &xs[0].name,
        }
    }
}

impl From<Vec<&'static Language>> for Answer {
    fn from(value: Vec<&'static Language>) -> Self {
        match value.as_slice() {
            [] => Answer::Unknown,
            [x] => Answer::Only(x),
            _ => Answer::Many(value),
        }
    }
}

impl From<Option<&'static Language>> for Answer {
    fn from(value: Option<&'static Language>) -> Self {
        match value {
            Some(x) => Answer::Only(x),
            None => Answer::Unknown,
        }
    }
}

impl From<Answer> for Option<&'static Language> {
    fn from(value: Answer) -> Self {
        match value {
            Answer::Unknown => None,
            Answer::Only(x) => Some(x),
            Answer::Many(mut xs) => {
                sort_by_popularity(&mut xs);
                match xs.as_slice() {
                    [x] => Some(x),
                    [x, _xs @ ..] => Some(x),
                    _ => None,
                }
            }
        }
    }
}

impl From<Answer> for Vec<&'static Language> {
    fn from(value: Answer) -> Self {
        match value {
            Answer::Unknown => vec![],
            Answer::Only(x) => vec![x],
            Answer::Many(xs) => xs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::get_language_by_name;

    #[test]
    fn answer_label() {
        assert_eq!("unknown", Answer::Unknown.label());
        assert_eq!(
            "Rust",
            Answer::Only(get_language_by_name("Rust").unwrap()).label()
        );
        assert_eq!(
            "Rust",
            Answer::Many(vec![get_language_by_name("Rust").unwrap()]).label()
        );
        assert_eq!(
            "Rust",
            Answer::Many(vec![
                get_language_by_name("Rust").unwrap(),
                get_language_by_name("Go").unwrap()
            ])
            .label()
        );
    }
}

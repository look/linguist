use std::sync::LazyLock;

use regex::Regex;

use crate::get_language_by_alias;
use crate::strategies::Answer;

pub(crate) fn init() {
    let _ = LazyLock::force(&EMACS_LANG_REGEX);
    let _ = LazyLock::force(&EMACS_MODELINE_REGEX);
    let _ = LazyLock::force(&VIM_LANG_REGEX);
    let _ = LazyLock::force(&VIM_MODELINE_REGEX);
}

/// Detects the language of a file by its emac modeline. Returns zero or one
/// language.
pub(crate) fn by_emacs_modeline(_: &str, content: &str) -> Answer {
    for content in limited_content(content) {
        if let Some(captures) = EMACS_MODELINE_REGEX.captures_iter(content).last() {
            let line = &captures[1];
            let alias = if let Some(alias) = EMACS_LANG_REGEX.captures(line) {
                alias.get(1).expect("regex capture").as_str()
            } else {
                line
            };
            return get_language_by_alias(alias).into();
        }
    }
    Answer::Unknown
}

static EMACS_MODELINE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r".*-\*-\s*(.+?)\s*-\*-.*(?m:$)").expect("invalid regex"));
static EMACS_LANG_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r".*(?i:mode)\s*:\s*([^\s;]+)\s*;*.*").expect("invalid regex"));

/// Detects the language of a file by its vim modeline. Returns zero or one
/// language.
pub(crate) fn by_vim_modeline(_: &str, content: &str) -> Answer {
    for content in limited_content(content) {
        if content.contains("UseVimball") {
            return Answer::Unknown;
        }

        if let Some(captures) = VIM_MODELINE_REGEX.captures_iter(content).last() {
            let line = &captures[1];
            let mut aliases = VIM_LANG_REGEX
                .captures_iter(line)
                .filter_map(|x| x.get(1).map(|y| y.as_str()))
                .collect::<Vec<_>>();
            aliases.dedup();
            // matchedAlias = [["syntax=ruby " "ruby"] ["ft=python " "python"] ["filetype=perl " "perl"]] returns OtherLanguage;
            // matchedAlias = [["syntax=python " "python"] ["ft=python " "python"] ["filetype=python " "python"]] returns "Python";
            match aliases.as_slice() {
                [alias] => return get_language_by_alias(alias).into(),
                [_, _] => return Answer::Unknown,
                _ => {}
            }
        }
    }
    Answer::Unknown
}

static VIM_MODELINE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:(?m:\s|^)vi(?:m[<=>]?\d+|m)?|[\t\x20]*ex)\s*[:]\s*(.*)(?m:$)")
        .expect("invalid regex")
});
static VIM_LANG_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i:filetype|ft|syntax)\s*=(\w+)(?:\s|:|$)").expect("invalid regex")
});

/// Limit the amount of content to scan with the modelines regexes.
///
/// Returns a vector of either one or two `&str`s. If the content is less than
/// 2x the search scope, returns the entire content. Otherwise returns the first
/// and last 5 lines.
fn limited_content(content: &str) -> Vec<&str> {
    const SEARCH_SCOPE: usize = 5;

    let len = content.len();
    let header_end = content
        .match_indices('\n')
        .nth(SEARCH_SCOPE)
        .map(|(i, _)| i)
        .unwrap_or(len);
    let footer_start = content
        .rmatch_indices('\n')
        .nth(SEARCH_SCOPE)
        .map(|(i, _)| i)
        .unwrap_or(0);
    if header_end >= footer_start {
        vec![content]
    } else {
        let header = content.get(0..header_end).unwrap_or("");
        let footer = content.get(footer_start..len).unwrap_or("");
        vec![header, footer]
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Into;

    use super::*;
    use crate::Language;

    #[test]
    fn emacs_modelines() {
        assert_eq!(
            Some("C++"),
            emacs_file(
                r"// -*- c++ -*-
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    template <typename X> class { X i; };
    // last line
    "
            )
        );
    }

    #[test]
    fn vim_modelines() {
        assert_eq!(
            Some("Ruby"),
            vim_file(
                r"/* vim: set filetype=ruby: */
    # I am Ruby"
            ),
        );
    }

    #[test]
    fn vim_last_modeline_wins() {
        assert_eq!(
            Some("Ruby"),
            vim_file(
                r"/* vim: set filetype=python: */
    /* vim: set filetype=ruby: */
    # I am Ruby"
            )
        );
    }

    #[test]
    fn vim_many_modelines_sameline() {
        assert_eq!(
            Some("Python"),
            vim_file(
                r"/* vim: set filetype=python: ft=python */
    # I am Ruby",
            )
        );

        assert!(
            vim_file(
                r"/* vim: set filetype=python: ft=ruby */
    # I am Ruby",
            )
            .is_none()
        );
    }

    fn vim_file(content: &str) -> Option<&str> {
        Into::<Option<&'static Language>>::into(by_vim_modeline("", content))
            .map(|x| x.name.as_str())
    }

    fn emacs_file(content: &str) -> Option<&str> {
        Into::<Option<&'static Language>>::into(by_emacs_modeline("", content))
            .map(|x| x.name.as_str())
    }
}

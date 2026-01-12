use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

use crate::strategies::lowercase_ext;
use crate::{Answer, languages_by_extension};

/// Detects the language of a file by its extension. Returns zero or more
/// languages.
pub(crate) fn by_extension(path: &str, _: &str) -> Answer {
    // Check against our list of very generic extensions. This looks at just the
    // final extension component. If a path's extension is too generic, we skip
    // doing extension based detection and defer to a subsequent strategy.
    if let Some(ext) = lowercase_ext(path)
        && GENERIC.contains(ext.as_str())
    {
        return Answer::Unknown;
    }

    // Check increasingly smaller extensions combinations. For example, if the
    // path is `foo.js.erb` we'll check: `js.erb`, `erb` by looking up those
    // extensions in the languages.yml data.
    //
    // Returns an answer as soon as we have one or more languages (won't
    // exhaustively check all suffixes).
    if let Some(name) = Path::new(&path).file_name().and_then(|x| x.to_str()) {
        let name = name.to_lowercase();
        let mut dots = name.match_indices('.').map(|(i, _)| i);
        dots.find_map(|dot| {
            let ext = &name[dot + 1..];
            let languages = languages_by_extension(ext);
            if languages.is_empty() {
                None
            } else {
                Some(languages.into())
            }
        })
        .unwrap_or(Answer::Unknown)
    } else {
        Answer::Unknown
    }
}

pub(crate) fn init() {
    let _ = LazyLock::force(&GENERIC);
}

static GENERIC: LazyLock<HashSet<String>> = LazyLock::new(|| {
    let mut out = HashSet::new();
    let yml = include_bytes!("../../../../lib/linguist/generic.yml");

    let value: serde_yaml::Value =
        serde_yaml::from_slice(&yml[..]).expect("unable to parse generic.yml!");
    let data = value
        .as_mapping()
        .expect("invalid mapping in generic.yml!")
        .to_owned();
    for item in data
        .get("extensions")
        .expect("must have extensions key")
        .as_sequence()
        .expect("invalid mapping in generic.yml!")
    {
        let item = item
            .as_str()
            .expect("extensions are strings")
            .strip_prefix('.')
            .expect("invalid extension")
            .to_lowercase()
            .to_string();
        out.insert(item);
    }
    out
});

#[cfg(test)]
mod test {
    use super::by_extension;
    use crate::strategies::Answer;

    #[test]
    fn by_extension_returns_none_for_generic_ext() {
        for ext in ["1", "2", "app", "sol", "url"] {
            assert_eq!(
                by_extension(&format!("foo/bar/file.{ext}"), "ignored content"),
                Answer::Unknown
            );
        }
    }
}

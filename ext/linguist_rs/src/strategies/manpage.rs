use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::{Answer, get_language_by_name};

/// Detects the language of a file by its manpage. Returns Answer::None unless
/// the file path is a known manpage extension, otherwise returns the two known
/// manpage languages and defers selection to a subsequent strategy.
pub(crate) fn by_manpage(path: &str, _: &str) -> Answer {
    if let Some(name) = Path::new(path).file_name().and_then(|x| x.to_str())
        && MANPAGE_EXTS.is_match(name)
    {
        return Answer::Many(vec![
            get_language_by_name("Roff Manpage").expect("known language"),
            get_language_by_name("Roff").expect("known language"),
        ]);
    }
    Answer::Unknown
}

pub(crate) fn init() {
    let _ = LazyLock::force(&MANPAGE_EXTS);
}

static MANPAGE_EXTS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\.(?:[1-9](?:[a-z_]+[a-z_0-9]*)?|0p|n|man|mdoc)(?:\.in)?$")
        .expect("invalid regex")
});

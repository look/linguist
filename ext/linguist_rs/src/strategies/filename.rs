use std::path::Path;

use super::Answer;
use crate::languages_by_filename;

/// Detects the language of a file by its filename. Returns zero or more
/// languages.
pub(crate) fn by_filename(path: &str, _: &str) -> Answer {
    if let Some(filename) = Path::new(path).file_name().and_then(|f| f.to_str()) {
        languages_by_filename(filename).into()
    } else {
        Answer::Unknown
    }
}

use std::sync::LazyLock;

use regex::Regex;

use crate::{Answer, get_language_by_name};

/// Detects the language of a file as XML if any of the first lines match an xml
/// opening tag, otherwise returns Answer::None.
pub(crate) fn by_xml(_: &str, content: &str) -> Answer {
    if content.lines().take(2).any(|l| XML.is_match(l)) {
        Answer::Only(get_language_by_name("XML").expect("known language"))
    } else {
        Answer::Unknown
    }
}

pub(crate) fn init() {
    let _ = LazyLock::force(&XML);
}

static XML: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<\?xml\s+version=").expect("invalid regex"));

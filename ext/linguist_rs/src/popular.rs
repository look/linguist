use std::cmp::Reverse;
use std::collections::HashSet;
use std::sync::LazyLock;

use crate::Language;

static POPULAR_LANGUAGES: LazyLock<HashSet<String>> = LazyLock::new(|| {
    let file = include_bytes!("../../../lib/linguist/popular.yml");
    let value: serde_yaml::Value =
        serde_yaml::from_slice(&file[..]).expect("unable to parse popular.yml!");
    value
        .as_sequence()
        .expect("invalid sequence in popular.yml!")
        .iter()
        .map(|item| item.as_str().expect("languages are strings").to_owned())
        .collect()
});

pub(crate) fn sort_by_popularity(langs: &mut [&'static Language]) {
    langs.sort_by_key(|a| Reverse(POPULAR_LANGUAGES.contains(&a.name)))
}

pub fn init() {
    let _ = LazyLock::force(&POPULAR_LANGUAGES);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::get_language_by_name;

    #[test]
    fn sort_by_popular() {
        let mut langs = ["Erlang", "JavaScript"]
            .iter()
            .map(|x| get_language_by_name(x).unwrap())
            .collect::<Vec<_>>();

        sort_by_popularity(&mut langs);

        assert_eq!(
            vec!["JavaScript", "Erlang"],
            langs
                .into_iter()
                .map(|x| x.name.clone())
                .collect::<Vec<_>>()
        )
    }
}

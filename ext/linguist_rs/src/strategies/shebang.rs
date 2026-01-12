use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::languages_by_interpreter;
use crate::strategies::Answer;

/// Detects the language of a file by its shebang. Returns zero or more
/// languages.
pub(crate) fn by_shebang(_: &str, content: &str) -> Answer {
    if let Some(line) = content.lines().next().and_then(|x| x.strip_prefix("#!")) {
        let mut parts = line.split_whitespace();
        if let Some(part) = parts.next()
            && let Some(base) = Path::new(part).file_name().and_then(|x| x.to_str())
        {
            let script = if base == "env" {
                parts.find(|part| !ENV_OPT_ARGS.is_match(part) && !ENV_VAR_ARGS.is_match(part))
            } else {
                Some(base)
            };

            if let Some(script) = script {
                let script = if script == "sh" {
                    content
                        .lines()
                        .take(5)
                        .filter_map(|l| {
                            MULTI_LINE_EXEC
                                .captures(l)
                                .map(|captures| captures.get(1).expect("regex capture").as_str())
                        })
                        .last()
                        .unwrap_or(script)
                } else {
                    script
                };

                if script == "osascript" && parts.any(|x| x == "-l") {
                    // osascript can be called with an optional `-l <language>`
                    // argument, which may not be a language with an interpreter. In
                    // this case, return and rely on the subsequent strategies to
                    // determine the language.
                    return Answer::Unknown;
                }

                // "python2.6" -> "python"
                let script = PYTHON_VERSION.replace(script, "").to_string();
                if let Some(script) = Path::new(&script).file_name().and_then(|x| x.to_str()) {
                    return languages_by_interpreter(script).into();
                }
            }
        }
    }
    Answer::Unknown
}

pub(crate) fn init() {
    let _ = LazyLock::force(&ENV_OPT_ARGS);
    let _ = LazyLock::force(&ENV_VAR_ARGS);
    let _ = LazyLock::force(&MULTI_LINE_EXEC);
    let _ = LazyLock::force(&PYTHON_VERSION);
}
// NB: We've customized this opt args regex to require at least one (instead
// of zero or more) chars after a single dash.
static ENV_OPT_ARGS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"-[i0uCSv]+|--\S+").expect("invalid regex"));
static ENV_VAR_ARGS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\S+=\S+").expect("invalid regex"));
static MULTI_LINE_EXEC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"exec (\w+).+\$0.+\$@").expect("invalid regex"));
static PYTHON_VERSION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\.\d+)$").expect("invalid regex"));

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Language;

    #[test]
    fn ruby() {
        assert_eq!(
            Some("Ruby"),
            file(
                r#"#!/usr/bin/env -vS ruby -w -Ilib:test
puts "Ruby"
"#,
            ),
        );
    }

    #[test]
    fn python() {
        assert_eq!(
            Some("Python"),
            file(
                r#"#!/usr/bin/env python2.4
print "Python"
"#,
            ),
        );
    }

    #[test]
    fn osascript() {
        assert_eq!(
            None,
            file(
                r#"#!/usr/bin/env osascript -l JavaScript

function run(argv) {}"#
            )
        );
    }

    #[test]
    fn parrot() {
        assert_eq!(
            Some("Parrot Assembly"),
            file(
                r#"#!/usr/bin/env parrot

.sub 'main' :main
    say "Hello!"
.end
"#
            )
        );
    }

    #[test]
    fn parse_interpreters() {
        [
            ("AppleScript", "#!/usr/bin/env osascript"),
            ("AppleScript", "#!/usr/bin/osascript"),
            ("Common Lisp", "#!/usr/bin/sbcl --script\n\n"),
            ("Crystal", "#!/usr/bin/env bin/crystal"),
            ("Perl", "#! perl"),
            ("Python", "#!/bin/python\n# foo\n# bar\n# baz"),
            ("Python", "#!/usr/bin/python2.7\n\n\n\n"),
            ("Python", "#!/usr/bin/python3\n\n\n\n"),
            (
                "Python",
                "#!/usr/bin/env foo=bar bar=foo python -cos=__import__(\"os\");",
            ),
            ("R", "#!/usr/bin/env Rscript\n# example R script\n#\n"),
            ("Ruby", "#!/usr/sbin/ruby\n# bar"),
            ("Ruby", "#!/usr/sbin/ruby"),
            ("Ruby", "#!/usr/sbin/ruby foo bar baz\n"),
            ("Ruby", "#!/usr/bin/env ruby\n# baz"),
            ("Ruby", "#!/bin/sh\n\n\nexec ruby $0 $@"),
            ("Ruby", "#!/usr/bin/env -vS ruby -wKU\nputs ?t+?e+?s+?t"),
            ("sed", "#!/usr/bin/env --split-string sed -f\ny/a/A/"),
            ("Shell", "#!/usr/bin/bash\n"),
            ("Shell", "#!/bin/sh"),
            (
                "Shell",
                "#! /usr/bin/env A=003 B=149 C=150 D=xzd E=base64 F=tar G=gz H=head I=tail sh",
            ),
            (
                "TypeScript",
                "#!/usr/bin/env -S GH_TOKEN=ghp_*** deno run --allow-net\nconsole.log(1);",
            ),
        ]
        .iter()
        .for_each(|(name, content)| {
            assert_eq!(Some(*name), file(content));
        });
    }

    fn file(content: &str) -> Option<&str> {
        Into::<Option<&'static Language>>::into(by_shebang("", content)).map(|x| x.name.as_str())
    }
}

//! Functions to detect if a file should be indexed for code search or not. There are also some
//! special purpose, relaxed vendored/generated checks in here that are more amenable to search.
//!
//! From our [website docs](https://docs.github.com/en/search-github/github-code-search/about-github-code-search#limitations):
//! * Vendored and generated code is excluded
//! * Empty files and files over 350 KiB are excluded
//! * Binary files (PDF, etc.) are excluded
//! * Only UTF-8 encoded files are included

use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::Instant;

use regex::Regex;

use crate::generated::*;
use crate::strategies::lowercase_ext;

use thiserror::Error;

use crate::generated::{ContentReason, FilePathReason, Reason};

// Default minimum size of content can be ingested.
const DEFAULT_MIN_CONTENT_SIZE_BYTES: usize = 3;
// Default maximum size of content can be ingested.
const DEFAULT_MAX_CONTENT_SIZE_BYTES: usize = 350 * 1024; // 350 KB

pub struct IngestFilter {
    min_content_bytes: usize,
    max_content_bytes: usize,
}

impl IngestFilter {
    // Note: While this is convenient internally, it may not be worth it if we want to provide this functionality to
    // external clients since it adds a dependency on blackbird_core.
    pub fn ingestable_bytes<'a>(&self, path: &str, content: &'a [u8]) -> Result<&'a str, Filtered> {
        match simdutf8::basic::from_utf8(content) {
            Ok(_) => {
                let content = unsafe { str::from_utf8_unchecked(content) };
                self.ingestable(path, content)?;
                Ok(content)
            }
            Err(_) => Err(Filtered::NotUtf8),
        }
    }

    pub fn ingestable_from_bytes(&self, path: &str, content: Vec<u8>) -> Result<String, Filtered> {
        self.ingestable_bytes(path, &content)?;
        Ok(unsafe { String::from_utf8_unchecked(content) })
    }

    pub fn ingestable_meta(&self, size: usize) -> Result<(), Filtered> {
        if size < self.min_content_bytes {
            Err(Filtered::TooSmall(size))
        } else if size > self.max_content_bytes {
            Err(Filtered::TooLarge(size))
        } else {
            Ok(())
        }
    }

    pub fn is_ingestable(&self, path: &str, content: &str) -> bool {
        self.ingestable(path, content).is_ok()
    }

    pub fn ingestable_path(&self, path: &str) -> Result<(), Filtered> {
        if let Some(reason) = skip_indexing_path(path) {
            Err(Filtered::from_reason(reason))
        } else {
            Ok(())
        }
    }

    pub fn ingestable(&self, path: &str, content: &str) -> Result<(), Filtered> {
        self.ingestable_meta(content.len())?;
        if let Some(reason) = skip_indexing(path, content) {
            Err(Filtered::from_reason(reason))
        } else {
            Ok(())
        }
    }
}

impl Default for IngestFilter {
    fn default() -> Self {
        Self {
            min_content_bytes: DEFAULT_MIN_CONTENT_SIZE_BYTES,
            max_content_bytes: DEFAULT_MAX_CONTENT_SIZE_BYTES,
        }
    }
}

#[derive(Error, Debug)]
pub enum Filtered {
    #[error("content is not UTF-8")]
    NotUtf8,
    #[error("content is too small")]
    TooSmall(usize),
    #[error("content is too large")]
    TooLarge(usize),
    #[error("file path is not indexable")]
    FilePath(FilePathReason),
    #[error("content is not indexable")]
    Content(ContentReason),
}

impl Filtered {
    fn from_reason(reason: Reason) -> Self {
        match reason {
            Reason::FilePath(fp) => Filtered::FilePath(fp),
            Reason::Content(cr) => Filtered::Content(cr),
        }
    }
}

/// Detects if the file and its content should be skipped for indexing. Returns
/// a reason why it should be skipped or `None` if it should be indexed.
pub fn skip_indexing(path: &str, content: &str) -> Option<Reason> {
    // This is a hack for now since this function is used by the external-ingest-utils
    // which does not support metrics.
    #[cfg(not(target_family = "wasm"))]
    let start = Instant::now();

    let skip_indexing = skip_indexing_path(path).or_else(|| skip_indexing_content(path, content));

    #[cfg(not(target_family = "wasm"))]
    metrics::histogram!("linguist.check.skip_indexing_duration",
        "skip_indexing" => skip_indexing.is_some().to_string(),
        "reason" => skip_indexing.map(|r| r.label()).unwrap_or_default())
    .record(start.elapsed());
    skip_indexing
}

/// Detects if the file path and its content is_indexable.
pub fn is_indexable(path: &str, content: &str) -> bool {
    skip_indexing(path, content).is_none()
}

/// Detects if the file path is suitable for indexing.
pub fn is_path_indexable(path: &str) -> bool {
    let start = Instant::now();
    let skip_indexing = skip_indexing_path(path);
    metrics::histogram!("linguist.check.skip_indexing_duration",
        "skip_indexing" => skip_indexing.is_some().to_string(),
        "reason" => skip_indexing.map(|r| r.label()).unwrap_or_default())
    .record(start.elapsed());
    skip_indexing.is_none()
}

fn skip_indexing_path(path: &str) -> Option<Reason> {
    UNSUITABLE_PATH_RULES
        .is_generated(path)
        .map(Reason::FilePath)
}

/// Detects if the content is suitable for indexing.
fn skip_indexing_content(path: &str, content: &str) -> Option<Reason> {
    let ext = lowercase_ext(path)?;
    UNSUITABLE_CONTENT_RULES
        .is_generated(&ext, content)
        .map(Reason::Content)
}

// In epoch 370, the following path rules changed:
//
// Removed:
//   - (r"(^|/)[Ee]xtern(als?)?/", FilePathReason::ExternalsDir),
static UNSUITABLE_PATH_RULES: LazyLock<GeneratedPathRules> = LazyLock::new(|| {
    GeneratedPathRules::new(&[
        (r"(^|/)[Pp]ackages/.+\.\d+/", FilePathReason::PackagesDir),
        (
            r"(^|/)lib/python.*/site-packages/",
            FilePathReason::PythonSitePackages,
        ),
        (r"(^|/)\.gitignore$", FilePathReason::GitIgnore),
        (r"(\.|-)min\.(js|css)$", FilePathReason::MinifiedJsOrCss),
        (r"\.(js|css)\.map$", FilePathReason::SourceMap),
        (r"(^|/)bower_components/", FilePathReason::BowerComponents),
        (
            r"vendor/([-0-9A-Za-z]+\.)+(com|edu|gov|in|me|net|org|fm|io)",
            FilePathReason::VendorDomain,
        ),
        (r"(^|/)dist/", FilePathReason::DistDir),
        (
            r"(3rd|[Tt]hird)[-_]?[Pp]arty/",
            FilePathReason::ThirdPartyDir,
        ),
        (r"^vendors?/", FilePathReason::VendorDir),
        (r"(^|/)node_modules/", FilePathReason::NodeModules),
        (r"(^Pods|/Pods)/", FilePathReason::CocaPods),
        (r"(^|/)go\.sum$", FilePathReason::GoSum),
        (r"(^|/)yarn\.lock$", FilePathReason::YarnLock),
    ])
});

// In epoch 370, the following content rules changed:
//
// Added:
//   - (&["rbi"], ContentReason::SorbetRBI, is_sorbet_rbi),
static UNSUITABLE_CONTENT_RULES: LazyLock<GeneratedContentRules> = LazyLock::new(|| {
    GeneratedContentRules::new(&[
        (&["go"], ContentReason::GeneratedGo, is_generated_go),
        (&["css", "js"], ContentReason::MinifiedJsOrCss, is_minified),
        (
            &["css", "js"],
            ContentReason::HasSourceMapRef,
            has_source_map_ref,
        ),
        (&["map"], ContentReason::SourceMap, is_source_map),
        (
            &["meta"],
            ContentReason::Unity3DMetadata,
            is_generated_unity_3d_meta,
        ),
        (&["rbi"], ContentReason::SorbetRBI, is_sorbet_rbi),
    ])
});

/// Returns true if the path should be considered vendored for purpose of code
/// search. This has some special cases to change behaviors we consider overly
/// broad.
pub fn is_vendored(path: &str) -> bool {
    if is_jenkinsfile(path) {
        return false;
    }

    let mut pb = PathBuf::from(path);
    remove_overly_broad_cache_components(&mut pb);
    remove_typescript_interface_extension(&mut pb);

    crate::is_vendored(pb.as_path().to_str().expect("invalid path"))
}

pub fn is_generated(path: &str, content: &str) -> Option<Reason> {
    let path_buf = PathBuf::from(path);
    let filename = path_buf
        .file_name()
        .expect("there must be a filename")
        .to_string_lossy();

    // Linguist doesn't consider these files to be generated, but we do.
    match filename.as_bytes() {
        b"go.sum" => return Some(Reason::FilePath(FilePathReason::GoSum)),
        b"yarn.lock" => return Some(Reason::FilePath(FilePathReason::YarnLock)),
        _ => {}
    }

    crate::is_generated(path, content)
}

/// Linguist considers useful .d.ts [interface definitions] as vendored.
///
/// Instead, we check if it would still be vendored if it just had a `.ts`
/// extension. This modifies `path` so we can do that check.
///
/// [interface definitions]: https://github.com/github/blackbird/blob/9649833ca65f8092d051cacc79dd9c9c477df642/crates/linguist/src/linguist/vendor.yml#L180-L181
fn remove_typescript_interface_extension(pb: &mut PathBuf) {
    const TYPESCRIPT_INTERFACE: &str = ".d.ts";
    const TYPESCRIPT_EXT: &str = "ts";
    if let Some(filename) = pb.file_name()
        && filename.to_string_lossy().ends_with(TYPESCRIPT_INTERFACE)
    {
        // NOTE: Rust's extension facilities only consider the final extension.
        let newpath = pb
            .with_extension("") // strip .ts
            .with_extension("") // strip .d
            .with_extension(TYPESCRIPT_EXT); // add back .ts
        pb.clear();
        pb.push(newpath);
    }
}

/// Linguist has a few overly broad vendor rules about directories names (e.g
/// cache, env). If `VENDOR_CACHE_REGEX` matches, we remove the directory
/// structure so only the file name is considered.
fn remove_overly_broad_cache_components(pb: &mut PathBuf) {
    if VENDORED_CACHE_REGEX.is_match(&pb.to_string_lossy()) {
        let filename: PathBuf = pb.clone().file_name().expect("must have a filename").into();
        pb.clear();
        pb.push(filename);
    }
}

/// Linguist has a few overly broad vendor rules for what it considers vendored.
/// In general, rules that exclude an entire directory just based on its name
/// aren't going to work for blackbird as we'll skip indexing content that
/// should be searchable. For example, [this rule] excludes anything in a
/// "cache" directory.
///
/// The compromise is to consider top level directories matching linguist's
/// rules as vendored, but anything nested should be considered for indexing.
///
/// TODO: Based on the special casing here and in the [is_generated] check, we
/// need a better long term solution.
///
/// [this rule]: https://github.com/github/linguist/blob/80f3531e8a1014a23f4606458e5a528053ed3cac/lib/linguist/vendor.yml#L13
static VENDORED_CACHE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("/(cache|env)/").expect("invalid regex"));

/// Jenkinsfiles are specifically [marked as vendored], but we get a lot of feedback
/// that people want to search these.
///
/// [marked as vendored]: https://github.com/github/blackbird/blob/9649833ca65f8092d051cacc79dd9c9c477df642/crates/linguist/src/linguist/vendor.yml#L381-L382
fn is_jenkinsfile(path: &str) -> bool {
    const JENKINSFILE: &str = "Jenkinsfile";

    if let Some(filename) = Path::new(path).file_name() {
        return filename.to_string_lossy() == JENKINSFILE;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexable() {
        crate::fixtures::run(
            "indexable",
            include_str!("../test/indexable.txt"),
            is_indexable,
        );
    }

    #[test]
    fn test_source_map_ref() {
        // blank lines at the end of the file
        assert!(has_source_map_ref(
            r#"/*# sourceMappingURL=bootstrap.css.map */


        "#
        ));
        // some trailing ; b/c javascript
        assert!(has_source_map_ref(
            r#"/*# sourceMappingURL=bootstrap.css.map */
        ;
        ;"#
        ));
    }

    #[test]
    fn test_source_maps() {
        let content = r#"//// [test.js.map]
{"version":3"}"#;
        let is_indexable = is_indexable("test.js.map", content);
        assert!(!is_indexable)
    }

    // Test out custom code search rules
    #[test]
    fn test_is_vendored() {
        let tests: Vec<(&str, bool)> = vec![
            ("some/long/path/Jenkinsfile", false), // we override this one
            ("vendor/Jenkinsfile/in/the/middle", true), // doesn't affect things when it's not a filename
            ("this/is/not/cache/example.go", false),    // handle overly broad Linguist rule
            ("this/is/not/env/example.go", false),      // handle overly broad Linguist rule
            ("some_typescript.d.ts", false), // Typescript interface definitions are not vendored
            (".vscode/settings.json", true), // Not covered by our overrides? Defer to linguist.
            ("src/com/example/FooBar.java", false),
        ];

        for (path, expected) in tests {
            assert_eq!(
                expected,
                is_vendored(path),
                "expected is_vendored to return {expected} for {path:?}"
            );
        }
    }

    #[test]
    fn test_is_generated() {
        let tests: Vec<(&str, &str, bool)> = vec![
            ("some/long/path/go.sum", "", true),    // we override this one
            ("some/long/path/yarn.lock", "", true), // we override this one
            ("pkg/internal/proto/proto.pb.go", "some go code", false), // Not covered by our overrides? Defer to linguist.
            (
                "pkg/internal/proto/proto.pb.go",
                "Autogenerated by Thrift Compiler",
                true,
            ),
        ];

        for (path, content, expected) in tests {
            assert_eq!(
                expected,
                is_generated(path, content).is_some(),
                "expected is_vendored to return {expected} for {path:?} with content {content:?}",
            );
        }
    }
}

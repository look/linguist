use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Instant;

use regex::{Regex, RegexSet};

use crate::strategies::{lowercase_ext, take_n_lines};

/// Detects if the path and content is generated.
pub fn is_generated(path: &str, content: &str) -> Option<Reason> {
    let start = Instant::now();
    let is_generated = is_generated_path(path).or_else(|| is_generated_content(path, content));
    metrics::histogram!("linguist.check.is_generated_duration",
        "is_generated" => is_generated.is_some().to_string(),
        "reason" => is_generated.map(|r| r.label()).unwrap_or_default())
    .record(start.elapsed());
    is_generated
}

fn is_generated_path(path: &str) -> Option<Reason> {
    GENERATED_PATH_RULES
        .is_generated(path)
        .map(Reason::FilePath)
}

fn is_generated_content(path: &str, content: &str) -> Option<Reason> {
    let ext = lowercase_ext(path)?;
    GENERATED_CONTENT_RULES
        .is_generated(&ext, content)
        .map(Reason::Content)
}

pub(crate) struct GeneratedPathRules {
    regexes: RegexSet,
    reasons: Vec<FilePathReason>,
}

impl GeneratedPathRules {
    pub(crate) fn new(rules: &[(&'static str, FilePathReason)]) -> Self {
        let (regexes, reasons): (Vec<&str>, _) =
            rules
                .iter()
                .fold((vec![], vec![]), |mut acc, (regex, reason)| {
                    acc.0.push(regex);
                    acc.1.push(*reason);
                    acc
                });
        Self {
            regexes: RegexSet::new(regexes).expect("invalid regex"),
            reasons,
        }
    }

    pub(crate) fn is_generated(&self, path: &str) -> Option<FilePathReason> {
        self.regexes
            .matches(path)
            .iter()
            .next()
            .map(|i| self.reasons[i])
    }
}

static GENERATED_PATH_RULES: LazyLock<GeneratedPathRules> = LazyLock::new(|| {
    GeneratedPathRules::new(&[
        (
            r"\.(nib|xcworkspacedata|xcuserstate)$",
            FilePathReason::Xcode,
        ),
        (r"(?:^|/)\.idea/", FilePathReason::IntelliJ),
        (r"(^Pods|/Pods)/", FilePathReason::CocaPods),
        (r"(^|/)Carthage/Build/", FilePathReason::Carthage),
        (r"(?i)\.designer\.(cs|vb)$", FilePathReason::NetDesigner),
        (r"(?i)\.feature.cs$", FilePathReason::NetSpecflow),
        (r"node_modules/", FilePathReason::NodeModules),
        (
            r"vendor/([-0-9A-Za-z]+\.)+(com|edu|gov|in|me|net|org|fm|io)",
            FilePathReason::GoVendor,
        ),
        (r"(Gopkg|glide)\.lock$", FilePathReason::GoLock),
        (r"poetry\.lock$", FilePathReason::PoetryLock),
        (r"pdm\.lock$", FilePathReason::PdmLock),
        (r"(^|/)(\w+\.)?esy.lock$", FilePathReason::EsyLock),
        (r"npm-shrinkwrap\.json$", FilePathReason::NpmShrinkwrap),
        (r"package-lock\.json$", FilePathReason::NpmPackageLock),
        (r"(^|/)\.pnp\..*$", FilePathReason::YarnPnp),
        (r"Godeps/", FilePathReason::Godeps),
        (r"composer\.lock$", FilePathReason::ComposwerLock),
        (r".\.zep\.(?:c|h|php)$", FilePathReason::Zephir),
        (r"Cargo\.lock$", FilePathReason::CargoLock),
        (r"(^|/)flake\.lock$", FilePathReason::NixFlakesLock),
        (r"Pipfile\.lock$", FilePathReason::PipenvLock),
        (
            r"(?:^|/)\.terraform\.lock\.hcl$",
            FilePathReason::TerraformLock,
        ),
        (r"__generated__/", FilePathReason::RelayCompilerGraphQL),
        (r"(?i)_tlb\.pas$", FilePathReason::DelphiInterface),
        (r"\.(css|js)\.map$", FilePathReason::SourceMap),
    ])
});

pub(crate) type GeneratedFn = fn(&str) -> bool;

pub(crate) struct Rule {
    reason: ContentReason,
    f: GeneratedFn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilePathReason {
    BowerComponents,
    CargoLock,
    Carthage,
    CocaPods,
    ComposwerLock,
    DelphiInterface,
    DistDir,
    EsyLock,
    ExternalsDir,
    GitIgnore,
    Godeps,
    GoLock,
    GoSum,
    GoVendor,
    IntelliJ,
    MinifiedJsOrCss,
    NetDesigner,
    NetSpecflow,
    NixFlakesLock,
    NodeModules,
    NonUTF8Path,
    NpmPackageLock,
    NpmShrinkwrap,
    PackagesDir,
    PdmLock,
    PipenvLock,
    PoetryLock,
    PythonSitePackages,
    RelayCompilerGraphQL,
    SourceMap,
    TerraformLock,
    ThirdPartyDir,
    VendorDir,
    VendorDomain,
    Xcode,
    YarnLock,
    YarnPnp,
    Zephir,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentReason {
    CompiledCoffeeScript,
    CompiledCython,
    GeneratedAntlr,
    GeneratedByGrammarKit,
    GeneratedByGRPC,
    GeneratedByHaxe,
    GeneratedByJavah,
    GeneratedByJFlex,
    GeneratedByJison,
    GeneratedByJOOQ,
    GeneratedByProtobufCompiler,
    GeneratedByRacc,
    GeneratedByRoxygen2,
    GeneratedByThrift,
    GeneratedDart,
    GeneratedGameMakerStudio,
    GeneratedGIMPCImage,
    GeneratedGo,
    GeneratedHTML,
    GeneratedMicrosoftVS6BuildFile,
    GeneratedNetDocfile,
    GeneratedPegJsParser,
    GeneratedPostscript,
    GeneratedPPPort,
    GeneratedProtobuf,
    GFortran,
    HasSourceMapRef,
    KiCAD,
    MinifiedJsOrCss,
    SorbetRBI,
    SourceMap,
    Unity3DMetadata,
    VCRCassette,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    FilePath(FilePathReason),
    Content(ContentReason),
}

impl Reason {
    // A label string suitable for stat tags and structured logging.
    pub fn label(&self) -> String {
        match &self {
            Reason::FilePath(reason) => format!("path-{reason:?}").to_lowercase(),
            Reason::Content(reason) => format!("content-{reason:?}").to_lowercase(),
        }
    }
}

#[derive(Default)]
pub(crate) struct GeneratedContentRules {
    rules: HashMap<String, Vec<Rule>>,
}

impl GeneratedContentRules {
    pub(crate) fn new(xs: &[(&[&str], ContentReason, GeneratedFn)]) -> Self {
        let mut rules: HashMap<String, Vec<Rule>> = HashMap::new();
        for (exts, reason, f) in xs {
            for e in *exts {
                let rule = Rule {
                    reason: *reason,
                    f: *f,
                };
                rules.entry(e.to_string()).or_default().push(rule);
            }
        }
        Self { rules }
    }

    pub(crate) fn is_generated(&self, ext: &str, content: &str) -> Option<ContentReason> {
        for rules in self.rules.get(ext).iter() {
            for rule in rules.iter() {
                if (rule.f)(content) {
                    return Some(rule.reason);
                }
            }
        }
        None
    }
}

static GENERATED_CONTENT_RULES: LazyLock<GeneratedContentRules> = LazyLock::new(|| {
    GeneratedContentRules::new(&[
        (
            &["c", "cpp"],
            ContentReason::CompiledCython,
            is_compiled_cython,
        ),
        (&["css", "js"], ContentReason::MinifiedJsOrCss, is_minified),
        (
            &["css", "js"],
            ContentReason::HasSourceMapRef,
            has_source_map_ref,
        ),
        (
            &["js"],
            ContentReason::CompiledCoffeeScript,
            is_compiled_coffeescript,
        ),
        (
            &["js"],
            ContentReason::GeneratedPegJsParser,
            is_generated_peg_js_parser,
        ),
        (&["go"], ContentReason::GeneratedGo, is_generated_go),
        (&["map"], ContentReason::SourceMap, is_source_map),
        (
            &["proto"],
            ContentReason::GeneratedProtobuf,
            is_generated_protobuf,
        ),
        (
            &["xml"],
            ContentReason::GeneratedNetDocfile,
            is_generated_net_docfile,
        ),
        (
            &["ps", "eps", "pfa"],
            ContentReason::GeneratedPostscript,
            is_generated_postscript,
        ),
        (
            &[
                "py", "java", "h", "cc", "cpp", "m", "rb", "php", "go", "cs", "js", "ts", "m",
                "kt", "kts", "dart", "swift", "rs", "thrift", "sol",
            ],
            ContentReason::GeneratedByProtobufCompiler,
            is_protoc_generated,
        ),
        (&["js"], ContentReason::GeneratedByProtobufCompiler, |c| {
            line_contains(c, 5, "GENERATED CODE -- DO NOT EDIT!")
        }),
        (
            &["rb", "py", "go", "js", "m", "java", "h", "cc", "cpp", "php"],
            ContentReason::GeneratedByThrift,
            |c| any_line_contains(c, 6, "Autogenerated by Thrift Compiler"),
        ),
        (
            &["h"],
            ContentReason::GeneratedByJavah,
            is_generated_jni_header,
        ),
        (&["yml"], ContentReason::VCRCassette, is_vcr_cassette),
        (&["g"], ContentReason::GeneratedAntlr, is_generated_antlr),
        (&["mod"], ContentReason::KiCAD, |c| {
            line_contains(c, 0, "PCBNEW-LibModule-V")
        }),
        (&["mod"], ContentReason::GFortran, |c| {
            line_contains(c, 0, "GFORTRAN module version '")
        }),
        (
            &["meta"],
            ContentReason::Unity3DMetadata,
            is_generated_unity_3d_meta,
        ),
        (&["rb"], ContentReason::GeneratedByRacc, |c| {
            line_contains(c, 2, "# This file is automatically generated by Racc")
        }),
        (&["java"], ContentReason::GeneratedByJFlex, |c| {
            line_contains(c, 0, "/* The following code was generated by JFlex ")
        }),
        (&["java"], ContentReason::GeneratedByGrammarKit, |c| {
            line_contains(
                c,
                0,
                "// This is a generated file. Not intended for manual editing.",
            )
        }),
        (&["rd"], ContentReason::GeneratedByRoxygen2, |c| {
            line_contains(c, 0, "% Generated by roxygen2: do not edit by hand")
        }),
        (
            &["html", "htm", "xhtml"],
            ContentReason::GeneratedHTML,
            is_generated_html,
        ),
        (&["js"], ContentReason::GeneratedByJison, |c| {
            line_contains(c, 0, "/* parser generated by jison ")
                || line_contains(c, 0, "/* generated by jison-lex ")
        }),
        (
            &["cpp", "hpp", "h", "cc"],
            ContentReason::GeneratedByGRPC,
            |c| line_contains(c, 0, "// Generated by the gRPC"),
        ),
        (&["dart"], ContentReason::GeneratedDart, is_generated_dart),
        // TODO: path name match of: /ppport\.h$/
        (&["h"], ContentReason::GeneratedPPPort, |c| {
            line_contains(c, 8, "Automatically created by Devel::PPPort")
        }),
        (
            &["yy", "yyp"],
            ContentReason::GeneratedGameMakerStudio,
            is_generated_gamemakerstudio,
        ),
        (
            &["c", "h"],
            ContentReason::GeneratedGIMPCImage,
            is_generated_gimp,
        ),
        (
            &["dsp"],
            ContentReason::GeneratedMicrosoftVS6BuildFile,
            |c| any_line_contains(c, 3, "# Microsoft Developer Studio Generated Build File"),
        ),
        (
            &["js", "py", "lua", "cpp", "h", "java", "cs", "php"],
            ContentReason::GeneratedByHaxe,
            |c| any_line_contains(c, 3, "Generated by Haxe"),
        ),
        (&["java"], ContentReason::GeneratedByJOOQ, |c| {
            any_line_contains(c, 2, "This file is generated by jOOQ.")
        }),
        (&["rbi"], ContentReason::SorbetRBI, is_sorbet_rbi),
    ])
});

pub(crate) fn is_minified(content: &str) -> bool {
    let (chars, lines) = content
        .lines()
        .fold((0, 0), |(c, l), line| (c + line.len(), l + 1));
    lines > 0 && chars / lines > 110
}

pub(crate) fn has_source_map_ref(content: &str) -> bool {
    static SOURCE_MAP_REF_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^/[*/][\#@] source(?:Mapping)?URL|sourceURL=").expect("invalid regex")
    });

    content
        .lines()
        .rev()
        .filter(|x| !x.is_empty())
        .take(3)
        .any(|line| SOURCE_MAP_REF_REGEX.is_match(line))
}

pub(crate) fn is_source_map(content: &str) -> bool {
    static SOURCE_MAP_REGEX: LazyLock<RegexSet> = LazyLock::new(|| {
        RegexSet::new([r#"^\{"version":\d+,"#, r"^/\*\* Begin line maps\. \*\*/\{"])
            .expect("invalid regex")
    });

    if let Some(l) = content.lines().next() {
        SOURCE_MAP_REGEX.is_match(l)
    } else {
        false
    }
}

fn is_compiled_coffeescript(content: &str) -> bool {
    static COFFESCRIPT_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^// Generated by ").expect("invalid regex"));

    let first_line = content.lines().next().unwrap_or("");
    // CoffeeScript generated by > 1.2 include a comment on the first line
    if COFFESCRIPT_REGEX.is_match(first_line) {
        return true;
    }

    let last_line = last_non_empty_line(content);
    if first_line == "(function() {" && last_line == "}).call(this);" {
        let score = content.lines().fold(0, |acc, line| {
            // Underscored temp vars are likely to be Coffee
            acc + count_matches(line, &["_fn", "_i", "_len", "_ref", "_results"])
                + 3 * count_matches(
                    line,
                    // bind and extend functions are very Coffee specific
                    &["__bind", "__extends", "__hasProp", "__indexOf", "__slice"],
                )
        });
        // Require a score of 3. This is fairly arbitrary. Consider tweaking later.
        // See: https://github.com/github/linguist/blob/master/lib/linguist/generated.rb#L176-L213
        score >= 3
    } else {
        false
    }
}

fn is_generated_peg_js_parser(content: &str) -> bool {
    static PEG_JS_PARSER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:[^/]|/[^\*])*/\*(?:[^\*]|\*[^/])*Generated by PEG.js")
            .expect("invalid regex")
    });

    let content = content.lines().take(5).collect::<Vec<_>>().join("");
    PEG_JS_PARSER_REGEX.is_match(&content)
}

fn is_generated_net_docfile(content: &str) -> bool {
    let lines = content.lines().take(3).collect::<Vec<_>>();
    lines.len() == 3
        && lines[1].contains("<doc>")
        && lines[2].contains("<assembly>")
        && last_non_empty_line(content).contains("</doc>")
}

fn is_generated_postscript(content: &str) -> bool {
    static POSTSCRIPT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(\n|\r\n|\r)\s*(?:currentfile eexec\s+|/sfnts\s+\[)").expect("invalid regex")
    });

    static POSTSCRIPT_REGEXS: LazyLock<RegexSet> = LazyLock::new(|| {
        RegexSet::new([
            r"[0-9]|draw|mpage|ImageMagick|inkscape|MATLAB",
            r"PCBNEW|pnmtops|\(Unknown\)|Serif Affinity|Filterimage -tops",
        ])
        .expect("invalid regex")
    });

    if POSTSCRIPT_REGEX.is_match(content) {
        return true;
    }

    let creator = content
        .lines()
        .take(10)
        .map(|line| {
            if line.starts_with("%%Creator: ") {
                Some(line)
            } else {
                None
            }
        })
        .next()
        .unwrap_or(None);

    if let Some(creator) = creator {
        if creator.contains("EAGLE")
            && content
                .lines()
                .take(5)
                .any(|line| line.starts_with("%%Title: EAGLE Drawing "))
        {
            true
        } else {
            POSTSCRIPT_REGEXS.is_match(creator)
        }
    } else {
        false
    }
}

fn is_generated_jni_header(content: &str) -> bool {
    let lines = content.lines().take(2).collect::<Vec<_>>();
    lines.len() == 2
        && lines[0].contains("/* DO NOT EDIT THIS FILE - it is machine generated */")
        && lines[1].contains("#include <jni.h>")
}

fn is_vcr_cassette(content: &str) -> bool {
    last_non_empty_line(content).contains("recorded_with: VCR")
}

fn is_generated_antlr(content: &str) -> bool {
    line_contains(content, 1, "generated by Xtest")
}

pub(crate) fn is_generated_go(content: &str) -> bool {
    any_line_contains(content, 40, "Code generated by")
}

fn is_generated_protobuf(content: &str) -> bool {
    any_line_contains(content, 20, "This file is automatically generated by")
}

fn is_compiled_cython(content: &str) -> bool {
    any_line_contains(content, 1, "Generated by Cython")
}

fn is_generated_html(content: &str) -> bool {
    let lines = take_n_lines(content, 3);

    // Pkgdown
    static PKGDOWN: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"<!-- Generated by pkgdown: do not edit by hand -->").expect("invalid regex")
    });
    if PKGDOWN.is_match(lines) {
        return true;
    }

    // Mandoc
    static MANDOC: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"<!-- This is an automatically generated file.").expect("invalid regex")
    });
    if MANDOC.is_match(lines) {
        return true;
    }

    // Now take first 30 lines of the file.
    let lines = take_n_lines(content, 30);

    // Doxygen
    static DOXYGEN: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)<!--\s+Generated by Doxygen\s+[.0-9]+\s*-->").expect("invalid regex")
    });
    if DOXYGEN.is_match(lines) {
        return true;
    }

    // HTML tag: <meta name="generator" content="…" />
    static HTML_META: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)<meta(\s+[^>]++)>").expect("invalid regex"));
    static HTML_META_CONTENT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?i)\s+(name|content|value)\s*=\s*("[^"]+"|'[^']+'|[^\s"']+)"#)
            .expect("invalid regex")
    });
    static GENERATOR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(jlatex2html|latex2html|groff|makeinfo|texi2html|ronn|(org\s+mode))")
            .expect("invalid regex")
    });

    for m in HTML_META.find_iter(lines) {
        let meta = m.as_str();
        let mut name = Default::default();
        let mut value = Default::default();
        let mut content = Default::default();
        let quotes: &[_] = &['"', '\''];
        for c in HTML_META_CONTENT.captures_iter(meta) {
            match c[1].to_lowercase().as_str() {
                "name" => name = c[2].trim_matches(quotes).to_lowercase(),
                "value" => value = c[2].to_lowercase(),
                "content" => content = c[2].to_lowercase(),
                _ => {}
            }
        }
        let val = if value.is_empty() { &content } else { &value }.trim_matches(quotes);
        if name == "generator" && GENERATOR.is_match(val) {
            return true;
        }
    }
    false
}

fn is_generated_dart(content: &str) -> bool {
    static DART_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"generated code\W{2,3}do not modify").expect("invalid regex"));

    if let Some(line) = content.lines().take(1).next() {
        DART_REGEX.is_match(line)
    } else {
        false
    }
}

fn is_generated_gamemakerstudio(content: &str) -> bool {
    static GAME_MAKER_STUDIO_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#""modelName":\s*"GM"#).expect("invalid regex"));

    static GAME_MAKER_STUDIO_REGEX2: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\d\.\d\.\d.+\|\{").expect("invalid regex"));

    let lines = content.lines().take(3).collect::<Vec<_>>();
    lines.len() == 3
        && (GAME_MAKER_STUDIO_REGEX.is_match(lines[2])
            || GAME_MAKER_STUDIO_REGEX2.is_match(lines[0]))
}

fn is_generated_gimp(content: &str) -> bool {
    static GIMP_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"/\* GIMP [a-zA-Z0-9\- ]+ C\-Source image dump \(.+?\.c\) \*/")
            .expect("invalid regex")
    });
    static GIMP_REGEX2: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"/\*  GIMP header image file format \([a-zA-Z0-9\- ]+\): .+?\.h  \*/")
            .expect("invalid regex")
    });

    content
        .lines()
        .next()
        .map(|line| GIMP_REGEX.is_match(line) || GIMP_REGEX2.is_match(line))
        .unwrap_or(false)
}

pub(crate) fn is_sorbet_rbi(content: &str) -> bool {
    static SORBET_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^# typed:").expect("invalid regex"));
    static SORBET_REGEX2: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^# Please.*run.*`.*tapioca").expect("invalid regex"));

    let lines = content.lines().take(5).collect::<Vec<_>>();
    lines.len() == 5
        && SORBET_REGEX.is_match(lines[0])
        && lines[2].contains("DO NOT EDIT MANUALLY")
        && SORBET_REGEX2.is_match(lines[4])
}

pub(crate) fn is_protoc_generated(content: &str) -> bool {
    content.lines().take(3).any(|l| {
        l.contains("Code generated by protoc")
            || l.contains("Generated by the protocol buffer compiler.")
    })
}

pub(crate) fn is_generated_unity_3d_meta(content: &str) -> bool {
    any_line_contains(content, 1, "fileFormatVersion: ")
}

fn count_matches(line: &str, patterns: &[&str]) -> usize {
    patterns
        .iter()
        .map(|pattern| line.matches(pattern).count())
        .sum()
}

/// Does the line on index `i` contain the pattern?
fn line_contains(content: &str, i: usize, pattern: &str) -> bool {
    content
        .lines()
        .nth(i)
        .is_some_and(|line| line.contains(pattern))
}

/// Does any line up to `n` contain the pattern?
fn any_line_contains(content: &str, n: usize, pattern: &str) -> bool {
    content.lines().take(n).any(|line| line.contains(pattern))
}

/// Returns the last non-empty line of some content. Checks at most 5 lines.
pub(crate) fn last_non_empty_line(content: &str) -> &str {
    content
        .lines()
        .rev()
        .take(5)
        .find(|x| !x.is_empty())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated() {
        crate::fixtures::run("generated", include_str!("../test/generated.txt"), {
            |path, content| is_generated(path, content).is_some()
        });
    }

    #[test]
    fn test_generated_by_protoc() {
        assert_eq!(
            is_generated(
                "asdf/asdf/service.rb",
                r#"
# Code generated by protoc-gen-twirp_ruby 1.9.0, DO NOT EDIT.
require 'twirp'
require_relative 'scoring_info_pb.rb'

module Blackbird
  module Query
    module V2
    end
  end
end
"#
            ),
            Some(Reason::Content(ContentReason::GeneratedByProtobufCompiler))
        )
    }

    #[test]
    fn reason_label() {
        assert_eq!(
            "content-compiledcoffeescript",
            Reason::Content(ContentReason::CompiledCoffeeScript).label()
        );
        assert_eq!(
            "path-cocapods",
            Reason::FilePath(FilePathReason::CocaPods).label()
        );
    }
}

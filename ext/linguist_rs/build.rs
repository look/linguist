use std::env;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

fn generate_languages_file(f: File) -> io::Result<()> {
    let mut f = BufWriter::new(f);

    let file = include_bytes!("src/linguist/languages.yml");
    let value: serde_yaml::Value =
        serde_yaml::from_slice(&file[..]).expect("unable to parse languages.yml");

    let data = value
        .as_mapping()
        .expect("invalid mapping in languages.yml!")
        .to_owned();

    writeln!(f, "pub type LanguageId = u32;")?;
    writeln!(f)?;

    for (key, language) in data.into_iter() {
        let language_id: u32 = language
            .get("language_id")
            .expect("languages.yml: each language must have an id")
            .as_u64()
            .expect("language_id must be a u64")
            .try_into()
            .expect("language_id must fit in a u32");

        let name = key
            .as_str()
            .expect("languages.yml: each language must have a name");

        let name = name
            .replace([' ', '\'', '.', '-'], "_")
            .replace(['#'], "sharp")
            .replace(['*'], "star")
            .replace(['+'], "plus")
            .replace(['(', ')'], "")
            .to_uppercase();

        let name = if name.starts_with(|c: char| c.is_numeric()) {
            format!("_{name}")
        } else {
            name
        };

        writeln!(f, "pub const {name}: LanguageId = {language_id};")?;
    }
    Ok(())
}

fn main() {
    let out_dir = env::var_os("OUT_DIR").expect("failed to load OUT_DIR from environment");
    let dest_path = Path::new(&out_dir).join("languages.rs");
    let f = File::create(&dest_path).unwrap_or_else(|err| {
        panic!("unable to create file {dest_path:?}: {err}");
    });
    generate_languages_file(f).unwrap_or_else(|err| {
        panic!("error writing to {dest_path:?}: {err}");
    });

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/linguist/languages.yml");

    #[cfg(feature = "c-bindings")]
    {
        println!("cargo:rerun-if-changed=cbindgen.toml");
        println!("cargo:rerun-if-changed=src/linguist_c.rs");

        // Generate C bindings.
        let crate_dir = env::var("CARGO_MANIFEST_DIR")
            .expect("build failed to load cargo manifest dir from the environment");
        cbindgen::generate(crate_dir)
            .expect("Unable to generate bindings")
            .write_to_file("include/blackbird_linguist.h");
    }
}

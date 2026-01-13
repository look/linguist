use magnus::{Error, Module, Ruby, function};


fn detect(path: String, content: String) -> Option<u64> {
    crate::detect(&path, &content).map(|l| l.language_id as u64)
}

fn is_test(path: String) -> bool {
    crate::is_test(&path)
}

fn is_documentation(path: String) -> bool {
    crate::is_documentation(&path)
}

fn is_dependency_management(path: String) -> bool {
    crate::is_dependency_management(&path)
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let linguist = ruby.define_module("Linguist")?;
    let rust = linguist.define_module("Rust")?;
    rust.define_module_function("detect", function!(detect, 2))?;
    rust.define_module_function("is_test?", function!(is_test, 1))?;
    rust.define_module_function("is_documentation?", function!(is_documentation, 1))?;
    rust.define_module_function("is_dependency_management?", function!(is_dependency_management, 1))?;
    Ok(())
}

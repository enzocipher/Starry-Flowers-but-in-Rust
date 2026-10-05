fn main() {
    println!("cargo:rerun-if-changed=../src/engine.rs");
    println!("cargo:rerun-if-changed=../src/layout.rs");
    println!("cargo:rerun-if-changed=story.json");
    let source = std::fs::read_to_string("../src/engine.rs").unwrap();
    let source = source
        .replace(
            "std::collections::HashMap",
            "alloc::collections::BTreeMap as HashMap",
        )
        .replace(
            "include_str!(\"../story.json\")",
            "include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/story.json\"))",
        );
    let imports = "use alloc::{vec,vec::Vec,format,string::{String,ToString},borrow::ToOwned};\n";
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    std::fs::write(out.join("engine.rs"), format!("{imports}{source}")).unwrap();
    let layout = std::fs::read_to_string("../src/layout.rs")
        .unwrap()
        .replace("//!", "//");
    std::fs::write(
        out.join("layout.rs"),
        format!("use alloc::{{vec,vec::Vec,string::String}};\n{layout}"),
    )
    .unwrap();
}

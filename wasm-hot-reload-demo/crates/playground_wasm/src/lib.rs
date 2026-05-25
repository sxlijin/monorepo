mod hot_reload_testdata;

pub use hot_reload_testdata::hot_reload_test_string;
use wasm_bindgen::prelude::*;

#[cfg(feature = "console_error_panic")]
extern crate console_error_panic_hook;

#[cfg(feature = "small_allocator")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(feature = "console_error_panic")]
    console_error_panic_hook::set_once();
}

/// Returns the version of the playground.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// A toy "BAML" project wrapper for WASM.
///
/// The real implementation in baml is backed by Salsa-tracked queries; this
/// demo just stores the source and finds function names with a string scan,
/// which is enough to drive the SplitPreview UI and exercise the WASM hot
/// reload pipeline.
#[wasm_bindgen]
pub struct BamlProject {
    src: String,
}

#[wasm_bindgen]
impl BamlProject {
    #[wasm_bindgen(constructor)]
    pub fn new(baml_src: String) -> BamlProject {
        BamlProject { src: baml_src }
    }

    #[wasm_bindgen]
    pub fn set_source(&mut self, baml_src: String) {
        self.src = baml_src;
    }

    #[wasm_bindgen]
    pub fn function_names(&self) -> Vec<String> {
        extract_function_names(&self.src)
    }
}

fn extract_function_names(src: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in src.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("function ") else {
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            names.push(name);
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_function_names() {
        let src = "function Foo(x: int) -> string {}\nfunction Bar() -> int {}\n";
        assert_eq!(extract_function_names(src), vec!["Foo", "Bar"]);
    }
}

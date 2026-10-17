use cfg_aliases::cfg_aliases;

// allow : l'expansion de cfg_aliases 0.2.1 émet des `;` en position d'expression (corrigé en 0.2.2).
#[allow(semicolon_in_expressions_from_non_local_macros)]
fn main() {
    // Setup cfg aliases
    cfg_aliases! {
        // Convenience aliases
        wasm_browser: { all(target_family = "wasm", target_os = "unknown") },
    }
}

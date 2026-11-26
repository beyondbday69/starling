# Fix Task: Remove Inner File Attributes from `mixins/timber_fall/runtime.rs` (`E_MODULE`)

## Goal
Fix the `horizon-mixins` `E_MODULE` error in Android APK CI run `45867275017`:
```json
{
  "code": "E_MODULE",
  "message": "support module source cannot have file attributes or a shebang",
  "mod_id": "timber_fall"
}
```

## Fix
- Remove any inner file attributes (`#![...]` AND `//!` inner doc comments, which `syn::File::attrs` treats as file attributes!) from `native-mods/timber_fall/mixins/timber_fall/runtime.rs` (convert `//!` at the top of `runtime.rs` to `//` plain comments).
- Also double-check `native-mods/timber_fall/mixins/timber_fall/*.rs` for any `#![...]` or `//!` inner attributes.
- Run `fmt --check`, `clippy --all-targets -- -D warnings`, and `cargo test` on `native-mods/timber_fall/Cargo.toml`, and update `tasks/REPORT.md`.

## Constraints
- Do not modify any files outside `native-mods/timber_fall/` (plus `tasks/REPORT.md`).

## Test Commands
```sh
cargo fmt --manifest-path native-mods/timber_fall/Cargo.toml --check
cargo clippy --manifest-path native-mods/timber_fall/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path native-mods/timber_fall/Cargo.toml
```

# Task 01 engineering plan

Capture the native UniFFI surface in `docs/api-baseline.json` before changing
source. Include all 128 functions/methods, records/enums, callback methods and
original signatures. Keep the file immutable; migration parity checks compare
its names to the generated new contract, with explicit constructor mappings.

Run the existing native tests with Rust 1.90 and the committed lockfile. Host
dependencies such as protoc must be available without committing machine paths.
Record missing tools separately from source failures. Before the migration,
retain the test result as the regression baseline and verify the inventory
counts against the previously inspected source.

Validation: native cargo test, inventory counts, baseline log and clean source
diff before starting task 03. The initial test attempt found protoc outside PATH;
retry with the installed Homebrew tools on PATH.

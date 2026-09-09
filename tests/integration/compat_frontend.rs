//! §AR-bindings.2: test-only executable access to the deprecated core process
//! adapter, so integration tests can compare it byte-for-byte with the CLI.

use std::process::ExitCode;

#[allow(deprecated)]
fn main() -> ExitCode {
    grund_core::main_entry()
}

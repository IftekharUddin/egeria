//! The `egeria` command-line interface.
//!
//! Subcommands (`import`, `check`, `compile`, `explain`, `capabilities`,
//! `graph`) arrive with milestone `V1-M3`.

fn main() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    println!("No subcommands yet — see milestone V1-M3 in the issue backlog.");
}

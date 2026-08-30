//! Repository automation. Run with `cargo xtask <task>`.
//!
//! `fetch-alloy` arrives with milestone `V1-M2`.

fn main() {
    let task = std::env::args().nth(1);
    match task.as_deref() {
        Some("help") | None => usage(),
        Some(other) => {
            eprintln!("unknown task: {other}");
            usage();
            std::process::exit(2);
        }
    }
}

fn usage() {
    eprintln!("usage: cargo xtask <task>");
    eprintln!();
    eprintln!("tasks:");
    eprintln!("  help    show this message");
}

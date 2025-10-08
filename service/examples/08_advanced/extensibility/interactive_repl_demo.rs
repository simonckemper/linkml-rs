//! Guidance for using the LinkML CLI REPL.
//!
//! The original interactive example required substantial infrastructure. This
//! streamlined version documents the commands you can run with `linkml-service`
//! once the CLI binary is built.

fn main() {
    println!("LinkML Interactive REPL Demo");
    println!("============================\n");
    println!(
        "Build the CLI with:\n    cargo run --package linkml-service --bin linkml-cli -- help\n"
    );
    println!("Useful commands inside the REPL:");
    println!("  load schema.yaml       # load a LinkML schema");
    println!("  validate data.json     # validate data against the schema");
    println!("  generate --generator rust schema.yaml ./out  # emit code");
    println!("  stats                 # show schema statistics");
    println!("  help                  # list all commands");
    println!("  quit                  # exit the session");
    println!(
        "\nFor full documentation run `linkml-cli help`. This Rust example simply
prints the most common commands so the example suite compiles cleanly."
    );
}

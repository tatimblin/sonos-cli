//! Generates shell completions and man pages.
//! Invoked by cargo-dist during release builds.

use clap::CommandFactory;
use clap_complete::generate_to;
use clap_complete::Shell;
use clap_mangen::Man;
use sonos_cli::cli::Cli;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = std::env::args()
        .skip_while(|a| a != "--out-dir")
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    // Shell completions
    let comp_dir = out_dir.join("completions");
    fs::create_dir_all(&comp_dir).unwrap();
    let mut cmd = Cli::command();
    for shell in [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::PowerShell] {
        generate_to(shell, &mut cmd, "sonos", &comp_dir).unwrap();
    }

    // Man page
    let man_dir = out_dir.join("man");
    fs::create_dir_all(&man_dir).unwrap();
    let man = Man::new(Cli::command());
    let mut buf = Vec::new();
    man.render(&mut buf).unwrap();
    fs::write(man_dir.join("sonos.1"), buf).unwrap();

    eprintln!(
        "Generated completions and man page in {}",
        out_dir.display()
    );
}

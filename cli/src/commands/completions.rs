use clap_complete::{generate, Shell};
use std::io;

pub fn run(shell: Shell, cmd: &mut clap::Command) {
    generate(shell, cmd, "planspec", &mut io::stdout());
}

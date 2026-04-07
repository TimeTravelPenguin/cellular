use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Parser, Debug)]
pub enum Command {
    Run {
        #[arg(long)]
        config: Option<PathBuf>,
    },
    Config {
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

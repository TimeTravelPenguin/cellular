
#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// The width of the simulation grid.
    #[arg(short, long, default_value_t = 100)]
    pub width: usize,

    /// The height of the simulation grid.
    #[arg(short, long, default_value_t = 100)]
    pub height: usize,
}

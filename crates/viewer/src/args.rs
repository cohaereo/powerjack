#[derive(clap::Parser)]
#[command(author, version, about)]
pub struct Args {
    /// Path to the BSP file to load
    #[clap(short, long)]
    pub bsp: String,
}

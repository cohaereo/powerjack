#[derive(clap::Parser)]
#[command(author, version, about)]
pub struct Args {
    /// Path to the BSP file to load
    #[clap(short, long)]
    pub bsp: Option<String>,

    /// Path to the MDL file to load
    #[clap(short, long)]
    pub mdl: Option<String>,

    /// Additional VPKs to mount in the virtual filesystem
    #[clap(short, long)]
    pub mount: Vec<String>,
}

use powerjack_bitbuf::ReaderExt;
use std::io::Read;

#[derive(Debug, Clone)]
pub struct DemoHeader {
    pub demo_protocol: i32,
    pub network_protocol: i32,

    /// Address/hostname of the server
    pub server_address: String,
    /// Name of the client who recorded the game
    pub client_name: String,
    /// Name of the map
    pub map_name: String,
    /// Name of the game directory
    pub game_directory: String,

    /// Playback time in seconds
    pub playback_time: f32,
    /// Number of ticks
    pub ticks: i32,
    /// Number of frames
    pub frames: i32,
    /// Length of the signon data in bytes
    pub signon_length: i32,
}

impl DemoHeader {
    pub const MAGIC: &[u8; 8] = b"HL2DEMO\0";
    pub const MAX_OSPATH: usize = 260;

    pub fn read<R: Read>(r: &mut R) -> anyhow::Result<Self> {
        let mut header = [0u8; 8];
        r.read_exact(&mut header)?;

        anyhow::ensure!(&header == Self::MAGIC, "Invalid header magic");

        let demo_protocol = r.read_i32()?;
        let network_protocol = r.read_i32()?;
        let server_address = r.read_string(Self::MAX_OSPATH)?;
        let client_name = r.read_string(Self::MAX_OSPATH)?;
        let map_name = r.read_string(Self::MAX_OSPATH)?;
        let game_directory = r.read_string(Self::MAX_OSPATH)?;
        let playback_time = r.read_f32()?;
        let ticks = r.read_i32()?;
        let frames = r.read_i32()?;
        let signon_length = r.read_i32()?;

        Ok(Self {
            demo_protocol,
            network_protocol,
            server_address,
            client_name,
            map_name,
            game_directory,
            playback_time,
            ticks,
            frames,
            signon_length,
        })
    }
}

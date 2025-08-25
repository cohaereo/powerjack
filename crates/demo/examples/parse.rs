use std::io::Seek;

use chroma_dbg::ChromaDebug;
use powerjack_demo::{Command, DemoHeader, usercmd::UserCmd};

fn main() -> anyhow::Result<()> {
    let filename = std::env::args().nth(1).expect("Usage: parse <file.dem>");
    let file = std::fs::File::open(&filename)?;
    let mut reader = std::io::BufReader::new(file);

    reader.seek(std::io::SeekFrom::End(0))?;
    let file_size = reader.stream_position()?;
    reader.seek(std::io::SeekFrom::Start(0))?;

    let header = DemoHeader::read(&mut reader)?;
    println!("{}", header.dbg_chroma());

    let mut usercmd = UserCmd::default();
    while (reader.stream_position()? + 1 + 4) < file_size {
        // let offset = reader.stream_position()?;
        // print!("0x{:08X} ", offset);
        let (tick, command) = Command::read(&mut reader, &header, &usercmd)?;
        match &command {
            Command::Packet(info, sequence, data) => {
                println!("Tick: {tick}, Packet(/* {} bytes */)", data.len())
            }
            Command::UserCmd(outgoing_sequence, cmd) => {
                // println!("UserCmd({}, {})", outgoing_sequence, chroma.format(cmd));
                usercmd = cmd.clone();
            }
            _ => println!("Tick: {tick}, Command: {command:?}"),
        };
        if let Command::Stop = command {
            break;
        }
    }

    Ok(())
}

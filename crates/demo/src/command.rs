use glam::Vec3;

use crate::{DemoHeader, stringtables::StringTable, usercmd::UserCmd};
use powerjack_bitbuf::{BitReader, ReaderExt};
use std::{
    fmt::Debug,
    io::{Read, Seek},
};

pub enum Command {
    SignOn(Vec<u8>),
    Packet(CmdInfo, SequenceInfo, Vec<u8>),
    SyncTick,
    ConsoleCmd(String),
    UserCmd(i32, UserCmd),
    DataTables(Vec<u8>),
    Stop,
    StringTables(Vec<StringTable>),
}

impl Command {
    pub const ID_SIGNON: u8 = 1;
    pub const ID_PACKET: u8 = 2;
    pub const ID_SYNCTICK: u8 = 3;
    pub const ID_CONSOLECMD: u8 = 4;
    pub const ID_USERCMD: u8 = 5;
    pub const ID_DATATABLES: u8 = 6;
    pub const ID_STOP: u8 = 7;
    pub const ID_STRINGTABLES: u8 = 8;

    /// Reads a command from the given reader.
    ///
    /// Returns (tick, command data)
    pub fn read<R: Read + Seek>(
        r: &mut R,
        header: &DemoHeader,
        usercmd: &UserCmd,
    ) -> anyhow::Result<(u32, Self)> {
        let cmd = r.read_u8()?;
        let tick = r.read_u32()?;

        let data = match cmd {
            Self::ID_SIGNON => Self::SignOn(r.read_bytes(header.signon_length as usize)?),
            Self::ID_PACKET => {
                let info = CmdInfo::read(r)?;
                let sequence = SequenceInfo::read(r)?;
                let len = r.read_u32()?;
                Self::Packet(info, sequence, r.read_bytes(len as usize)?)
            }
            Self::ID_SYNCTICK => Self::SyncTick,
            Self::ID_CONSOLECMD => {
                let len = r.read_u32()?;
                Self::ConsoleCmd(r.read_string(len as usize)?)
            }
            Self::ID_USERCMD => {
                let outgoing_sequence = r.read_i32()?;
                let len = r.read_u32()?;
                let data = r.read_bytes(len as usize)?;
                Self::UserCmd(
                    outgoing_sequence,
                    UserCmd::read(&mut BitReader::new(data), usercmd)?,
                )
            }
            // Self::ID_DATATABLES => Ok((tick, Self::DataTables(read_bytes(r)?))),
            Self::ID_STOP => Self::Stop,
            Self::ID_STRINGTABLES => {
                let len = r.read_u32()?;
                let data = r.read_bytes(len as usize)?;
                let mut br = BitReader::new(data);
                let num_tables = br.read_u8()?;
                let mut tables = Vec::with_capacity(num_tables as usize);
                for _ in 0..num_tables {
                    tables.push(StringTable::read(&mut br)?);
                }
                Self::StringTables(tables)
            }
            _ => anyhow::bail!("Unknown command ID: {} @ 0x{:X}", cmd, r.stream_position()?),
        };

        Ok((tick, data))
    }
}

impl Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::SignOn(data) => write!(f, "SignOn(/* {} bytes */)", data.len()),
            Command::Packet(info, sequence, data) => write!(
                f,
                "Packet({:?}, {:?}, /* {} bytes */)",
                info,
                sequence,
                data.len()
            ),
            Command::SyncTick => f.write_str("SyncTick"),
            Command::ConsoleCmd(cmd) => write!(f, "ConsoleCmd({})", cmd),
            Command::UserCmd(outgoing_sequence, cmd) => {
                write!(f, "UserCmd({}, {:X?})", outgoing_sequence, cmd)
            }
            Command::DataTables(items) => write!(f, "DataTables(/* {} bytes */)", items.len()),
            Command::Stop => f.write_str("Stop"),
            Command::StringTables(items) => write!(f, "StringTables(/* {} tables */)", items.len()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CmdInfo {
    pub flags: u32,

    pub view_origin: Vec3,
    pub view_angles: Vec3,
    pub local_view_angles: Vec3,

    pub view_origin2: Vec3,
    pub view_angles2: Vec3,
    pub local_view_angles2: Vec3,
}

impl CmdInfo {
    pub fn read<R: Read>(r: &mut R) -> std::io::Result<Self> {
        let flags = r.read_u32()?;
        let view_origin = r.read_vec3()?;
        let view_angles = r.read_vec3()?;
        let local_view_angles = r.read_vec3()?;
        let view_origin2 = r.read_vec3()?;
        let view_angles2 = r.read_vec3()?;
        let local_view_angles2 = r.read_vec3()?;

        Ok(Self {
            flags,
            view_origin,
            view_angles,
            local_view_angles,
            view_origin2,
            view_angles2,
            local_view_angles2,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SequenceInfo {
    pub seq_nr_in: i32,
    pub seq_nr_out: i32,
}

impl SequenceInfo {
    pub fn read<R: Read>(r: &mut R) -> std::io::Result<Self> {
        let seq_nr_in = r.read_i32()?;
        let seq_nr_out = r.read_i32()?;

        Ok(Self {
            seq_nr_in,
            seq_nr_out,
        })
    }
}

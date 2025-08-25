use powerjack_bitbuf::{BitReader, ReaderExt};

pub struct StringTable {
    pub name: String,
    pub entries: Vec<StringTableString>,
}

impl StringTable {
    pub fn read(br: &mut BitReader) -> std::io::Result<Self> {
        let name = br.read_nullstring()?;
        let mut table = Self {
            name,
            entries: vec![],
        };

        let num_strings = br.read_u16()?;
        for _ in 0..num_strings {
            let string = br.read_nullstring()?;
            if br.read_bit() {
                let userdata_size = br.read_u16()?;
                let userdata = br.read_bytes(userdata_size as usize)?;
                table.entries.push(StringTableString {
                    is_server: true,
                    string,
                    userdata: Some(userdata),
                });
            } else {
                table.entries.push(StringTableString {
                    is_server: false,
                    string,
                    userdata: None,
                });
            }
        }

        // client-side stuff
        if br.read_bit() {
            let num_strings = br.read_u16()?;
            for _ in 0..num_strings {
                let string = br.read_nullstring()?;
                if br.read_bit() {
                    let userdata_size = br.read_u16()?;
                    let userdata = br.read_bytes(userdata_size as usize)?;
                    table.entries.push(StringTableString {
                        is_server: false,
                        string,
                        userdata: Some(userdata),
                    });
                } else {
                    table.entries.push(StringTableString {
                        is_server: false,
                        string,
                        userdata: None,
                    });
                }
            }
        }

        Ok(table)
    }
}

pub struct StringTableString {
    pub is_server: bool,
    pub string: String,
    pub userdata: Option<Vec<u8>>,
}

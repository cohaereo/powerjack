use bitflags::bitflags;
use glam::{I16Vec2, Vec3};

use crate::{bitreader::BitReader, reader::ReaderExt};

#[derive(Default, Debug, Clone)]
pub struct UserCmd {
    pub command_number: u32,
    pub tick_count: u32,

    pub view_angles: Vec3,
    pub movement: Vec3,

    pub buttons: Buttons,
    pub impulse: u8,

    pub weapon_select: u32,
    pub weapon_subtype: u32,

    pub mouse_delta: I16Vec2,
}

impl UserCmd {
    /// Reads a delta-compressed user command
    pub fn read(br: &mut BitReader, from: &UserCmd) -> std::io::Result<Self> {
        let mut cmd = from.clone();
        // println!("from {from:X?}");

        if br.read_bit() {
            cmd.command_number = br.read_u32()?;
        } else {
            cmd.command_number += 1;
        }

        if br.read_bit() {
            cmd.tick_count = br.read_u32()?;
        } else {
            cmd.tick_count += 1;
        }

        if br.read_bit() {
            cmd.view_angles[0] = br.read_f32()?;
        }
        if br.read_bit() {
            cmd.view_angles[1] = br.read_f32()?;
        }
        if br.read_bit() {
            cmd.view_angles[2] = br.read_f32()?;
        }

        if br.read_bit() {
            cmd.movement[0] = br.read_f32()?;
        }
        if br.read_bit() {
            cmd.movement[1] = br.read_f32()?;
        }
        if br.read_bit() {
            cmd.movement[2] = br.read_f32()?;
        }

        if br.read_bit() {
            cmd.buttons = Buttons::from_bits_truncate(br.read_u32()?);
        }

        if br.read_bit() {
            cmd.impulse = br.read_u8()?;
        }

        if br.read_bit() {
            cmd.weapon_select = br.read_bits(11); // MAX_EDICT_BITS
            if br.read_bit() {
                cmd.weapon_subtype = br.read_bits(6); // WEAPON_SUBTYPE_BITS
            }
        }

        if br.read_bit() {
            cmd.mouse_delta[0] = br.read_i16()?;
        }

        if br.read_bit() {
            cmd.mouse_delta[1] = br.read_i16()?;
        }
        // println!("to {cmd:X?}");
        // println!("remaining bytes: {:?}", br.remaining_bytes());

        Ok(cmd)
    }
}

bitflags! {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Buttons : u32 {
        const ATTACK     = (1 << 0);
        const JUMP       = (1 << 1);
        const DUCK       = (1 << 2);
        const FORWARD    = (1 << 3);
        const BACK       = (1 << 4);
        const USE        = (1 << 5);
        const CANCEL     = (1 << 6);
        const LEFT       = (1 << 7);
        const RIGHT      = (1 << 8);
        const MOVELEFT   = (1 << 9);
        const MOVERIGHT  = (1 << 10);
        const ATTACK2    = (1 << 11);
        const RUN        = (1 << 12);
        const RELOAD     = (1 << 13);
        const ALT1       = (1 << 14);
        const ALT2       = (1 << 15);
        const SCORE      = (1 << 16);
        const SPEED      = (1 << 17);
        const WALK       = (1 << 18);
        const ZOOM       = (1 << 19);
        const WEAPON1    = (1 << 20);
        const WEAPON2    = (1 << 21);
        const BULLRUSH   = (1 << 22);
        const GRENADE1   = (1 << 23);
        const GRENADE2   = (1 << 24);
        const ATTACK3    = (1 << 25);
    }
}

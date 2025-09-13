use std::io::Seek;

use chroma_dbg::ChromaDebug;
use powerjack_bitbuf::{BitReader, ReaderExt};
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
                println!(
                    "Tick: {tick}, SeqIn: {}, SeqOut: {}, {} bytes",
                    sequence.seq_nr_in,
                    sequence.seq_nr_out,
                    data.len()
                );
                let mut br = BitReader::new(data.clone());
                while br.bits_remaining() > 6 {
                    let cmd = br.read_bits(6);
                    println!("  cmd: {cmd}");
                    match cmd {
                        0 => {
                            // net_NOP
                            println!("    net_NOP");
                        }
                        3 => {
                            // net_Tick
                            let tick = br.read_u32()?;
                            const NET_TICK_SCALEUP: f32 = 100000.0;
                            let host_frame_time = br.read_bits(16) as f32 / NET_TICK_SCALEUP;
                            let host_frame_time_std_dev =
                                br.read_bits(16) as f32 / NET_TICK_SCALEUP;
                            println!(
                                "    net_Tick: {}, host_frame_time: {host_frame_time}, host_frame_time_std_dev: {host_frame_time_std_dev}",
                                tick
                            );
                        }
                        4 => {
                            // net_StringCmd
                            let command = br.read_nullstring()?;
                            println!("    net_StringCmd: {command}");
                        }
                        17 => {
                            // svc_Sounds
                            let reliable_sound = br.read_bit();
                            let (num_sounds, length) = if reliable_sound {
                                (1, br.read_bits(8))
                            } else {
                                (br.read_bits(8), br.read_bits(16))
                            };
                            let data = br.read_bits_vec(length as usize);
                            println!(
                                "    svc_Sounds: reliable_sound: {}, num_sounds: {}, length: {}, data: {:02X?}",
                                reliable_sound, num_sounds, length, data
                            );
                        }
                        19 => {
                            // svc_FixAngle
                            let relative = br.read_bit();
                            let angle_x = br.read_angle(16);
                            let angle_y = br.read_angle(16);
                            let angle_z = br.read_angle(16);
                            println!(
                                "    svc_FixAngle: relative: {}, angle_x: {}, angle_y: {}, angle_z: {}",
                                relative, angle_x, angle_y, angle_z
                            );
                        }
                        21 => {
                            // svc_BSPDecal
                            let pos = br.read_vec3_compressed();
                            let decal_texture_index = br.read_bits(9);
                            let (entity_index, model_index) = if br.read_bit() {
                                (Some(br.read_bits(11)), Some(br.read_bits(11)))
                            } else {
                                (None, None)
                            };
                            let low_priority = br.read_bit();
                            println!(
                                "    svc_BSPDecal: pos: {}, decal_texture_index: {}, entity_index: {:?}, model_index: {:?}, low_priority: {}",
                                pos, decal_texture_index, entity_index, model_index, low_priority
                            );
                        }
                        23 => {
                            // svc_UserMessage
                            let msg_type = br.read_bits(8);
                            let length = br.read_bits(11);
                            let data = br.read_bits_vec(length as usize);

                            let type_name = match msg_type {
                                0 => "Geiger",
                                1 => "Train",
                                2 => "HudText",
                                3 => "SayText",
                                4 => "SayText2",
                                5 => "TextMsg",
                                6 => "ResetHUD",
                                7 => "GameTitle",
                                8 => "ItemPickup",
                                9 => "ShowMenu",
                                10 => "Shake",
                                11 => "Fade",
                                12 => "VGuiMenu",
                                13 => "Rumble",
                                14 => "CloseCaption",
                                15 => "SendAudio",
                                16 => "VoiceMask",
                                17 => "RequestState",
                                18 => "Damage",
                                19 => "HintText",
                                20 => "KeyHintText",
                                21 => "HudMsg",
                                22 => "AmmoDenied",
                                23 => "AchievementEvent",
                                24 => "UpdateRadar",
                                25 => "VoiceSubtitle",
                                26 => "HudNotify",
                                27 => "HudNotifyCustom",
                                28 => "PlayerStatsUpdate",
                                29 => "PlayerIgnited",
                                30 => "PlayerIgnitedInv",
                                31 => "HudArenaNotify",
                                32 => "UpdateAchievement",
                                33 => "TrainingMsg",
                                34 => "TrainingObjective",
                                35 => "DamageDodged",
                                36 => "PlayerJarated",
                                37 => "PlayerExtinguished",
                                38 => "PlayerJaratedFade",
                                39 => "PlayerShieldBlocked",
                                40 => "BreakModel",
                                41 => "CheapBreakModel",
                                42 => "BreakModelPumpkin",
                                43 => "BreakModelRocketDud",
                                44 => "CallVoteFailed",
                                45 => "VoteStart",
                                46 => "VotePass",
                                47 => "VoteFailed",
                                48 => "VoteSetup",
                                49 => "PlayerBonusPoints",
                                50 => "SpawnFlyingBird",
                                51 => "PlayerGodRayEffect",
                                52 => "SPHapWeapEvent",
                                53 => "HapDmg",
                                54 => "HapPunch",
                                55 => "HapSetDrag",
                                56 => "HapSet",
                                57 => "HapMeleeContact",
                                _ => "<unknown>",
                            };

                            println!(
                                "    svc_UserMessage: msg_type: {} ({type_name}), length: {}, data: {:02X?}",
                                msg_type, length, data
                            );
                        }
                        25 => {
                            // svc_GameEvent
                            let length = br.read_bits(11);
                            let data = br.read_bits_vec(length as usize);
                            println!("    svc_GameEvent: length: {}, data: {:02X?}", length, data);
                        }
                        26 => {
                            // svc_PacketEntities
                            let max_entries = br.read_bits(11);
                            let is_delta = br.read_bit();
                            let delta_from = is_delta.then(|| br.read_bits(32));
                            let baseline = br.read_bits(1);
                            let updated_entries = br.read_bits(11);
                            let length = br.read_bits(20);
                            let update_baseline = br.read_bit();
                            let data = br.read_bits_vec(length as usize);
                            println!(
                                "    svc_PacketEntities: max_entries: {}, is_delta: {}, delta_from: {:?}, baseline: {}, updated_entries: {}, length: {}, update_baseline: {}",
                                max_entries,
                                is_delta,
                                delta_from,
                                baseline,
                                updated_entries,
                                length,
                                update_baseline
                            );
                        }
                        27 => {
                            // svc_TempEntities
                            let num_entries = br.read_bits(8);
                            let length = br.read_varint32();
                            let data = br.read_bits_vec(length as usize);
                            println!(
                                "    svc_TempEntities: num_entries: {}, length: {}, data: {:02X?}",
                                num_entries, length, data
                            );
                        }
                        28 => {
                            // svc_Prefetch
                            let sound_index = br.read_bits(14);
                            println!("    svc_Prefetch: sound_index: {}", sound_index);
                        }
                        _ => {
                            println!("Unhandled cmd: {cmd}");
                            break;
                        }
                    }
                }
                println!();
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

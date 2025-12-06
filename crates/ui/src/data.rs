use std::path::{Path, PathBuf};

use chroma_dbg::ChromaDebug;
use chrono::TimeZone;
use powerjack_demo::DemoHeader;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use tf_demo_parser::demo::parser::gamestateanalyser::Class as TfDemoClass;
use tf_demo_parser::{Demo, DemoParser};

use crate::analyzer::DemoAnalyzer;

#[derive(Clone)]
pub struct DemoData {
    pub header: DemoHeader,
    pub complete: bool,

    pub path: PathBuf,
    pub attributes: Vec<Attribute>,
    pub date: chrono::DateTime<chrono::Local>,
    pub kd_ratio: String,
    pub map_name: String,
    pub num_friends: usize,
    pub primary_class: Class,
    pub time_played: String,
}

#[derive(Debug, Clone, Copy)]
pub enum AttributeKind {
    Excellent,
    Positive,
    Informative,
    Negative,
}

#[derive(Clone)]
pub struct Attribute {
    pub kind: AttributeKind,
    pub text: String,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Class {
    Scout,
    Soldier,
    Pyro,

    Demoman,
    Heavy,
    Engineer,

    Medic,
    Sniper,
    Spy,

    Other,
}

impl Class {
    pub fn from_int(value: u16) -> Self {
        match value {
            1 => Class::Scout,
            2 => Class::Soldier,
            3 => Class::Pyro,
            4 => Class::Demoman,
            5 => Class::Heavy,
            6 => Class::Engineer,
            7 => Class::Medic,
            8 => Class::Sniper,
            9 => Class::Spy,
            _ => Class::Other,
        }
    }
}

impl From<TfDemoClass> for Class {
    fn from(value: TfDemoClass) -> Self {
        match value {
            TfDemoClass::Scout => Class::Scout,
            TfDemoClass::Soldier => Class::Soldier,
            TfDemoClass::Pyro => Class::Pyro,
            TfDemoClass::Demoman => Class::Demoman,
            TfDemoClass::Heavy => Class::Heavy,
            TfDemoClass::Engineer => Class::Engineer,
            TfDemoClass::Medic => Class::Medic,
            TfDemoClass::Sniper => Class::Sniper,
            TfDemoClass::Spy => Class::Spy,
            TfDemoClass::Other => Class::Other,
        }
    }
}

pub fn find_demos() -> Vec<DemoData> {
    let path = Path::new(
        "/run/media/luca/Deep Stone Crypt/Steam/steamapps/common/Team Fortress 2/tf/demos/",
    );
    let mut demos = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("dem") {
                continue;
            }

            let mut file = match std::fs::File::open(&path) {
                Ok(f) => f,
                Err(_) => continue,
            };

            let date = match path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(parse_datetime_from_filename)
            {
                Some(dt) => dt,
                None => std::fs::metadata(&path)
                    .and_then(|meta| meta.modified())
                    .ok()
                    .map(|time| {
                        let datetime: chrono::DateTime<chrono::Local> = time.into();
                        datetime
                    })
                    .unwrap_or_else(chrono::Local::now),
            };

            // let now = chrono::Local::now();
            // if now - date > chrono::Duration::days(2) {
            //     // skip demos older than a week
            //     continue;
            // }

            match DemoHeader::read(&mut file) {
                Ok(header) => {
                    demos.push(DemoData {
                        complete: false,
                        path: path.clone(),
                        attributes: vec![],
                        date,
                        kd_ratio: "".to_string(),
                        map_name: header.map_name.clone(),
                        num_friends: 0,
                        primary_class: Class::Other,
                        time_played: format_time(header.playback_time),
                        header,
                    });
                }
                Err(_) => continue,
            }
        }
    }

    demos
}

/// Parses standard demo filename, filename is without extension, eg `2025-08-21_20-31-25`
fn parse_datetime_from_filename(filename: &str) -> Option<chrono::DateTime<chrono::Local>> {
    let datetime = chrono::NaiveDateTime::parse_from_str(filename, "%Y-%m-%d_%H-%M-%S").ok()?;
    let local_datetime = chrono::Local.from_local_datetime(&datetime).single()?;
    Some(local_datetime)
}

fn format_time(seconds: f32) -> String {
    if seconds < 60.0 {
        format!("{:.0}s", seconds)
    } else if seconds < 3600.0 {
        format!("{:.0}m", (seconds / 60.0).round())
    } else {
        format!(
            "{:.0}h {:.0}m",
            (seconds / 3600.0).floor(),
            ((seconds % 3600.0) / 60.0).floor()
        )
    }
}

pub fn populate_demo_data(data: &mut DemoData) {
    let demo_raw = std::fs::read(&data.path).expect("Failed to read demo file");
    let demo = Demo::new(&demo_raw);
    let parser = DemoParser::new_all_with_analyser(demo.get_stream(), DemoAnalyzer::default());
    let (_header, state) = parser.parse().expect("Failed to parse demo");

    let Some((_, player)) = state
        .players
        .iter()
        .find(|(_, u)| u.name == data.header.client_name)
    else {
        println!(
            "Could not find user info for player {}",
            data.header.client_name
        );
        return;
    };

    data.primary_class = player.most_played_class();

    let kd = if player.deaths > 0 {
        player.kills as f32 / player.deaths as f32
    } else {
        player.kills as f32
    };
    data.kd_ratio = format!("{:.1} K/D", kd);

    if player.deaths == 0 {
        data.attributes.push(Attribute {
            kind: AttributeKind::Excellent,
            text: "No Deaths".to_string(),
        });
    }

    if kd >= 5.0 {
        data.attributes.push(Attribute {
            kind: AttributeKind::Excellent,
            text: "High K/D Ratio".to_string(),
        });
    } else if kd >= 3.0 {
        data.attributes.push(Attribute {
            kind: AttributeKind::Positive,
            text: "Good K/D Ratio".to_string(),
        });
    } else if kd < 1.0 {
        data.attributes.push(Attribute {
            kind: AttributeKind::Negative,
            text: "Low K/D Ratio".to_string(),
        });
    }

    if data.header.playback_time < 60.0 * 2.0 {
        // Attributes for very short games are irrelevant
        data.attributes.clear();
        data.attributes.push(Attribute {
            kind: AttributeKind::Informative,
            text: "Short Game".to_string(),
        });
    }

    if state
        .players
        .iter()
        .filter(|(_, p)| p.user_id != player.user_id)
        .count()
        == 0
    {
        data.attributes.clear();
        data.attributes.push(Attribute {
            kind: AttributeKind::Informative,
            text: "Solo/Bot Match".to_string(),
        });
    }

    data.complete = true;
}

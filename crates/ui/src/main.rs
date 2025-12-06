#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::{Datelike, TimeZone};
use eframe::egui::{self, Color32, CornerRadius, Rect, RichText, Vec2, Widget, pos2, vec2};
use powerjack_demo::DemoHeader;
use std::{collections::HashMap, path::Path};

fn main() {
    let native_options = eframe::NativeOptions {
        window_builder: Some(Box::new(|viewport_builder| {
            viewport_builder.with_inner_size([1600.0, 900.0])
        })),
        ..Default::default()
    };
    eframe::run_native(
        "Powerjack",
        native_options,
        Box::new(|cc| Ok(Box::new(PowerjackApp::new(cc)))),
    )
    .expect("Failed to start eframe");
}

#[derive(Default)]
struct PowerjackApp {
    demos: Vec<DemoData>,
}

impl PowerjackApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut demos = find_demos();
        // Sort by date descending
        demos.sort_by(|a, b| b.date.cmp(&a.date));

        egui_extras::install_image_loaders(&cc.egui_ctx);
        Self { demos }
    }
}

impl eframe::App for PowerjackApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.style_mut(|s| {
            s.interaction.selectable_labels = false;
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Powerjack Demo Viewer");

            egui::ScrollArea::new([false, true]).auto_shrink([false, false]).show(ui, |ui| {
                const COLUMN_WIDTH: f32 = 384.0;
                let total_width = ui.available_width();
                let num_columns = (total_width / COLUMN_WIDTH).floor() as usize;
                let spacing_per_column = (total_width - (num_columns as f32 * COLUMN_WIDTH)) / (num_columns as f32);

                egui::Grid::new("le grid")
                    .spacing(egui::vec2(spacing_per_column, 32.0))
                    .show(ui, |ui| {

                        for (i, demo) in self.demos.iter().enumerate() {
                            let map_image_path = format!("images/maps/{}.webp", demo.map_name);
                            let map_image = if std::fs::exists(&map_image_path).ok() == Some(true) {
                                format!("file://{}", map_image_path)
                            } else {
                                "file://images/maps/unknown.webp".to_string()
                            };
                            ui.allocate_ui(vec2(384.0, 214.0), |ui| {
                                ui.vertical(|ui| {
                                    ui.style_mut().spacing.item_spacing = Vec2::ZERO;
                                    let img_rect = egui::Image::new(map_image)
                                        .corner_radius(
                                            CornerRadius {
                                                ne: 16,
                                                nw: 16,
                                                ..Default::default()
                                            },)
                                        .ui(ui)
                                        .rect;

                                    let subtitle_rect = Rect {
                                        min: pos2(img_rect.min.x, img_rect.max.y),
                                        max: img_rect.max + vec2(0.0, 48.0)
                                    };

                                    ui.painter().rect_filled(subtitle_rect,
                                        CornerRadius {
                                            se: 16,
                                            sw: 16,
                                            ..Default::default()
                                        }, Color32::from_gray(48));

                                    ui.allocate_rect(subtitle_rect, egui::Sense::hover());
                                    ui.scope_builder(egui::UiBuilder::default().max_rect(subtitle_rect.shrink2(vec2(16.0, 14.0))), |ui| {
                                        ui.horizontal(|ui| {

                                        ui.strong(RichText::new(&demo.map_name).color(Color32::WHITE).size(18.0));

                                            ui.vertical(|ui| {
                                            ui.add_space(2.0);
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {

                                            ui.label(
                                                RichText::new(format_date_short(&demo.date))
                                                    .color(Color32::from_gray(162))
                                                    .italics()
                                                    .font(egui::FontId::proportional(14.0)),
                                            );
                                        });
                                            });
                                        });
                                    });

                                    ui.painter().rect_filled(
                                        img_rect,
                                        CornerRadius {
                                            ne: 16,
                                            nw: 16,
                                            ..Default::default()
                                        },
                                        egui::Color32::from_black_alpha(100),
                                    );

                                    ui.scope_builder(
                                        egui::UiBuilder::default().max_rect(img_rect),
                                        |ui| {
                                            ui.add_space(8.0);

                                            ui.horizontal_top(|ui| {
                                                ui.add_space(12.0);
                                                let class_name = format!("{:?}", demo.primary_class).to_lowercase();
                                                let class_image_path = format!("file://images/classes/{}.png", class_name);
                                                ui.image(class_image_path);
                                            });
                                        });


                                    let img_text_rect = img_rect.shrink2(vec2(16.0, 16.0)).with_min_y(img_rect.min.y + 8.0);
                                    ui.scope_builder(
                                        egui::UiBuilder::default().max_rect(img_text_rect),
                                        |ui| {
                                            ui.add_space(8.0);

                                            ui.horizontal_top(|ui| {
                                                // Use vertical layout with spacing to push content
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                                    ui.vertical(|ui| {
                                                        // Top-right stats
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                                            ui.label(
                                                                RichText::new(format!("🕑 {}", demo.time_played))
                                                                    .color(Color32::WHITE)
                                                                    .font(egui::FontId::proportional(24.0)),
                                                            );
                                                        });
                                                        if !demo.kd_ratio.is_empty() {
                                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                                                ui.label(
                                                                    RichText::new(format!("🎯 {}", demo.kd_ratio))
                                                                        .color(Color32::WHITE)
                                                                        .font(egui::FontId::proportional(24.0)),
                                                                );
                                                            });
                                                        }
                                                        if demo.num_friends > 0 {
                                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                                                ui.label(
                                                                    RichText::new(format!("👥 {} friends", demo.num_friends))
                                                                        .color(Color32::from_rgb(64, 255, 64))
                                                                        .font(egui::FontId::proportional(24.0)),
                                                                );
                                                            });
                                                        }

                                                        // Add flexible space to push attributes to bottom
                                                        ui.add_space(ui.available_height() - (demo.attributes.len() as f32 * 20.0));

                                                        // Bottom-right attributes
                                                        for attr in &demo.attributes {
                                                            let (prefix, color) = match attr.kind {
                                                                AttributeKind::Excellent => ("⭐", Color32::GOLD),
                                                                AttributeKind::Positive => ("👍", Color32::LIGHT_GREEN),
                                                                AttributeKind::Negative => ("👎", Color32::LIGHT_RED),
                                                            };
                                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                                                ui.label(
                                                                    RichText::new(format!("{} {}", prefix, attr.text))
                                                                        .color(color)
                                                                        .font(egui::FontId::proportional(16.0)).strong(),
                                                                );
                                                            });
                                                        }
                                                    });
                                                });
                                            });
                                        },
                                    );
                                });
                            });

                            if (i + 1) % num_columns == 0 {
                                ui.end_row();
                            }
                        }
                    });
            });
        });
    }
}

#[derive(Clone)]
struct DemoData {
    pub attributes: Vec<Attribute>,
    pub date: chrono::DateTime<chrono::Local>,
    pub kd_ratio: String,
    pub map_name: String,
    pub num_friends: usize,
    pub primary_class: Class,
    pub time_played: String,
}

#[derive(Debug, Clone, Copy)]
enum AttributeKind {
    Excellent,
    Positive,
    Negative,
}

#[derive(Clone)]
struct Attribute {
    pub kind: AttributeKind,
    pub text: String,
}

#[derive(Debug, Clone, Copy)]
enum Class {
    Scout,
    Soldier,
    Pyro,

    Demoman,
    Heavy,
    Engineer,

    Medic,
    Sniper,
    Spy,

    Random,
}

fn find_demos() -> Vec<DemoData> {
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

            match DemoHeader::read(&mut file) {
                Ok(header) => {
                    demos.push(DemoData {
                        attributes: vec![],
                        date,
                        kd_ratio: "".to_string(),
                        map_name: header.map_name,
                        num_friends: 0,
                        primary_class: Class::Soldier,
                        time_played: format_time(header.playback_time),
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

/// Formats date in short form, e.g. `Aug 21, 2025`, or `Aug 21` if the year is the current year.
/// When the date is in the past month, returns `DD days ago`.
/// When the date is today, returns `Today`.
fn format_date_short(dt: &chrono::DateTime<chrono::Local>) -> String {
    let now = chrono::Local::now();
    let duration = now.signed_duration_since(*dt);

    if duration.num_days() <= 1 {
        if dt.date_naive() == now.date_naive() {
            "Today".to_string()
        } else {
            "Yesterday".to_string()
        }
    } else if duration.num_days() < 30 {
        format!(
            "{} day{} ago",
            duration.num_days(),
            if duration.num_days() > 1 { "s" } else { "" }
        )
    } else if dt.year() == now.year() {
        dt.format("%b %d").to_string()
    } else {
        dt.format("%b %d, %Y").to_string()
    }
}

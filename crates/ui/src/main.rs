#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, Color32, CornerRadius, RichText, Widget, vec2};
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
    );
}

#[derive(Default)]
struct PowerjackApp {
    demos: Vec<DemoData>,
}

impl PowerjackApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let demos = find_demos();
        egui_extras::install_image_loaders(&cc.egui_ctx);
        Self { demos }
    }
}

impl eframe::App for PowerjackApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Powerjack Demo Viewer");

            egui::ScrollArea::new([false, true]).show(ui, |ui| {
                const COLUMN_WIDTH: f32 = 384.0;
                let total_width = ui.available_width();
                let num_columns = (total_width / COLUMN_WIDTH).floor() as usize;
                let spacing_per_column = (total_width - (num_columns as f32 * COLUMN_WIDTH)) / (num_columns as f32);

                egui::Grid::new("le grid")
                    .max_col_width(512.0)
                    .spacing(egui::vec2(spacing_per_column, 32.0))
                    .show(ui, |ui| {

                        for (i, demo) in self.demos.iter().enumerate() {
                            let map_image_path = format!("images/maps/{}.jpg", demo.map_name);
                            let map_image = if std::fs::exists(&map_image_path).ok() == Some(true) {
                                format!("file://{}", map_image_path)
                            } else {
                                "file://images/maps/unknown.jpg".to_string()
                            };
                            ui.allocate_ui(vec2(384.0, 214.0), |ui| {
                                ui.vertical(|ui| {
                                    let img_rect = egui::Image::new(map_image)
                                        .corner_radius(CornerRadius::same(16))
                                        .ui(ui)
                                        .rect;

                                    ui.painter().rect_filled(
                                        img_rect,
                                        CornerRadius::same(16),
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


                                    ui.scope_builder(
                                        egui::UiBuilder::default().max_rect(img_rect.shrink(12.0)),
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
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                                            ui.label(
                                                                RichText::new(format!("🎯 {}", demo.kd_ratio))
                                                                    .color(Color32::WHITE)
                                                                    .font(egui::FontId::proportional(24.0)),
                                                            );
                                                        });
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
    pub date: String,
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

            match DemoHeader::read(&mut file) {
                Ok(header) => {
                    demos.push(DemoData {
                        attributes: vec![
                            Attribute {
                                kind: AttributeKind::Excellent,
                                text: "No Deaths".to_string(),
                            },
                            Attribute {
                                kind: AttributeKind::Positive,
                                text: "MVP #3".to_string(),
                            },
                        ],
                        date: "Today".to_string(),
                        kd_ratio: "2.5".to_string(),
                        map_name: header.map_name,
                        num_friends: 1,
                        primary_class: Class::Random,
                        time_played: format_time(header.playback_time),
                    });
                }
                Err(_) => continue,
            }
        }
    }

    demos
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

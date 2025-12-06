#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod analyzer;
mod data;

use std::{hash::Hash, ops::Range};

use chrono::Datelike;
use eframe::egui::{
    self, Color32, CornerRadius, Rect, RichText, Sense, Spinner, Vec2, Widget, pos2, vec2,
};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::data::{AttributeKind, DemoData, find_demos, populate_demo_data};

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

struct PowerjackApp {
    demos: Vec<DemoData>,
    demo_meta_rx: crossbeam_channel::Receiver<DemoData>,
    demo_meta_thread: Option<std::thread::JoinHandle<()>>,
}

impl PowerjackApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut demos = find_demos();
        // Sort by date descending
        demos.sort_by(|a, b| b.date.cmp(&a.date));

        let (demo_meta_tx, demo_meta_rx) = crossbeam_channel::unbounded();
        let demos_clone = demos.clone();
        let demo_meta_thread = Some(std::thread::spawn(move || {
            demos_clone.into_par_iter().for_each(|mut demo| {
                populate_demo_data(&mut demo);
                demo_meta_tx.send(demo).ok();
            });
        }));

        egui_extras::install_image_loaders(&cc.egui_ctx);
        Self {
            demos,
            demo_meta_rx,
            demo_meta_thread,
        }
    }

    fn process_demo_metadata(&mut self) {
        while let Ok(demo) = self.demo_meta_rx.try_recv() {
            if let Some(existing_demo) = self.demos.iter_mut().find(|d| d.path == demo.path) {
                *existing_demo = demo;
            }
        }
    }

    fn demo_grid_ui(&self, ui: &mut egui::Ui, demos: &[DemoData], id: impl Hash) {
        const COLUMN_WIDTH: f32 = 384.0;
        let total_width = ui.available_width();
        let num_columns = (total_width / COLUMN_WIDTH).floor() as usize;
        let spacing_per_column =
            (total_width - (num_columns as f32 * COLUMN_WIDTH)) / (num_columns as f32);

        egui::Grid::new(id)
            .spacing(egui::vec2(spacing_per_column, 32.0))
            .show(ui, |ui| {

                for (i, demo) in demos.iter().enumerate() {
                    let map_image_path = format!("images/maps/{}.webp", demo.map_name);
                    let map_image = if std::fs::exists(&map_image_path).ok() == Some(true) {
                        format!("file://{}", map_image_path)
                    } else {
                        "file://images/maps/unknown.webp".to_string()
                    };
                    ui.allocate_ui(vec2(384.0, 214.0), |ui| {
                        ui.vertical(|ui| {
                            ui.style_mut().spacing.item_spacing = Vec2::ZERO;
                            let img_response = egui::Image::new(map_image)
                                .corner_radius(
                                    CornerRadius {
                                        ne: 16,
                                        nw: 16,
                                        ..Default::default()
                                    },)
                                .ui(ui).interact(Sense::click());

                            if img_response.clicked() {
                                opener::open(&demo.path).ok();
                            }


                            let img_rect = img_response.rect;

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

                            if demo.complete {
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
                                                                RichText::new(format!("👥 {}", demo.num_friends))
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
                                                            AttributeKind::Excellent => ('⭐', Color32::GOLD),
                                                            AttributeKind::Positive => ('👍', Color32::GREEN),
                                                            AttributeKind::Informative => ('\u{2139}', Color32::LIGHT_GRAY),
                                                            AttributeKind::Negative => ('👎', Color32::LIGHT_RED),
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
                            }

                            let card_rect = img_rect.union(subtitle_rect);
                            if !demo.complete {
                                ui.painter().rect_filled(
                                    card_rect,
                                    CornerRadius {
                                        ne: 16,
                                        nw: 16,
                                        se: 16,
                                        sw: 16,
                                    },
                                    egui::Color32::from_black_alpha(150),
                                );


                                ui.scope_builder(
                                    egui::UiBuilder::default().max_rect(Rect::from_center_size(card_rect.center(), vec2(32.0, 64.0))),
                                    |ui| {
                                        Spinner::new().size(32.0).ui(ui)
                                    },
                                );
                            }
                        });
                    });

                    if (i + 1) % num_columns == 0 {
                        ui.end_row();
                    }
                }
            });
    }
}

impl eframe::App for PowerjackApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_demo_metadata();

        ctx.style_mut(|s| {
            s.interaction.selectable_labels = false;
        });

        // Each entry represents a range of demos for a single day
        let mut date_ranges: Vec<(chrono::NaiveDate, Range<usize>)> = Vec::new();
        if !self.demos.is_empty() {
            let mut start_idx = 0;
            let mut current_date = self.demos[0].date.date_naive();
            for (i, demo) in self.demos.iter().enumerate() {
                let demo_date = demo.date.date_naive();
                if demo_date != current_date {
                    date_ranges.push((current_date, start_idx..i));
                    current_date = demo_date;
                    start_idx = i;
                }
            }
            // Add the last range
            date_ranges.push((current_date, start_idx..self.demos.len()));
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Powerjack Demo Viewer");
            if self
                .demo_meta_thread
                .as_ref()
                .map(|t| !t.is_finished())
                .unwrap_or(false)
            {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(format!(
                        "Loading demo metadata... ({} left)",
                        self.demos.iter().filter(|d| !d.complete).count()
                    ));
                });
            }

            egui::ScrollArea::new([false, true])
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (date, range) in &date_ranges {
                        ui.add_space(16.0);
                        ui.heading(date.format("%B %d, %Y").to_string());
                        ui.separator();
                        self.demo_grid_ui(ui, &self.demos[range.clone()], date);
                    }
                });
        });
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

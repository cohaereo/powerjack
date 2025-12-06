#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use iced::Element;
use iced::alignment::{Horizontal, Vertical};
use iced::border::Radius;
use iced::widget::{
    Column, Container, Row, Scrollable, button, column, container, grid, row, stack, text,
};
use iced::{Background, Border, Color, Length, Padding, Shadow, Theme};
use powerjack_demo::DemoHeader;
use std::error::Error;
use std::path::Path;

fn main() -> iced::Result {
    dioxus_devtools::connect_subsecond();
    iced::run(DemoList::update, DemoList::view)
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Refresh,
}

#[derive(Default)]
pub struct DemoList {
    demos: Vec<DemoData>,
}

impl DemoList {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Refresh => {
                self.demos = find_demos();
            }
        }
    }

    fn demo_tile<'a>(&'a self, demo: &'a DemoData) -> Element<'a, Message> {
        let map_image_path = Path::new("images/maps")
            .join(&demo.map_name)
            .with_extension("jpg");
        let map_image = if map_image_path.exists() {
            iced::widget::image::Handle::from_path(map_image_path)
        } else {
            iced::widget::image::Handle::from_path("images/maps/unknown.jpg")
        };

        let class_portrait_path =
            format!("images/classes/{:?}.png", demo.primary_class).to_lowercase();
        let class_portrait = iced::widget::image::Handle::from_path(class_portrait_path);

        let image_section = stack![
            iced::widget::image(map_image.clone())
                .width(450)
                .height(256)
                .content_fit(iced::ContentFit::Cover),
            container(text(""))
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_theme: &Theme| container::Style {
                    background: Some(Background::Color(Color::from_rgba8(0, 0, 0, 0.53))),
                    ..Default::default()
                }),
            container(
                column![
                    row![
                        container(iced::widget::image(class_portrait).width(128).height(128))
                            .padding(Padding {
                                top: 18.0,
                                right: 0.0,
                                bottom: 0.0,
                                left: 18.0,
                            }),
                        container(
                            column![
                                text(format!("🕒 {}", demo.time_played))
                                    .size(24)
                                    .color(Color::WHITE),
                                if !demo.kd_ratio.is_empty() {
                                    text(format!("🎯 {}", demo.kd_ratio))
                                        .size(16)
                                        .color(Color::WHITE)
                                } else {
                                    text("")
                                },
                                if demo.num_friends > 0 {
                                    row![
                                        text(format!(
                                            "👥 {} friend{}",
                                            demo.num_friends,
                                            if demo.num_friends > 1 { "s" } else { "" }
                                        ))
                                        .size(16)
                                        .color(Color::from_rgb8(0x66, 0xff, 0x66))
                                    ]
                                    .align_y(Vertical::Bottom)
                                } else {
                                    row![text("")].align_y(Vertical::Bottom)
                                }
                            ]
                            .spacing(8)
                        )
                        .padding(Padding {
                            top: 18.0,
                            right: 16.0,
                            bottom: 0.0,
                            left: 0.0,
                        })
                        .width(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Right)
                    ]
                    .align_y(Vertical::Top),
                    container(
                        column(
                            demo.attributes
                                .iter()
                                .map(|attr| {
                                    let (icon, color) = match attr.kind {
                                        AttributeKind::Excellent => {
                                            ("⭐", Color::from_rgb8(0xff, 0xff, 0x66))
                                        }
                                        AttributeKind::Positive => {
                                            ("⬆", Color::from_rgb8(0x66, 0xff, 0x66))
                                        }
                                        AttributeKind::Negative => {
                                            ("⬇", Color::from_rgb8(0xff, 0x66, 0x66))
                                        }
                                    };

                                    row![
                                        text(icon).size(16).color(color),
                                        text(&attr.text).size(16).color(color)
                                    ]
                                    .spacing(4)
                                    .align_y(Vertical::Center)
                                    .into()
                                })
                                .collect::<Vec<_>>()
                        )
                        .spacing(8)
                        .align_x(Horizontal::Right)
                    )
                    .width(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Right)
                    .padding(Padding {
                        top: 0.0,
                        right: 16.0,
                        bottom: 12.0,
                        left: 0.0,
                    })
                ]
                .spacing(0)
                .height(Length::Fill)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Bottom)
        ]
        .width(450)
        .height(256);

        let label_section = container(row![
            container(text(&demo.map_name).size(20).color(Color::WHITE))
                .padding(Padding {
                    top: 0.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: 16.0,
                })
                .width(Length::Fill),
            container(
                text(&demo.date)
                    .size(16)
                    .color(Color::from_rgb8(0xaa, 0xaa, 0xaa))
            )
            .padding(Padding {
                top: 0.0,
                right: 16.0,
                bottom: 0.0,
                left: 0.0,
            })
        ])
        .width(450)
        .height(48)
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgb8(0x22, 0x22, 0x22))),
            border: Border {
                radius: Radius::default().bottom(16.0),
                ..Default::default()
            },
            ..Default::default()
        });

        container(
            column![
                container(image_section).style(|_theme: &Theme| container::Style {
                    background: Some(Background::Color(Color::from_rgb8(0x22, 0x22, 0x22))),
                    border: Border {
                        radius: Radius::default().top(24.0),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                label_section
            ]
            .spacing(0),
        )
        .into()
    }

    pub fn view(&'_ self) -> Column<'_, Message> {
        subsecond::call(|| {
            let mut col = Column::new()
                .spacing(20)
                .padding(20)
                .push(button("Refresh").on_press(Message::Refresh));

            let mut elements: Vec<Element<'_, Message>> = vec![];
            for demo in self.demos.iter().take(16) {
                elements.push(self.demo_tile(demo));
            }

            col = col.push(Scrollable::new(grid(elements).spacing(20)));

            col
        })
    }
}

#[derive(Clone)]
struct DemoData {
    attributes: Vec<Attribute>,
    date: String,
    kd_ratio: String,
    map_name: String,
    num_friends: usize,
    primary_class: Class,
    time_played: String,
}

#[derive(Debug, Clone, Copy)]
enum AttributeKind {
    Excellent,
    Positive,
    Negative,
}

#[derive(Clone)]
struct Attribute {
    kind: AttributeKind,
    text: String,
}

#[derive(Debug, Clone, Copy)]
enum Class {
    Random,
    Scout,
    Soldier,
    Pyro,
    Demoman,
    Heavy,
    Engineer,
    Medic,
    Sniper,
    Spy,
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
                        attributes: vec![Attribute {
                            kind: AttributeKind::Positive,
                            text: "High Score".to_string(),
                        }],
                        date: "Today".to_string(),
                        kd_ratio: "2.5".to_string(),
                        map_name: header.map_name,
                        num_friends: 0,
                        primary_class: Class::Engineer,
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

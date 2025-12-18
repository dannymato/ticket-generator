#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use rand::seq::IndexedRandom;
use std::{collections::HashSet, fmt::Write, fs::File, slice};

use egui_inbox::UiInbox;

const CAPTIALS: &str = "ABCDEFGHIJKLMNOPQRSTUVQXYZ";
const LOWERS: &str = "abcdefghijklmnopqrstuvqxyz";
const NUMBERS: &str = "0123456789";
const SPECIALS: &str = ",.;:\"'!%#";

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_pixels_per_point(1.2);
            Ok(Box::<RandomizerApp>::default())
        }),
    )
}

#[derive(Debug, Default)]
struct RandomizerApp {
    capital_letters: bool,
    lowercase_letters: bool,
    numbers: bool,
    specials: bool,
    rejected_chars: String,
    ticket_count: usize,
    ticket_count_str: String,
    ticket_length: usize,
    ticket_length_str: String,
    file_path: Option<String>,
    is_processing: bool,
    inbox: UiInbox<String>,
    last_thread_message: String,
}

impl RandomizerApp {
    fn build_character_set(&self) -> String {
        let mut buf = String::new();
        if self.capital_letters {
            buf.push_str(CAPTIALS);
        }

        if self.lowercase_letters {
            buf.push_str(LOWERS);
        }
        if self.numbers {
            buf.push_str(NUMBERS);
        }

        if self.specials {
            buf.push_str(SPECIALS);
        }

        let rejected_chars: Vec<char> = self.rejected_chars.chars().collect();

        buf.replace(&rejected_chars[..], "")
    }

    fn start_processing(&mut self) {
        if self.is_processing {
            return;
        }

        if self.file_path.is_none() {
            return;
        }

        let character_set = self.build_character_set();
        if character_set.is_empty() {
            return;
        }

        if self.ticket_length == 0 || self.ticket_count == 0 {
            return;
        }

        self.is_processing = true;
        let tx = self.inbox.sender();
        let file_path = self.file_path.clone().unwrap();
        let token_count = self.ticket_count;
        let ticket_length = self.ticket_length;

        std::thread::spawn(move || {
            // TODO: We should do something here
            let _ = match build_csv(character_set, file_path, token_count, ticket_length) {
                Ok(()) => tx.send("Successfully wrote to CSV".to_owned()),
                Err(str) => tx.send(str),
            };
        });
    }
}

fn build_csv(
    character_set: String,
    file_path: String,
    token_count: usize,
    token_length: usize,
) -> Result<(), String> {
    let file = File::create(&file_path)
        .map_err(|err| format!("Failed to create file {file_path}: {err}"))?;

    let mut set: HashSet<String> = HashSet::new();

    let character_set: Vec<char> = character_set.chars().collect();
    let mut rng = rand::rng();

    let mut csv_writer = csv::WriterBuilder::new().from_writer(file);

    for _ in 0..token_count {
        let new_token = gen_token(&mut rng, &character_set, &set, token_length)?;
        csv_writer
            .write_record(slice::from_ref(&new_token))
            .map_err(|err| format!("Failed to write to file: {err}"))?;

        set.insert(new_token);
    }

    Ok(())
}

fn gen_token(
    rng: &mut impl rand::Rng,
    character_set: &[char],
    already_generated: &HashSet<String>,
    token_length: usize,
) -> Result<String, String> {
    let mut buf = String::with_capacity(token_length);
    for _ in 0..token_length {
        let char = character_set.choose(rng).unwrap();
        buf.write_char(*char)
            .map_err(|_e| "Failed to write to string")?;
    }

    if already_generated.contains(&buf) {
        return gen_token(rng, character_set, already_generated, token_length);
    }

    Ok(buf)
}

impl eframe::App for RandomizerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Ticket Randomizer");
            ui.checkbox(&mut self.capital_letters, "Capital Letters (A-Z)");
            ui.checkbox(&mut self.lowercase_letters, "Lowercase Letters (a-z)");
            ui.checkbox(&mut self.numbers, "Number (0-9)");
            ui.checkbox(&mut self.specials, "Specials (,.;:\"'!%#)");

            ui.horizontal(|ui| {
                let rejected_chars_label = ui.label("Excluded Characters: ");
                ui.text_edit_singleline(&mut self.rejected_chars)
                    .labelled_by(rejected_chars_label.id);
            });

            ui.horizontal(|ui| {
                let count_label = ui.label("Ticket Count: ");
                if ui
                    .text_edit_singleline(&mut self.ticket_count_str)
                    .labelled_by(count_label.id)
                    .lost_focus()
                {
                    if let Ok(parsed) = self.ticket_count_str.parse::<usize>() {
                        self.ticket_count = parsed;
                    } else {
                        self.ticket_count_str = self.ticket_count.to_string();
                    }
                }
            });

            ui.horizontal(|ui| {
                let length_label = ui.label("Ticket Length: ");
                if ui
                    .text_edit_singleline(&mut self.ticket_length_str)
                    .labelled_by(length_label.id)
                    .lost_focus()
                {
                    if let Ok(parsed) = self.ticket_length_str.parse::<usize>() {
                        self.ticket_length = parsed;
                    } else {
                        self.ticket_length_str = self.ticket_length.to_string();
                    }
                }
            });

            ui.horizontal(|ui| {
                if ui.button("Select destination...").clicked() {
                    let file_dialog = rfd::FileDialog::new().add_filter("csv", &["csv"]);

                    if let Some(path) = file_dialog.save_file() {
                        self.file_path = Some(path.display().to_string());
                    }
                }

                if let Some(file_path) = &self.file_path {
                    ui.label(file_path);
                }
            });

            ui.label(format!(
                "Current character set {}",
                self.build_character_set()
            ));

            if ui
                .add_enabled(!self.is_processing, egui::Button::new("Submit"))
                .clicked()
            {
                self.start_processing();
            }

            if let Some(last) = self.inbox.read(ui).last() {
                self.last_thread_message = last;
                self.is_processing = false;
            }
            ui.label(&self.last_thread_message);
        });
    }
}

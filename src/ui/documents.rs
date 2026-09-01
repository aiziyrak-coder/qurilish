//! Obyekt hujjatlari va fotolari (TZ I.1).
//!
//! Fayllar ko'chirilmaydi — bazada faqat yo'l saqlanadi. Shunday qilingani
//! uchun fayl joyidan olib qo'yilsa, ro'yxatda buni ochiq ko'rsatamiz.

use super::*;
use crate::domain::Document;
use crate::model::Section;

/// Rasm sifatida ko'rsatiladigan kengaytmalar.
fn is_photo(format: &str) -> bool {
    matches!(
        format.to_lowercase().as_str(),
        "jpg" | "jpeg" | "png" | "bmp" | "gif" | "webp"
    )
}

/// Pasport ekranidagi «Hujjatlar va foto» kartochkasi.
pub fn card(ui: &mut egui::Ui, app: &mut App, w: f32) {
    let mut added: Vec<std::path::PathBuf> = Vec::new();
    let mut removed: Option<i64> = None;
    let mut edited: Option<Document> = None;

    card_frame(ui, t("card_documents"), w, |ui| {
        ui.horizontal(|ui| {
            if ui.button(t("add_document")).clicked() {
                if let Some(files) = rfd::FileDialog::new().pick_files() {
                    added = files;
                }
            }
            ui.label(
                RichText::new(t("documents_hint"))
                    .size(11.0)
                    .color(theme::muted()),
            );
        });

        if app.documents.is_empty() {
            ui.add_space(4.0);
            ui.label(RichText::new(t("documents_empty")).color(theme::muted()));
            return;
        }

        ui.add_space(8.0);

        // --- Foto galereyasi ---
        let photos: Vec<&Document> = app
            .documents
            .iter()
            .filter(|d| is_photo(&d.format))
            .collect();
        if !photos.is_empty() {
            ui.label(
                RichText::new(t("photos"))
                    .size(11.0)
                    .color(theme::muted())
                    .strong(),
            );
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                for d in &photos {
                    let exists = std::path::Path::new(&d.path).exists();
                    ui.vertical(|ui| {
                        ui.set_width(148.0);
                        if exists {
                            let uri = format!("file://{}", d.path);
                            let img = egui::Image::new(uri)
                                .fit_to_exact_size(egui::vec2(148.0, 100.0))
                                .maintain_aspect_ratio(true)
                                .corner_radius(4);
                            if ui
                                .add(egui::ImageButton::new(img).frame(false))
                                .on_hover_text(t("open_file"))
                                .clicked()
                            {
                                open_path(&d.path);
                            }
                        } else {
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(148.0, 100.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(rect, 4.0, theme::track());
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                t("file_missing"),
                                egui::FontId::proportional(11.0),
                                theme::danger(),
                            );
                        }
                        ui.label(
                            RichText::new(super::issues::truncate(&d.name, 20))
                                .size(11.0)
                                .color(theme::muted()),
                        );
                    });
                }
            });
            ui.add_space(10.0);
        }

        // --- Hujjatlar jadvali ---
        let head = |ui: &mut egui::Ui, cw: f32, s: &str| {
            ui.add_sized(
                [cw, 16.0],
                egui::Label::new(RichText::new(s).color(theme::muted()).size(11.0)),
            );
        };
        ui.horizontal(|ui| {
            head(ui, 300.0, t("col_name"));
            head(ui, 60.0, t("col_format"));
            head(ui, 90.0, t("col_section_short"));
            head(ui, 100.0, t("col_added"));
        });

        for doc in &app.documents {
            let mut d = doc.clone();
            let mut changed = false;
            let exists = std::path::Path::new(&d.path).exists();
            ui.horizontal(|ui| {
                changed |= ui
                    .add_sized([300.0, 22.0], egui::TextEdit::singleline(&mut d.name))
                    .changed();
                ui.add_sized(
                    [60.0, 22.0],
                    egui::Label::new(
                        RichText::new(d.format.to_uppercase())
                            .size(11.0)
                            .color(theme::muted())
                            .monospace(),
                    ),
                );
                egui::ComboBox::from_id_salt(("doc_sec", d.id))
                    .selected_text(d.section.label())
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        for s in Section::ALL {
                            changed |= ui.selectable_value(&mut d.section, s, s.label()).changed();
                        }
                    });
                ui.add_sized(
                    [100.0, 22.0],
                    egui::Label::new(RichText::new(&d.added_at).size(11.0).color(theme::muted())),
                );
                if ui
                    .add_enabled(exists, egui::Button::new(t("open")))
                    .on_hover_text(&d.path)
                    .clicked()
                {
                    open_path(&d.path);
                }
                if !exists {
                    ui.label(
                        RichText::new(t("file_missing"))
                            .size(11.0)
                            .color(theme::danger()),
                    );
                }
                if ui
                    .small_button(RichText::new("x").color(theme::danger()))
                    .on_hover_text(t("remove_from_list"))
                    .clicked()
                {
                    removed = Some(d.id);
                }
            });
            if changed {
                edited = Some(d);
            }
        }
    });

    if !added.is_empty() {
        if let Some(pid) = app.current {
            for path in added {
                let name = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                let format = path
                    .extension()
                    .map(|s| s.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                app.db.insert_document(&Document {
                    id: 0,
                    project_id: pid,
                    section: Section::None,
                    name,
                    format,
                    path: path.to_string_lossy().to_string(),
                    sheets: 0,
                    added_at: String::new(),
                });
            }
            app.reload_modules();
        }
    }
    if let Some(d) = edited {
        app.db.update_document(&d);
        if let Some(slot) = app.documents.iter_mut().find(|x| x.id == d.id) {
            *slot = d;
        }
    }
    if let Some(id) = removed {
        app.db.del("document", id);
        app.reload_modules();
    }
}

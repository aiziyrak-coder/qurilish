//! Izoh va biriktirma paneli — har qanday yozuv uchun (umumiy mexanizm).
//!
//! TZ da izoh va foto o'nlab joyda talab qilinadi: ish, ariza, xarid, ijro
//! hujjati, sifat tekshiruvi, xavfsizlik holati, texnik nazorat izohi
//! (IV.8, V.18, VI.22, VII.20/22, IX.28, X.28, XI.44, XIV.11/21). Ularning
//! har biriga alohida jadval va alohida ekran qilish — bir xil kodni o'n
//! marta yozish demak. Shuning uchun mexanizm bitta: yozuv **turi va
//! raqami** bilan belgilanadi, panel esa istalgan ekranga qo'yiladi.
//!
//! Ikki qoida:
//! 1. **Izoh o'chirilmaydi va tahrirlanmaydi.** Muhokama tarixi o'zgarsa,
//!    uning ma'nosi qolmaydi. Hal qilingani yopiq deb belgilanadi.
//! 2. **Fayl ko'chirilmaydi**, faqat yo'li saqlanadi. Fayl joyidan
//!    ketsa — ekranda ochiq ko'rsatiladi, soxta ishonch berilmaydi.

use super::*;
use crate::domain::{Attachment, Note, NoteTarget, PhotoStage};

/// Yozuv bo'yicha izohlar soni — ro'yxatda belgi qo'yish uchun.
pub fn count(app: &App, target: NoteTarget, id: i64) -> usize {
    app.notes
        .iter()
        .filter(|n| n.target == target && n.target_id == id)
        .count()
}

/// Yozuv bo'yicha biriktirmalar soni.
pub fn photo_count(app: &App, target: NoteTarget, id: i64) -> usize {
    app.attachments
        .iter()
        .filter(|a| a.target == target && a.target_id == id)
        .count()
}

/// Ro'yxat qatoriga qo'yiladigan qisqa belgi: «2 izoh · 3 foto».
///
/// Belgi bosilsa panel ochiladi — shuning uchun u tugma, yozuv emas.
pub fn badge(ui: &mut egui::Ui, app: &App, target: NoteTarget, id: i64) -> bool {
    let n = count(app, target, id);
    let p = photo_count(app, target, id);
    let label = if n == 0 && p == 0 {
        t("nt_add").to_string()
    } else {
        format!("{n} · {p}")
    };
    ui.small_button(RichText::new(label).size(11.0))
        .on_hover_text(t("nt_badge_hint"))
        .clicked()
}

/// Jadval ostiga qo'yiladigan panel.
///
/// Qaysi qator ochilgani ekran xotirasida saqlanadi: jadvalning o'zini
/// o'zgartirmasdan, ostiga bitta panel chiqadi. Shu belgi qayta bosilsa
/// panel yopiladi.
pub fn below_table(ui: &mut egui::Ui, app: &mut App, target: NoteTarget, clicked: Option<i64>) {
    let key = egui::Id::new(("nt_open", target.code()));
    let mut open = ui.data(|d| d.get_temp::<i64>(key));
    if let Some(id) = clicked {
        // O'sha qator qayta bosilsa — yopiladi.
        open = if open == Some(id) { None } else { Some(id) };
    }
    let Some(id) = open else {
        ui.data_mut(|d| d.remove::<i64>(key));
        return;
    };
    ui.data_mut(|d| d.insert_temp(key, id));

    ui.add_space(10.0);
    let mut close = false;
    egui::Frame::new()
        .fill(theme::card())
        .stroke(egui::Stroke::new(1.0_f32, theme::line()))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{} · {}", target.label(), t("nt_panel")))
                        .size(12.0)
                        .color(theme::muted()),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button(t("nt_close")).clicked() {
                        close = true;
                    }
                });
            });
            ui.add_space(6.0);
            panel(ui, app, target, id);
        });
    if close {
        ui.data_mut(|d| d.remove::<i64>(key));
    }
}

/// Izoh va biriktirma paneli.
///
/// Panelni istalgan ekranga qo'yish mumkin: u faqat yozuv turi va raqamini
/// biladi, yozuvning o'zi haqida hech narsa bilmaydi.
pub fn panel(ui: &mut egui::Ui, app: &mut App, target: NoteTarget, id: i64) {
    let Some(pid) = app.current else { return };
    let can = app.can_edit(app.screen);

    ui.push_id(("notes", target.code(), id), |ui| {
        notes_block(ui, app, pid, target, id, can);
        ui.add_space(8.0);
        files_block(ui, app, pid, target, id, can);
    });
}

// ================================================================ Izohlar

fn notes_block(ui: &mut egui::Ui, app: &mut App, pid: i64, target: NoteTarget, id: i64, can: bool) {
    let mut rows: Vec<Note> = app
        .notes
        .iter()
        .filter(|n| n.target == target && n.target_id == id)
        .cloned()
        .collect();
    // Hal qilinganlari oxirida: ochiq savol ko'z oldida tursin.
    rows.sort_by_key(|n| (n.resolved, n.id));

    ui.label(
        RichText::new(format!("{} ({})", t("nt_notes"), rows.len()))
            .size(12.5)
            .strong(),
    );
    ui.add_space(4.0);

    let mut toggle: Option<(i64, bool)> = None;
    let mut reply_to: Option<i64> = None;
    for n in &rows {
        let indent = if n.parent.is_some() { 18.0 } else { 0.0 };
        ui.horizontal(|ui| {
            ui.add_space(indent);
            ui.vertical(|ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(if n.author.trim().is_empty() {
                            t("nt_unknown_author")
                        } else {
                            &n.author
                        })
                        .size(11.5)
                        .strong()
                        .color(if n.resolved {
                            theme::muted()
                        } else {
                            theme::text()
                        }),
                    );
                    ui.label(RichText::new(&n.at).size(10.5).color(theme::muted()));
                    if n.resolved {
                        ui.label(
                            RichText::new(t("nt_resolved"))
                                .size(10.5)
                                .color(theme::ok()),
                        );
                    }
                    if can {
                        let label = if n.resolved {
                            t("nt_reopen")
                        } else {
                            t("nt_resolve")
                        };
                        if ui.small_button(RichText::new(label).size(10.5)).clicked() {
                            toggle = Some((n.id, !n.resolved));
                        }
                        if n.parent.is_none()
                            && ui
                                .small_button(RichText::new(t("nt_reply")).size(10.5))
                                .clicked()
                        {
                            reply_to = Some(n.id);
                        }
                    }
                });
                ui.label(RichText::new(&n.text).size(12.0).color(if n.resolved {
                    theme::muted()
                } else {
                    theme::text()
                }));
            });
        });
        ui.add_space(4.0);
    }

    if rows.is_empty() {
        ui.label(
            RichText::new(t("nt_no_notes"))
                .size(11.5)
                .color(theme::muted()),
        );
        ui.add_space(4.0);
    }

    if let Some((nid, value)) = toggle {
        app.db.set_note_resolved(nid, value);
        app.notes = app.db.notes(pid);
    }

    if !can {
        return;
    }

    // Yozish qatori. Javob rejimi alohida kalitda saqlanadi.
    let parent_key = egui::Id::new("nt_parent");
    let text_key = egui::Id::new("nt_text");
    if let Some(p) = reply_to {
        ui.data_mut(|d| d.insert_temp(parent_key, p));
    }
    let parent = ui.data(|d| d.get_temp::<i64>(parent_key));
    let mut text = ui
        .data(|d| d.get_temp::<String>(text_key))
        .unwrap_or_default();

    if let Some(p) = parent {
        ui.horizontal(|ui| {
            let who = rows
                .iter()
                .find(|n| n.id == p)
                .map(|n| n.author.clone())
                .unwrap_or_default();
            ui.label(
                RichText::new(format!("{} {}", t("nt_replying_to"), who))
                    .size(11.0)
                    .color(theme::muted()),
            );
            if ui.small_button(RichText::new("x").size(10.5)).clicked() {
                ui.data_mut(|d| d.remove::<i64>(parent_key));
            }
        });
    }

    let mut send = false;
    ui.horizontal(|ui| {
        let width = (ui.available_width() - 96.0).max(180.0);
        let edit = ui.add_sized(
            [width, 26.0],
            egui::TextEdit::singleline(&mut text).hint_text(t("nt_placeholder")),
        );
        let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if ui
            .add_enabled(!text.trim().is_empty(), egui::Button::new(t("nt_send")))
            .clicked()
            || (enter && !text.trim().is_empty())
        {
            send = true;
        }
    });

    if send {
        app.db.insert_note(&Note {
            id: 0,
            project_id: pid,
            target,
            target_id: id,
            author: app.current_user_name(),
            at: app.stamp(),
            text: text.trim().to_string(),
            parent,
            resolved: false,
        });
        app.notes = app.db.notes(pid);
        text.clear();
        ui.data_mut(|d| d.remove::<i64>(parent_key));
    }
    ui.data_mut(|d| d.insert_temp(text_key, text));
}

// ================================================================ Biriktirmalar

fn files_block(ui: &mut egui::Ui, app: &mut App, pid: i64, target: NoteTarget, id: i64, can: bool) {
    let rows: Vec<Attachment> = app
        .attachments
        .iter()
        .filter(|a| a.target == target && a.target_id == id)
        .cloned()
        .collect();

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{} ({})", t("nt_files"), rows.len()))
                .size(12.5)
                .strong(),
        );
        if can {
            // «Oldin» va «keyin» alohida tugma: keyin belgilashni unutmasin.
            for (label, stage) in [
                (t("nt_add_file"), PhotoStage::Plain),
                (t("ps_before"), PhotoStage::Before),
                (t("ps_after"), PhotoStage::After),
            ] {
                if ui.small_button(label).clicked() {
                    add_files(app, pid, target, id, stage);
                }
            }
        }
    });
    ui.add_space(4.0);

    if rows.is_empty() {
        ui.label(
            RichText::new(t("nt_no_files"))
                .size(11.5)
                .color(theme::muted()),
        );
        return;
    }

    let mut removed: Option<i64> = None;
    let mut edited: Option<Attachment> = None;
    for a in &rows {
        let mut item = a.clone();
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            let colour = match item.stage {
                PhotoStage::Before => theme::warn(),
                PhotoStage::After => theme::ok(),
                PhotoStage::Plain => theme::muted(),
            };
            if item.stage != PhotoStage::Plain {
                ui.label(RichText::new(item.stage.label()).size(10.5).color(colour));
            }
            // Fayl joyida bo'lmasa — buni yashirmaymiz.
            let alive = item.exists();
            let name = ui.label(RichText::new(item.file_name()).size(12.0).color(if alive {
                theme::text()
            } else {
                theme::danger()
            }));
            name.on_hover_text(if alive {
                item.path.clone()
            } else {
                format!("{}\n{}", t("nt_file_missing"), item.path)
            });

            if alive
                && ui
                    .small_button(RichText::new(t("nt_open")).size(10.5))
                    .clicked()
            {
                open_file(&item.path);
            }
            if can {
                let resp = ui.add_sized(
                    [180.0, 20.0],
                    egui::TextEdit::singleline(&mut item.caption).hint_text(t("nt_caption")),
                );
                changed |= resp.lost_focus() && resp.changed();
                egui::ComboBox::from_id_salt(("att_stage", item.id))
                    .selected_text(item.stage.label())
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        for s in PhotoStage::ALL {
                            if ui.selectable_label(item.stage == *s, s.label()).clicked() {
                                item.stage = *s;
                                changed = true;
                            }
                        }
                    });
                if ui
                    .small_button(RichText::new("x").size(10.5).color(theme::danger()))
                    .on_hover_text(t("nt_detach_hint"))
                    .clicked()
                {
                    removed = Some(item.id);
                }
            }
        });
        if changed {
            edited = Some(item);
        }
    }

    if let Some(a) = edited {
        app.db.update_attachment(&a);
        app.attachments = app.db.attachments(pid);
    }
    if let Some(rid) = removed {
        app.db.delete_attachment(rid);
        app.attachments = app.db.attachments(pid);
    }
}

/// Fayl tanlash oynasini ochadi va tanlanganlarni biriktiradi.
fn add_files(app: &mut App, pid: i64, target: NoteTarget, id: i64, stage: PhotoStage) {
    let Some(files) = rfd::FileDialog::new()
        .add_filter(
            t("photos"),
            &["jpg", "jpeg", "png", "bmp", "webp", "pdf", "xlsx", "docx"],
        )
        .pick_files()
    else {
        return;
    };
    let author = app.current_user_name();
    let at = app.stamp();
    for f in files {
        app.db.insert_attachment(&Attachment {
            id: 0,
            project_id: pid,
            target,
            target_id: id,
            path: f.display().to_string(),
            stage,
            caption: String::new(),
            author: author.clone(),
            at: at.clone(),
        });
    }
    app.attachments = app.db.attachments(pid);
}

/// Faylni tizim dasturida ochadi.
fn open_file(path: &str) {
    #[cfg(target_os = "windows")]
    {
        // `explorer` yo'lni o'zi tushunadi va nolga teng bo'lmagan kod
        // qaytarishi mumkin — bu xato emas, shuning uchun natija o'qilmaydi.
        let _ = std::process::Command::new("explorer").arg(path).spawn();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
}

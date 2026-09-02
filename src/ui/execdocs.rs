//! «Ijro hujjatlari» ekrani (TZ IV).
//!
//! Modulning asosiy g'oyasi: tizim qurilishning har bosqichida qaysi ijro
//! hujjati rasmiylashtirilgan bo'lishi kerakligini bilishi kerak. Shuning
//! uchun ro'yxat yonida «tugallangan, lekin hujjatsiz ishlar» paneli turadi.

use super::*;
use crate::domain::{ExecDoc, ExecDocKind, ExecDocStatus};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    if app.current.is_none() {
        ui.vertical_centered(|ui| {
            ui.add_space(120.0);
            ui.label(
                RichText::new(t("no_object_selected"))
                    .color(theme::muted())
                    .size(18.0),
            );
        });
        return;
    }

    let mut add_for: Option<(Option<i64>, ExecDocKind)> = None;

    let mut make_ks2 = false;
    let mut make_ks3 = false;
    ui.horizontal_wrapped(|ui| {
        if ui.button(t("add_exec_doc")).clicked() {
            add_for = Some((None, ExecDocKind::Hidden));
        }
        ui.separator();
        // Rasmiy shakllar bazadagi ma'lumotdan yig'iladi (TZ IV.5–7).
        if ui
            .button(t("doc_ks2_short"))
            .on_hover_text(t("doc_ks2_hint"))
            .clicked()
        {
            make_ks2 = true;
        }
        if ui
            .button(t("doc_ks3_short"))
            .on_hover_text(t("doc_ks3_hint"))
            .clicked()
        {
            make_ks3 = true;
        }
        ui.label(
            RichText::new(t("exec_docs_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    if make_ks2 || make_ks3 {
        build_act(app, make_ks2);
    }

    kpis(ui, app);
    ui.add_space(10.0);

    // Yon panel `SidePanel` bilan: `set_width` ni ichkaridagi ScrollArea
    // hurmat qilmasdi va jadval panel ostiga chiqib ketardi.
    let wide = ui.available_width() > 900.0;
    let panel_w = (ui.available_width() * 0.32).clamp(280.0, 400.0);
    if wide {
        egui::SidePanel::right("req_docs")
            .resizable(false)
            .exact_width(panel_w)
            .frame(egui::Frame::new().inner_margin(egui::Margin {
                left: 12,
                ..Default::default()
            }))
            .show_inside(ui, |ui| {
                if let Some((task_id, kind)) = required_panel(ui, app, panel_w) {
                    add_for = Some((Some(task_id), kind));
                }
            });
        list(ui, app, true);
    } else {
        // Tor oynada panel jadval ostida turadi — hammasi bitta oqimda suriladi.
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    list(ui, app, false);
                    ui.add_space(10.0);
                    if let Some((task_id, kind)) = required_panel(ui, app, ui.available_width()) {
                        add_for = Some((Some(task_id), kind));
                    }
                });
            });
    }

    if let Some((task_id, kind)) = add_for {
        if let Some(pid) = app.current {
            let name = task_id
                .and_then(|id| app.task(id).map(|t| t.name.clone()))
                .unwrap_or_default();
            app.db.insert_exec_doc(&ExecDoc {
                id: 0,
                project_id: pid,
                kind,
                number: String::new(),
                name,
                date: app.today,
                task_id,
                status: ExecDocStatus::Draft,
                responsible: String::new(),
                note: String::new(),
            });
            app.reload_modules();
        }
    }
}

/// KS-2 yoki KS-3 ni yig'ib faylga yozadi (TZ IV.5–7).
///
/// KS-3 KS-2 dan chiqadi: davr summasi aynan shu davrdagi bajarilgan ish
/// qiymati bo'lishi kerak, shuning uchun ikkalasi bitta hisobdan olinadi.
fn build_act(app: &mut App, ks2: bool) {
    let Some(project) = app.project().cloned() else {
        return;
    };
    let (from, to) = super::doc_period(app);
    let lines = crate::docgen::ks2_lines(&app.tasks, &app.estimate_items, from, to);
    if lines.is_empty() {
        app.notify(t("doc_no_work").to_string());
        return;
    }

    // Fayl avval tanlanadi: shundan keyin `app` dan qarz olish kerak bo'lmaydi.
    let file = format!(
        "{}-{}.xlsx",
        if ks2 { "KS-2" } else { "KS-3" },
        to.format("%Y-%m")
    );
    let Some(path) = rfd::FileDialog::new()
        .set_title(t("doc_save"))
        .set_file_name(&file)
        .add_filter("Excel", &["xlsx"])
        .save_file()
    else {
        return;
    };

    // Shartnoma boshidan jami — KS-3 uchun; KS-2 da ishlatilmaydi.
    let since_start: f64 = crate::docgen::ks2_lines(
        &app.tasks,
        &app.estimate_items,
        project.start_date,
        app.today,
    )
    .iter()
    .map(|l| l.cost)
    .sum();
    let period_total: f64 = lines.iter().map(|l| l.cost).sum();
    let cost = app.cost_summary();

    let inp = crate::docgen::DocInput {
        project: &project,
        parties: &app.parties,
        tasks: &app.tasks,
        today: app.today,
        from,
        to,
    };
    let result = if ks2 {
        crate::docgen::write_ks2(&path, &inp, &lines).map(money)
    } else {
        crate::docgen::write_ks3(&path, &inp, period_total, since_start, &cost)
            .map(|()| money(period_total))
    };
    match result {
        Ok(sum) => app.notify(format!("{} {sum} · {}", t("doc_saved"), path.display())),
        Err(e) => app.notify(format!("{}: {e}", t("doc_failed"))),
    }
}

/// Yashirin ishlar dalolatnomasini faylga yozadi (TZ IV.5).
fn write_aosr(app: &mut App, doc_id: i64) {
    let Some(project) = app.project().cloned() else {
        return;
    };
    let Some(doc) = app.exec_docs.iter().find(|d| d.id == doc_id).cloned() else {
        return;
    };
    let (from, to) = super::doc_period(app);
    let file = format!(
        "AOSR-{}-{}.xlsx",
        doc.number.replace(['/', '\\', ' '], "-"),
        doc.date.format("%Y-%m-%d")
    );
    let Some(path) = rfd::FileDialog::new()
        .set_title(t("doc_save"))
        .set_file_name(&file)
        .add_filter("Excel", &["xlsx"])
        .save_file()
    else {
        return;
    };
    let inp = crate::docgen::DocInput {
        project: &project,
        parties: &app.parties,
        tasks: &app.tasks,
        today: app.today,
        from,
        to,
    };
    match crate::docgen::write_aosr(&path, &inp, &doc, &app.stock_moves, &app.materials) {
        Ok(()) => app.notify(format!("{} {}", t("doc_saved"), path.display())),
        Err(e) => app.notify(format!("{}: {e}", t("doc_failed"))),
    }
}

fn kpis(ui: &mut egui::Ui, app: &App) {
    let total = app.exec_docs.len();
    let signed = app
        .exec_docs
        .iter()
        .filter(|d| d.status == ExecDocStatus::Signed)
        .count();
    let review = app
        .exec_docs
        .iter()
        .filter(|d| d.status == ExecDocStatus::OnReview)
        .count();
    let need = required(app);
    let missing = need.iter().filter(|r| !r.exists).count();
    // Ish tugagan, hujjat esa yo'q — eng jiddiy holat.
    let overdue = need.iter().filter(|r| !r.exists && r.task_done).count();

    stat_row(
        ui,
        vec![
            stat(
                t("kpi_docs_total"),
                total.to_string(),
                t("kpi_docs_hint"),
                theme::accent(),
            ),
            stat(
                t("kpi_docs_signed"),
                signed.to_string(),
                t("kpi_docs_signed_hint"),
                if signed == total && total > 0 {
                    theme::ok()
                } else {
                    theme::text()
                },
            ),
            stat(
                t("kpi_docs_review"),
                review.to_string(),
                t("kpi_docs_review_hint"),
                if review == 0 {
                    theme::muted()
                } else {
                    theme::warn()
                },
            ),
            stat(
                t("kpi_docs_missing"),
                missing.to_string(),
                t("kpi_docs_missing_hint"),
                if missing == 0 {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
            stat(
                t("kpi_docs_overdue"),
                overdue.to_string(),
                t("kpi_docs_overdue_hint"),
                if overdue == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
        ],
    );
}

/// TZ IV.1: boshlangan ishlar bo'yicha talab qilinadigan hujjatlar reyestri.
fn required(app: &App) -> Vec<crate::checks::RequiredDoc> {
    crate::checks::required_docs(&app.tasks, &app.exec_docs, true)
}

/// Yopilmagan talablar: hujjat umuman yo'q yoki bor-u imzolanmagan.
fn open_requirements(app: &App) -> Vec<crate::checks::RequiredDoc> {
    required(app)
        .into_iter()
        .filter(|r| !r.exists || !r.signed)
        .collect()
}

/// Talablar reyestri: qaysi ishga qaysi hujjat kerak va u qaysi holatda.
/// Bosilganda o'sha tur bilan hujjat yaratiladi.
fn required_panel(ui: &mut egui::Ui, app: &App, w: f32) -> Option<(i64, ExecDocKind)> {
    let mut create_for = None;
    let items = open_requirements(app);

    card_frame(ui, t("docs_required"), w - 32.0, |ui| {
        ui.label(
            RichText::new(t("docs_required_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
        ui.add_space(6.0);
        if items.is_empty() {
            ui.label(RichText::new(t("docs_required_none")).color(theme::ok()));
            return;
        }
        egui::ScrollArea::vertical()
            .max_height(360.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for r in &items {
                    ui.horizontal(|ui| {
                        // Uch holat: hujjat yo'q va ish tugagan (kechikdi),
                        // hujjat yo'q va ish ketmoqda (kutilmoqda),
                        // hujjat bor, lekin imzolanmagan.
                        let (mark, color) = if !r.exists && r.task_done {
                            (t("docs_late"), theme::danger())
                        } else if !r.exists {
                            (t("docs_pending"), theme::warn())
                        } else {
                            (t("docs_unsigned"), theme::accent())
                        };
                        ui.add_sized(
                            [64.0, 18.0],
                            egui::Label::new(RichText::new(mark).size(10.5).color(color)),
                        );
                        ui.add_sized(
                            [30.0, 18.0],
                            egui::Label::new(
                                RichText::new(r.section.label())
                                    .size(11.0)
                                    .color(theme::section_color(r.section.color())),
                            ),
                        );
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(super::issues::truncate(&r.task_name, 24)).size(12.0),
                            );
                            ui.label(
                                RichText::new(r.kind.label())
                                    .size(10.5)
                                    .color(theme::muted()),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Hujjat allaqachon bor — uni ro'yxatdan imzolash kerak.
                            if !r.exists
                                && ui
                                    .small_button(t("create"))
                                    .on_hover_text(t("docs_create_hint"))
                                    .clicked()
                            {
                                create_for = Some((r.task_id, r.kind));
                            }
                        });
                    });
                    ui.add_space(2.0);
                }
            });
    });
    create_for
}

/// `fill` — jadval qolgan balandlikni to'liq egallaydimi. Tor oynada pastda
/// yana bloklar bo'lgani uchun jadval faqat o'z balandligini oladi.
fn list(ui: &mut egui::Ui, app: &mut App, fill: bool) {
    if app.exec_docs.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(t("exec_docs_empty")).color(theme::muted()));
        });
        return;
    }

    let mut edited: Option<ExecDoc> = None;
    let mut removed: Option<i64> = None;
    let mut make_aosr: Option<i64> = None;

    egui::ScrollArea::both()
        .auto_shrink([false, !fill])
        .max_height(if fill { f32::INFINITY } else { 320.0 })
        .show(ui, |ui| {
            egui::Grid::new("exec_grid")
                .num_columns(9)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    let head = |ui: &mut egui::Ui, w: f32, s: &str| {
                        ui.add_sized(
                            [w, 16.0],
                            egui::Label::new(RichText::new(s).color(theme::muted()).size(11.0)),
                        );
                    };
                    head(ui, 190.0, t("col_kind"));
                    head(ui, 90.0, t("col_number"));
                    head(ui, 210.0, t("col_name"));
                    head(ui, 110.0, t("col_date"));
                    head(ui, 220.0, t("col_task"));
                    head(ui, 150.0, t("col_status"));
                    head(ui, 150.0, t("col_responsible"));
                    head(ui, 70.0, "");
                    head(ui, 24.0, "");
                    ui.end_row();

                    for doc in &app.exec_docs {
                        let mut d = doc.clone();
                        let mut changed = false;

                        egui::ComboBox::from_id_salt(("ed_kind", d.id))
                            .selected_text(d.kind.label())
                            .width(190.0)
                            .show_ui(ui, |ui| {
                                for k in ExecDocKind::ALL {
                                    changed |=
                                        ui.selectable_value(&mut d.kind, *k, k.label()).changed();
                                }
                            });
                        changed |= ui
                            .add_sized([90.0, 22.0], egui::TextEdit::singleline(&mut d.number))
                            .changed();
                        changed |= ui
                            .add_sized([210.0, 22.0], egui::TextEdit::singleline(&mut d.name))
                            .changed();
                        changed |=
                            super::passport::date_edit(ui, &format!("ed{}", d.id), &mut d.date);
                        changed |= task_picker(ui, app, ("ed_task", d.id), &mut d.task_id, 220.0);
                        egui::ComboBox::from_id_salt(("ed_st", d.id))
                            .selected_text(d.status.label())
                            .width(150.0)
                            .show_ui(ui, |ui| {
                                for s in ExecDocStatus::ALL {
                                    changed |=
                                        ui.selectable_value(&mut d.status, *s, s.label()).changed();
                                }
                            });
                        changed |= ui
                            .add_sized(
                                [150.0, 22.0],
                                egui::TextEdit::singleline(&mut d.responsible),
                            )
                            .changed();
                        // Blankani faqat yashirin ishlar hujjati uchun
                        // chiqaramiz: qolgan turlarning shakli boshqacha.
                        let hidden = d.kind == ExecDocKind::Hidden;
                        if ui
                            .add_enabled(
                                hidden,
                                egui::Button::new(RichText::new(t("doc_blank")).size(11.0)),
                            )
                            .on_hover_text(t("doc_aosr_hint"))
                            .on_disabled_hover_text(t("doc_only_hidden"))
                            .clicked()
                        {
                            make_aosr = Some(d.id);
                        }
                        if ui
                            .small_button(RichText::new("x").color(theme::danger()))
                            .clicked()
                        {
                            removed = Some(d.id);
                        }
                        ui.end_row();

                        if changed {
                            edited = Some(d);
                        }
                    }
                });
        });

    if let Some(id) = make_aosr {
        write_aosr(app, id);
    }
    if let Some(d) = edited {
        app.db.update_exec_doc(&d);
        if let Some(slot) = app.exec_docs.iter_mut().find(|x| x.id == d.id) {
            *slot = d;
        }
    }
    if let Some(id) = removed {
        app.db.del("exec_doc", id);
        app.reload_modules();
    }
}

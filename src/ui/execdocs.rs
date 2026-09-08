//! «Ijro hujjatlari» ekrani (TZ IV).
//!
//! Modulning asosiy g'oyasi: tizim qurilishning har bosqichida qaysi ijro
//! hujjati rasmiylashtirilgan bo'lishi kerakligini bilishi kerak. Shuning
//! uchun ro'yxat yonida «tugallangan, lekin hujjatsiz ishlar» paneli turadi.

use super::*;
use crate::domain::NoteTarget;
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

    let tab_key = egui::Id::new("ed_tab");
    let mut tab = ui.data(|d| d.get_temp::<u8>(tab_key)).unwrap_or(0);
    // Kunlik: hujjatlar ro'yxati va imzodan oldingi tekshiruv.
    super::tab_row(
        ui,
        &mut tab,
        &[(0, t("ed_tab_docs")), (1, t("ed_tab_review"))],
        &[
            (2, t("ed_tab_schemes")),
            (3, t("ed_tab_author")),
            (4, t("ed_tab_matrix")),
        ],
    );
    ui.data_mut(|d| d.insert_temp(tab_key, tab));
    ui.add_space(8.0);
    if tab == 1 {
        review_tab(ui, app);
        return;
    }
    if tab == 2 {
        schemes_tab(ui, app);
        return;
    }
    if tab == 3 {
        author_tab(ui, app);
        return;
    }
    if tab == 4 {
        matrix_tab(ui, app);
        return;
    }

    let mut add_for: Option<(Option<i64>, ExecDocKind)> = None;

    let mut make_ks2 = false;
    let mut make_ks3 = false;
    let mut make_archive = false;
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
        // Obyekt arxivi (TZ IV.27): hujjatlar reyestri.
        if ui
            .button(t("ed_archive"))
            .on_hover_text(t("ed_archive_hint"))
            .clicked()
        {
            make_archive = true;
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
    if make_archive {
        save_archive(app);
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
                version: 1,
                replaces: None,
                note: String::new(),
            });
            app.reload_modules();
        }
    }
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    super::warehouse::cell_l(ui, w, RichText::new(s).size(11.0).color(theme::muted()));
}

fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    super::warehouse::cell_r(ui, w, RichText::new(s).size(11.0).color(theme::muted()));
}

/// Obyekt arxivi reyestrini faylga yozadi (TZ IV.27).
fn save_archive(app: &mut App) {
    let Some(project) = app.project().cloned() else {
        return;
    };
    let (from, to) = super::doc_period(app);
    let file = format!("ARX-{}.xlsx", to.format("%Y-%m-%d"));
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
    match crate::docgen::write_archive(
        &path,
        &inp,
        &app.documents,
        &app.exec_docs,
        &app.inspections,
    ) {
        Ok(()) => app.notify(format!("{} {}", t("doc_saved"), path.display())),
        Err(e) => app.notify(format!("{}: {e}", t("doc_failed"))),
    }
}

// ================================================================ Matritsa

/// Hujjat matritsasi: ishlar × hujjat turlari (TZ IV.4).
///
/// Matritsa yangi talab o'ylab topmaydi — qaysi hujjat kerakligini
/// bo'lim belgilaydi. Qiymati ko'rinishda: bitta jadvalda qaysi ish
/// bo'yicha nima yetishmayotgani darhol ko'zga tashlanadi.
fn matrix_tab(ui: &mut egui::Ui, app: &mut App) {
    use crate::checks::MatrixCell;
    use crate::domain::ExecDocKind;

    let rows = app.document_matrix();
    let late = rows.iter().filter(|r| r.done && r.gaps() > 0).count();
    let gaps: usize = rows.iter().map(|r| r.gaps()).sum();

    ui.label(
        RichText::new(t("ed_matrix_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    stat_row(
        ui,
        vec![
            stat(
                t("ed_matrix_late"),
                late.to_string(),
                t("ed_matrix_late_hint"),
                if late == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("ed_matrix_gaps"),
                gaps.to_string(),
                t("ed_matrix_gaps_hint"),
                if gaps == 0 {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
        ],
    );
    ui.add_space(12.0);

    if rows.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            ui.label(
                RichText::new(t("ed_matrix_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
        return;
    }

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("ed_matrix")
                .num_columns(ExecDocKind::ALL.len() + 2)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 280.0, t("col_task"));
                    for kind in ExecDocKind::ALL {
                        head_l(ui, 110.0, kind.label());
                    }
                    head_l(ui, 90.0, t("col_status"));
                    ui.end_row();

                    for r in &rows {
                        let name = app
                            .task(r.task_id)
                            .map(|x| format!("{} {}", x.wbs, x.name))
                            .unwrap_or_default();
                        super::warehouse::cell_l(
                            ui,
                            280.0,
                            RichText::new(super::issues::truncate(&name, 38)).size(12.5),
                        );
                        for cell in &r.cells {
                            let (text, colour) = match cell {
                                MatrixCell::NotRequired => (t("mx_na"), theme::line()),
                                MatrixCell::Missing => (t("mx_missing"), theme::danger()),
                                MatrixCell::Draft => (t("mx_draft"), theme::warn()),
                                MatrixCell::Signed => (t("mx_signed"), theme::ok()),
                            };
                            super::warehouse::cell_l(
                                ui,
                                110.0,
                                RichText::new(text).size(11.5).color(colour),
                            );
                        }
                        super::warehouse::cell_l(
                            ui,
                            90.0,
                            RichText::new(if r.done {
                                t("mx_done")
                            } else {
                                t("mx_running")
                            })
                            .size(11.0)
                            .color(if r.done && r.gaps() > 0 {
                                theme::danger()
                            } else {
                                theme::muted()
                            }),
                        );
                        ui.end_row();
                    }
                });
        });
}

// ================================================================ Ijro sxemalari

/// Ijro sxemalari va ular ortidagi geodezik o'lchov (TZ IV.7).
///
/// Sxema — bu «qanday qurildi» degan hujjat, shuning uchun uning ortida
/// o'lchangan nuqtalar turishi kerak. O'lchovsiz sxemada tasdiqlanadigan
/// narsa yo'q.
fn schemes_tab(ui: &mut egui::Ui, app: &mut App) {
    use super::warehouse::{cell_l, cell_r};
    let rows = app.scheme_status();
    let ready = rows.iter().filter(|s| s.ready()).count();

    ui.label(
        RichText::new(t("ed_schemes_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    if rows.is_empty() {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("ed_schemes_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
        return;
    }

    stat_row(
        ui,
        vec![
            stat(
                t("ed_schemes_ready"),
                format!("{ready} / {}", rows.len()),
                t("ed_schemes_ready_hint"),
                if ready == rows.len() {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
            stat(
                t("ed_schemes_out"),
                rows.iter()
                    .map(|s| s.out_of_tolerance)
                    .sum::<usize>()
                    .to_string(),
                t("ed_schemes_out_hint"),
                theme::text(),
            ),
        ],
    );
    ui.add_space(12.0);

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("ed_schemes")
                .num_columns(6)
                .spacing([10.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 120.0, t("col_number"));
                    head_l(ui, 240.0, t("col_task"));
                    head_r(ui, 110.0, t("ed_schemes_points"));
                    head_r(ui, 120.0, t("ed_schemes_bad"));
                    head_r(ui, 140.0, t("ed_schemes_max"));
                    head_l(ui, 120.0, t("col_status"));
                    ui.end_row();

                    for s in &rows {
                        cell_l(ui, 120.0, RichText::new(&s.number).size(12.5));
                        cell_l(
                            ui,
                            240.0,
                            RichText::new(
                                s.task_id
                                    .and_then(|id| app.task(id))
                                    .map(|x| super::issues::truncate(&x.name, 32))
                                    .unwrap_or_default(),
                            )
                            .size(12.0),
                        );
                        cell_r(
                            ui,
                            110.0,
                            RichText::new(s.points.to_string()).size(12.0).color(
                                if s.points == 0 {
                                    theme::danger()
                                } else {
                                    theme::text()
                                },
                            ),
                        );
                        cell_r(
                            ui,
                            120.0,
                            RichText::new(s.out_of_tolerance.to_string())
                                .size(12.0)
                                .color(if s.out_of_tolerance == 0 {
                                    theme::muted()
                                } else {
                                    theme::danger()
                                }),
                        );
                        cell_r(
                            ui,
                            140.0,
                            RichText::new(if s.points == 0 {
                                t("dash").to_string()
                            } else {
                                format!("{:.3} {}", s.max_deviation, s.unit)
                            })
                            .size(12.0)
                            .color(theme::muted()),
                        );
                        cell_l(
                            ui,
                            120.0,
                            RichText::new(if s.ready() {
                                t("ed_schemes_ok")
                            } else if s.points == 0 {
                                t("ed_schemes_no_points")
                            } else {
                                t("ed_schemes_deviation")
                            })
                            .size(11.5)
                            .color(if s.ready() {
                                theme::ok()
                            } else {
                                theme::warn()
                            }),
                        );
                        ui.end_row();
                    }
                });
        });
}

// ================================================================ Mualliflik nazorati

/// Loyihachi javob berishi kerak bo'lgan ishlar (TZ IV.24).
///
/// Kabinet yangi ma'lumot yaratmaydi: boshqa modullardagi yozuvlardan
/// loyihachiga tegishlilarini yig'adi — har qator o'z ekranida ham turadi.
fn author_tab(ui: &mut egui::Ui, app: &mut App) {
    let rows = app.author_supervision();
    let late = rows.iter().filter(|a| a.late()).count();
    let mut go: Option<Screen> = None;

    ui.label(
        RichText::new(t("ed_author_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    stat_row(
        ui,
        vec![
            stat(
                t("ed_author_total"),
                rows.len().to_string(),
                t("ed_author_total_hint"),
                theme::text(),
            ),
            stat(
                t("ed_author_late"),
                late.to_string(),
                t("ed_author_late_hint"),
                if late == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
        ],
    );
    ui.add_space(12.0);

    if rows.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            ui.label(
                RichText::new(t("ed_author_none"))
                    .color(theme::ok())
                    .size(15.0),
            );
        });
        return;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for a in &rows {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("· {}", author_text(a)))
                            .size(12.0)
                            .color(if a.late() {
                                theme::danger()
                            } else {
                                theme::text()
                            }),
                    );
                    if ui.small_button(t("an_open")).clicked() {
                        go = Some(author_screen(a));
                    }
                });
            }
        });

    if let Some(s) = go {
        app.screen = s;
    }
}

/// Mualliflik nazorati ishini gapga aylantiradi.
fn author_text(a: &crate::checks::AuthorTask) -> String {
    use crate::checks::AuthorTask as A;
    match a {
        A::Issue { code, title } => format!("{code} — {title}"),
        A::Change { number, days } => {
            format!("{} {number} ({days} {})", t("ea_change"), t("days"))
        }
        A::Version { name, revision } => {
            format!("{} {name} ({revision})", t("ea_version"))
        }
        A::Inspection { number, days } => {
            format!("{} {number} ({days} {})", t("ea_inspection"), t("days"))
        }
    }
}

/// Ish qaysi ekranda yopiladi.
fn author_screen(a: &crate::checks::AuthorTask) -> Screen {
    use crate::checks::AuthorTask as A;
    match a {
        A::Issue { .. } => Screen::AiCheck,
        A::Change { .. } => Screen::Contracts,
        A::Version { .. } => Screen::Passport,
        A::Inspection { .. } => Screen::Inspections,
    }
}

// ================================================================ Tekshiruv

/// Imzolashdan oldingi tekshiruv va yashirin ishlar to'sig'i (TZ IV.16, 20).
///
/// Bu yerda yangi hisob-kitob yo'q: har bir e'tiroz boshqa modulda yozilgan
/// yozuvga tayanadi — GPR bajarilishi, texnik nazorat natijasi, laboratoriya
/// sinovi. Shuning uchun bu ekran boshqa ekran bilan ziddiyatga tushmaydi.
fn review_tab(ui: &mut egui::Ui, app: &mut App) {
    let checks = app.doc_readiness();
    let blocks = app.hidden_blocks();
    let blocking = checks.iter().filter(|c| !c.ready()).count();

    ui.label(
        RichText::new(t("ed_review_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    stat_row(
        ui,
        vec![
            stat(
                t("ed_review_blocked"),
                blocking.to_string(),
                t("ed_review_blocked_hint"),
                if blocking == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("ed_review_warn"),
                (checks.len() - blocking).to_string(),
                t("ed_review_warn_hint"),
                theme::text(),
            ),
            stat(
                t("ed_review_hidden"),
                blocks.len().to_string(),
                t("ed_review_hidden_hint"),
                if blocks.iter().any(|b| b.already_started) {
                    theme::danger()
                } else {
                    theme::text()
                },
            ),
        ],
    );
    ui.add_space(12.0);

    let mut new_version: Option<i64> = None;
    // Serverda imzolash: hujjat raqami va imzolanadigan matn.
    let mut remote_sign: Option<(String, String)> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.label(RichText::new(t("ed_review_docs")).size(13.5).strong());
            ui.add_space(6.0);
            if checks.is_empty() {
                ui.label(
                    RichText::new(t("ed_review_clean"))
                        .color(theme::ok())
                        .size(12.5),
                );
            }
            for c in &checks {
                let Some(doc) = app.exec_docs.iter().find(|d| d.id == c.doc_id) else {
                    continue;
                };
                let colour = if c.ready() {
                    theme::warn()
                } else {
                    theme::danger()
                };
                egui::Frame::new()
                    .fill(theme::card())
                    .inner_margin(10.0)
                    .corner_radius(6.0)
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "{} {} · {}",
                                    doc.kind.label(),
                                    if c.number.is_empty() {
                                        t("dash").to_string()
                                    } else {
                                        c.number.clone()
                                    },
                                    doc.status.label()
                                ))
                                .size(12.5)
                                .strong()
                                .color(colour),
                            );
                            if doc.version > 1 {
                                ui.label(
                                    RichText::new(format!("v{}", doc.version))
                                        .size(11.0)
                                        .color(theme::muted()),
                                );
                            }
                            if ui.small_button(t("ed_new_version")).clicked() {
                                new_version = Some(doc.id);
                            }
                            // Server sozlangan bo'lsagina ko'rinadi.
                            if app.sync.ready()
                                && ui
                                    .small_button(t("sync_sign_btn"))
                                    .on_hover_text(t("sync_sign_hint"))
                                    .clicked()
                            {
                                remote_sign = Some((
                                    doc.number.clone(),
                                    format!(
                                        "{} · {} · {} · v{}",
                                        doc.number,
                                        doc.name,
                                        doc.date.format("%d.%m.%Y"),
                                        doc.version
                                    ),
                                ));
                            }
                        });
                        for p in &c.problems {
                            ui.label(
                                RichText::new(format!("· {}", problem_text(p)))
                                    .size(12.0)
                                    .color(if p.blocking() {
                                        theme::danger()
                                    } else {
                                        theme::muted()
                                    }),
                            );
                        }
                    });
                ui.add_space(6.0);
            }

            // ---------- Qurilish yozuvlari bilan solishtirish (TZ IV.22) ----------
            ui.add_space(14.0);
            ui.label(RichText::new(t("ed_evidence")).size(13.5).strong());
            ui.label(
                RichText::new(t("ed_evidence_hint"))
                    .size(11.0)
                    .color(theme::muted()),
            );
            ui.add_space(4.0);
            let evidence = app.doc_evidence();
            let unsupported = evidence.iter().filter(|e| e.unsupported()).count();
            if evidence.is_empty() || unsupported == 0 {
                ui.label(
                    RichText::new(t("ed_evidence_ok"))
                        .size(12.5)
                        .color(theme::ok()),
                );
            }
            for e in evidence.iter().filter(|e| e.sources() < 2).take(10) {
                let mut have: Vec<&str> = Vec::new();
                if e.in_journal {
                    have.push(t("ed_ev_journal"));
                }
                if e.in_timesheet {
                    have.push(t("ed_ev_timesheet"));
                }
                if e.has_material {
                    have.push(t("ed_ev_material"));
                }
                ui.label(
                    RichText::new(format!(
                        "· {} — {}",
                        e.number,
                        if have.is_empty() {
                            t("ed_ev_none").to_string()
                        } else {
                            have.join(", ")
                        }
                    ))
                    .size(12.0)
                    .color(if e.unsupported() {
                        theme::danger()
                    } else {
                        theme::muted()
                    }),
                )
                .on_hover_text(
                    app.exec_docs
                        .iter()
                        .find(|d| d.id == e.doc_id)
                        .map(|d| format!("{} · {}", d.name, d.date.format("%d.%m.%Y")))
                        .unwrap_or_default(),
                );
            }

            ui.add_space(14.0);
            ui.label(RichText::new(t("ed_review_hidden")).size(13.5).strong());
            ui.add_space(6.0);
            if blocks.is_empty() {
                ui.label(
                    RichText::new(t("ed_hidden_clean"))
                        .color(theme::ok())
                        .size(12.5),
                );
            }
            for b in &blocks {
                let text = if b.already_started {
                    t("ed_hidden_violated")
                } else {
                    t("ed_hidden_waiting")
                };
                let pred_wbs = app
                    .task(b.pred_id)
                    .map(|t| t.wbs.clone())
                    .unwrap_or_default();
                ui.label(
                    RichText::new(format!(
                        "{} — {} ← {} {} ({})",
                        text,
                        super::issues::truncate(&b.task_name, 34),
                        pred_wbs,
                        super::issues::truncate(&b.pred_name, 30),
                        if b.missing {
                            t("ed_hidden_missing")
                        } else {
                            t("ed_hidden_unsigned")
                        }
                    ))
                    .size(12.0)
                    .color(if b.already_started {
                        theme::danger()
                    } else {
                        theme::warn()
                    }),
                );
            }
        });

    if let Some((number, text)) = remote_sign {
        app.sync_sign(&number, &text, "");
    }

    if let Some(id) = new_version {
        make_new_version(app, id);
    }
}

/// Kamchilikni odam o'qiydigan gapga aylantiradi.
fn problem_text(p: &crate::checks::DocProblem) -> String {
    use crate::checks::DocProblem as P;
    match p {
        P::WorkUnfinished { progress } => format!("{} ({:.0}%)", t("dp_unfinished"), progress),
        P::NoTask => t("dp_no_task").to_string(),
        P::NoInspection => t("dp_no_inspection").to_string(),
        P::InspectionFailed { number } => format!("{} — {}", t("dp_inspection_failed"), number),
        P::NoLabTest => t("dp_no_lab").to_string(),
        P::LabFailed { number } => format!("{} — {}", t("dp_lab_failed"), number),
        P::ConcreteWeak { sample } => format!("{} — {}", t("dp_concrete_weak"), sample),
        P::DatedBeforeWork { days } => format!("{} ({})", t("dp_dated_before"), days),
        P::Duplicate { number } => format!("{} — {}", t("dp_duplicate"), number),
        P::Incomplete => t("dp_incomplete").to_string(),
    }
}

/// Hujjatning yangi versiyasini yaratadi (TZ IV.19).
///
/// Eskisi o'chirilmaydi — u arxivda qoladi, chunki nima sababdan qayta
/// ishlanganini keyin ko'rsatish kerak bo'ladi.
fn make_new_version(app: &mut App, id: i64) {
    let Some(old) = app.exec_docs.iter().find(|d| d.id == id).cloned() else {
        return;
    };
    let mut fresh = old.clone();
    fresh.id = 0;
    fresh.version = old.version + 1;
    fresh.replaces = Some(old.id);
    fresh.status = ExecDocStatus::Draft;
    fresh.date = app.today;
    app.db.insert_exec_doc(&fresh);
    app.reload_modules();
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
    let mut open_notes: Option<i64> = None;

    // QR yorliq hujjat jildiga bosiladi: qog'oz papkadan bazadagi
    // yozuvga o'tish uchun.
    if ui
        .button(t("qr_labels"))
        .on_hover_text(t("qr_labels_hint"))
        .clicked()
    {
        let rows: Vec<(String, String, String)> = app
            .exec_docs
            .iter()
            .map(|d| {
                (
                    d.number.clone(),
                    format!("{} {}", d.kind.label(), d.number),
                    format!("{} · {}", d.name, d.date.format("%d.%m.%Y")),
                )
            })
            .collect();
        app.save_labels(crate::qr::Kind::Document, rows);
    }
    ui.add_space(6.0);

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
                        // Kelishuv marshruti (TZ IV.17): bosqich hujjat
                        // holatidan aniqlanadi — ikki joyda holat saqlash
                        // ularning bir-biriga zid bo'lishiga olib keladi.
                        let route = crate::checks::doc_route(&d);
                        ui.horizontal(|ui| {
                            for step in &route {
                                let (mark, colour) = if step.done {
                                    ("v", theme::ok())
                                } else if step.current {
                                    ("»", theme::accent())
                                } else {
                                    ("·", theme::muted())
                                };
                                ui.label(RichText::new(mark).size(12.0).color(colour))
                                    .on_hover_text(t(step.role_key));
                            }
                        });

                        // Hujjatga foto va izoh biriktirish (TZ IV.8).
                        if super::notes::badge(ui, app, NoteTarget::ExecDoc, d.id) {
                            open_notes = Some(d.id);
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
    super::notes::below_table(ui, app, NoteTarget::ExecDoc, open_notes);
}

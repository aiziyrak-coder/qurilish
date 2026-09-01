//! Nomuvofiqliklar bilan ishlash uchun umumiy vidjetlar (TZ II.16, III).
//!
//! Loyiha va smeta tekshiruvlari bitta ro'yxat va bitta kartochkani ishlatadi —
//! foydalanuvchi uchun ikkala ekran bir xil ko'rinadi.

use super::*;
use crate::domain::{Issue, IssueModule, IssueStatus, Severity};
use crate::model::Section;

/// Muhimlik darajasi rangli yorliq ko'rinishida.
pub fn severity_chip(ui: &mut egui::Ui, sev: Severity, width: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 17.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 4.0, sev.color().gamma_multiply(0.22));
    p.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        sev.label(),
        egui::FontId::proportional(11.0),
        sev.color(),
    );
}

/// Tekshiruv natijasi bo'yicha ko'rsatkichlar qatori.
pub fn issue_kpis(ui: &mut egui::Ui, list: &[&Issue], extra: Option<(&str, String, &str)>) {
    let total = list.len();
    let crit = list
        .iter()
        .filter(|i| i.severity == Severity::Critical)
        .count();
    let major = list
        .iter()
        .filter(|i| i.severity == Severity::Major)
        .count();
    let open = list
        .iter()
        .filter(|i| i.status == IssueStatus::Open)
        .count();

    let mut cards = vec![
        stat(
            t("kpi_issues_total"),
            total.to_string(),
            t("kpi_issues_hint"),
            theme::accent(),
        ),
        stat(
            t("kpi_issues_critical"),
            crit.to_string(),
            t("kpi_issues_critical_hint"),
            if crit == 0 {
                theme::ok()
            } else {
                theme::danger()
            },
        ),
        stat(
            t("kpi_issues_major"),
            major.to_string(),
            t("kpi_issues_major_hint"),
            if major == 0 {
                theme::ok()
            } else {
                theme::warn()
            },
        ),
        stat(
            t("kpi_issues_open"),
            open.to_string(),
            t("kpi_issues_open_hint"),
            if open == 0 {
                theme::ok()
            } else {
                theme::accent()
            },
        ),
    ];
    // Modulga xos qo'shimcha ko'rsatkich (masalan smeta summasi).
    if let Some((title, value, hint)) = extra {
        cards.push(stat(title, value, hint, theme::text()));
    }
    stat_row(ui, cards);
}

/// TZ II.15: yakuniy hisobot — bo'limlar kesimida muhimlik bo'yicha sanoq.
pub fn section_report(ui: &mut egui::Ui, list: &[&Issue]) {
    egui::CollapsingHeader::new(
        RichText::new(t("report_by_section"))
            .size(13.0)
            .color(theme::accent()),
    )
    .id_salt("section_report")
    .show(ui, |ui| {
        if list.is_empty() {
            ui.label(
                RichText::new(t("report_empty"))
                    .color(theme::muted())
                    .size(12.0),
            );
            return;
        }
        let ranks = [
            Severity::Critical,
            Severity::Major,
            Severity::Warning,
            Severity::Info,
        ];
        egui::Grid::new("section_report_grid")
            .num_columns(6)
            .spacing([14.0, 3.0])
            .striped(true)
            .show(ui, |ui| {
                ui.add_sized(
                    [70.0, 16.0],
                    egui::Label::new(
                        RichText::new(t("col_section_short"))
                            .color(theme::muted())
                            .size(11.0),
                    ),
                );
                for s in ranks {
                    ui.add_sized(
                        [80.0, 16.0],
                        egui::Label::new(RichText::new(s.label()).color(s.color()).size(11.0)),
                    );
                }
                ui.add_sized(
                    [60.0, 16.0],
                    egui::Label::new(
                        RichText::new(t("report_total"))
                            .color(theme::muted())
                            .size(11.0),
                    ),
                );
                ui.end_row();

                let mut totals = [0usize; 4];
                for sec in Section::ALL {
                    let in_sec: Vec<&&Issue> = list.iter().filter(|i| i.section == sec).collect();
                    if in_sec.is_empty() {
                        continue;
                    }
                    ui.add_sized(
                        [70.0, 16.0],
                        egui::Label::new(
                            RichText::new(sec.label())
                                .color(theme::section_color(sec.color()))
                                .size(12.0)
                                .strong(),
                        ),
                    );
                    for (k, sev) in ranks.iter().enumerate() {
                        let n = in_sec.iter().filter(|i| i.severity == *sev).count();
                        totals[k] += n;
                        ui.add_sized(
                            [80.0, 16.0],
                            egui::Label::new(
                                RichText::new(if n == 0 {
                                    t("dash").to_string()
                                } else {
                                    n.to_string()
                                })
                                .size(12.0)
                                .color(if n == 0 {
                                    theme::muted()
                                } else {
                                    sev.color()
                                }),
                            ),
                        );
                    }
                    ui.add_sized(
                        [60.0, 16.0],
                        egui::Label::new(
                            RichText::new(in_sec.len().to_string()).size(12.0).strong(),
                        ),
                    );
                    ui.end_row();
                }

                ui.add_sized(
                    [70.0, 16.0],
                    egui::Label::new(
                        RichText::new(t("report_total"))
                            .color(theme::muted())
                            .size(12.0),
                    ),
                );
                for n in totals {
                    ui.add_sized(
                        [80.0, 16.0],
                        egui::Label::new(RichText::new(n.to_string()).size(12.0).strong()),
                    );
                }
                ui.add_sized(
                    [60.0, 16.0],
                    egui::Label::new(RichText::new(list.len().to_string()).size(12.0).strong()),
                );
                ui.end_row();
            });
    });
}

/// Muhimlik, holat va matn bo'yicha filtrlar hamda tekshiruvni ishga tushirish tugmasi.
pub fn filter_bar(ui: &mut egui::Ui, app: &mut App, run_label: &str) -> bool {
    let mut run = false;
    ui.horizontal(|ui| {
        if ui
            .button(RichText::new(run_label).strong())
            .on_hover_text(t("run_check_hint"))
            .clicked()
        {
            run = true;
        }
        ui.separator();

        egui::ComboBox::from_id_salt("f_sev")
            .selected_text(match app.issue_sev {
                Some(s) => s.label().to_string(),
                None => t("all_severities").to_string(),
            })
            .width(150.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.issue_sev, None, t("all_severities"));
                for s in Severity::ALL {
                    ui.selectable_value(&mut app.issue_sev, Some(*s), s.label());
                }
            });

        egui::ComboBox::from_id_salt("f_status")
            .selected_text(match app.issue_status {
                Some(s) => s.label().to_string(),
                None => t("all_statuses").to_string(),
            })
            .width(150.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.issue_status, None, t("all_statuses"));
                for s in IssueStatus::ALL {
                    ui.selectable_value(&mut app.issue_status, Some(*s), s.label());
                }
            });

        ui.add(
            egui::TextEdit::singleline(&mut app.issue_search)
                .hint_text(t("search_issues"))
                .desired_width(200.0),
        );
    });
    run
}

/// Nomuvofiqliklar jadvali. Tanlangan yozuv identifikatorini qaytaradi.
pub fn issue_table(ui: &mut egui::Ui, app: &App, module: IssueModule, height: f32) -> Option<i64> {
    let mut picked = None;
    let list = app.filtered_issues(module);

    if list.is_empty() {
        ui.add_space(24.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(t("issues_empty")).color(theme::muted()));
        });
        return None;
    }

    egui::ScrollArea::vertical()
        .max_height(height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new(("issue_grid", module.code()))
                .num_columns(7)
                .spacing([10.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    let head = |ui: &mut egui::Ui, w: f32, s: &str| {
                        ui.add_sized(
                            [w, 16.0],
                            egui::Label::new(RichText::new(s).color(theme::muted()).size(11.0)),
                        );
                    };
                    head(ui, 74.0, t("col_code"));
                    head(ui, 96.0, t("col_severity"));
                    head(ui, 40.0, t("col_section_short"));
                    head(ui, 250.0, t("col_title"));
                    head(ui, 150.0, t("col_element"));
                    head(ui, 140.0, t("col_location"));
                    head(ui, 100.0, t("col_status"));
                    ui.end_row();

                    for issue in list {
                        let sel = app.selected_issue == Some(issue.id);
                        let code = egui::Label::new(
                            RichText::new(&issue.code)
                                .size(12.0)
                                .color(if sel { theme::accent() } else { theme::muted() })
                                .monospace(),
                        )
                        .sense(egui::Sense::click());
                        if ui.add_sized([74.0, 17.0], code).clicked() {
                            picked = Some(issue.id);
                        }

                        severity_chip(ui, issue.severity, 96.0);

                        ui.add_sized(
                            [40.0, 17.0],
                            egui::Label::new(
                                RichText::new(issue.section.label())
                                    .size(11.0)
                                    .color(theme::section_color(issue.section.color())),
                            ),
                        );

                        let title = egui::Label::new(
                            RichText::new(truncate(&issue.title, 42))
                                .size(12.0)
                                .color(theme::text())
                                .strong_if(sel),
                        )
                        .sense(egui::Sense::click());
                        if ui.add_sized([250.0, 17.0], title).clicked() {
                            picked = Some(issue.id);
                        }

                        ui.add_sized(
                            [150.0, 17.0],
                            egui::Label::new(
                                RichText::new(truncate(&issue.element, 24))
                                    .size(12.0)
                                    .color(theme::muted()),
                            ),
                        );
                        ui.add_sized(
                            [140.0, 17.0],
                            egui::Label::new(
                                RichText::new(truncate(&issue.location, 22))
                                    .size(12.0)
                                    .color(theme::muted()),
                            ),
                        );
                        ui.add_sized(
                            [100.0, 17.0],
                            egui::Label::new(
                                RichText::new(issue.status.label())
                                    .size(12.0)
                                    .color(status_color(issue.status)),
                            ),
                        );
                        ui.end_row();
                    }
                });
        });
    picked
}

fn status_color(s: IssueStatus) -> Color32 {
    match s {
        IssueStatus::Open => theme::danger(),
        IssueStatus::InWork => theme::warn(),
        IssueStatus::Fixed => theme::ok(),
        IssueStatus::Rejected => theme::muted(),
    }
}

/// Tanlangan nomuvofiqlik kartochkasi: tavsif, me'yoriy asos, tavsiya, holat.
pub fn issue_detail(ui: &mut egui::Ui, app: &mut App, height: f32) {
    let Some(id) = app.selected_issue else {
        ui.add_space(20.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(t("select_issue")).color(theme::muted()));
        });
        return;
    };
    let Some(mut issue) = app.issues.iter().find(|i| i.id == id).cloned() else {
        app.selected_issue = None;
        return;
    };
    let mut dirty = false;

    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_height(height);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        severity_chip(ui, issue.severity, 100.0);
                        ui.label(
                            RichText::new(&issue.code)
                                .monospace()
                                .size(12.0)
                                .color(theme::muted()),
                        );
                        ui.label(
                            RichText::new(if issue.auto {
                                t("issue_auto")
                            } else {
                                t("issue_manual")
                            })
                            .size(11.0)
                            .color(theme::muted()),
                        );
                    });
                    ui.add_space(4.0);
                    ui.label(RichText::new(&issue.title).size(15.0).strong());
                    ui.add_space(6.0);

                    if !issue.description.is_empty() {
                        ui.label(RichText::new(&issue.description).size(13.0));
                        ui.add_space(8.0);
                    }

                    // TZ II.17: me'yoriy asos reyestrdan keladi. Bo'sh bo'lsa buni
                    // yashirmaymiz — muhandis tekshiruvi kerakligi ochiq yoziladi.
                    let has_norm = !issue.norm_doc.trim().is_empty();
                    egui::Frame::new()
                        .fill(if has_norm {
                            theme::ok().gamma_multiply(0.10)
                        } else {
                            theme::warn().gamma_multiply(0.12)
                        })
                        .corner_radius(6)
                        .inner_margin(egui::Margin::symmetric(10, 8))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width() - 8.0);
                            ui.label(
                                RichText::new(t("issue_norm"))
                                    .size(11.0)
                                    .color(theme::muted())
                                    .strong(),
                            );
                            if has_norm {
                                ui.label(
                                    RichText::new(format!(
                                        "{} — {}",
                                        issue.norm_doc, issue.norm_clause
                                    ))
                                    .size(12.0)
                                    .strong(),
                                );
                            }
                            if !issue.norm_text.is_empty() {
                                ui.label(RichText::new(&issue.norm_text).size(12.0));
                            }
                        });
                    ui.add_space(8.0);

                    if !issue.recommendation.is_empty() {
                        ui.label(
                            RichText::new(t("issue_recommendation"))
                                .size(11.0)
                                .color(theme::muted())
                                .strong(),
                        );
                        ui.label(RichText::new(&issue.recommendation).size(13.0));
                        ui.add_space(8.0);
                    }

                    let meta = |ui: &mut egui::Ui, k: &str, v: &str| {
                        if v.trim().is_empty() {
                            return;
                        }
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [120.0, 18.0],
                                egui::Label::new(RichText::new(k).color(theme::muted()).size(12.0)),
                            );
                            ui.label(RichText::new(v).size(12.0));
                        });
                    };
                    meta(ui, t("col_element"), &issue.element);
                    meta(ui, t("col_location"), &issue.location);
                    meta(ui, t("col_sheet"), &issue.sheet);
                    meta(ui, t("issue_responsible"), &issue.responsible);
                    meta(ui, t("col_created"), &issue.created_at);

                    ui.add_space(10.0);
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(t("issue_status")).color(theme::muted()));
                        for s in IssueStatus::ALL {
                            let sel = issue.status == *s;
                            if ui
                                .selectable_label(
                                    sel,
                                    RichText::new(s.label()).color(if sel {
                                        status_color(*s)
                                    } else {
                                        theme::text()
                                    }),
                                )
                                .clicked()
                                && !sel
                            {
                                issue.status = *s;
                                dirty = true;
                            }
                        }
                    });
                    ui.label(
                        RichText::new(t("issue_status_hint"))
                            .size(11.0)
                            .color(theme::muted()),
                    );
                });
        });

    if dirty {
        app.save_issue(issue);
    }
}

/// Uzun matnni jadval katagiga sig'diradi.
pub fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// `RichText` uchun shartli qalinlashtirish — jadvalda tanlangan qatorni ajratadi.
trait StrongIf {
    fn strong_if(self, yes: bool) -> RichText;
}

impl StrongIf for RichText {
    fn strong_if(self, yes: bool) -> RichText {
        if yes {
            self.strong()
        } else {
            self
        }
    }
}

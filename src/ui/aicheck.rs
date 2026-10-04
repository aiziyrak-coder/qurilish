//! «Smeta» ekrani: loyihadan mijozga taklifgacha olti bosqich.
//!
//! Yo'l chapdan o'ngga, namunadagidek:
//!
//! 1. **Yuklash** — PDF; dastur jadvallarni joylashuv bo'yicha o'qiydi,
//!    AI har varaqni ko'rib chiqadi.
//! 2. **Savollar** — loyihada ko'rinmagan narsalar; javob tugma bilan.
//! 3. **Obyekt ma'lumoti** — varaqlardan nima olingani, nima hisoblangani.
//! 4. **Spetsifikatsiya** — bosqichlar, ishlar, materiallar; har qatorda
//!    manba belgisi.
//! 5. **Smeta** — narxlar katalogdan, ustamalar, jami.
//! 6. **Taklif** — mijozga hujjat, PDF.
//!
//! Hech bir son o'ylab topilmaydi: manbasi yo'q qator «taxmin» deb
//! belgilanadi, narxsiz qator jamiga kirmaydi va sanab ko'rsatiladi.

use super::*;
use crate::app::CheckTab;
use crate::smeta::{Origin, Source};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    if app.current.is_none() {
        empty(ui, t("no_object_selected"));
        return;
    }

    // Fon ishlari tugagan bo'lsa natijani olamiz.
    app.poll_pdf();
    app.poll_pages();
    app.poll_questions();
    app.poll_spec();
    if app.pdf_job.is_some()
        || app.pages_job.is_some()
        || app.questions_rx.is_some()
        || app.spec_job.is_some()
    {
        // Fon ipi kadr so'ramaydi — jarayon ko'rinib tursin.
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(300));
    }

    ui.horizontal(|ui| {
        stepper(ui, app);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let n = app.catalog.works.len() + app.catalog.materials.len();
            if ui
                .selectable_label(app.catalog_open, format!("{} · {n}", t("sm_catalog")))
                .on_hover_text(t("sm_catalog_hint"))
                .clicked()
            {
                app.catalog_open = !app.catalog_open;
            }
        });
    });
    ui.add_space(10.0);

    if app.catalog_open {
        catalog_view(ui, app);
        return;
    }
    if app.sketch_open {
        sketch_view(ui, app);
        return;
    }

    match app.check_tab {
        CheckTab::Upload => upload_tab(ui, app),
        CheckTab::Questions => questions_tab(ui, app),
        CheckTab::Data => data_tab(ui, app),
        CheckTab::Spec => spec_tab(ui, app),
        CheckTab::Smeta => smeta_tab(ui, app),
        CheckTab::Offer => offer_tab(ui, app),
    }
}

/// Bosqichlar chizig'i: raqam, nom, izoh; tayyori belgi bilan.
fn stepper(ui: &mut egui::Ui, app: &mut App) {
    let current = app.check_tab;
    ui.horizontal_wrapped(|ui| {
        for (i, tab) in CheckTab::ALL.into_iter().enumerate() {
            let ready = app.smeta_ready(tab);
            let active = tab == current;
            let fill = if active {
                theme::accent().gamma_multiply(0.16)
            } else {
                egui::Color32::TRANSPARENT
            };
            let resp = egui::Frame::new()
                .fill(fill)
                .corner_radius(8)
                .inner_margin(egui::Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let mark = if ready && !active && i < 5 {
                            "●".to_string()
                        } else {
                            (i + 1).to_string()
                        };
                        let circle = if active {
                            theme::accent()
                        } else if ready {
                            theme::ok()
                        } else {
                            theme::muted()
                        };
                        ui.label(RichText::new(mark).strong().color(circle).size(13.0));
                        ui.vertical(|ui| {
                            ui.label(RichText::new(tab.label()).strong().size(12.5).color(
                                if ready || active {
                                    theme::text()
                                } else {
                                    theme::muted()
                                },
                            ));
                            ui.label(RichText::new(tab.hint()).size(10.0).color(theme::muted()));
                        });
                    });
                })
                .response
                .interact(egui::Sense::click());
            if resp.clicked() && (ready || tab == CheckTab::Upload) {
                app.check_tab = tab;
            }
            if i < 5 {
                ui.label(RichText::new("—").color(theme::line()));
            }
        }
    });
}

fn empty(ui: &mut egui::Ui, msg: &str) {
    ui.add_space(40.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(msg).color(theme::muted()).size(15.0));
    });
}

fn card(ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(8)
        .inner_margin(egui::Margin::same(12))
        .show(ui, body);
}

/// Sonni qisqa yozadi: butun bo'lsa kasrsiz.
fn num(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{v:.0}")
    } else {
        format!("{v:.2}")
    }
}

fn kpi(ui: &mut egui::Ui, title: &str, value: &str, hint: &str) {
    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(8)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_min_width(170.0);
                ui.set_max_width(280.0);
                ui.label(RichText::new(title).size(11.0).color(theme::muted()));
                ui.label(
                    RichText::new(super::issues::truncate(value, 32))
                        .size(18.0)
                        .color(theme::accent()),
                );
                ui.label(RichText::new(hint).size(11.0).color(theme::muted()));
            });
        });
}

/// Manba belgisi: loyihadan / hisob / me'yor / taxmin / qo'lda.
fn badge(ui: &mut egui::Ui, source: &Source) {
    let (fill, color) = match source {
        Source::Project { .. } => (theme::ok().gamma_multiply(0.18), theme::ok()),
        Source::Calc { .. } | Source::Standard { .. } => {
            (theme::accent().gamma_multiply(0.14), theme::accent())
        }
        Source::Assumption { .. } => (theme::warn().gamma_multiply(0.18), theme::warn()),
        Source::Manual => (theme::line(), theme::muted()),
    };
    let detail = source.detail();
    let resp = egui::Frame::new()
        .fill(fill)
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(7, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(t(source.badge())).size(10.5).color(color));
        })
        .response;
    if !detail.is_empty() {
        resp.on_hover_text(detail);
    }
}

fn next_button(ui: &mut egui::Ui, app: &mut App, to: CheckTab, label: &str) {
    // Bitta qator: aks holda o'ngga tekislash qolgan butun balandlikni
    // egallab, pastdagi ro'yxat ko'rinmay qolardi.
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(app.smeta_ready(to), egui::Button::new(format!("{label} →")))
                .clicked()
            {
                app.check_tab = to;
            }
        });
    });
}

// ================================================================ 1. Yuklash

fn upload_tab(ui: &mut egui::Ui, app: &mut App) {
    let busy = app.pdf_job.is_some() || app.pages_job.is_some();
    let mut pick = false;
    let mut clear = false;
    let mut start_ai = false;
    let mut cancel = false;
    let mut sketch = false;
    let mut model: Option<String> = None;

    card(ui, |ui| {
        ui.label(RichText::new(t("sm_upload_title")).strong().size(14.0));
        ui.add_space(2.0);
        ui.label(
            RichText::new(t("sm_upload_hint"))
                .size(11.5)
                .color(theme::muted()),
        );
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!busy, egui::Button::new(t("tk_load_button")))
                .clicked()
            {
                pick = true;
            }
            if app.takeoff.is_some()
                && ui
                    .add_enabled(!busy, egui::Button::new(t("tk_clear")))
                    .on_hover_text(t("tk_clear_hint"))
                    .clicked()
            {
                clear = true;
            }
            if ui
                .add_enabled(!busy, egui::Button::new(t("sm_sketch_button")))
                .on_hover_text(t("sm_sketch_hint"))
                .clicked()
            {
                sketch = true;
            }
            ui.label(
                RichText::new(t("tk_ai_model"))
                    .size(11.5)
                    .color(theme::muted()),
            );
            egui::ComboBox::from_id_salt("sm_ai_model")
                .selected_text(app.ai_model.clone())
                .show_ui(ui, |ui| {
                    for m in crate::llm::MODELS {
                        if ui.selectable_label(app.ai_model == *m, *m).clicked() {
                            model = Some(m.to_string());
                        }
                    }
                })
                .response
                .on_hover_text(t("tk_ai_model_hint"));
        });
        if !app.llm.is_ready() {
            ui.add_space(4.0);
            ui.label(
                RichText::new(t("sm_no_key"))
                    .size(11.5)
                    .color(theme::warn()),
            );
        }

        // ---- Konveyer: fayl → AI varaqlar → savollar → tayyor.
        ui.add_space(10.0);
        let file_done = app.takeoff.is_some() && app.pdf_job.is_none();
        let ai_done = app.smeta.as_ref().is_some_and(|m| !m.digest.is_empty());
        let asked = app.smeta.as_ref().is_some_and(|m| !m.questions.is_empty());
        let steps = [
            (t("sm_conv_file"), file_done, app.pdf_job.is_some()),
            (t("sm_conv_ai"), ai_done, app.pages_job.is_some()),
            (t("sm_conv_questions"), asked, app.questions_rx.is_some()),
            (
                t("sm_conv_ready"),
                ai_done && asked && app.questions_rx.is_none(),
                false,
            ),
        ];
        ui.horizontal_wrapped(|ui| {
            for (i, (name, done, running)) in steps.iter().enumerate() {
                if *running {
                    ui.spinner();
                } else {
                    ui.label(
                        RichText::new(if *done { "●" } else { "○" })
                            .color(if *done { theme::ok() } else { theme::muted() })
                            .strong(),
                    );
                }
                ui.label(RichText::new(*name).size(12.0).color(if *done || *running {
                    theme::text()
                } else {
                    theme::muted()
                }));
                if i < 3 {
                    ui.label(RichText::new("→").color(theme::line()));
                }
            }
        });

        if let Some(job) = &app.pdf_job {
            ui.add_space(6.0);
            ui.label(
                RichText::new(format!(
                    "{} · {} · {} s",
                    t("pdf_reading"),
                    job.file,
                    job.started.elapsed().as_secs()
                ))
                .size(11.5)
                .color(theme::muted()),
            );
        } else if app.takeoff.is_none() && !app.pdf_note.is_empty() {
            ui.add_space(6.0);
            ui.label(RichText::new(&app.pdf_note).color(theme::danger()));
        }
        if let Some(job) = &app.pages_job {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let total = job.plan.len();
                let ready = job.done.len() + job.failed.len();
                ui.label(
                    RichText::new(if total == 0 {
                        format!(
                            "{} · {} s",
                            t("tk_ai_preparing"),
                            job.started.elapsed().as_secs()
                        )
                    } else {
                        format!(
                            "{} {ready} / {total} · {} s · {}",
                            t("tk_ai_reading"),
                            job.started.elapsed().as_secs(),
                            job.model
                        )
                    })
                    .size(11.5),
                );
                if ui.button(t("tk_ai_stop")).clicked() {
                    cancel = true;
                }
            });
            if !job.failed.is_empty() {
                ui.label(
                    RichText::new(format!("{} {}", job.failed.len(), t("tk_ai_failed")))
                        .size(11.5)
                        .color(theme::warn()),
                );
            }
        } else if file_done && !ai_done && app.llm.is_ready() {
            ui.add_space(6.0);
            if ui.button(t("sm_start_ai")).clicked() {
                start_ai = true;
            }
        }
        if !app.smeta_note.is_empty() && app.pages_job.is_none() {
            ui.add_space(4.0);
            ui.label(RichText::new(&app.smeta_note).size(11.5));
        }
    });

    if let Some(m) = model {
        app.set_ai_model(&m);
    }
    if sketch {
        if let Some(sk) = app.smeta.as_ref().and_then(|m| m.sketch.clone()) {
            app.sketch_draft = sk;
        }
        app.sketch_open = true;
        return;
    }
    if pick {
        if let Some(path) = rfd::FileDialog::new()
            .set_title(t("tk_load_button"))
            .add_filter("PDF", &["pdf", "PDF"])
            .pick_file()
        {
            app.start_pdf(&path);
        }
    }
    if clear {
        app.clear_takeoff();
        return;
    }
    if cancel {
        app.cancel_jobs();
    }
    // Dastur o'qib bo'lgach AI o'zi boshlanadi — odam tugma kutib
    // o'tirmasin. Kalit bo'lmasa, yuqorida sabab yozilgan.
    let fresh = app.takeoff.is_some()
        && app.pdf_job.is_none()
        && app.pages_job.is_none()
        && app.smeta.is_none()
        && app.llm.is_ready()
        && app.pdf_note.ends_with(" s");
    if start_ai || fresh {
        app.start_pages();
    }

    // ---- O'qilgan varaqlar.
    let (digest_len, facts_len, tokens, model_name) = match &app.smeta {
        Some(m) if !m.digest.is_empty() => {
            (m.digest.len(), m.facts.len(), m.tokens, m.model.clone())
        }
        _ => return,
    };
    let (pages_total, tables_total) = app
        .takeoff
        .as_ref()
        .map(|t| (t.pages, t.tables.len()))
        .unwrap_or((0, 0));
    ui.add_space(10.0);
    ui.horizontal_wrapped(|ui| {
        kpi(
            ui,
            t("sm_pages_read"),
            &digest_len.to_string(),
            &format!("{pages_total} {}", t("tk_pages")),
        );
        kpi(
            ui,
            t("sm_facts"),
            &facts_len.to_string(),
            t("sm_facts_hint"),
        );
        kpi(
            ui,
            t("tk_tables"),
            &tables_total.to_string(),
            t("tk_tables_hint"),
        );
        kpi(ui, t("tk_ai_tokens"), &tokens.to_string(), &model_name);
    });
    ui.add_space(6.0);
    next_button(ui, app, CheckTab::Questions, t("sm_tab_questions"));
    let Some(m) = &app.smeta else { return };
    ui.add_space(6.0);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("sm_pages")
                .striped(true)
                .spacing([14.0, 4.0])
                .show(ui, |ui| {
                    for h in [
                        "pdf_page",
                        "sm_col_kind",
                        "sm_col_sheet",
                        "sm_facts",
                        "tk_tables",
                    ] {
                        ui.label(RichText::new(t(h)).size(11.0).color(theme::muted()));
                    }
                    ui.end_row();
                    for d in &m.digest {
                        ui.label(d.page.to_string());
                        ui.label(RichText::new(&d.kind).strong());
                        ui.label(super::issues::truncate(&d.sheet, 70));
                        ui.label(d.facts.len().to_string());
                        ui.label(
                            app.takeoff
                                .as_ref()
                                .map(|t| t.tables.iter().filter(|x| x.page == d.page).count())
                                .unwrap_or(0)
                                .to_string(),
                        );
                        ui.end_row();
                    }
                });
        });
}

// =============================================================== 2. Savollar

fn questions_tab(ui: &mut egui::Ui, app: &mut App) {
    let Some(m) = app.smeta.clone() else {
        empty(ui, t("tk_not_loaded"));
        return;
    };
    let mut answer: Option<(usize, String)> = None;
    let mut refresh = false;
    let mut build = false;

    card(ui, |ui| {
        ui.label(RichText::new(t("sm_questions_title")).strong().size(14.0));
        ui.label(
            RichText::new(t("sm_questions_hint"))
                .size(11.5)
                .color(theme::muted()),
        );
        ui.add_space(6.0);
        let total = m.questions.len();
        let done = m.answered();
        ui.horizontal(|ui| {
            if app.questions_rx.is_some() {
                ui.spinner();
                ui.label(RichText::new(t("sm_questions_wait")).color(theme::muted()));
            } else {
                ui.label(format!("{} {done} / {total}", t("sm_answered")));
                if ui.button(t("sm_questions_refresh")).clicked() {
                    refresh = true;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = if m.stages.is_empty() {
                    t("sm_build_spec")
                } else {
                    t("sm_rebuild_spec")
                };
                if ui
                    .add_enabled(
                        app.spec_job.is_none() && !m.digest.is_empty() && app.llm.is_ready(),
                        egui::Button::new(label),
                    )
                    .on_hover_text(t("sm_build_spec_hint"))
                    .clicked()
                {
                    build = true;
                }
                if !m.stages.is_empty() && ui.button(format!("{} →", t("sm_tab_spec"))).clicked()
                {
                    app.check_tab = CheckTab::Spec;
                }
            });
        });
        if total > 0 {
            let frac = done as f32 / total as f32;
            ui.add(egui::ProgressBar::new(frac).desired_height(6.0));
        }
        if let Some(job) = &app.spec_job {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!(
                    "{} {} / {} · {} s",
                    t("sm_spec_building"),
                    job.done.len() + job.failed.len(),
                    job.plan.len(),
                    job.started.elapsed().as_secs()
                ));
            });
        }
        if !app.smeta_note.is_empty() {
            ui.label(
                RichText::new(&app.smeta_note)
                    .size(11.0)
                    .color(theme::muted()),
            );
        }
    });

    ui.add_space(8.0);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if !m.summary.is_empty() {
                card(ui, |ui| {
                    ui.label(
                        RichText::new(t("sm_summary"))
                            .size(11.0)
                            .color(theme::muted()),
                    );
                    ui.label(RichText::new(&m.summary).size(12.5));
                });
                ui.add_space(6.0);
            }
            for (i, q) in m.questions.iter().enumerate() {
                card(ui, |ui| {
                    ui.horizontal(|ui| {
                        let answered = !q.answer.trim().is_empty();
                        ui.label(RichText::new(if answered { "●" } else { "○" }).color(
                            if answered {
                                theme::ok()
                            } else {
                                theme::muted()
                            },
                        ));
                        egui::Frame::new()
                            .fill(theme::line())
                            .corner_radius(8)
                            .inner_margin(egui::Margin::symmetric(6, 1))
                            .show(ui, |ui| {
                                ui.label(RichText::new(&q.topic).size(10.5));
                            });
                        ui.label(RichText::new(&q.text).size(12.5).strong());
                    });
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        for o in &q.options {
                            let on = q.answer == *o;
                            if ui.selectable_label(on, o).clicked() {
                                answer = Some((i, if on { String::new() } else { o.clone() }));
                            }
                        }
                        // Variantlardan tashqari javob.
                        let mut text = if q.options.contains(&q.answer) {
                            String::new()
                        } else {
                            q.answer.clone()
                        };
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut text)
                                .hint_text(t("sm_answer_other"))
                                .desired_width(260.0),
                        );
                        if resp.lost_focus() && !text.trim().is_empty() {
                            answer = Some((i, text.trim().to_string()));
                        }
                    });
                });
                ui.add_space(4.0);
            }
        });

    if let Some((i, a)) = answer {
        app.edit_smeta(|m| {
            if let Some(q) = m.questions.get_mut(i) {
                q.answer = a;
            }
        });
    }
    if refresh {
        app.start_questions();
    }
    if build {
        app.start_spec();
    }
}

// ========================================================= 3. Obyekt ma'lumoti

fn data_tab(ui: &mut egui::Ui, app: &mut App) {
    let Some(m) = app.smeta.clone() else {
        empty(ui, t("tk_not_loaded"));
        return;
    };
    let mut count_edit: Option<(String, f64)> = None;

    ui.horizontal_wrapped(|ui| {
        kpi(
            ui,
            t("sm_facts"),
            &m.facts.len().to_string(),
            t("sm_facts_hint"),
        );
        kpi(
            ui,
            t("sm_from_project"),
            &app.cost_rows.len().to_string(),
            t("sm_from_project_hint"),
        );
        kpi(
            ui,
            t("tab_constructs"),
            &app.takeoff
                .as_ref()
                .map(|t| t.constructs.len())
                .unwrap_or(0)
                .to_string(),
            t("tk_constructs_hint"),
        );
    });
    ui.add_space(6.0);
    next_button(ui, app, CheckTab::Spec, t("sm_tab_spec"));
    ui.add_space(6.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if !m.summary.is_empty() {
                card(ui, |ui| {
                    ui.label(RichText::new(&m.summary).size(12.5));
                });
                ui.add_space(8.0);
            }

            // ---- Ko'rsatkichlar.
            egui::CollapsingHeader::new(format!("{} ({})", t("sm_facts"), m.facts.len()))
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("sm_facts")
                        .striped(true)
                        .spacing([14.0, 3.0])
                        .show(ui, |ui| {
                            for f in &m.facts {
                                ui.label(RichText::new(&f.name).size(12.0));
                                ui.label(RichText::new(format!("{} {}", f.value, f.unit)).strong());
                                ui.label(
                                    RichText::new(format!(
                                        "{} {}",
                                        t("pdf_page"),
                                        f.page.unwrap_or(0)
                                    ))
                                    .size(11.0)
                                    .color(theme::muted()),
                                );
                                ui.end_row();
                            }
                        });
                });

            // ---- Loyihadan olingan materiallar.
            ui.add_space(6.0);
            egui::CollapsingHeader::new(format!(
                "{} ({})",
                t("sm_from_project"),
                app.cost_rows.len()
            ))
            .default_open(true)
            .show(ui, |ui| {
                ui.label(
                    RichText::new(t("sm_from_project_note"))
                        .size(11.0)
                        .color(theme::muted()),
                );
                egui::Grid::new("sm_mat")
                    .striped(true)
                    .spacing([14.0, 3.0])
                    .show(ui, |ui| {
                        for h in [
                            "tk_col_material",
                            "tk_col_amount",
                            "pdf_page",
                            "sm_col_status",
                        ] {
                            ui.label(RichText::new(t(h)).size(11.0).color(theme::muted()));
                        }
                        ui.end_row();
                        for r in &app.cost_rows {
                            let mut pages: Vec<usize> = app
                                .takeoff_lines
                                .iter()
                                .filter(|l| {
                                    l.item.material == r.total.material
                                        && l.item.unit == r.total.unit
                                })
                                .map(|l| l.page)
                                .collect();
                            pages.sort_unstable();
                            pages.dedup();
                            ui.label(RichText::new(&r.total.material).size(12.0));
                            ui.label(
                                RichText::new(format!("{} {}", num(r.total.amount), r.total.unit))
                                    .strong(),
                            );
                            ui.label(
                                RichText::new(
                                    pages
                                        .iter()
                                        .map(|p| p.to_string())
                                        .collect::<Vec<_>>()
                                        .join(", "),
                                )
                                .size(11.0)
                                .color(theme::muted()),
                            );
                            badge(ui, &Source::Project { page: None });
                            ui.end_row();
                        }
                    });
            });

            // ---- Konstruksiyalar soni — shu yerda tuzatiladi.
            if let Some(tk) = &app.takeoff {
                ui.add_space(6.0);
                egui::CollapsingHeader::new(format!(
                    "{} ({})",
                    t("tab_constructs"),
                    tk.constructs.len()
                ))
                .show(ui, |ui| {
                    egui::Grid::new("sm_cons")
                        .striped(true)
                        .spacing([14.0, 3.0])
                        .show(ui, |ui| {
                            for c in &tk.constructs {
                                ui.label(RichText::new(&c.mark).strong());
                                ui.label(super::issues::truncate(&c.name, 50));
                                let mut v = c.count;
                                if ui
                                    .add(egui::DragValue::new(&mut v).speed(1.0).range(0.0..=1e6))
                                    .changed()
                                {
                                    count_edit = Some((c.mark.clone(), v));
                                }
                                ui.label(RichText::new(&c.unit).color(theme::muted()));
                                ui.label(
                                    RichText::new(format!("{} {}", t("pdf_page"), c.page))
                                        .size(11.0)
                                        .color(theme::muted()),
                                );
                                ui.end_row();
                            }
                        });
                });
            }
        });

    if let Some((mark, v)) = count_edit {
        app.set_construct_count(&mark, v);
    }
}

// ========================================================= 4. Spetsifikatsiya

fn spec_tab(ui: &mut egui::Ui, app: &mut App) {
    let Some(m) = app.smeta.clone() else {
        empty(ui, t("tk_not_loaded"));
        return;
    };
    if m.stages.is_empty() {
        if app.spec_job.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(t("sm_spec_building"));
            });
        } else {
            empty(ui, t("sm_no_spec"));
            ui.vertical_centered(|ui| {
                if ui.button(t("sm_build_spec")).clicked() {
                    app.start_spec();
                }
            });
        }
        return;
    }

    let works: usize = m.stages.iter().map(|s| s.works.len()).sum();
    let materials: usize = m
        .stages
        .iter()
        .flat_map(|s| &s.works)
        .map(|w| w.materials.len())
        .sum();
    ui.horizontal_wrapped(|ui| {
        kpi(
            ui,
            t("sm_stages"),
            &m.stages.len().to_string(),
            t("sm_stages_hint"),
        );
        kpi(ui, t("sm_works"), &works.to_string(), "");
        kpi(ui, t("sm_materials"), &materials.to_string(), "");
        kpi(
            ui,
            t("sm_src_assumption"),
            &m.assumptions().to_string(),
            t("sm_assumptions_hint"),
        );
    });
    ui.add_space(6.0);
    let mut rebuild = false;
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                app.spec_job.is_none(),
                egui::Button::new(t("sm_rebuild_spec")),
            )
            .on_hover_text(t("sm_rebuild_hint"))
            .clicked()
        {
            rebuild = true;
        }
        if let Some(job) = &app.spec_job {
            ui.spinner();
            ui.label(format!(
                "{} {} / {}",
                t("sm_spec_building"),
                job.done.len() + job.failed.len(),
                job.plan.len()
            ));
        }
        ui.label(
            RichText::new(t("sm_spec_legend"))
                .size(11.0)
                .color(theme::muted()),
        );
        next_button(ui, app, CheckTab::Smeta, t("sm_tab_smeta"));
    });
    if rebuild {
        app.start_spec();
    }
    ui.add_space(6.0);

    let mut qty_edit: Option<(usize, usize, Option<usize>, f64)> = None;
    let mut remove: Option<(usize, usize)> = None;
    let mut open = app.smeta_open;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (si, st) in m.stages.iter().enumerate() {
                let is_open = open == Some(si);
                let mats: usize = st.works.iter().map(|w| w.materials.len()).sum();
                let head = ui.add(
                    egui::Label::new(
                        RichText::new(format!(
                            "{}  {}   ·   {} {} · {} {}",
                            if is_open { "-" } else { "+" },
                            st.name,
                            st.works.len(),
                            t("sm_works_short"),
                            mats,
                            t("sm_materials_short")
                        ))
                        .size(13.5)
                        .strong(),
                    )
                    .sense(egui::Sense::click()),
                );
                if head.clicked() {
                    open = if is_open { None } else { Some(si) };
                }
                if !is_open {
                    ui.add_space(4.0);
                    continue;
                }
                egui::Grid::new(("sm_spec", si))
                    .striped(true)
                    .spacing([12.0, 4.0])
                    .min_col_width(40.0)
                    .show(ui, |ui| {
                        for h in [
                            "sm_col_work",
                            "tk_col_amount",
                            "tk_col_unit",
                            "sm_col_source",
                            "",
                            "sm_col_material",
                            "tk_col_amount",
                            "tk_col_unit",
                            "sm_col_source",
                        ] {
                            let text = if h.is_empty() { "" } else { t(h) };
                            ui.label(RichText::new(text).size(11.0).color(theme::muted()));
                        }
                        ui.end_row();
                        for (wi, w) in st.works.iter().enumerate() {
                            let rows = w.materials.len().max(1);
                            for mi in 0..rows {
                                if mi == 0 {
                                    ui.label(RichText::new(&w.name).size(12.0));
                                    let mut v = w.qty;
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut v)
                                                .speed(0.1)
                                                .range(0.0..=1e9),
                                        )
                                        .changed()
                                    {
                                        qty_edit = Some((si, wi, None, v));
                                    }
                                    ui.label(RichText::new(&w.unit).color(theme::muted()));
                                    badge(ui, &w.source);
                                    if ui
                                        .small_button("×")
                                        .on_hover_text(t("sm_remove_work"))
                                        .clicked()
                                    {
                                        remove = Some((si, wi));
                                    }
                                } else {
                                    for _ in 0..5 {
                                        ui.label("");
                                    }
                                }
                                match w.materials.get(mi) {
                                    Some(mat) => {
                                        ui.label(RichText::new(&mat.name).size(12.0));
                                        let mut v = mat.qty;
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut v)
                                                    .speed(0.1)
                                                    .range(0.0..=1e9),
                                            )
                                            .changed()
                                        {
                                            qty_edit = Some((si, wi, Some(mi), v));
                                        }
                                        ui.label(RichText::new(&mat.unit).color(theme::muted()));
                                        badge(ui, &mat.source);
                                    }
                                    None => {
                                        for _ in 0..4 {
                                            ui.label("");
                                        }
                                    }
                                }
                                ui.end_row();
                            }
                        }
                    });
                ui.add_space(8.0);
            }
        });
    app.smeta_open = open;

    if let Some((si, wi, mi, v)) = qty_edit {
        app.edit_smeta(|m| {
            if let Some(w) = m.stages.get_mut(si).and_then(|s| s.works.get_mut(wi)) {
                match mi {
                    Some(mi) => {
                        if let Some(mat) = w.materials.get_mut(mi) {
                            mat.qty = v.max(0.0);
                            mat.source = Source::Manual;
                        }
                    }
                    None => {
                        w.qty = v.max(0.0);
                        w.source = Source::Manual;
                    }
                }
            }
        });
    }
    if let Some((si, wi)) = remove {
        app.edit_smeta(|m| {
            if let Some(s) = m.stages.get_mut(si) {
                if wi < s.works.len() {
                    s.works.remove(wi);
                }
            }
        });
    }
}

// ================================================================== 5. Smeta

fn price_text(p: &crate::smeta::Priced) -> RichText {
    match p.sum {
        Some(v) => RichText::new(money(v)).size(12.0),
        None => RichText::new(t("dash")).color(theme::warn()),
    }
}

fn smeta_tab(ui: &mut egui::Ui, app: &mut App) {
    let Some(m) = app.smeta.clone() else {
        empty(ui, t("tk_not_loaded"));
        return;
    };
    if m.stages.is_empty() {
        empty(ui, t("sm_no_spec"));
        return;
    }
    let view = app.smeta_view.clone();

    // ---- Jami.
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(t("sm_total"))
                        .size(11.0)
                        .color(theme::muted()),
                );
                ui.label(
                    RichText::new(format!("{} {}", money(view.total), t("tk_sum_unit")))
                        .size(24.0)
                        .strong()
                        .color(theme::accent()),
                );
                ui.label(
                    RichText::new(format!(
                        "{} {} · {} {}",
                        t("sm_works"),
                        money(view.work_sum),
                        t("sm_materials"),
                        money(view.material_sum)
                    ))
                    .size(11.5)
                    .color(theme::muted()),
                );
                ui.label(if view.missing > 0 {
                    RichText::new(format!("{} {}", view.missing, t("tk_total_partial")))
                        .size(11.5)
                        .color(theme::warn())
                } else {
                    RichText::new(t("tk_total_full"))
                        .size(11.5)
                        .color(theme::muted())
                });
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(format!("{} →", t("sm_tab_offer"))).clicked() {
                    app.check_tab = CheckTab::Offer;
                }
                if ui.button(t("tk_export_pdf")).clicked() {
                    export_smeta(app, true);
                }
                if ui.button(t("tk_export_excel")).clicked() {
                    export_smeta(app, false);
                }
            });
        });
    });
    ui.add_space(6.0);

    // ---- Tahrir qayerga yoziladi.
    let mut scope: Option<bool> = None;
    let mut markup: Option<f64> = None;
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(t("sm_edit_scope"))
                .size(11.5)
                .color(theme::muted()),
        );
        if ui
            .selectable_label(!app.price_to_catalog, t("sm_scope_smeta"))
            .on_hover_text(t("sm_scope_smeta_hint"))
            .clicked()
        {
            scope = Some(false);
        }
        if ui
            .selectable_label(app.price_to_catalog, t("sm_scope_catalog"))
            .on_hover_text(t("sm_scope_catalog_hint"))
            .clicked()
        {
            scope = Some(true);
        }
        ui.add_space(16.0);
        ui.label(
            RichText::new(t("sm_markup_all"))
                .size(11.5)
                .color(theme::muted()),
        );
        let mut v = m.markup;
        if ui
            .add(
                egui::DragValue::new(&mut v)
                    .speed(0.5)
                    .range(-50.0..=200.0)
                    .suffix(" %"),
            )
            .changed()
        {
            markup = Some(v);
        }
        ui.label(
            RichText::new(format!(
                "{}: {}",
                t("sm_catalog_size"),
                app.catalog.works.len() + app.catalog.materials.len()
            ))
            .size(11.0)
            .color(theme::muted()),
        );
    });
    if let Some(s) = scope {
        app.set_price_scope(s);
    }
    if let Some(v) = markup {
        app.edit_smeta(|m| m.markup = v);
    }
    ui.add_space(6.0);

    let mut price_edit: Option<(usize, usize, Option<usize>, f64)> = None;
    let mut stage_markup: Option<(usize, f64)> = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (si, st) in m.stages.iter().enumerate() {
                let Some(sv) = view.stages.get(si) else {
                    continue;
                };
                egui::Frame::new()
                    .fill(theme::accent().gamma_multiply(0.10))
                    .corner_radius(6)
                    .inner_margin(egui::Margin::symmetric(8, 5))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&st.name).size(13.5).strong());
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "{} {}",
                                            money(sv.total),
                                            t("tk_sum_unit")
                                        ))
                                        .size(13.0)
                                        .strong(),
                                    );
                                    let mut v = st.markup;
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut v)
                                                .speed(0.5)
                                                .range(-50.0..=200.0)
                                                .suffix(" %"),
                                        )
                                        .on_hover_text(t("sm_markup_stage"))
                                        .changed()
                                    {
                                        stage_markup = Some((si, v));
                                    }
                                    ui.label(
                                        RichText::new(t("sm_markup_short"))
                                            .size(11.0)
                                            .color(theme::muted()),
                                    );
                                    let missing = if sv.missing > 0 {
                                        format!(" · {} {}", sv.missing, t("tk_no_price"))
                                    } else {
                                        String::new()
                                    };
                                    ui.label(
                                        RichText::new(format!(
                                            "{} {} · {} {}{missing}",
                                            t("sm_works"),
                                            money(sv.work_sum),
                                            t("sm_materials"),
                                            money(sv.material_sum),
                                        ))
                                        .size(11.0)
                                        .color(
                                            if sv.missing > 0 {
                                                theme::warn()
                                            } else {
                                                theme::muted()
                                            },
                                        ),
                                    );
                                },
                            );
                        });
                    });
                egui::Grid::new(("sm_price", si))
                    .striped(true)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        for h in [
                            "sm_col_work",
                            "tk_col_amount",
                            "tk_col_price",
                            "tk_col_sum",
                            "",
                            "sm_col_material",
                            "tk_col_amount",
                            "tk_col_price",
                            "tk_col_sum",
                        ] {
                            let text = if h.is_empty() { "" } else { t(h) };
                            ui.label(RichText::new(text).size(11.0).color(theme::muted()));
                        }
                        ui.end_row();
                        for (wi, w) in st.works.iter().enumerate() {
                            let Some((wp, mps)) = sv.works.get(wi) else {
                                continue;
                            };
                            let rows = w.materials.len().max(1);
                            for mi in 0..rows {
                                if mi == 0 {
                                    ui.label(RichText::new(&w.name).size(12.0));
                                    ui.label(format!("{} {}", num(w.qty), w.unit));
                                    price_cell(ui, wp, &mut |v| {
                                        price_edit = Some((si, wi, None, v))
                                    });
                                    ui.label(price_text(wp));
                                } else {
                                    for _ in 0..4 {
                                        ui.label("");
                                    }
                                }
                                ui.label("");
                                match (w.materials.get(mi), mps.get(mi)) {
                                    (Some(mat), Some(mp)) => {
                                        ui.label(RichText::new(&mat.name).size(12.0));
                                        ui.label(format!("{} {}", num(mat.qty), mat.unit));
                                        price_cell(ui, mp, &mut |v| {
                                            price_edit = Some((si, wi, Some(mi), v))
                                        });
                                        ui.label(price_text(mp));
                                    }
                                    _ => {
                                        for _ in 0..4 {
                                            ui.label("");
                                        }
                                    }
                                }
                                ui.end_row();
                            }
                        }
                    });
                ui.add_space(10.0);
            }
        });

    if let Some((si, wi, mi, v)) = price_edit {
        app.set_price(si, wi, mi, v);
    }
    if let Some((si, v)) = stage_markup {
        app.edit_smeta(|m| {
            if let Some(s) = m.stages.get_mut(si) {
                s.markup = v;
            }
        });
    }
}

/// Narx katagi: qiymat va manbasi (shu smeta / katalog / baza / yo'q).
fn price_cell(ui: &mut egui::Ui, p: &crate::smeta::Priced, on_change: &mut dyn FnMut(f64)) {
    ui.horizontal(|ui| {
        let mut v = p.price.unwrap_or(0.0);
        let resp = ui.add(
            egui::DragValue::new(&mut v)
                .speed(500.0)
                .range(0.0..=1e12)
                .custom_formatter(|v, _| money(v))
                .custom_parser(|s| {
                    s.replace([' ', '\u{a0}'], "")
                        .replace(',', ".")
                        .parse::<f64>()
                        .ok()
                }),
        );
        if resp.changed() {
            on_change(v);
        }
        let color = match p.origin {
            Origin::None => theme::warn(),
            Origin::Line => theme::accent(),
            _ => theme::muted(),
        };
        ui.label(RichText::new(t(p.origin.label())).size(10.0).color(color));
    });
}

/// Smetani faylga saqlaydi: Excel yoki PDF — ekrandagi ko'rinishdan.
fn export_smeta(app: &mut App, pdf: bool) {
    use crate::docgen::{Cell, Table};
    let Some(m) = &app.smeta else { return };
    let view = &app.smeta_view;
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    let text = |s: &str| Cell::Text(s.to_string());
    let money_cell = |v: Option<f64>| v.map(Cell::Money).unwrap_or(Cell::Empty);
    for (si, st) in m.stages.iter().enumerate() {
        let Some(sv) = view.stages.get(si) else {
            continue;
        };
        let mut head = vec![text(&st.name.to_uppercase())];
        head.resize(6, Cell::Empty);
        rows.push(head);
        for (wi, w) in st.works.iter().enumerate() {
            let Some((wp, mps)) = sv.works.get(wi) else {
                continue;
            };
            rows.push(vec![
                text(&w.name),
                text(&w.unit),
                Cell::Num(w.qty),
                money_cell(wp.price),
                money_cell(wp.sum),
                text(&format!(
                    "{} · {}",
                    t(w.source.badge()),
                    t(wp.origin.label())
                )),
            ]);
            for (mi, mat) in w.materials.iter().enumerate() {
                let Some(mp) = mps.get(mi) else { continue };
                rows.push(vec![
                    text(&format!("    {}", mat.name)),
                    text(&mat.unit),
                    Cell::Num(mat.qty),
                    money_cell(mp.price),
                    money_cell(mp.sum),
                    text(&format!(
                        "{} · {}",
                        t(mat.source.badge()),
                        t(mp.origin.label())
                    )),
                ]);
            }
        }
        rows.push(vec![
            text(&format!(
                "{} — {} ({}%)",
                t("tk_subtotal"),
                st.name,
                num(st.markup)
            )),
            Cell::Empty,
            Cell::Empty,
            Cell::Empty,
            Cell::Money(sv.total),
            if sv.missing > 0 {
                text(&format!("{} {}", sv.missing, t("tk_no_price")))
            } else {
                Cell::Empty
            },
        ]);
    }
    rows.push(vec![
        text(&format!("{} ({}%)", t("sm_total"), num(m.markup))),
        Cell::Empty,
        Cell::Empty,
        Cell::Empty,
        Cell::Money(view.total),
        if view.missing > 0 {
            text(&format!("{} {}", view.missing, t("tk_no_price")))
        } else {
            Cell::Empty
        },
    ]);
    let table = Table {
        name: t("sm_export_name").to_string(),
        headers: [
            "sm_col_work",
            "tk_col_unit",
            "tk_col_amount",
            "tk_col_price",
            "tk_col_sum",
            "sm_col_source",
        ]
        .iter()
        .map(|k| t(k).to_string())
        .collect(),
        rows,
    };
    let ext = if pdf { "pdf" } else { "xlsx" };
    let name = format!(
        "{}_{}.{ext}",
        t("sm_export_name"),
        app.today.format("%Y-%m-%d")
    );
    let Some(path) = rfd::FileDialog::new()
        .set_file_name(&name)
        .add_filter(if pdf { "PDF" } else { "Excel" }, &[ext])
        .save_file()
    else {
        return;
    };
    let result = if pdf {
        let subtitle = format!(
            "{} · {}",
            app.project().map(|p| p.name.clone()).unwrap_or_default(),
            app.today.format("%d.%m.%Y")
        );
        crate::pdf::write_table(&path, &table, &subtitle)
    } else {
        crate::docgen::write_table(&path, &table).map_err(|e| format!("{e}"))
    };
    match result {
        Ok(()) => app.notify(format!(
            "{} {} · {}",
            t("export_done"),
            table.rows.len(),
            path.display()
        )),
        Err(e) => app.notify(format!("{}: {e}", t("export_failed"))),
    }
}

// ================================================================= 6. Taklif

fn offer_tab(ui: &mut egui::Ui, app: &mut App) {
    let Some(m) = app.smeta.clone() else {
        empty(ui, t("tk_not_loaded"));
        return;
    };
    if m.stages.is_empty() {
        empty(ui, t("sm_no_spec"));
        return;
    }
    let view = app.smeta_view.clone();
    let mut offer = m.offer.clone();
    let mut changed = false;
    let mut export = false;

    ui.columns(2, |cols| {
        // ---- Chapda sozlamalar.
        card(&mut cols[0], |ui| {
            ui.label(RichText::new(t("sm_offer_settings")).strong().size(14.0));
            ui.add_space(6.0);
            let field = |ui: &mut egui::Ui, label: &str, v: &mut String, changed: &mut bool| {
                ui.label(RichText::new(label).size(11.0).color(theme::muted()));
                if ui
                    .add(egui::TextEdit::singleline(v).desired_width(f32::INFINITY))
                    .changed()
                {
                    *changed = true;
                }
            };
            field(ui, t("sm_offer_title"), &mut offer.title, &mut changed);
            field(ui, t("sm_offer_company"), &mut offer.company, &mut changed);
            field(
                ui,
                t("sm_offer_contacts"),
                &mut offer.contacts,
                &mut changed,
            );
            field(
                ui,
                t("sm_offer_customer"),
                &mut offer.customer,
                &mut changed,
            );
            ui.label(
                RichText::new(t("sm_offer_terms"))
                    .size(11.0)
                    .color(theme::muted()),
            );
            if ui
                .add(
                    egui::TextEdit::multiline(&mut offer.terms)
                        .desired_rows(4)
                        .desired_width(f32::INFINITY),
                )
                .changed()
            {
                changed = true;
            }
            ui.label(
                RichText::new(t("sm_offer_excluded"))
                    .size(11.0)
                    .color(theme::muted()),
            );
            if ui
                .add(
                    egui::TextEdit::multiline(&mut offer.excluded)
                        .desired_rows(3)
                        .desired_width(f32::INFINITY),
                )
                .changed()
            {
                changed = true;
            }
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(t("sm_offer_accent"))
                        .size(11.0)
                        .color(theme::muted()),
                );
                for (i, (name, rgb)) in crate::smeta::ACCENTS.iter().enumerate() {
                    let color = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::click());
                    ui.painter().rect_filled(rect, 4.0, color);
                    if offer.accent == i {
                        ui.painter().rect_stroke(
                            rect,
                            4.0,
                            Stroke::new(2.0_f32, theme::text()),
                            egui::StrokeKind::Outside,
                        );
                    }
                    if resp.on_hover_text(*name).clicked() {
                        offer.accent = i;
                        changed = true;
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                if ui
                    .checkbox(&mut offer.show_numbers, t("sm_offer_numbers"))
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .checkbox(&mut offer.show_schedule, t("sm_offer_schedule"))
                    .changed()
                {
                    changed = true;
                }
                ui.label(
                    RichText::new(t("sm_offer_advance"))
                        .size(11.0)
                        .color(theme::muted()),
                );
                if ui
                    .add(
                        egui::DragValue::new(&mut offer.advance_pct)
                            .range(0.0..=100.0)
                            .suffix(" %"),
                    )
                    .changed()
                {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(t("sm_offer_valid"))
                        .size(11.0)
                        .color(theme::muted()),
                );
                let mut d = offer.valid_days as i64;
                if ui
                    .add(egui::DragValue::new(&mut d).range(1..=365))
                    .changed()
                {
                    offer.valid_days = d as u32;
                    changed = true;
                }
                if ui
                    .checkbox(&mut offer.detailed, t("sm_offer_detailed"))
                    .changed()
                {
                    changed = true;
                }
            });
            ui.add_space(8.0);
            if ui.button(t("sm_offer_pdf")).clicked() {
                export = true;
            }
        });

        // ---- O'ngda hujjat ko'rinishi.
        let ui = &mut cols[1];
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Frame::new()
                    .fill(theme::canvas())
                    .stroke(Stroke::new(1.0_f32, theme::line()))
                    .inner_margin(egui::Margin::same(18))
                    .show(ui, |ui| {
                        let rgb = crate::smeta::ACCENTS
                            .get(m.offer.accent)
                            .map(|a| a.1)
                            .unwrap_or([31, 78, 160]);
                        let accent = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                        let (bar, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 6.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(bar, 2.0, accent);
                        ui.add_space(6.0);
                        for line in offer_lines(app, &m, &view) {
                            match line {
                                DocLine::Title(s) => {
                                    ui.label(RichText::new(s).size(18.0).strong().color(accent));
                                }
                                DocLine::Head(s) => {
                                    ui.add_space(6.0);
                                    ui.label(RichText::new(s).size(13.0).strong().color(accent));
                                }
                                DocLine::Text(s) => {
                                    ui.label(RichText::new(s).size(11.5));
                                }
                                DocLine::Row(a, b) => {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(a).size(11.5));
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(RichText::new(b).size(11.5).strong());
                                            },
                                        );
                                    });
                                }
                                DocLine::Muted(s) => {
                                    ui.label(RichText::new(s).size(10.5).color(theme::muted()));
                                }
                                DocLine::Gap => {
                                    ui.add_space(8.0);
                                }
                            }
                        }
                    });
            });
    });

    if changed {
        app.edit_smeta(|m| m.offer = offer.clone());
    }
    if export {
        export_offer(app);
    }
}

/// Hujjat qatori — ekran va PDF bitta ro'yxatdan chiziladi.
enum DocLine {
    Title(String),
    Head(String),
    Text(String),
    Row(String, String),
    Muted(String),
    Gap,
}

fn offer_lines(app: &App, m: &crate::smeta::Smeta, view: &crate::smeta::View) -> Vec<DocLine> {
    let o = &m.offer;
    let project = app.project().map(|p| p.name.clone()).unwrap_or_default();
    let mut out = Vec::new();
    if !o.company.is_empty() {
        out.push(DocLine::Head(o.company.clone()));
    }
    if !o.contacts.is_empty() {
        out.push(DocLine::Muted(o.contacts.clone()));
    }
    out.push(DocLine::Gap);
    out.push(DocLine::Title(if o.title.is_empty() {
        format!("{} — {project}", t("sm_offer_default_title"))
    } else {
        o.title.clone()
    }));
    if !o.customer.is_empty() {
        out.push(DocLine::Text(format!(
            "{}: {}",
            t("sm_offer_customer"),
            o.customer
        )));
    }
    out.push(DocLine::Muted(format!(
        "{} {} · {} {} {}",
        t("sm_offer_date"),
        app.today.format("%d.%m.%Y"),
        t("sm_offer_valid_until"),
        o.valid_days,
        t("sm_days")
    )));
    if !m.summary.is_empty() {
        out.push(DocLine::Gap);
        out.push(DocLine::Text(m.summary.clone()));
    }
    // Obyekt raqamlarda: varaqlardan yoki xolstdan olingan asosiy
    // ko'rsatkichlar.
    if o.show_numbers && !m.facts.is_empty() {
        out.push(DocLine::Gap);
        out.push(DocLine::Head(t("sm_offer_numbers_title").to_string()));
        for f in m.facts.iter().take(10) {
            out.push(DocLine::Row(
                f.name.clone(),
                format!("{} {}", f.value, f.unit).trim().to_string(),
            ));
        }
    }
    out.push(DocLine::Gap);
    out.push(DocLine::Head(t("sm_offer_price_title").to_string()));
    out.push(DocLine::Row(
        t("sm_total").to_string(),
        format!("{} {}", money(view.total), t("tk_sum_unit")),
    ));
    out.push(DocLine::Muted(format!(
        "{} {} · {} {}",
        t("sm_works"),
        money(view.work_sum),
        t("sm_materials"),
        money(view.material_sum)
    )));
    if view.missing > 0 {
        out.push(DocLine::Muted(format!(
            "* {} {}",
            view.missing,
            t("sm_offer_missing")
        )));
    }
    out.push(DocLine::Gap);
    out.push(DocLine::Head(t("sm_offer_stages_title").to_string()));
    for (si, st) in m.stages.iter().enumerate() {
        let Some(sv) = view.stages.get(si) else {
            continue;
        };
        out.push(DocLine::Row(
            format!("{}. {}", si + 1, st.name),
            format!(
                "{}{}",
                money(sv.total),
                if sv.missing > 0 { " *" } else { "" }
            ),
        ));
        if o.detailed {
            for w in &st.works {
                out.push(DocLine::Muted(format!(
                    "      {} — {} {}",
                    w.name,
                    num(w.qty),
                    w.unit
                )));
            }
        }
    }
    if o.show_schedule {
        let stages: Vec<(String, f64)> = m
            .stages
            .iter()
            .enumerate()
            .map(|(i, st)| {
                (
                    st.name.clone(),
                    view.stages.get(i).map(|v| v.total).unwrap_or(0.0),
                )
            })
            .collect();
        let plan = crate::smeta::schedule(&m.offer, &stages, view.total);
        if !plan.is_empty() {
            out.push(DocLine::Gap);
            out.push(DocLine::Head(t("sm_offer_schedule_title").to_string()));
            for (i, p) in plan.iter().enumerate() {
                out.push(DocLine::Row(
                    format!("{}. {} ({:.0}%)", i + 1, p.title, p.pct),
                    money(p.amount),
                ));
            }
        }
    }
    if !o.excluded.is_empty() {
        out.push(DocLine::Gap);
        out.push(DocLine::Head(t("sm_offer_excluded").to_string()));
        for l in o.excluded.lines() {
            out.push(DocLine::Text(l.to_string()));
        }
    }
    if !o.terms.is_empty() {
        out.push(DocLine::Gap);
        out.push(DocLine::Head(t("sm_offer_terms").to_string()));
        for l in o.terms.lines() {
            out.push(DocLine::Text(l.to_string()));
        }
    }
    out
}

/// Taklifni PDF ga yozadi — ekrandagi hujjat ro'yxatidan.
fn export_offer(app: &mut App) {
    let Some(m) = app.smeta.clone() else { return };
    let view = app.smeta_view.clone();
    let lines = offer_lines(app, &m, &view);
    let name = format!(
        "{}_{}.pdf",
        t("sm_offer_file"),
        app.today.format("%Y-%m-%d")
    );
    let Some(path) = rfd::FileDialog::new()
        .set_file_name(&name)
        .add_filter("PDF", &["pdf"])
        .save_file()
    else {
        return;
    };
    // Sahifaga sig'adigan qatorlar: tik A4, chekka 18 mm.
    let (w, h, margin) = (
        crate::pdf::PAGE_W_PORTRAIT,
        crate::pdf::PAGE_H_PORTRAIT,
        18.0_f32,
    );
    let step = |l: &DocLine| -> f32 {
        match l {
            DocLine::Title(_) => 10.0,
            DocLine::Head(_) => 8.0,
            DocLine::Gap => 4.0,
            _ => 5.5,
        }
    };
    let mut pages: Vec<Vec<&DocLine>> = vec![Vec::new()];
    let mut y = h - margin;
    for l in &lines {
        if y - step(l) < margin + 8.0 {
            pages.push(Vec::new());
            y = h - margin;
        }
        y -= step(l);
        if let Some(last) = pages.last_mut() {
            last.push(l);
        }
    }
    let total = pages.len();
    let rgb = crate::smeta::ACCENTS
        .get(m.offer.accent)
        .map(|a| a.1)
        .unwrap_or([31, 78, 160]);
    let result = crate::pdf::write_pages(&path, t("sm_offer_file"), total, |page, index| {
        page.bar(margin, h - margin + 3.0, w - 2.0 * margin, 2.0, rgb);
        let mut y = h - margin;
        for l in &pages[index] {
            y -= step(l);
            match l {
                DocLine::Title(s) => page.text(s, 15.0, margin, y),
                DocLine::Head(s) => page.text(s, 11.5, margin, y),
                DocLine::Text(s) => page.text(s, 9.5, margin, y),
                DocLine::Muted(s) => page.text(s, 8.5, margin, y),
                DocLine::Row(a, b) => {
                    page.text(a, 9.5, margin, y);
                    // O'ng ustun: taxminiy kenglik bo'yicha o'ngga tekis.
                    let bw = b.chars().count() as f32 * 9.5 * 0.19;
                    page.text(b, 9.5, w - margin - bw, y);
                }
                DocLine::Gap => {}
            }
        }
        page.text(
            &format!("{} {} / {total}", t("pdf_page"), index + 1),
            7.5,
            margin,
            margin - 6.0,
        );
    });
    match result {
        Ok(()) => app.notify(format!("{} · {}", t("export_done"), path.display())),
        Err(e) => app.notify(format!("{}: {e}", t("export_failed"))),
    }
}

// ================================================================= Prays

/// Kompaniya katalogi: barcha obyektlarga umumiy narxlar.
fn catalog_view(ui: &mut egui::Ui, app: &mut App) {
    let mut edit: Option<(bool, String, f64)> = None;
    let mut remove: Option<(bool, String)> = None;
    let mut import: Option<bool> = None;
    let mut add = false;
    let mut from_smeta = false;

    card(ui, |ui| {
        ui.label(RichText::new(t("sm_catalog_title")).strong().size(14.0));
        ui.label(
            RichText::new(t("sm_catalog_hint"))
                .size(11.5)
                .color(theme::muted()),
        );
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.catalog_query)
                    .hint_text(t("sm_search"))
                    .desired_width(240.0),
            );
            if ui.button(t("sm_catalog_import_m")).clicked() {
                import = Some(false);
            }
            if ui.button(t("sm_catalog_import_w")).clicked() {
                import = Some(true);
            }
            if app.smeta.as_ref().is_some_and(|m| !m.stages.is_empty())
                && ui
                    .button(t("sm_catalog_from_smeta"))
                    .on_hover_text(t("sm_catalog_from_smeta_hint"))
                    .clicked()
            {
                from_smeta = true;
            }
        });
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(t("sm_catalog_add"))
                    .size(11.5)
                    .color(theme::muted()),
            );
            let (name, unit, price, work) = &mut app.catalog_new;
            ui.add(
                egui::TextEdit::singleline(name)
                    .hint_text(t("tk_col_material"))
                    .desired_width(260.0),
            );
            ui.add(
                egui::TextEdit::singleline(unit)
                    .hint_text(t("tk_col_unit"))
                    .desired_width(60.0),
            );
            ui.add(
                egui::DragValue::new(price)
                    .speed(500.0)
                    .range(0.0..=1e12)
                    .custom_formatter(|v, _| money(v)),
            );
            ui.checkbox(work, t("sm_catalog_is_work"));
            if ui.button("+").clicked() {
                add = true;
            }
        });
    });

    ui.add_space(8.0);
    let q = app.catalog_query.to_lowercase();
    let rows: Vec<(bool, String, f64)> = app
        .catalog
        .works
        .iter()
        .map(|(k, v)| (true, k.clone(), *v))
        .chain(
            app.catalog
                .materials
                .iter()
                .map(|(k, v)| (false, k.clone(), *v)),
        )
        .filter(|r| q.is_empty() || r.1.contains(&q))
        .collect();
    if rows.is_empty() {
        empty(ui, t("sm_catalog_empty"));
    }
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("sm_catalog")
                .striped(true)
                .spacing([14.0, 4.0])
                .show(ui, |ui| {
                    for h in [
                        "sm_col_kind",
                        "tk_col_material",
                        "tk_col_unit",
                        "tk_col_price",
                        "sm_col_changed",
                        "",
                    ] {
                        let text = if h.is_empty() { "" } else { t(h) };
                        ui.label(RichText::new(text).size(11.0).color(theme::muted()));
                    }
                    ui.end_row();
                    for (work, key, price) in &rows {
                        let (name, unit) = crate::smeta::split_key(key);
                        ui.label(
                            RichText::new(t(if *work { "sm_works" } else { "sm_materials" }))
                                .size(11.0)
                                .color(theme::muted()),
                        );
                        ui.label(RichText::new(name).size(12.0));
                        ui.label(RichText::new(unit).color(theme::muted()));
                        let mut v = *price;
                        let hist = app.catalog.history(*work, key);
                        let tip = hist
                            .iter()
                            .take(6)
                            .map(|(d, p)| format!("{d}: {}", money(*p)))
                            .collect::<Vec<_>>()
                            .join("\n");
                        if ui
                            .add(
                                egui::DragValue::new(&mut v)
                                    .speed(500.0)
                                    .range(0.0..=1e12)
                                    .custom_formatter(|v, _| money(v))
                                    .custom_parser(|s| {
                                        s.replace([' ', '\u{a0}'], "")
                                            .replace(',', ".")
                                            .parse::<f64>()
                                            .ok()
                                    }),
                            )
                            .on_hover_text(format!("{}\n{tip}", t("sm_col_changed")))
                            .changed()
                        {
                            edit = Some((*work, key.clone(), v));
                        }
                        ui.label(
                            RichText::new(hist.first().map(|h| h.0.clone()).unwrap_or_default())
                                .size(11.0)
                                .color(theme::muted()),
                        );
                        if ui
                            .small_button("×")
                            .on_hover_text(t("sm_catalog_remove"))
                            .clicked()
                        {
                            remove = Some((*work, key.clone()));
                        }
                        ui.end_row();
                    }
                });
        });

    if let Some((work, key, v)) = edit {
        app.set_catalog_price(work, &key, v);
    }
    if let Some((work, key)) = remove {
        app.set_catalog_price(work, &key, 0.0);
    }
    if add {
        let (name, unit, price, work) = app.catalog_new.clone();
        app.add_catalog_item(work, &name, &unit, price);
        app.catalog_new = (String::new(), String::new(), 0.0, work);
    }
    if from_smeta {
        let n = app.smeta_to_catalog();
        app.notify(format!("{} {n}", t("sm_catalog_saved")));
    }
    if let Some(work) = import {
        if let Some(path) = rfd::FileDialog::new()
            .set_title(t("pb_import"))
            .add_filter(
                t("import_file_filter"),
                &["xlsx", "xlsm", "xls", "ods", "csv"],
            )
            .pick_file()
        {
            match app.import_catalog(&path, work) {
                Ok(n) => app.notify(format!("{} {n}", t("pb_imported"))),
                Err(e) => app.notify(format!("{}: {e}", t("import_failed"))),
            }
        }
    }
}

// ================================================================= Xolst

/// Loyihasiz obyekt: reja xolstda chiziladi.
///
/// Sichqoncha bilan nuqta qo'yiladi (0,5 m qadam), o'ng tugma oxirgisini
/// o'chiradi. Maydon va perimetr darhol hisoblanadi.
fn sketch_view(ui: &mut egui::Ui, app: &mut App) {
    const SCALE: f32 = 18.0; // piksel / metr
    const STEP: f32 = 0.5; // metr
    let mut apply = false;
    let mut close = false;

    ui.columns(2, |cols| {
        let ui = &mut cols[0];
        card(ui, |ui| {
            ui.label(RichText::new(t("sm_sketch_title")).strong().size(14.0));
            ui.label(
                RichText::new(t("sm_sketch_how"))
                    .size(11.5)
                    .color(theme::muted()),
            );
            ui.add_space(6.0);
            let size = egui::vec2(ui.available_width().max(300.0), 420.0);
            let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 4.0, theme::canvas());
            // To'r: har metr.
            let mut x = rect.left();
            let mut i = 0;
            while x <= rect.right() {
                let w = if i % 5 == 0 { 1.0_f32 } else { 0.5_f32 };
                painter.line_segment(
                    [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                    Stroke::new(w, theme::line()),
                );
                x += SCALE;
                i += 1;
            }
            let mut y = rect.top();
            i = 0;
            while y <= rect.bottom() {
                let w = if i % 5 == 0 { 1.0_f32 } else { 0.5_f32 };
                painter.line_segment(
                    [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                    Stroke::new(w, theme::line()),
                );
                y += SCALE;
                i += 1;
            }
            let to_screen =
                |p: (f32, f32)| egui::pos2(rect.left() + p.0 * SCALE, rect.bottom() - p.1 * SCALE);
            let sk = &mut app.sketch_draft;
            let pts: Vec<egui::Pos2> = sk.points.iter().map(|p| to_screen(*p)).collect();
            // Bo'yoq yo'q: kontur noqavariq (G shaklida) bo'lishi mumkin,
            // qavariq bo'yoq esa uni buzib ko'rsatardi.
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], Stroke::new(3.0_f32, theme::accent()));
            }
            if pts.len() >= 3 {
                painter.line_segment(
                    [pts[pts.len() - 1], pts[0]],
                    Stroke::new(1.5_f32, theme::accent().gamma_multiply(0.6)),
                );
            }
            for (i, p) in pts.iter().enumerate() {
                painter.circle_filled(*p, 4.0, theme::accent());
                if i + 1 < pts.len() {
                    let a = sk.points[i];
                    let b = sk.points[i + 1];
                    let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
                    let mid = egui::pos2((p.x + pts[i + 1].x) / 2.0, (p.y + pts[i + 1].y) / 2.0);
                    painter.text(
                        mid,
                        egui::Align2::CENTER_BOTTOM,
                        format!("{len:.1} m"),
                        egui::FontId::proportional(11.0),
                        theme::text(),
                    );
                }
            }
            if resp.clicked() {
                if let Some(pos) = resp.interact_pointer_pos() {
                    let mx = ((pos.x - rect.left()) / SCALE / STEP).round() * STEP;
                    let my = ((rect.bottom() - pos.y) / SCALE / STEP).round() * STEP;
                    sk.points.push((mx.max(0.0), my.max(0.0)));
                }
            }
            if resp.secondary_clicked() {
                sk.points.pop();
            }
            ui.horizontal(|ui| {
                if ui.button(t("sm_sketch_undo")).clicked() {
                    sk.points.pop();
                }
                if ui.button(t("sm_sketch_clear")).clicked() {
                    sk.points.clear();
                }
                ui.label(
                    RichText::new(format!(
                        "{}: {:.1} m² · {}: {:.1} m · {} {}",
                        t("sm_sketch_area"),
                        sk.area(),
                        t("sm_sketch_perimeter"),
                        sk.perimeter(),
                        sk.points.len(),
                        t("sm_sketch_points")
                    ))
                    .size(11.5),
                );
            });
        });

        let ui = &mut cols[1];
        card(ui, |ui| {
            ui.label(RichText::new(t("sm_sketch_params")).strong().size(14.0));
            ui.add_space(6.0);
            let sk = &mut app.sketch_draft;
            egui::Grid::new("sm_sketch_params")
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label(t("sm_sketch_floors"));
                    ui.add(egui::DragValue::new(&mut sk.floors).range(1..=30));
                    ui.end_row();
                    ui.label(t("sm_sketch_height"));
                    ui.add(
                        egui::DragValue::new(&mut sk.height)
                            .speed(0.1)
                            .range(2.0..=12.0)
                            .suffix(" m"),
                    );
                    ui.end_row();
                    ui.label(t("sm_sketch_inner"));
                    ui.add(
                        egui::DragValue::new(&mut sk.inner_walls)
                            .speed(0.5)
                            .range(0.0..=5000.0)
                            .suffix(" m"),
                    );
                    ui.end_row();
                    ui.label(t("sm_sketch_windows"));
                    ui.add(egui::DragValue::new(&mut sk.windows).range(0..=1000));
                    ui.end_row();
                    ui.label(t("sm_sketch_doors"));
                    ui.add(egui::DragValue::new(&mut sk.doors).range(0..=1000));
                    ui.end_row();
                    for (label, value, options) in [
                        (
                            "sm_sketch_roof",
                            &mut sk.roof,
                            [
                                "Скатная металлочерепица",
                                "Плоская рулонная",
                                "Сэндвич-панели",
                                "Профнастил",
                            ],
                        ),
                        (
                            "sm_sketch_foundation",
                            &mut sk.foundation,
                            [
                                "Монолитная лента",
                                "Монолитная плита",
                                "Сваи с ростверком",
                                "Столбчатый",
                            ],
                        ),
                        (
                            "sm_sketch_walls",
                            &mut sk.walls,
                            ["Кирпич", "Газобетон", "Каркас", "Сэндвич-панели"],
                        ),
                    ] {
                        ui.label(t(label));
                        ui.horizontal_wrapped(|ui| {
                            egui::ComboBox::from_id_salt(label)
                                .selected_text(if value.is_empty() {
                                    t("dash")
                                } else {
                                    value.as_str()
                                })
                                .show_ui(ui, |ui| {
                                    for o in options {
                                        ui.selectable_value(value, o.to_string(), o);
                                    }
                                });
                            ui.add(
                                egui::TextEdit::singleline(value)
                                    .desired_width(140.0)
                                    .hint_text(t("sm_answer_other")),
                            );
                        });
                        ui.end_row();
                    }
                });
            ui.add_space(8.0);
            ui.label(
                RichText::new(t("sm_sketch_note"))
                    .size(11.0)
                    .color(theme::muted()),
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        app.sketch_draft.points.len() >= 3,
                        egui::Button::new(t("sm_sketch_apply")),
                    )
                    .clicked()
                {
                    apply = true;
                }
                if ui.button(t("cancel")).clicked() {
                    close = true;
                }
            });
        });
    });

    if apply {
        let sk = app.sketch_draft.clone();
        app.apply_sketch(sk);
    }
    if close {
        app.sketch_open = false;
    }
}

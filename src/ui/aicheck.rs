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
    app.poll_prices();
    app.poll_review();
    app.poll_consolidate();
    app.poll_answers();
    app.poll_report();
    if app.pdf_job.is_some()
        || app.pages_job.is_some()
        || app.questions_rx.is_some()
        || app.spec_job.is_some()
        || app.price_job.is_some()
        || app.review_rx.is_some()
        || app.consolidate_rx.is_some()
        || app.answers_rx.is_some()
        || app.report_rx.is_some()
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
        CheckTab::Report => report_tab(ui, app),
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
                        let mark = if ready && !active && i < 6 {
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
            if i < 6 {
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
            ui.horizontal_wrapped(|ui| {
                if ui.button(t("sm_start_ai")).clicked() {
                    start_ai = true;
                }
                // Xarajat taxmini: haqiqiy loyihada bir varaq ~6 ming token.
                let pages = app.takeoff.as_ref().map(|t| t.pages).unwrap_or(0);
                ui.label(
                    RichText::new(format!(
                        "{} ~{} {}",
                        t("sm_cost_estimate"),
                        pages * 6,
                        t("sm_cost_estimate_unit")
                    ))
                    .size(11.0)
                    .color(theme::muted()),
                );
            });
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
    // O'qilgan varaqlar — hisobot uslubidagi jadval.
    let rows: Vec<Vec<String>> = m
        .digest
        .iter()
        .map(|d| {
            vec![
                format!("{} {} - {}", t("pdf_page"), d.page, d.sheet),
                d.kind.clone(),
                d.facts.len().to_string(),
                app.takeoff
                    .as_ref()
                    .map(|t| t.tables.iter().filter(|x| x.page == d.page).count())
                    .unwrap_or(0)
                    .to_string(),
            ]
        })
        .collect();
    let lines = vec![
        DocLine::Head(format!("{} ({})", t("sm_pages_read"), rows.len())),
        DocLine::Table(
            vec![
                t("sm_col_sheet").into(),
                t("sm_col_kind").into(),
                t("sm_facts").into(),
                t("tk_tables").into(),
            ],
            rows,
        ),
    ];
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Frame::new()
                .fill(theme::canvas())
                .stroke(Stroke::new(1.0_f32, theme::line()))
                .inner_margin(egui::Margin::same(14))
                .show(ui, |ui| draw_doc(ui, &lines, theme::accent()));
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
    let mut auto = false;

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
                if app.answers_rx.is_some() {
                    ui.spinner();
                } else if done < total
                    && ui
                        .add_enabled(
                            app.llm.is_ready(),
                            egui::Button::new(t("sm_answers_button")),
                        )
                        .on_hover_text(t("sm_answers_hint"))
                        .clicked()
                {
                    auto = true;
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
    if auto {
        app.start_answers();
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

            // ---- Ko'rsatkichlar va loyihadan olingan materiallar —
            // hisobotdagi jadval uslubida (tekis ustunlar, rangli sarlavha).
            {
                use crate::takeoff::Kind;
                use DocLine::*;
                let mut lines = Vec::new();
                lines.push(Head(format!("{} ({})", t("sm_facts"), m.facts.len())));
                lines.push(Table(
                    vec![
                        t("sm_col_fact").into(),
                        t("sm_col_value").into(),
                        t("sm_col_source").into(),
                    ],
                    m.facts
                        .iter()
                        .map(|f| {
                            vec![
                                f.name.clone(),
                                format!("{} {}", f.value, f.unit).trim().to_string(),
                                f.page
                                    .map(|p| format!("{} {p}", t("pdf_page")))
                                    .unwrap_or_else(|| t("sm_sketch").to_string()),
                            ]
                        })
                        .collect(),
                ));
                if !app.cost_rows.is_empty() {
                    lines.push(Head(format!(
                        "{} ({})",
                        t("sm_from_project"),
                        app.cost_rows.len()
                    )));
                    lines.push(Muted(t("sm_from_project_note").to_string()));
                    let kg = |v: f64| {
                        if v >= 1000.0 {
                            format!("{:.1} t", v / 1000.0)
                        } else {
                            format!("{} kg", num(v))
                        }
                    };
                    for kind in [Kind::Concrete, Kind::Rebar, Kind::Steel, Kind::Other] {
                        let rows: Vec<Vec<String>> = app
                            .cost_rows
                            .iter()
                            .filter(|r| r.total.kind == kind)
                            .map(|r| {
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
                                vec![
                                    r.total.material.clone(),
                                    if r.total.unit == "kg" {
                                        kg(r.total.amount)
                                    } else {
                                        format!("{} {}", num(r.total.amount), r.total.unit)
                                    },
                                    pages
                                        .iter()
                                        .map(|p| p.to_string())
                                        .collect::<Vec<_>>()
                                        .join(", "),
                                    t("sm_src_project").to_string(),
                                ]
                            })
                            .collect();
                        if rows.is_empty() {
                            continue;
                        }
                        let title = match kind {
                            Kind::Concrete => t("tk_concrete"),
                            Kind::Rebar => t("tk_rebar"),
                            Kind::Steel => t("tk_steel"),
                            Kind::Other => t("tk_other"),
                        };
                        lines.push(Sub(title.to_string()));
                        lines.push(Table(
                            vec![
                                t("tk_col_material").into(),
                                t("tk_col_amount").into(),
                                t("pdf_page").into(),
                                t("sm_col_status").into(),
                            ],
                            rows,
                        ));
                    }
                }
                egui::Frame::new()
                    .fill(theme::canvas())
                    .stroke(Stroke::new(1.0_f32, theme::line()))
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| draw_doc(ui, &lines, theme::accent()));
                ui.add_space(8.0);
            }

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
    let mut add_work: Option<usize> = None;
    let mut rename: Option<(usize, usize, String)> = None;
    let mut open = app.smeta_open;
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut app.smeta_filter)
                .hint_text(t("sm_filter_hint"))
                .desired_width(260.0),
        );
        if !app.smeta_filter.is_empty() && ui.small_button("×").clicked() {
            app.smeta_filter.clear();
        }
    });
    let filter = app.smeta_filter.to_lowercase();
    let hit = |w: &crate::smeta::Work| {
        filter.is_empty()
            || w.name.to_lowercase().contains(&filter)
            || w.materials
                .iter()
                .any(|m| m.name.to_lowercase().contains(&filter))
    };
    ui.add_space(4.0);
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
                // Filtr bo'lsa — mos ish bor bosqichlar ochiq turadi.
                let is_open = is_open || (!filter.is_empty() && st.works.iter().any(hit));
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
                            if !hit(w) {
                                continue;
                            }
                            let rows = w.materials.len().max(1);
                            for mi in 0..rows {
                                if mi == 0 {
                                    let mut name = w.name.clone();
                                    let resp = ui.add(
                                        egui::TextEdit::singleline(&mut name)
                                            .desired_width(260.0)
                                            .font(egui::TextStyle::Body),
                                    );
                                    if resp.lost_focus() && name.trim() != w.name {
                                        rename = Some((si, wi, name.trim().to_string()));
                                    }
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
                if ui.small_button(t("sm_add_work")).clicked() {
                    add_work = Some(si);
                }
                ui.add_space(8.0);
            }
        });
    app.smeta_open = open;
    if let Some(si) = add_work {
        app.edit_smeta(|m| {
            if let Some(st) = m.stages.get_mut(si) {
                st.works.push(crate::smeta::Work {
                    name: t("sm_new_work").to_string(),
                    qty: 1.0,
                    unit: "компл.".into(),
                    source: Source::Manual,
                    price: None,
                    materials: Vec::new(),
                });
            }
        });
    }

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
    if let Some((si, wi, name)) = rename {
        if !name.is_empty() {
            app.edit_smeta(|m| {
                if let Some(w) = m.stages.get_mut(si).and_then(|s| s.works.get_mut(wi)) {
                    w.name = name;
                }
            });
        }
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
                } else if view.hinted > 0 {
                    RichText::new(format!("{} {}", view.hinted, t("sm_hinted_note")))
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

    // ---- AI: narx taklifi va tekshiruv.
    let mut hints_start = false;
    let mut hints_accept = false;
    let mut review = false;
    let mut finding_apply: Option<usize> = None;
    let mut finding_dismiss: Option<usize> = None;
    card(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(t("sm_ai_prices_title")).strong());
            if let Some(job) = &app.price_job {
                ui.spinner();
                ui.label(format!(
                    "{} {} / {} · {} s",
                    t("sm_hints_running"),
                    job.got,
                    job.asked,
                    job.started.elapsed().as_secs()
                ));
            } else {
                let need = crate::smeta::unpriced(&m, &view).len();
                if ui
                    .add_enabled(
                        app.llm.is_ready() && need > 0,
                        egui::Button::new(format!("{} ({need})", t("sm_hints_button"))),
                    )
                    .on_hover_text(t("sm_hints_hint"))
                    .clicked()
                {
                    hints_start = true;
                }
            }
            if !app.price_hints.is_empty()
                && ui
                    .button(format!(
                        "{} ({})",
                        t("sm_hints_accept"),
                        app.price_hints.len()
                    ))
                    .on_hover_text(t("sm_hints_accept_hint"))
                    .clicked()
            {
                hints_accept = true;
            }
            ui.add_space(12.0);
            if app.review_rx.is_some() {
                ui.spinner();
                ui.label(RichText::new(t("ai_thinking")).color(theme::muted()));
            } else if ui
                .add_enabled(app.llm.is_ready(), egui::Button::new(t("sm_review_button")))
                .on_hover_text(t("sm_review_hint"))
                .clicked()
            {
                review = true;
            }
        });
        if view.hinted > 0 {
            ui.label(
                RichText::new(format!("{} {}", view.hinted, t("sm_hinted_note")))
                    .size(11.5)
                    .color(theme::warn()),
            );
        }
        if !app.smeta_note.is_empty() {
            ui.label(
                RichText::new(&app.smeta_note)
                    .size(11.0)
                    .color(theme::muted()),
            );
        }
        if app.consolidate_rx.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(
                    RichText::new(t("sm_consolidating"))
                        .size(11.5)
                        .color(theme::muted()),
                );
            });
        }
        if !m.review_summary.is_empty() || !m.review.is_empty() {
            ui.add_space(4.0);
            let open = m.review.iter().filter(|f| !f.done).count();
            egui::CollapsingHeader::new(format!("{} ({open})", t("sm_review_title")))
                .default_open(true)
                .show(ui, |ui| {
                    if !m.review_summary.is_empty() {
                        ui.label(RichText::new(&m.review_summary).size(12.0));
                        ui.add_space(4.0);
                    }
                    egui::ScrollArea::vertical()
                        .id_salt("sm_review")
                        .max_height(260.0)
                        .show(ui, |ui| {
                            for (i, f) in m.review.iter().enumerate() {
                                if f.done {
                                    continue;
                                }
                                ui.horizontal_wrapped(|ui| {
                                    let (label, color) = match f.kind.as_str() {
                                        "qty" => (t("sm_f_qty"), theme::warn()),
                                        "missing" => (t("sm_f_missing"), theme::accent()),
                                        "dup" => (t("sm_f_dup"), theme::danger()),
                                        "price" => (t("sm_f_price"), theme::warn()),
                                        _ => (t("sm_f_ask"), theme::muted()),
                                    };
                                    egui::Frame::new()
                                        .fill(color.gamma_multiply(0.16))
                                        .corner_radius(8)
                                        .inner_margin(egui::Margin::symmetric(6, 1))
                                        .show(ui, |ui| {
                                            ui.label(RichText::new(label).size(10.5).color(color));
                                        });
                                    if !f.id.is_empty() {
                                        ui.label(
                                            RichText::new(&f.id).size(10.5).color(theme::muted()),
                                        );
                                    }
                                    ui.label(RichText::new(&f.text).size(12.0));
                                    let action = match f.kind.as_str() {
                                        "qty" if f.qty.is_some() => Some(format!(
                                            "{} {} {}",
                                            t("sm_f_apply"),
                                            num(f.qty.unwrap_or(0.0)),
                                            f.unit
                                        )),
                                        "missing" => Some(t("sm_f_add").to_string()),
                                        "dup" => Some(t("sm_f_remove").to_string()),
                                        _ => None,
                                    };
                                    if let Some(a) = action {
                                        if ui.small_button(a).clicked() {
                                            finding_apply = Some(i);
                                        }
                                    }
                                    if ui.small_button(t("sm_f_dismiss")).clicked() {
                                        finding_dismiss = Some(i);
                                    }
                                });
                            }
                        });
                });
        }
    });
    if hints_start {
        app.start_price_hints();
    }
    if hints_accept {
        let n = app.accept_hints();
        app.notify(format!("{} {n}", t("sm_catalog_saved")));
    }
    if review {
        app.start_review();
    }
    if let Some(i) = finding_apply {
        app.apply_finding(i);
    }
    if let Some(i) = finding_dismiss {
        app.dismiss_finding(i);
    }
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
        ui.add_space(12.0);
        ui.checkbox(&mut app.smeta_only_open, t("sm_only_open"))
            .on_hover_text(t("sm_only_open_hint"));
    });
    let only_open = app.smeta_only_open;
    if let Some(s) = scope {
        app.set_price_scope(s);
    }
    if let Some(v) = markup {
        app.edit_smeta(|m| m.markup = v);
    }
    ui.add_space(6.0);

    let mut price_edit: Option<(usize, usize, Option<usize>, f64)> = None;
    let mut stage_markup: Option<(usize, f64)> = None;
    let mut hint_act: Option<(bool, String, bool)> = None;
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
                            if only_open
                                && wp.origin.trusted()
                                && mps.iter().all(|p| p.origin.trusted())
                            {
                                continue;
                            }
                            let rows = w.materials.len().max(1);
                            for mi in 0..rows {
                                if mi == 0 {
                                    ui.label(RichText::new(&w.name).size(12.0));
                                    ui.label(format!("{} {}", num(w.qty), w.unit));
                                    ui.horizontal(|ui| {
                                        price_cell(ui, wp, &mut |v| {
                                            price_edit = Some((si, wi, None, v))
                                        });
                                        if wp.origin == Origin::Hint {
                                            hint_buttons(
                                                ui,
                                                app,
                                                true,
                                                &w.name,
                                                &w.unit,
                                                &mut hint_act,
                                            );
                                        }
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
                                        ui.horizontal(|ui| {
                                            price_cell(ui, mp, &mut |v| {
                                                price_edit = Some((si, wi, Some(mi), v))
                                            });
                                            if mp.origin == Origin::Hint {
                                                hint_buttons(
                                                    ui,
                                                    app,
                                                    false,
                                                    &mat.name,
                                                    &mat.unit,
                                                    &mut hint_act,
                                                );
                                            }
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
    if let Some((work, k, accept)) = hint_act {
        app.accept_hint(work, &k, accept);
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
            Origin::None | Origin::Hint => theme::warn(),
            Origin::Line => theme::accent(),
            _ => theme::muted(),
        };
        ui.label(RichText::new(t(p.origin.label())).size(10.0).color(color));
    });
}

/// AI taxminini qabul qilish / rad etish tugmachalari.
fn hint_buttons(
    ui: &mut egui::Ui,
    app: &App,
    work: bool,
    name: &str,
    unit: &str,
    out: &mut Option<(bool, String, bool)>,
) {
    let k = crate::smeta::key(name, unit);
    let Some((_, note)) = app.price_hints.get(&k) else {
        return;
    };
    ui.horizontal(|ui| {
        if ui
            .small_button("✓")
            .on_hover_text(format!("{}\n{note}", t("sm_hint_accept")))
            .clicked()
        {
            *out = Some((work, k.clone(), true));
        }
        if ui
            .small_button("×")
            .on_hover_text(t("sm_hint_reject"))
            .clicked()
        {
            *out = Some((work, k.clone(), false));
        }
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
    // Excel: yana ikki varaq — xarid ro'yxati (ta'minot uchun) va KP
    // yig'masi; hammasi bitta hisobdan.
    let purchase = Table {
        name: t("sm_sheet_purchase").to_string(),
        headers: [
            "tk_col_material",
            "tk_col_unit",
            "tk_col_amount",
            "sm_stages",
        ]
        .iter()
        .map(|k| t(k).to_string())
        .collect(),
        rows: crate::smeta::purchase(m)
            .into_iter()
            .map(|(name, unit, qty, stages)| {
                vec![
                    Cell::Text(name),
                    Cell::Text(unit),
                    Cell::Num(qty),
                    Cell::Text(stages.join(", ")),
                ]
            })
            .collect(),
    };
    let mut summary_rows: Vec<Vec<Cell>> = m
        .stages
        .iter()
        .enumerate()
        .map(|(i, st)| {
            let sv = view.stages.get(i);
            vec![
                Cell::Text(st.name.clone()),
                Cell::Money(sv.map(|v| v.work_sum).unwrap_or(0.0)),
                Cell::Money(sv.map(|v| v.material_sum).unwrap_or(0.0)),
                Cell::Num(st.markup),
                Cell::Money(sv.map(|v| v.total).unwrap_or(0.0)),
            ]
        })
        .collect();
    summary_rows.push(vec![
        Cell::Text(t("sm_total").to_string()),
        Cell::Money(view.work_sum),
        Cell::Money(view.material_sum),
        Cell::Num(m.markup),
        Cell::Money(view.total),
    ]);
    let summary = Table {
        name: t("sm_sheet_summary").to_string(),
        headers: [
            "sm_stages",
            "sm_works",
            "sm_materials",
            "sm_markup_short",
            "tk_col_sum",
        ]
        .iter()
        .map(|k| t(k).to_string())
        .collect(),
        rows: summary_rows,
    };
    let row_count = table.rows.len();
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
        crate::docgen::write_book(&path, &[table, purchase, summary]).map_err(|e| format!("{e}"))
    };
    match result {
        Ok(()) => app.notify(format!(
            "{} {row_count} · {}",
            t("export_done"),
            path.display()
        )),
        Err(e) => app.notify(format!("{}: {e}", t("export_failed"))),
    }
}

// ================================================================= 6. Taklif

fn offer_tab(ui: &mut egui::Ui, app: &mut App) {
    // Bo'sh taklifga kompaniya rekvizitlari o'zi tushadi.
    app.offer_defaults();
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
    let mut export_word = false;
    let mut remember = false;

    // Sozlamalar yig'iladigan bo'limda, hujjat butun enda: ustunli
    // joylashuvda hujjat eni noto'g'ri hisoblanib matn chetdan chiqardi.
    egui::CollapsingHeader::new(t("sm_offer_settings"))
        .default_open(false)
        .show(ui, |ui| {
            card(ui, |ui| {
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
                            .desired_rows(2)
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
                            .desired_rows(2)
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
                ui.horizontal(|ui| {
                    if ui.button(t("sm_offer_pdf")).clicked() {
                        export = true;
                    }
                    if ui.button(t("sm_offer_docx")).clicked() {
                        export_word = true;
                    }
                    if ui
                        .button(t("sm_offer_remember"))
                        .on_hover_text(t("sm_offer_remember_hint"))
                        .clicked()
                    {
                        remember = true;
                    }
                });
            });
        });
    ui.add_space(6.0);
    let rgb = crate::smeta::ACCENTS
        .get(m.offer.accent)
        .map(|a| a.1)
        .unwrap_or([31, 78, 160]);
    let accent = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    let lines = offer_lines(app, &m, &view);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Frame::new()
                .fill(theme::canvas())
                .stroke(Stroke::new(1.0_f32, theme::line()))
                .inner_margin(egui::Margin::same(18))
                .show(ui, |ui| draw_doc(ui, &lines, accent));
        });

    if changed {
        app.edit_smeta(|m| m.offer = offer.clone());
    }
    if remember {
        app.set_company(&offer.company, &offer.contacts, &offer.terms);
        app.notify(t("sm_offer_remembered").to_string());
    }
    if export {
        export_offer(app);
    }
    if export_word {
        let rgb = crate::smeta::ACCENTS
            .get(m.offer.accent)
            .map(|a| a.1)
            .unwrap_or([31, 78, 160]);
        let lines = offer_lines(app, &m, &view);
        export_docx(app, &lines, t("sm_offer_file"), t("sm_offer_file"), rgb);
    }
}

/// Hujjat qatori — ekran va PDF bitta ro'yxatdan chiziladi.
enum DocLine {
    Title(String),
    Head(String),
    /// Kichik sarlavha (bo'lim ichida).
    Sub(String),
    Text(String),
    /// Jadval: sarlavhalar va qatorlar. Birinchi ustun matn (keng),
    /// qolganlari o'ngga tekis.
    Table(Vec<String>, Vec<Vec<String>>),
    /// Ko'rsatkich plitkalari: `(nom, qiymat, izoh)`.
    Tiles(Vec<(String, String, String)>),
    Muted(String),
    Gap,
}

/// Hujjat qatorlarini ekranga chizadi (taklif va hisobot bir xil).
fn draw_doc(ui: &mut egui::Ui, lines: &[DocLine], accent: egui::Color32) {
    // Hujjat eni berilgan joy bilan cheklanadi: matn shu enda o'raladi,
    // ustunli joylashuvda ham tashqariga chiqmaydi.
    let width = ui.available_width();
    ui.set_max_width(width);
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
    let (bar, _) = ui.allocate_exact_size(egui::vec2(width, 6.0), egui::Sense::hover());
    ui.painter().rect_filled(bar, 2.0, accent);
    ui.add_space(6.0);
    let mut table_no = 0usize;
    for line in lines {
        match line {
            DocLine::Title(s) => {
                ui.label(RichText::new(s).size(19.0).strong().color(accent));
            }
            DocLine::Head(s) => {
                ui.add_space(10.0);
                // Bo'lim sarlavhasi: chap rangli chiziq va fon — ko'z
                // bo'limlarni darrov ajratadi.
                egui::Frame::new()
                    .fill(accent.gamma_multiply(0.10))
                    .corner_radius(4)
                    .inner_margin(egui::Margin::symmetric(10, 5))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            let (r, _) =
                                ui.allocate_exact_size(egui::vec2(4.0, 16.0), egui::Sense::hover());
                            ui.painter().rect_filled(r, 2.0, accent);
                            ui.label(RichText::new(s).size(13.5).strong().color(accent));
                        });
                    });
                ui.add_space(4.0);
            }
            DocLine::Sub(s) => {
                ui.add_space(4.0);
                ui.label(RichText::new(s).size(12.0).strong());
            }
            DocLine::Text(s) => {
                ui.add(egui::Label::new(RichText::new(s).size(11.5)).wrap());
            }
            DocLine::Table(headers, rows) => {
                table_no += 1;
                let n = headers.len().max(1);
                // Jadval butun enni egallaydi; tor oynada ham ustunlar
                // siqilib ketmaydi.
                let total_w = (ui.available_width() - 8.0).max(420.0);
                let first_w = if n > 1 { total_w * 0.44 } else { total_w };
                let rest_w = if n > 1 {
                    (total_w - first_w) / (n - 1) as f32
                } else {
                    0.0
                };
                egui::Frame::new()
                    .stroke(Stroke::new(1.0_f32, theme::line()))
                    .corner_radius(6)
                    .inner_margin(egui::Margin::same(4))
                    .show(ui, |ui| {
                        egui::Grid::new(("doc_table", table_no))
                            .striped(true)
                            .spacing([0.0, 2.0])
                            .show(ui, |ui| {
                                for (i, h) in headers.iter().enumerate() {
                                    let w = if i == 0 { first_w } else { rest_w };
                                    let layout = if i == 0 {
                                        egui::Layout::left_to_right(egui::Align::Center)
                                    } else {
                                        egui::Layout::right_to_left(egui::Align::Center)
                                    };
                                    ui.allocate_ui_with_layout(egui::vec2(w, 20.0), layout, |ui| {
                                        let r = ui.max_rect();
                                        ui.painter().rect_filled(
                                            r.expand2(egui::vec2(0.0, 2.0)),
                                            0.0,
                                            accent.gamma_multiply(0.14),
                                        );
                                        ui.add_space(6.0);
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(h).size(11.0).strong().color(accent),
                                            )
                                            .truncate(),
                                        );
                                        ui.add_space(6.0);
                                    });
                                }
                                ui.end_row();
                                for row in rows {
                                    for i in 0..n {
                                        let c = row.get(i).map(String::as_str).unwrap_or("");
                                        let w = if i == 0 { first_w } else { rest_w };
                                        let layout = if i == 0 {
                                            egui::Layout::left_to_right(egui::Align::Center)
                                        } else {
                                            egui::Layout::right_to_left(egui::Align::Center)
                                        };
                                        ui.allocate_ui_with_layout(
                                            egui::vec2(w, 18.0),
                                            layout,
                                            |ui| {
                                                ui.add_space(6.0);
                                                let text = RichText::new(c).size(11.0);
                                                let text =
                                                    if i == 0 { text } else { text.strong() };
                                                ui.add(egui::Label::new(text).truncate());
                                                ui.add_space(6.0);
                                            },
                                        );
                                    }
                                    ui.end_row();
                                }
                            });
                    });
            }
            DocLine::Tiles(tiles) => {
                ui.horizontal_wrapped(|ui| {
                    for (name, value, hint) in tiles {
                        egui::Frame::new()
                            .fill(accent.gamma_multiply(0.08))
                            .stroke(Stroke::new(1.0_f32, accent.gamma_multiply(0.35)))
                            .corner_radius(8)
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.set_min_width(150.0);
                                    ui.set_max_width(260.0);
                                    ui.label(RichText::new(name).size(10.5).color(theme::muted()));
                                    ui.label(
                                        RichText::new(value).size(17.0).strong().color(accent),
                                    );
                                    if !hint.is_empty() {
                                        ui.label(
                                            RichText::new(hint).size(10.0).color(theme::muted()),
                                        );
                                    }
                                });
                            });
                    }
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
}

/// Hujjatni Word (.docx) ga yozadi — xuddi shu qatorlardan.
fn export_docx(app: &mut App, lines: &[DocLine], title: &str, file: &str, rgb: [u8; 3]) {
    use crate::docx::Block;
    let name = format!("{file}_{}.docx", app.today.format("%Y-%m-%d"));
    let Some(path) = rfd::FileDialog::new()
        .set_file_name(&name)
        .add_filter("Word", &["docx"])
        .save_file()
    else {
        return;
    };
    let blocks: Vec<Block> = lines
        .iter()
        .filter_map(|l| match l {
            DocLine::Title(s) => Some(Block::Title(s.clone())),
            DocLine::Head(s) => Some(Block::Heading(s.clone())),
            DocLine::Sub(s) => Some(Block::Sub(s.clone())),
            DocLine::Text(s) => Some(Block::Para(s.clone())),
            DocLine::Muted(s) => Some(Block::Muted(s.clone())),
            DocLine::Table(h, r) => Some(Block::Table(h.clone(), r.clone())),
            DocLine::Tiles(t) => Some(Block::Tiles(t.clone())),
            DocLine::Gap => None,
        })
        .collect();
    match crate::docx::write(&path, title, &blocks, rgb) {
        Ok(()) => app.notify(format!("{} · {}", t("export_done"), path.display())),
        Err(e) => app.notify(format!("{}: {e}", t("export_failed"))),
    }
}

/// Hujjatni PDF ga yozadi: tik A4, jadvallar va plitkalar bilan.
fn export_doc(app: &mut App, lines: &[DocLine], title: &str, file: &str, rgb: [u8; 3]) {
    let name = format!("{file}_{}.pdf", app.today.format("%Y-%m-%d"));
    let Some(path) = rfd::FileDialog::new()
        .set_file_name(&name)
        .add_filter("PDF", &["pdf"])
        .save_file()
    else {
        return;
    };
    let (w, h, margin) = (
        crate::pdf::PAGE_W_PORTRAIT,
        crate::pdf::PAGE_H_PORTRAIT,
        18.0_f32,
    );
    let width = w - 2.0 * margin;
    // Chizish birligi: oddiy qator yoki jadval qatori (sarlavha bayrog'i
    // bilan). Jadval sahifa chegarasida bo'linsa sarlavha qaytariladi.
    enum Unit<'a> {
        Line(&'a DocLine),
        Wrapped(f32, String),
        Tab {
            cols: Vec<String>,
            n: usize,
            head: bool,
        },
        Tiles(&'a [(String, String, String)]),
    }
    let step = |u: &Unit| -> f32 {
        match u {
            Unit::Line(DocLine::Title(_)) => 10.0,
            Unit::Line(DocLine::Head(_)) => 9.5,
            Unit::Line(DocLine::Sub(_)) => 6.5,
            Unit::Line(DocLine::Gap) => 4.0,
            Unit::Line(_) => 5.5,
            Unit::Wrapped(_, _) => 5.0,
            Unit::Tab { head: true, .. } => 6.5,
            Unit::Tab { .. } => 5.4,
            Unit::Tiles(_) => 16.0,
        }
    };
    let wrap = |s: &str, per: usize| -> Vec<String> {
        let mut out = Vec::new();
        for para in s.split('\n') {
            let mut cur = String::new();
            for word in para.split_whitespace() {
                if cur.chars().count() + word.chars().count() + 1 > per && !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                if !cur.is_empty() {
                    cur.push(' ');
                }
                cur.push_str(word);
            }
            out.push(cur);
        }
        out
    };
    let mut units: Vec<Unit> = Vec::new();
    for l in lines {
        match l {
            DocLine::Text(s) => {
                units.extend(wrap(s, 105).into_iter().map(|t| Unit::Wrapped(9.5, t)))
            }
            DocLine::Muted(s) => {
                units.extend(wrap(s, 125).into_iter().map(|t| Unit::Wrapped(8.0, t)))
            }
            DocLine::Table(headers, rows) => {
                let n = headers.len().max(1);
                units.push(Unit::Tab {
                    cols: headers.clone(),
                    n,
                    head: true,
                });
                for r in rows {
                    units.push(Unit::Tab {
                        cols: r.clone(),
                        n,
                        head: false,
                    });
                }
            }
            DocLine::Tiles(t) => units.push(Unit::Tiles(t)),
            other => units.push(Unit::Line(other)),
        }
    }
    let mut pages: Vec<Vec<&Unit>> = vec![Vec::new()];
    let mut y = h - margin;
    let mut last_head: Option<&Unit> = None;
    for u in &units {
        if let Unit::Tab { head: true, .. } = u {
            last_head = Some(u);
        }
        if y - step(u) < margin + 8.0 {
            pages.push(Vec::new());
            y = h - margin;
            // Yangi sahifada jadval davom etsa — sarlavha qaytadi.
            if let (Unit::Tab { head: false, .. }, Some(hd)) = (u, last_head) {
                y -= step(hd);
                if let Some(last) = pages.last_mut() {
                    last.push(hd);
                }
            }
        }
        y -= step(u);
        if let Some(last) = pages.last_mut() {
            last.push(u);
        }
        if !matches!(u, Unit::Tab { .. }) {
            last_head = None;
        }
    }
    let total = pages.len();
    let fit = |s: &str, size: f32, max: f32| -> String {
        let per = size * 0.19;
        let n = (max / per) as usize;
        if s.chars().count() <= n {
            s.to_string()
        } else {
            let cut: String = s.chars().take(n.saturating_sub(1)).collect();
            format!("{cut}…")
        }
    };
    let light = [
        (255 - (255 - rgb[0] as u16) * 15 / 100) as u8,
        (255 - (255 - rgb[1] as u16) * 15 / 100) as u8,
        (255 - (255 - rgb[2] as u16) * 15 / 100) as u8,
    ];
    let result = crate::pdf::write_pages(&path, title, total, |page, index| {
        page.bar(margin, h - margin + 3.0, width, 2.0, rgb);
        let mut y = h - margin;
        for u in &pages[index] {
            y -= step(u);
            match u {
                Unit::Line(DocLine::Title(s)) => page.text(s, 15.0, margin, y),
                Unit::Line(DocLine::Head(s)) => {
                    page.bar(margin, y - 2.0, width, 8.0, light);
                    page.bar(margin, y - 2.0, 1.2, 8.0, rgb);
                    page.text(s, 11.5, margin + 3.0, y);
                }
                Unit::Line(DocLine::Sub(s)) => page.text(s, 10.0, margin, y),
                Unit::Line(DocLine::Text(s)) => page.text(s, 9.5, margin, y),
                Unit::Line(DocLine::Muted(s)) => page.text(s, 8.0, margin, y),
                Unit::Line(_) => {}
                Unit::Wrapped(size, s) => page.text(s, *size, margin, y),
                Unit::Tab { cols, n, head } => {
                    let first = if *n > 1 { width * 0.46 } else { width };
                    let rest = if *n > 1 {
                        (width - first) / (*n - 1) as f32
                    } else {
                        0.0
                    };
                    if *head {
                        page.bar(margin, y - 1.8, width, 6.2, light);
                    }
                    let mut x = margin;
                    for i in 0..*n {
                        let c = cols.get(i).map(String::as_str).unwrap_or("");
                        let cw = if i == 0 { first } else { rest };
                        let size = 8.5_f32;
                        let text = fit(c, size, cw - 3.0);
                        if i == 0 {
                            page.text(&text, size, x + 1.5, y);
                        } else {
                            let tw = text.chars().count() as f32 * size * 0.19;
                            page.text(&text, size, x + cw - tw - 1.5, y);
                        }
                        x += cw;
                    }
                    // Qatorlar orasidagi ingichka chiziq.
                    page.bar(margin, y - 1.9, width, 0.15, [210, 214, 220]);
                }
                Unit::Tiles(tiles) => {
                    let n = tiles.len().max(1) as f32;
                    let tw = (width - 3.0 * (n - 1.0)) / n;
                    let mut x = margin;
                    for (name, value, hint) in tiles.iter() {
                        page.bar(x, y - 3.0, tw, 15.0, light);
                        page.bar(x, y - 3.0, tw, 0.6, rgb);
                        page.text(&fit(name, 7.5, tw - 4.0), 7.5, x + 2.0, y + 7.5);
                        page.text(&fit(value, 11.0, tw - 4.0), 11.0, x + 2.0, y + 2.0);
                        page.text(&fit(hint, 7.0, tw - 4.0), 7.0, x + 2.0, y - 1.8);
                        x += tw + 3.0;
                    }
                }
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

/// Taklif qatorlari — mijoz ko'radigan hujjat.
fn offer_lines(app: &App, m: &crate::smeta::Smeta, view: &crate::smeta::View) -> Vec<DocLine> {
    use DocLine::*;
    let o = &m.offer;
    let project = app.project().map(|p| p.name.clone()).unwrap_or_default();
    let mut out = Vec::new();
    if !o.company.is_empty() {
        out.push(Head(o.company.clone()));
    }
    if !o.contacts.is_empty() {
        out.push(Muted(o.contacts.clone()));
    }
    out.push(Gap);
    out.push(Title(if o.title.is_empty() {
        format!("{} — {project}", t("sm_offer_default_title"))
    } else {
        o.title.clone()
    }));
    if !o.customer.is_empty() {
        out.push(Text(format!("{}: {}", t("sm_offer_customer"), o.customer)));
    }
    out.push(Muted(format!(
        "{} {} · {} {} {}",
        t("sm_offer_date"),
        app.today.format("%d.%m.%Y"),
        t("sm_offer_valid_until"),
        o.valid_days,
        t("sm_days")
    )));

    // ---- Asosiy raqamlar plitkalarda.
    let mut tiles = vec![(
        t("sm_offer_price_title").to_string(),
        format!("{} {}", money(view.total), t("tk_sum_unit")),
        format!(
            "{} {} · {} {}",
            t("sm_works"),
            money(view.work_sum),
            t("sm_materials"),
            money(view.material_sum)
        ),
    )];
    tiles.push((
        t("sm_stages").to_string(),
        m.stages.len().to_string(),
        format!(
            "{} {}",
            m.stages.iter().map(|s| s.works.len()).sum::<usize>(),
            t("sm_works_short")
        ),
    ));
    for f in m.facts.iter().take(2) {
        tiles.push((
            f.name.clone(),
            format!("{} {}", f.value, f.unit).trim().to_string(),
            String::new(),
        ));
    }
    out.push(Gap);
    out.push(Tiles(tiles));
    if view.missing > 0 {
        out.push(Muted(format!(
            "* {} {}",
            view.missing,
            t("sm_offer_missing")
        )));
    }

    if !m.summary.is_empty() {
        out.push(Head(t("sm_summary").to_string()));
        out.push(Text(m.summary.clone()));
    }

    // Obyekt raqamlarda.
    if o.show_numbers && !m.facts.is_empty() {
        out.push(Head(t("sm_offer_numbers_title").to_string()));
        out.push(Table(
            vec![t("sm_col_fact").into(), t("sm_col_value").into()],
            m.facts
                .iter()
                .take(12)
                .map(|f| {
                    vec![
                        f.name.clone(),
                        format!("{} {}", f.value, f.unit).trim().to_string(),
                    ]
                })
                .collect(),
        ));
    }

    // Bosqichlar.
    out.push(Head(t("sm_offer_stages_title").to_string()));
    let rows: Vec<Vec<String>> = m
        .stages
        .iter()
        .enumerate()
        .map(|(si, st)| {
            let sv = view.stages.get(si);
            vec![
                format!("{}. {}", si + 1, st.name),
                st.works.len().to_string(),
                sv.map(|v| money(v.work_sum)).unwrap_or_default(),
                sv.map(|v| money(v.material_sum)).unwrap_or_default(),
                format!(
                    "{}{}",
                    sv.map(|v| money(v.total)).unwrap_or_default(),
                    if sv.is_some_and(|v| v.missing > 0) {
                        " *"
                    } else {
                        ""
                    }
                ),
            ]
        })
        .collect();
    out.push(Table(
        vec![
            t("sm_stages").into(),
            t("sm_works").into(),
            format!("{}, {}", t("sm_works"), t("tk_sum_unit")),
            format!("{}, {}", t("sm_materials"), t("tk_sum_unit")),
            t("tk_col_sum").into(),
        ],
        rows,
    ));
    if o.detailed {
        for (si, st) in m.stages.iter().enumerate() {
            if st.works.is_empty() {
                continue;
            }
            out.push(Sub(format!("{}. {}", si + 1, st.name)));
            out.push(Table(
                vec![t("sm_col_work").into(), t("tk_col_amount").into()],
                st.works
                    .iter()
                    .map(|w| vec![w.name.clone(), format!("{} {}", num(w.qty), w.unit)])
                    .collect(),
            ));
        }
    }

    // To'lov jadvali.
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
            out.push(Head(t("sm_offer_schedule_title").to_string()));
            out.push(Table(
                vec![
                    "№".into(),
                    t("sm_pay_title").into(),
                    "%".into(),
                    t("tk_col_sum").into(),
                ],
                plan.iter()
                    .enumerate()
                    .map(|(i, p)| {
                        vec![
                            (i + 1).to_string(),
                            p.title.clone(),
                            format!("{:.0}", p.pct),
                            money(p.amount),
                        ]
                    })
                    .collect(),
            ));
        }
    }
    if !o.excluded.is_empty() {
        out.push(Head(t("sm_offer_excluded").to_string()));
        for l in o.excluded.lines() {
            out.push(Text(format!("• {l}")));
        }
    }
    if !o.terms.is_empty() {
        out.push(Head(t("sm_offer_terms").to_string()));
        for l in o.terms.lines() {
            out.push(Text(l.to_string()));
        }
    }
    out
}

/// Taklifni PDF ga yozadi — ekrandagi hujjat ro'yxatidan.
fn export_offer(app: &mut App) {
    let Some(m) = app.smeta.clone() else { return };
    let view = app.smeta_view.clone();
    let lines = offer_lines(app, &m, &view);
    let rgb = crate::smeta::ACCENTS
        .get(m.offer.accent)
        .map(|a| a.1)
        .unwrap_or([31, 78, 160]);
    export_doc(app, &lines, t("sm_offer_file"), t("sm_offer_file"), rgb);
}

// ================================================================= Prays

/// Kompaniya katalogi: barcha obyektlarga umumiy narxlar.
fn catalog_view(ui: &mut egui::Ui, app: &mut App) {
    let mut edit: Option<(bool, String, f64)> = None;
    let mut remove: Option<(bool, String)> = None;
    let mut import: Option<bool> = None;
    let mut add = false;
    let mut from_smeta = false;
    let mut template = false;

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
            if app.smeta.as_ref().is_some_and(|m| !m.stages.is_empty()) {
                if ui
                    .button(t("sm_catalog_from_smeta"))
                    .on_hover_text(t("sm_catalog_from_smeta_hint"))
                    .clicked()
                {
                    from_smeta = true;
                }
                if ui
                    .button(t("sm_catalog_template"))
                    .on_hover_text(t("sm_catalog_template_hint"))
                    .clicked()
                {
                    template = true;
                }
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
    if template {
        export_price_template(app);
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

/// Narxsiz pozitsiyalar shabloni: ikki varaq (ishlar, materiallar) —
/// nom, birlik, bo'sh narx. Mijoz to'ldirib, «Excel: … narxi» tugmasi
/// bilan qaytaradi.
fn export_price_template(app: &mut App) {
    use crate::docgen::{Cell, Table};
    let Some(m) = &app.smeta else { return };
    let items = crate::smeta::unpriced(m, &app.smeta_view);
    let sheet = |work: bool, name: &str| Table {
        name: name.to_string(),
        headers: [t("tk_col_material"), t("tk_col_unit"), t("tk_col_price")]
            .iter()
            .map(|h| h.to_string())
            .collect(),
        rows: items
            .iter()
            .filter(|i| i.0 == work)
            .map(|i| {
                vec![
                    Cell::Text(i.1.clone()),
                    Cell::Text(i.2.clone()),
                    Cell::Empty,
                ]
            })
            .collect(),
    };
    let tables = [sheet(true, t("sm_works")), sheet(false, t("sm_materials"))];
    let name = format!(
        "{}_{}.xlsx",
        t("sm_catalog_template_file"),
        app.today.format("%Y-%m-%d")
    );
    let Some(path) = rfd::FileDialog::new()
        .set_file_name(&name)
        .add_filter("Excel", &["xlsx"])
        .save_file()
    else {
        return;
    };
    match crate::docgen::write_book(&path, &tables) {
        Ok(()) => app.notify(format!(
            "{} {} · {}",
            t("export_done"),
            items.len(),
            path.display()
        )),
        Err(e) => app.notify(format!("{}: {e}", t("export_failed"))),
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

// ================================================================ 7. Hisobot

/// AI loyiha hisoboti: dastur jadvallari + AI matnli bo'limlari.
fn report_tab(ui: &mut egui::Ui, app: &mut App) {
    let Some(m) = app.smeta.clone() else {
        empty(ui, t("tk_not_loaded"));
        return;
    };
    let view = app.smeta_view.clone();
    let mut write = false;
    let mut export = false;
    let mut export_word = false;

    card(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(t("sm_report_title")).strong().size(14.0));
            if app.report_rx.is_some() {
                ui.spinner();
                ui.label(RichText::new(t("sm_report_writing")).color(theme::muted()));
            } else if ui
                .add_enabled(
                    app.llm.is_ready(),
                    egui::Button::new(if m.report.is_empty() {
                        t("sm_report_write")
                    } else {
                        t("sm_report_rewrite")
                    }),
                )
                .on_hover_text(t("sm_report_hint"))
                .clicked()
            {
                write = true;
            }
            if ui.button(t("sm_offer_pdf")).clicked() {
                export = true;
            }
            if ui.button(t("sm_offer_docx")).clicked() {
                export_word = true;
            }
        });
        ui.label(
            RichText::new(t("sm_report_note"))
                .size(11.0)
                .color(theme::muted()),
        );
        if !app.smeta_note.is_empty() && app.report_rx.is_none() {
            ui.label(
                RichText::new(&app.smeta_note)
                    .size(11.0)
                    .color(theme::muted()),
            );
        }
    });
    ui.add_space(6.0);
    let rgb = crate::smeta::ACCENTS
        .get(m.offer.accent)
        .map(|a| a.1)
        .unwrap_or([31, 78, 160]);
    let accent = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    let lines = report_lines(app, &m, &view);
    let mut area = egui::ScrollArea::vertical().auto_shrink([false, false]);
    // Ekran suratlari uchun: tashqaridan berilgan siljish.
    if let Some(off) = std::env::var("QURAI_SCROLL")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
    {
        area = area.vertical_scroll_offset(off);
    }
    area.show(ui, |ui| {
        egui::Frame::new()
            .fill(theme::canvas())
            .stroke(Stroke::new(1.0_f32, theme::line()))
            .inner_margin(egui::Margin::same(18))
            .show(ui, |ui| draw_doc(ui, &lines, accent));
    });
    if write {
        app.start_report();
    }
    if export {
        export_doc(app, &lines, t("sm_report_title"), t("sm_report_file"), rgb);
    }
    if export_word {
        export_docx(app, &lines, t("sm_report_title"), t("sm_report_file"), rgb);
    }
}

/// Hisobot qatorlari: hammasi dastur ma'lumotidan, AI bo'limlari esa
/// alohida belgilangan.
fn report_lines(app: &App, m: &crate::smeta::Smeta, view: &crate::smeta::View) -> Vec<DocLine> {
    use crate::takeoff::Kind;
    use DocLine::*;
    let project = app.project().map(|p| p.name.clone()).unwrap_or_default();
    let mut out = vec![
        Title(format!("{} — {project}", t("sm_report_title"))),
        Muted(format!(
            "{} {} · {} {} · {} {} · {}",
            t("sm_offer_date"),
            app.today.format("%d.%m.%Y"),
            t("sm_pages_read"),
            m.digest.len(),
            t("tk_ai_tokens"),
            m.tokens,
            if m.model.is_empty() {
                "—".to_string()
            } else {
                m.model.clone()
            }
        )),
    ];
    if let Some(tk) = &app.takeoff {
        out.push(Muted(format!(
            "{}: {} ({} {})",
            t("tk_file"),
            tk.file,
            tk.pages,
            t("tk_pages")
        )));
    }

    // ---- Asosiy ko'rsatkichlar plitkalarda.
    let sum_of = |k: Kind, unit: &str| -> f64 {
        app.cost_rows
            .iter()
            .filter(|r| r.total.kind == k && r.total.unit == unit)
            .map(|r| r.total.amount)
            .sum()
    };
    let kg = |v: f64| {
        if v >= 1000.0 {
            format!("{:.1} t", v / 1000.0)
        } else {
            format!("{} kg", num(v))
        }
    };
    let mut tiles = Vec::new();
    if !m.stages.is_empty() {
        tiles.push((
            t("sm_total").to_string(),
            format!("{} {}", money(view.total), t("tk_sum_unit")),
            if view.missing > 0 {
                format!("{} {}", view.missing, t("tk_no_price"))
            } else if view.hinted > 0 {
                format!("{} {}", view.hinted, t("sm_price_hint"))
            } else {
                t("tk_total_full").to_string()
            },
        ));
    }
    let beton = sum_of(Kind::Concrete, "m3");
    if beton > 0.0 {
        tiles.push((
            t("tk_concrete").to_string(),
            format!("{} m³", num(beton)),
            t("tk_all_classes").to_string(),
        ));
    }
    let rebar = sum_of(Kind::Rebar, "kg");
    if rebar > 0.0 {
        tiles.push((
            t("tk_rebar").to_string(),
            kg(rebar),
            t("tk_all_diameters").to_string(),
        ));
    }
    let steel = sum_of(Kind::Steel, "kg");
    if steel > 0.0 {
        tiles.push((
            t("tk_steel").to_string(),
            kg(steel),
            t("tk_steel_hint").to_string(),
        ));
    }
    if !m.stages.is_empty() {
        let works: usize = m.stages.iter().map(|s| s.works.len()).sum();
        tiles.push((
            t("sm_tab_spec").to_string(),
            format!("{} / {works}", m.stages.len()),
            format!("{} / {}", t("sm_stages"), t("sm_works")),
        ));
    }
    if !tiles.is_empty() {
        out.push(Gap);
        out.push(Tiles(tiles));
    }

    // ---- AI matnli bo'limlari (bo'lsa) — birinchi, resume sifatida.
    if m.report.is_empty() {
        if !m.summary.is_empty() {
            out.push(Head(t("sm_summary").to_string()));
            out.push(Text(m.summary.clone()));
        }
    } else {
        for (title, body) in &m.report {
            out.push(Head(title.clone()));
            out.push(Text(body.clone()));
        }
        out.push(Muted(t("sm_report_ai_mark").to_string()));
    }

    // ---- Obyekt raqamlarda.
    if !m.facts.is_empty() {
        out.push(Head(t("sm_offer_numbers_title").to_string()));
        let rows: Vec<Vec<String>> = m
            .facts
            .iter()
            .take(30)
            .map(|f| {
                vec![
                    f.name.clone(),
                    format!("{} {}", f.value, f.unit).trim().to_string(),
                    f.page
                        .map(|p| format!("{} {p}", t("pdf_page")))
                        .unwrap_or_else(|| t("sm_sketch").to_string()),
                ]
            })
            .collect();
        out.push(Table(
            vec![
                t("sm_col_fact").into(),
                t("sm_col_value").into(),
                t("sm_col_source").into(),
            ],
            rows,
        ));
        if m.facts.len() > 30 {
            out.push(Muted(format!(
                "… {} {}",
                m.facts.len() - 30,
                t("sm_report_more")
            )));
        }
    }

    // ---- Konstruksiyalar.
    if let Some(tk) = &app.takeoff {
        if !tk.constructs.is_empty() {
            out.push(Head(t("tab_constructs").to_string()));
            let rows: Vec<Vec<String>> = tk
                .constructs
                .iter()
                .map(|c| {
                    vec![
                        format!("{} {}", c.mark, c.name).trim().to_string(),
                        format!("{} {}", num(c.count), c.unit),
                        format!("{} {}", t("pdf_page"), c.page),
                    ]
                })
                .collect();
            out.push(Table(
                vec![
                    t("sm_col_construct").into(),
                    t("tk_col_amount").into(),
                    t("sm_col_source").into(),
                ],
                rows,
            ));
        }
    }

    // ---- Loyihadan olingan materiallar — tur bo'yicha.
    if !app.cost_rows.is_empty() {
        out.push(Head(t("sm_from_project").to_string()));
        out.push(Muted(t("sm_from_project_note").to_string()));
        for kind in [Kind::Concrete, Kind::Rebar, Kind::Steel, Kind::Other] {
            let rows: Vec<Vec<String>> = app
                .cost_rows
                .iter()
                .filter(|r| r.total.kind == kind)
                .map(|r| {
                    vec![
                        r.total.material.clone(),
                        if r.total.unit == "kg" {
                            kg(r.total.amount)
                        } else {
                            format!("{} {}", num(r.total.amount), r.total.unit)
                        },
                        r.total.lines.to_string(),
                    ]
                })
                .collect();
            if rows.is_empty() {
                continue;
            }
            let title = match kind {
                Kind::Concrete => t("tk_concrete"),
                Kind::Rebar => t("tk_rebar"),
                Kind::Steel => t("tk_steel"),
                Kind::Other => t("tk_other"),
            };
            out.push(Sub(title.to_string()));
            out.push(Table(
                vec![
                    t("tk_col_material").into(),
                    t("tk_col_amount").into(),
                    t("tk_col_lines").into(),
                ],
                rows,
            ));
        }
    }

    // ---- Savollar va javoblar.
    if !m.questions.is_empty() {
        out.push(Head(t("sm_tab_questions").to_string()));
        let rows: Vec<Vec<String>> = m
            .questions
            .iter()
            .map(|q| {
                vec![
                    q.text.clone(),
                    if q.answer.is_empty() {
                        t("sm_report_no_answer").to_string()
                    } else {
                        q.answer.clone()
                    },
                ]
            })
            .collect();
        out.push(Table(
            vec![t("sm_col_question").into(), t("sm_col_answer").into()],
            rows,
        ));
    }

    // ---- Spetsifikatsiya va smeta yig'masi.
    if !m.stages.is_empty() {
        out.push(Head(t("sm_tab_spec").to_string()));
        let is_assume = |s: &Source| matches!(s, Source::Assumption { .. });
        let rows: Vec<Vec<String>> = m
            .stages
            .iter()
            .enumerate()
            .map(|(i, st)| {
                let mats: usize = st.works.iter().map(|w| w.materials.len()).sum();
                let assume = st
                    .works
                    .iter()
                    .map(|w| {
                        is_assume(&w.source) as usize
                            + w.materials.iter().filter(|x| is_assume(&x.source)).count()
                    })
                    .sum::<usize>();
                vec![
                    format!("{}. {}", i + 1, st.name),
                    st.works.len().to_string(),
                    mats.to_string(),
                    assume.to_string(),
                    view.stages
                        .get(i)
                        .map(|v| money(v.total))
                        .unwrap_or_default(),
                ]
            })
            .collect();
        out.push(Table(
            vec![
                t("sm_stages").into(),
                t("sm_works").into(),
                t("sm_materials").into(),
                t("sm_src_assumption").into(),
                t("tk_col_sum").into(),
            ],
            rows,
        ));
        out.push(Head(t("sm_tab_smeta").to_string()));
        out.push(Table(
            vec![String::new(), t("tk_col_sum").into()],
            vec![
                vec![t("sm_works").into(), money(view.work_sum)],
                vec![t("sm_materials").into(), money(view.material_sum)],
                vec![
                    format!("{} ({}%)", t("sm_markup_all"), num(m.markup)),
                    String::new(),
                ],
                vec![
                    t("sm_total").into(),
                    format!("{} {}", money(view.total), t("tk_sum_unit")),
                ],
            ],
        ));
        let mut by: std::collections::BTreeMap<&'static str, usize> = Default::default();
        for sv in &view.stages {
            for (wp, mps) in &sv.works {
                *by.entry(wp.origin.label()).or_default() += 1;
                for p in mps {
                    *by.entry(p.origin.label()).or_default() += 1;
                }
            }
        }
        out.push(Sub(t("sm_report_price_sources").to_string()));
        out.push(Table(
            vec![t("sm_col_source").into(), t("tk_col_lines").into()],
            by.into_iter()
                .map(|(k, n)| vec![t(k).to_string(), n.to_string()])
                .collect(),
        ));
        if view.missing > 0 {
            out.push(Muted(format!("{} {}", view.missing, t("tk_total_partial"))));
        }
        if view.hinted > 0 {
            out.push(Muted(format!("{} {}", view.hinted, t("sm_hinted_note"))));
        }
    }

    // ---- AI topilmalari.
    let open: Vec<&crate::smeta::Finding> = m.review.iter().filter(|f| !f.done).collect();
    if !m.review_summary.is_empty() || !open.is_empty() {
        out.push(Head(t("sm_review_title").to_string()));
        if !m.review_summary.is_empty() {
            out.push(Text(m.review_summary.clone()));
        }
        if !open.is_empty() {
            let rows: Vec<Vec<String>> = open
                .iter()
                .map(|f| {
                    let kind = match f.kind.as_str() {
                        "qty" => t("sm_f_qty"),
                        "missing" => t("sm_f_missing"),
                        "dup" => t("sm_f_dup"),
                        "price" => t("sm_f_price"),
                        _ => t("sm_f_ask"),
                    };
                    vec![f.text.clone(), kind.to_string()]
                })
                .collect();
            out.push(Table(
                vec![t("sm_col_finding").into(), t("sm_col_kind").into()],
                rows,
            ));
        }
    }

    // ---- Chegaralar — doim.
    out.push(Head(t("sm_report_limits").to_string()));
    for l in t("sm_report_limits_text").lines() {
        out.push(Text(l.to_string()));
    }
    out
}

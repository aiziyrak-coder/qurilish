use super::*;
use crate::smeta::{Smeta, Stage, Work};
use crate::smeta_ai::StageMsg;
use std::sync::{atomic::AtomicBool, mpsc::channel, Arc};

fn app() -> App {
    App::new(Db::open(&std::path::PathBuf::from(":memory:")).unwrap())
}

fn stage(unit: &str, price: Option<f64>) -> Stage {
    Stage {
        name: "Poydevor".into(),
        works: vec![Work {
            name: "Beton quyish".into(),
            qty: 10.0,
            unit: unit.into(),
            price,
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn spec_job(app: &mut App, plan: &[&str], messages: Vec<StageMsg>) {
    let (tx, rx) = channel();
    for message in messages {
        tx.send(message).unwrap();
    }
    app.spec_job = Some(SpecJob {
        started: crate::plat::Instant::now(),
        plan: plan.iter().map(|s| s.to_string()).collect(),
        done: Vec::new(),
        failed: Vec::new(),
        cancel: Arc::new(AtomicBool::new(false)),
        rx,
    });
}

#[test]
fn incomplete_spec_must_not_replace_existing_estimate() {
    let mut app = app();
    let old = stage("m3", Some(100.0));
    app.smeta = Some(Smeta {
        stages: vec![old.clone()],
        ..Default::default()
    });
    let mut partial = stage("m3", None);
    partial.works[0].qty = 999.0;
    spec_job(
        &mut app,
        &["Poydevor", "Tom"],
        vec![
            StageMsg::Stage(0, Ok(partial)),
            StageMsg::Stage(1, Err("timeout".into())),
            StageMsg::Done,
        ],
    );
    assert!(app.poll_spec());
    assert_eq!(app.smeta.unwrap().stages, vec![old]);
    assert!(app.smeta_note.contains("timeout"));
}

#[test]
fn disconnected_spec_must_not_publish_partial_estimate() {
    let mut app = app();
    app.smeta = Some(Smeta::default());
    spec_job(
        &mut app,
        &["Poydevor", "Tom"],
        vec![StageMsg::Stage(0, Ok(stage("m3", None)))],
    );
    app.poll_spec();
    assert!(app.smeta.unwrap().stages.is_empty());
}

#[test]
fn changed_unit_must_not_inherit_old_unit_price() {
    let mut app = app();
    app.smeta = Some(Smeta {
        stages: vec![stage("m3", Some(100.0))],
        ..Default::default()
    });
    spec_job(
        &mut app,
        &["Poydevor"],
        vec![StageMsg::Stage(0, Ok(stage("m2", None))), StageMsg::Done],
    );
    app.poll_spec();
    assert_eq!(app.smeta.unwrap().stages[0].works[0].price, None);
}

#[test]
fn changing_project_discards_pending_pdf() {
    let mut app = app();
    let (tx, rx) = channel();
    app.pdf_job = Some(PdfJob {
        file: "old.pdf".into(),
        path: "old.pdf".into(),
        started: crate::plat::Instant::now(),
        rx,
    });
    app.load_takeoff();
    let _ = tx.send(Ok(crate::takeoff::Takeoff {
        file: "old.pdf".into(),
        ..Default::default()
    }));
    app.poll_pdf();
    assert!(
        app.takeoff.is_none(),
        "Old PDF was applied to the newly selected project"
    );
}

#[test]
fn clearing_estimate_discards_pending_questions() {
    let mut app = app();
    let (tx, rx) = channel();
    app.questions_rx = Some(rx);
    app.clear_takeoff();
    assert!(app.questions_rx.is_none());
    drop(tx);
}

#[test]
fn successful_new_pdf_resets_old_estimate_and_prices() {
    let mut app = app();
    app.smeta = Some(Smeta {
        stages: vec![stage("m3", Some(100.0))],
        ..Default::default()
    });
    app.takeoff_path = Some("old.pdf".into());
    app.price_hints.insert("old".into(), (100.0, String::new()));
    let (tx, rx) = channel();
    app.pdf_job = Some(PdfJob {
        file: "new.pdf".into(),
        path: "new.pdf".into(),
        started: crate::plat::Instant::now(),
        rx,
    });
    tx.send(Ok(crate::takeoff::Takeoff {
        file: "new.pdf".into(),
        ..Default::default()
    }))
    .unwrap();
    app.poll_pdf();
    assert!(app.smeta.is_none());
    assert!(app.price_hints.is_empty());
    assert_eq!(app.smeta_view.total, 0.0);
    assert_eq!(app.takeoff_path, Some("new.pdf".into()));
}

#[test]
fn failed_new_pdf_keeps_previous_document_and_calculation() {
    let mut app = app();
    let old = Smeta {
        stages: vec![stage("m3", Some(100.0))],
        ..Default::default()
    };
    app.smeta = Some(old.clone());
    app.takeoff_path = Some("old.pdf".into());
    let (tx, rx) = channel();
    app.pdf_job = Some(PdfJob {
        file: "new.pdf".into(),
        path: "new.pdf".into(),
        started: crate::plat::Instant::now(),
        rx,
    });
    tx.send(Err("broken PDF".into())).unwrap();
    app.poll_pdf();
    assert_eq!(app.smeta, Some(old));
    assert_eq!(app.takeoff_path, Some("old.pdf".into()));
}

#[test]
fn complete_spec_retains_compatible_prices_but_discards_old_report() {
    let mut app = app();
    app.smeta = Some(Smeta {
        stages: vec![stage("m3", Some(100.0))],
        report: vec![("Old".into(), "Old total".into())],
        ..Default::default()
    });
    let mut new = stage("m3", None);
    new.works[0].qty = 20.0;
    spec_job(
        &mut app,
        &["Poydevor"],
        vec![StageMsg::Stage(0, Ok(new)), StageMsg::Done],
    );
    app.poll_spec();
    assert_eq!(app.smeta_view.total, 2000.0);
    assert!(app.smeta.unwrap().report.is_empty());
}

#[test]
fn equal_quantities_from_different_pages_are_not_duplicates() {
    let mut first = stage("m3", Some(100.0));
    first.works[0].source = crate::smeta::Source::Project { page: Some(1) };
    let mut second = first.clone();
    second.name = "Karkas".into();
    second.works[0].source = crate::smeta::Source::Project { page: Some(2) };
    let mut smeta = Smeta {
        stages: vec![first, second],
        ..Default::default()
    };
    assert_eq!(crate::smeta::dedupe(&mut smeta), 0);
    assert_eq!(
        smeta
            .stages
            .iter()
            .flat_map(|s| &s.works)
            .map(|w| w.qty)
            .sum::<f64>(),
        20.0
    );
}

#[test]
fn different_material_sources_and_compositions_are_preserved() {
    let mut first = stage("m3", None);
    first.works[0].materials.push(crate::smeta::Resource {
        name: "Beton B20".into(),
        unit: "m3".into(),
        qty: 10.0,
        source: crate::smeta::Source::Project { page: Some(1) },
        ..Default::default()
    });
    let mut second = first.clone();
    second.name = "Karkas".into();
    second.works[0].materials[0].source = crate::smeta::Source::Project { page: Some(2) };
    let mut smeta = Smeta {
        stages: vec![first, second],
        ..Default::default()
    };
    assert_eq!(crate::smeta::dedupe(&mut smeta), 0);
    assert_eq!(crate::smeta::purchase(&smeta)[0].2, 20.0);
}

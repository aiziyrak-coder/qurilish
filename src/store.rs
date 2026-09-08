//! II–XVI modullarning saqlash qatlami: sxema va CRUD.
//!
//! Xatolar yuqoriga chiqarilmaydi — ro'yxatlar bo'sh qaytadi, yozish esa
//! `bool` bilan javob beradi. Interfeys uchun shu yetarli, chunki lokal
//! SQLite dagi xato faqat disk muammosida yuz beradi.

// IV-XVI modullar sxemasi va CRUD i tayyor, ekranlari keyingi bosqichda ulanadi.
#![allow(dead_code)]

use crate::db::Db;
use crate::domain::*;
use crate::model::Section;
use crate::roles::{Role, User};
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use rusqlite::{params, Row};

fn date(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap_or_else(|_| chrono::Local::now().date_naive())
}

/// Vaqt belgisi: `2026-09-08T07:41:12` yoki `...Z`.
///
/// O'qib bo'lmasa kun boshiga tushadi — yozuv yo'qolib ketgandan ko'ra
/// noaniq vaqt bilan turgani yaxshiroq, va bu ekranda ko'rinadi.
pub fn parse_stamp(s: &str) -> NaiveDateTime {
    let clean = s.trim().trim_end_matches('Z');
    NaiveDateTime::parse_from_str(clean, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(clean, "%Y-%m-%d %H:%M:%S"))
        .unwrap_or_else(|_| date(clean).and_hms_opt(0, 0, 0).unwrap_or_default())
}

fn odate(s: Option<String>) -> Option<NaiveDate> {
    s.map(|x| date(&x))
}

fn ods(d: Option<NaiveDate>) -> Option<String> {
    d.map(|x| x.to_string())
}

/// SQL dan jadval nomini ajratadi: `INSERT INTO x`, `UPDATE x SET`.
///
/// SQL kod ichida yozilgani uchun shakl oldindan ma'lum — bu yerda tashqi
/// matn tahlil qilinmaydi.
fn table_of_sql(sql: &str) -> &str {
    let s = sql.trim_start();
    let rest = if let Some(r) = s.strip_prefix("INSERT INTO ") {
        r
    } else if let Some(r) = s.strip_prefix("UPDATE ") {
        r
    } else {
        return "";
    };
    rest.split(|c: char| c.is_whitespace() || c == '(')
        .next()
        .unwrap_or("")
}

impl Db {
    /// Modullar jadvallari. Asosiy sxema `db.rs` da yaratilgandan keyin chaqiriladi.
    pub fn migrate_modules(&self) -> rusqlite::Result<()> {
        self.conn().execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS issue (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                module TEXT NOT NULL DEFAULT 'project',
                section TEXT NOT NULL DEFAULT 'NONE',
                code TEXT NOT NULL DEFAULT '',
                sheet TEXT NOT NULL DEFAULT '',
                location TEXT NOT NULL DEFAULT '',
                element TEXT NOT NULL DEFAULT '',
                title TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL DEFAULT '',
                severity TEXT NOT NULL DEFAULT 'warning',
                norm_doc TEXT NOT NULL DEFAULT '',
                norm_clause TEXT NOT NULL DEFAULT '',
                norm_text TEXT NOT NULL DEFAULT '',
                recommendation TEXT NOT NULL DEFAULT '',
                responsible TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'open',
                auto INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (date('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_issue_project ON issue(project_id);

            CREATE TABLE IF NOT EXISTS element (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                section TEXT NOT NULL DEFAULT 'NONE',
                kind TEXT NOT NULL DEFAULT 'other',
                mark TEXT NOT NULL DEFAULT '',
                room TEXT NOT NULL DEFAULT '',
                axis TEXT NOT NULL DEFAULT '',
                level TEXT NOT NULL DEFAULT '',
                size REAL NOT NULL DEFAULT 0,
                unit TEXT NOT NULL DEFAULT '',
                value REAL NOT NULL DEFAULT 0,
                value_name TEXT NOT NULL DEFAULT '',
                sheet TEXT NOT NULL DEFAULT '',
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_element_project ON element(project_id);

            CREATE TABLE IF NOT EXISTS element_link (
                id INTEGER PRIMARY KEY,
                from_el INTEGER NOT NULL REFERENCES element(id) ON DELETE CASCADE,
                to_el INTEGER NOT NULL REFERENCES element(id) ON DELETE CASCADE,
                relation TEXT NOT NULL DEFAULT 'related',
                UNIQUE(from_el, to_el, relation)
            );

            CREATE TABLE IF NOT EXISTS document (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                section TEXT NOT NULL DEFAULT 'NONE',
                name TEXT NOT NULL DEFAULT '',
                format TEXT NOT NULL DEFAULT '',
                path TEXT NOT NULL DEFAULT '',
                sheets INTEGER NOT NULL DEFAULT 0,
                added_at TEXT NOT NULL DEFAULT (date('now')),
                revision TEXT NOT NULL DEFAULT '',
                version INTEGER NOT NULL DEFAULT 1,
                replaces INTEGER,
                change_note TEXT NOT NULL DEFAULT '',
                issued TEXT
            );

            CREATE TABLE IF NOT EXISTS estimate (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                name TEXT NOT NULL DEFAULT '',
                currency TEXT NOT NULL DEFAULT 'UZS',
                declared_total REAL NOT NULL DEFAULT 0,
                added_at TEXT NOT NULL DEFAULT (date('now'))
            );

            CREATE TABLE IF NOT EXISTS estimate_item (
                id INTEGER PRIMARY KEY,
                estimate_id INTEGER NOT NULL REFERENCES estimate(id) ON DELETE CASCADE,
                pos INTEGER NOT NULL DEFAULT 0,
                section TEXT NOT NULL DEFAULT 'NONE',
                code TEXT NOT NULL DEFAULT '',
                name TEXT NOT NULL DEFAULT '',
                unit TEXT NOT NULL DEFAULT '',
                qty REAL NOT NULL DEFAULT 0,
                price REAL NOT NULL DEFAULT 0,
                cost REAL NOT NULL DEFAULT 0,
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_eitem ON estimate_item(estimate_id);

            CREATE TABLE IF NOT EXISTS ppr (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                kind TEXT NOT NULL DEFAULT 'ppr',
                number TEXT NOT NULL DEFAULT '',
                name TEXT NOT NULL DEFAULT '',
                section TEXT NOT NULL DEFAULT 'NONE',
                task_id INTEGER,
                workers INTEGER NOT NULL DEFAULT 0,
                machines INTEGER NOT NULL DEFAULT 0,
                path TEXT NOT NULL DEFAULT '',
                approved INTEGER NOT NULL DEFAULT 0,
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_ppr_project ON ppr(project_id);

            CREATE TABLE IF NOT EXISTS exec_doc (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                kind TEXT NOT NULL DEFAULT 'hidden',
                number TEXT NOT NULL DEFAULT '',
                name TEXT NOT NULL DEFAULT '',
                date TEXT NOT NULL,
                task_id INTEGER,
                status TEXT NOT NULL DEFAULT 'draft',
                responsible TEXT NOT NULL DEFAULT '',
                version INTEGER NOT NULL DEFAULT 1,
                replaces INTEGER,
                note TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS journal (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                date TEXT NOT NULL,
                author TEXT NOT NULL DEFAULT '',
                weather TEXT NOT NULL DEFAULT '',
                temperature REAL NOT NULL DEFAULT 0,
                workers INTEGER NOT NULL DEFAULT 0,
                machines INTEGER NOT NULL DEFAULT 0,
                task_id INTEGER,
                volume REAL NOT NULL DEFAULT 0,
                unit TEXT NOT NULL DEFAULT '',
                text TEXT NOT NULL DEFAULT '',
                remarks TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS request (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                number TEXT NOT NULL DEFAULT '',
                date TEXT NOT NULL,
                kind TEXT NOT NULL DEFAULT 'material',
                title TEXT NOT NULL DEFAULT '',
                material_id INTEGER,
                qty REAL NOT NULL DEFAULT 0,
                unit TEXT NOT NULL DEFAULT '',
                requester TEXT NOT NULL DEFAULT '',
                need_date TEXT NOT NULL,
                priority TEXT NOT NULL DEFAULT 'normal',
                status TEXT NOT NULL DEFAULT 'new',
                task_id INTEGER,
                note TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS purchase (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                request_id INTEGER,
                number TEXT NOT NULL DEFAULT '',
                date TEXT NOT NULL,
                supplier TEXT NOT NULL DEFAULT '',
                title TEXT NOT NULL DEFAULT '',
                qty REAL NOT NULL DEFAULT 0,
                unit TEXT NOT NULL DEFAULT '',
                price REAL NOT NULL DEFAULT 0,
                currency TEXT NOT NULL DEFAULT 'UZS',
                delivery_date TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'draft',
                note TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS material (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                code TEXT NOT NULL DEFAULT '',
                name TEXT NOT NULL DEFAULT '',
                unit TEXT NOT NULL DEFAULT '',
                section TEXT NOT NULL DEFAULT 'NONE',
                spec TEXT NOT NULL DEFAULT '',
                cert_no TEXT NOT NULL DEFAULT '',
                cert_until TEXT,
                min_stock REAL NOT NULL DEFAULT 0,
                price REAL NOT NULL DEFAULT 0,
                note TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS stock_move (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                material_id INTEGER NOT NULL REFERENCES material(id) ON DELETE CASCADE,
                date TEXT NOT NULL,
                kind TEXT NOT NULL DEFAULT 'in',
                qty REAL NOT NULL DEFAULT 0,
                price REAL NOT NULL DEFAULT 0,
                document TEXT NOT NULL DEFAULT '',
                counterparty TEXT NOT NULL DEFAULT '',
                task_id INTEGER,
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_move_mat ON stock_move(material_id);

            CREATE TABLE IF NOT EXISTS worker (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                name TEXT NOT NULL DEFAULT '',
                position TEXT NOT NULL DEFAULT '',
                org TEXT NOT NULL DEFAULT '',
                hourly_rate REAL NOT NULL DEFAULT 0,
                active INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS timesheet (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                worker_id INTEGER NOT NULL REFERENCES worker(id) ON DELETE CASCADE,
                date TEXT NOT NULL,
                hours REAL NOT NULL DEFAULT 0,
                task_id INTEGER,
                note TEXT NOT NULL DEFAULT '',
                UNIQUE(worker_id, date)
            );

            CREATE TABLE IF NOT EXISTS message (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                server_id INTEGER NOT NULL DEFAULT 0,
                author TEXT NOT NULL DEFAULT '',
                role TEXT NOT NULL DEFAULT '',
                text TEXT NOT NULL DEFAULT '',
                at TEXT NOT NULL DEFAULT '',
                UNIQUE(project_id, server_id)
            );

            CREATE TABLE IF NOT EXISTS attendance (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                worker TEXT NOT NULL,
                at TEXT NOT NULL,
                kind TEXT NOT NULL DEFAULT 'in',
                gps TEXT NOT NULL DEFAULT '',
                source TEXT NOT NULL DEFAULT '',
                UNIQUE(project_id, worker, at, kind)
            );

            CREATE TABLE IF NOT EXISTS quality_check (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                kind TEXT NOT NULL DEFAULT 'operational',
                date TEXT NOT NULL,
                task_id INTEGER,
                material_id INTEGER,
                subject TEXT NOT NULL DEFAULT '',
                inspector TEXT NOT NULL DEFAULT '',
                result TEXT NOT NULL DEFAULT 'pass',
                defect TEXT NOT NULL DEFAULT '',
                deadline TEXT,
                note TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS safety_event (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                date TEXT NOT NULL,
                kind TEXT NOT NULL DEFAULT 'violation',
                severity TEXT NOT NULL DEFAULT 'warning',
                place TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL DEFAULT '',
                responsible TEXT NOT NULL DEFAULT '',
                measure TEXT NOT NULL DEFAULT '',
                deadline TEXT,
                status TEXT NOT NULL DEFAULT 'open'
            );

            CREATE TABLE IF NOT EXISTS machine (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                name TEXT NOT NULL DEFAULT '',
                kind TEXT NOT NULL DEFAULT 'other',
                reg_no TEXT NOT NULL DEFAULT '',
                owner TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'idle',
                hour_rate REAL NOT NULL DEFAULT 0,
                operator TEXT NOT NULL DEFAULT '',
                inspection_until TEXT
            );

            CREATE TABLE IF NOT EXISTS machine_log (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                machine_id INTEGER NOT NULL REFERENCES machine(id) ON DELETE CASCADE,
                date TEXT NOT NULL,
                hours REAL NOT NULL DEFAULT 0,
                fuel REAL NOT NULL DEFAULT 0,
                task_id INTEGER,
                note TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS block (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                name TEXT NOT NULL DEFAULT '',
                floors INTEGER NOT NULL DEFAULT 9,
                first_floor INTEGER NOT NULL DEFAULT 1,
                note TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS unit (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                block_id INTEGER NOT NULL REFERENCES block(id) ON DELETE CASCADE,
                number TEXT NOT NULL DEFAULT '',
                floor INTEGER NOT NULL DEFAULT 1,
                position INTEGER NOT NULL DEFAULT 1,
                kind TEXT NOT NULL DEFAULT 'flat',
                rooms INTEGER NOT NULL DEFAULT 0,
                area REAL NOT NULL DEFAULT 0,
                area_living REAL NOT NULL DEFAULT 0,
                price_per_m2 REAL NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'free',
                layout TEXT NOT NULL DEFAULT '',
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_unit_block ON unit(block_id);

            CREATE TABLE IF NOT EXISTS deal (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                unit_id INTEGER NOT NULL REFERENCES unit(id) ON DELETE CASCADE,
                number TEXT NOT NULL DEFAULT '',
                date TEXT NOT NULL,
                client TEXT NOT NULL DEFAULT '',
                phone TEXT NOT NULL DEFAULT '',
                client_doc TEXT NOT NULL DEFAULT '',
                pay_kind TEXT NOT NULL DEFAULT 'cash',
                price REAL NOT NULL DEFAULT 0,
                discount REAL NOT NULL DEFAULT 0,
                prepayment REAL NOT NULL DEFAULT 0,
                months INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'reserved',
                manager TEXT NOT NULL DEFAULT '',
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_deal_unit ON deal(unit_id);

            CREATE TABLE IF NOT EXISTS payment (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                deal_id INTEGER NOT NULL REFERENCES deal(id) ON DELETE CASCADE,
                due TEXT NOT NULL,
                planned REAL NOT NULL DEFAULT 0,
                paid REAL NOT NULL DEFAULT 0,
                paid_date TEXT,
                kind TEXT NOT NULL DEFAULT 'cash',
                document TEXT NOT NULL DEFAULT '',
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_pay_deal ON payment(deal_id);

            CREATE TABLE IF NOT EXISTS app_user (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL DEFAULT '',
                role TEXT NOT NULL DEFAULT 'admin',
                note TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS warehouse (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                name TEXT NOT NULL DEFAULT '',
                kind TEXT NOT NULL DEFAULT 'object',
                responsible TEXT NOT NULL DEFAULT '',
                note TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS batch (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                material_id INTEGER NOT NULL REFERENCES material(id) ON DELETE CASCADE,
                number TEXT NOT NULL DEFAULT '',
                received TEXT NOT NULL,
                supplier TEXT NOT NULL DEFAULT '',
                cert_no TEXT NOT NULL DEFAULT '',
                cert_until TEXT,
                expires TEXT,
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_batch_mat ON batch(material_id);

            CREATE TABLE IF NOT EXISTS reservation (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                material_id INTEGER NOT NULL REFERENCES material(id) ON DELETE CASCADE,
                task_id INTEGER,
                qty REAL NOT NULL DEFAULT 0,
                date TEXT NOT NULL,
                until TEXT,
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_res_mat ON reservation(material_id);

            CREATE TABLE IF NOT EXISTS material_alt (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                material_id INTEGER NOT NULL REFERENCES material(id) ON DELETE CASCADE,
                alt_id INTEGER NOT NULL REFERENCES material(id) ON DELETE CASCADE,
                approved_by TEXT NOT NULL DEFAULT '',
                approved_at TEXT,
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_alt_mat ON material_alt(material_id);

            CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY,
                at TEXT NOT NULL,
                user TEXT NOT NULL DEFAULT '',
                action TEXT NOT NULL DEFAULT 'update',
                table_name TEXT NOT NULL DEFAULT '',
                row_id INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_audit_at ON audit_log(at DESC);

            CREATE TABLE IF NOT EXISTS worker_permit (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                worker_id INTEGER NOT NULL REFERENCES worker(id) ON DELETE CASCADE,
                kind TEXT NOT NULL DEFAULT 'induction',
                number TEXT NOT NULL DEFAULT '',
                issued TEXT NOT NULL,
                valid_until TEXT NOT NULL,
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_wp_worker ON worker_permit(worker_id);

            CREATE TABLE IF NOT EXISTS ppe_issue (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                worker_id INTEGER NOT NULL REFERENCES worker(id) ON DELETE CASCADE,
                item TEXT NOT NULL DEFAULT 'helmet',
                issued TEXT NOT NULL,
                months INTEGER NOT NULL DEFAULT 12,
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_ppe_worker ON ppe_issue(worker_id);

            CREATE TABLE IF NOT EXISTS work_permit (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                number TEXT NOT NULL DEFAULT '',
                kind TEXT NOT NULL DEFAULT 'height',
                task_id INTEGER,
                place TEXT NOT NULL DEFAULT '',
                date_from TEXT NOT NULL,
                date_to TEXT NOT NULL,
                issuer TEXT NOT NULL DEFAULT '',
                supervisor TEXT NOT NULL DEFAULT '',
                workers TEXT NOT NULL DEFAULT '',
                measures TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'draft',
                note TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS checklist (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                name TEXT NOT NULL DEFAULT '',
                section TEXT NOT NULL DEFAULT 'none',
                kind TEXT NOT NULL DEFAULT 'operational',
                note TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS checklist_item (
                id INTEGER PRIMARY KEY,
                checklist_id INTEGER NOT NULL REFERENCES checklist(id) ON DELETE CASCADE,
                pos INTEGER NOT NULL DEFAULT 1,
                text TEXT NOT NULL DEFAULT '',
                norm_doc TEXT NOT NULL DEFAULT '',
                norm_clause TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_cli_list ON checklist_item(checklist_id);

            CREATE TABLE IF NOT EXISTS check_point (
                id INTEGER PRIMARY KEY,
                check_id INTEGER NOT NULL REFERENCES quality_check(id) ON DELETE CASCADE,
                pos INTEGER NOT NULL DEFAULT 1,
                text TEXT NOT NULL DEFAULT '',
                norm_doc TEXT NOT NULL DEFAULT '',
                norm_clause TEXT NOT NULL DEFAULT '',
                result TEXT NOT NULL DEFAULT 'pending',
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_cp_check ON check_point(check_id);

            CREATE TABLE IF NOT EXISTS approval (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                request_id INTEGER NOT NULL REFERENCES request(id) ON DELETE CASCADE,
                step INTEGER NOT NULL DEFAULT 1,
                role TEXT NOT NULL DEFAULT '',
                approver TEXT NOT NULL DEFAULT '',
                decision TEXT NOT NULL DEFAULT 'pending',
                decided_at TEXT,
                comment TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_appr_req ON approval(request_id);

            CREATE TABLE IF NOT EXISTS supplier (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                name TEXT NOT NULL DEFAULT '',
                inn TEXT NOT NULL DEFAULT '',
                contact TEXT NOT NULL DEFAULT '',
                phone TEXT NOT NULL DEFAULT '',
                blocked INTEGER NOT NULL DEFAULT 0,
                note TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS quote (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                request_id INTEGER,
                supplier TEXT NOT NULL DEFAULT '',
                title TEXT NOT NULL DEFAULT '',
                qty REAL NOT NULL DEFAULT 0,
                unit TEXT NOT NULL DEFAULT '',
                price REAL NOT NULL DEFAULT 0,
                currency TEXT NOT NULL DEFAULT 'UZS',
                delivery_days INTEGER NOT NULL DEFAULT 0,
                valid_until TEXT,
                chosen INTEGER NOT NULL DEFAULT 0,
                date TEXT NOT NULL,
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_quote_req ON quote(request_id);

            CREATE TABLE IF NOT EXISTS purchase_budget (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                section TEXT NOT NULL DEFAULT 'none',
                planned REAL NOT NULL DEFAULT 0,
                note TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS brigade (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                name TEXT NOT NULL DEFAULT '',
                foreman TEXT NOT NULL DEFAULT '',
                task_id INTEGER,
                note TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS material_norm (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                task_id INTEGER NOT NULL REFERENCES task(id) ON DELETE CASCADE,
                material_id INTEGER NOT NULL REFERENCES material(id) ON DELETE CASCADE,
                per_unit REAL NOT NULL DEFAULT 0,
                tolerance REAL NOT NULL DEFAULT 0,
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_norm_task ON material_norm(task_id);

            CREATE TABLE IF NOT EXISTS inventory (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                warehouse_id INTEGER,
                date TEXT NOT NULL,
                responsible TEXT NOT NULL DEFAULT '',
                closed INTEGER NOT NULL DEFAULT 0,
                note TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS inventory_line (
                id INTEGER PRIMARY KEY,
                inventory_id INTEGER NOT NULL REFERENCES inventory(id) ON DELETE CASCADE,
                material_id INTEGER NOT NULL REFERENCES material(id) ON DELETE CASCADE,
                book REAL NOT NULL DEFAULT 0,
                fact REAL NOT NULL DEFAULT 0,
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_invline ON inventory_line(inventory_id);

            CREATE TABLE IF NOT EXISTS inspection (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                task_id INTEGER,
                kind TEXT NOT NULL DEFAULT 'hidden',
                number TEXT NOT NULL DEFAULT '',
                planned TEXT NOT NULL,
                done TEXT,
                requested_by TEXT NOT NULL DEFAULT '',
                inspector TEXT NOT NULL DEFAULT '',
                place TEXT NOT NULL DEFAULT '',
                result TEXT NOT NULL DEFAULT 'waiting',
                deadline TEXT,
                fixed_at TEXT,
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_inspection_pid ON inspection(project_id);

            CREATE TABLE IF NOT EXISTS concrete_test (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                inspection_id INTEGER,
                task_id INTEGER,
                sample TEXT NOT NULL DEFAULT '',
                grade TEXT NOT NULL DEFAULT '',
                structure TEXT NOT NULL DEFAULT '',
                poured TEXT NOT NULL,
                age_days INTEGER NOT NULL DEFAULT 28,
                required REAL NOT NULL DEFAULT 0,
                actual REAL,
                lab TEXT NOT NULL DEFAULT '',
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_concrete_pid ON concrete_test(project_id);

            CREATE TABLE IF NOT EXISTS geodesy_point (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                inspection_id INTEGER,
                mark TEXT NOT NULL DEFAULT '',
                axis TEXT NOT NULL DEFAULT '',
                level TEXT NOT NULL DEFAULT '',
                design REAL NOT NULL DEFAULT 0,
                fact REAL NOT NULL DEFAULT 0,
                tolerance REAL NOT NULL DEFAULT 0,
                unit TEXT NOT NULL DEFAULT 'mm',
                measured TEXT NOT NULL,
                surveyor TEXT NOT NULL DEFAULT '',
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_geodesy_pid ON geodesy_point(project_id);

            CREATE TABLE IF NOT EXISTS tool (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                code TEXT NOT NULL DEFAULT '',
                name TEXT NOT NULL DEFAULT '',
                kind TEXT NOT NULL DEFAULT 'hand',
                inventory_no TEXT NOT NULL DEFAULT '',
                price REAL NOT NULL DEFAULT 0,
                condition TEXT NOT NULL DEFAULT 'good',
                check_due TEXT,
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_tool_pid ON tool(project_id);

            CREATE TABLE IF NOT EXISTS tool_issue (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                tool_id INTEGER NOT NULL REFERENCES tool(id) ON DELETE CASCADE,
                worker_id INTEGER NOT NULL,
                issued TEXT NOT NULL,
                due TEXT,
                returned TEXT,
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_toolissue_pid ON tool_issue(project_id);

            CREATE TABLE IF NOT EXISTS timesheet_period (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                month TEXT NOT NULL,
                closed INTEGER NOT NULL DEFAULT 0,
                closed_at TEXT,
                closed_by TEXT NOT NULL DEFAULT '',
                reopen_reason TEXT NOT NULL DEFAULT '',
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_tsperiod_pid ON timesheet_period(project_id);

            CREATE TABLE IF NOT EXISTS lab_test (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                task_id INTEGER,
                kind TEXT NOT NULL DEFAULT 'other',
                number TEXT NOT NULL DEFAULT '',
                subject TEXT NOT NULL DEFAULT '',
                date TEXT NOT NULL,
                value REAL,
                required REAL,
                unit TEXT NOT NULL DEFAULT '',
                result TEXT NOT NULL DEFAULT 'waiting',
                lab TEXT NOT NULL DEFAULT '',
                retest TEXT,
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_labtest_pid ON lab_test(project_id);

            CREATE TABLE IF NOT EXISTS machine_booking (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                machine_id INTEGER NOT NULL REFERENCES machine(id) ON DELETE CASCADE,
                task_id INTEGER,
                start TEXT NOT NULL,
                finish TEXT NOT NULL,
                shifts REAL NOT NULL DEFAULT 1,
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_booking_pid ON machine_booking(project_id);

            CREATE TABLE IF NOT EXISTS machine_repair (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                machine_id INTEGER NOT NULL REFERENCES machine(id) ON DELETE CASCADE,
                kind TEXT NOT NULL DEFAULT 'fault',
                started TEXT NOT NULL,
                finished TEXT,
                reason TEXT NOT NULL DEFAULT '',
                cost REAL NOT NULL DEFAULT 0,
                hours_at REAL NOT NULL DEFAULT 0,
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_repair_pid ON machine_repair(project_id);

            CREATE TABLE IF NOT EXISTS machine_check (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                machine_id INTEGER NOT NULL,
                date TEXT NOT NULL,
                by_whom TEXT NOT NULL DEFAULT '',
                items_ok INTEGER NOT NULL DEFAULT 0,
                items_total INTEGER NOT NULL DEFAULT 0,
                fault TEXT NOT NULL DEFAULT '',
                allowed INTEGER NOT NULL DEFAULT 1,
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_mcheck_pid ON machine_check(project_id,date);

            CREATE TABLE IF NOT EXISTS note (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                target TEXT NOT NULL DEFAULT 'other',
                target_id INTEGER NOT NULL DEFAULT 0,
                author TEXT NOT NULL DEFAULT '',
                at TEXT NOT NULL DEFAULT '',
                text TEXT NOT NULL DEFAULT '',
                parent INTEGER,
                resolved INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_note_target ON note(project_id,target,target_id);

            CREATE TABLE IF NOT EXISTS attachment (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                target TEXT NOT NULL DEFAULT 'other',
                target_id INTEGER NOT NULL DEFAULT 0,
                path TEXT NOT NULL DEFAULT '',
                stage TEXT NOT NULL DEFAULT 'plain',
                caption TEXT NOT NULL DEFAULT '',
                author TEXT NOT NULL DEFAULT '',
                at TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_att_target ON attachment(project_id,target,target_id);

            CREATE TABLE IF NOT EXISTS safety_zone (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                kind TEXT NOT NULL DEFAULT 'danger',
                name TEXT NOT NULL DEFAULT '',
                place TEXT NOT NULL DEFAULT '',
                measure TEXT NOT NULL DEFAULT '',
                responsible TEXT NOT NULL DEFAULT '',
                check_due TEXT,
                checked_at TEXT,
                ready INTEGER NOT NULL DEFAULT 0,
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_zone_pid ON safety_zone(project_id);

            CREATE TABLE IF NOT EXISTS contract (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                number TEXT NOT NULL DEFAULT '',
                name TEXT NOT NULL DEFAULT '',
                kind TEXT NOT NULL DEFAULT 'general',
                party_id INTEGER,
                signed TEXT NOT NULL,
                start TEXT NOT NULL,
                finish TEXT NOT NULL,
                sum REAL NOT NULL DEFAULT 0,
                advance_pct REAL NOT NULL DEFAULT 0,
                retention_pct REAL NOT NULL DEFAULT 0,
                currency TEXT NOT NULL DEFAULT 'UZS',
                status TEXT NOT NULL DEFAULT 'active',
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_contract_pid ON contract(project_id);

            CREATE TABLE IF NOT EXISTS contract_change (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                contract_id INTEGER,
                number TEXT NOT NULL DEFAULT '',
                kind TEXT NOT NULL DEFAULT 'extra',
                date TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                amount REAL NOT NULL DEFAULT 0,
                days INTEGER NOT NULL DEFAULT 0,
                reason TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'draft',
                decided_at TEXT,
                decided_by TEXT NOT NULL DEFAULT '',
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_change_pid ON contract_change(project_id);

            CREATE TABLE IF NOT EXISTS payment_stage (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                contract_id INTEGER,
                number TEXT NOT NULL DEFAULT '',
                basis TEXT NOT NULL DEFAULT '',
                due TEXT NOT NULL,
                amount REAL NOT NULL DEFAULT 0,
                paid REAL NOT NULL DEFAULT 0,
                paid_at TEXT,
                note TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_pstage_pid ON payment_stage(project_id);

            CREATE TABLE IF NOT EXISTS work_acceptance (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                task_id INTEGER,
                number TEXT NOT NULL DEFAULT '',
                date TEXT NOT NULL,
                volume REAL NOT NULL DEFAULT 0,
                unit TEXT NOT NULL DEFAULT '',
                amount REAL NOT NULL DEFAULT 0,
                state TEXT NOT NULL DEFAULT 'submitted',
                decided_at TEXT,
                decided_by TEXT NOT NULL DEFAULT '',
                comment TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_accept_pid ON work_acceptance(project_id);
            "#,
        )?;

        // Keyin qo'shilgan ustunlar: eski bazada bo'lmasa yaratamiz.
        for sql in [
            "ALTER TABLE ppr ADD COLUMN author TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE ppr ADD COLUMN approved_at TEXT",
            "ALTER TABLE issue ADD COLUMN deadline TEXT",
            "ALTER TABLE journal ADD COLUMN photos TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE journal ADD COLUMN gps TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE stock_move ADD COLUMN warehouse_id INTEGER",
            "ALTER TABLE worker ADD COLUMN brigade_id INTEGER",
            "ALTER TABLE purchase ADD COLUMN delivered_qty REAL NOT NULL DEFAULT 0",
            "ALTER TABLE request ADD COLUMN reject_reason TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE quality_check ADD COLUMN checklist_id INTEGER",
            "ALTER TABLE material ADD COLUMN estimate_code TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE estimate ADD COLUMN overhead_pct REAL NOT NULL DEFAULT 0",
            "ALTER TABLE estimate ADD COLUMN profit_pct REAL NOT NULL DEFAULT 0",
            "ALTER TABLE estimate ADD COLUMN vat_pct REAL NOT NULL DEFAULT 0",
            "ALTER TABLE estimate_item ADD COLUMN task_id INTEGER",
            "ALTER TABLE material ADD COLUMN spec_ref TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE material ADD COLUMN special TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE material ADD COLUMN banned INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE material ADD COLUMN ban_reason TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE machine ADD COLUMN fuel_norm REAL NOT NULL DEFAULT 0",
            "ALTER TABLE machine ADD COLUMN service_hours REAL NOT NULL DEFAULT 0",
            "ALTER TABLE machine ADD COLUMN service_done REAL NOT NULL DEFAULT 0",
            "ALTER TABLE machine ADD COLUMN rented INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE machine ADD COLUMN price REAL NOT NULL DEFAULT 0",
            "ALTER TABLE safety_event ADD COLUMN root_cause TEXT NOT NULL DEFAULT 'unknown'",
            "ALTER TABLE purchase ADD COLUMN paid REAL NOT NULL DEFAULT 0",
            "ALTER TABLE purchase ADD COLUMN pay_due TEXT",
            "ALTER TABLE purchase ADD COLUMN material_id INTEGER",
            "ALTER TABLE purchase ADD COLUMN substitute_for INTEGER",
            "ALTER TABLE purchase ADD COLUMN tech_ok INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE purchase ADD COLUMN tech_by TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE document ADD COLUMN revision TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE document ADD COLUMN version INTEGER NOT NULL DEFAULT 1",
            "ALTER TABLE document ADD COLUMN replaces INTEGER",
            "ALTER TABLE document ADD COLUMN change_note TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE document ADD COLUMN issued TEXT",
            "ALTER TABLE exec_doc ADD COLUMN version INTEGER NOT NULL DEFAULT 1",
            "ALTER TABLE exec_doc ADD COLUMN replaces INTEGER",
            "ALTER TABLE machine_log ADD COLUMN number TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE machine_log ADD COLUMN driver TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE machine_log ADD COLUMN route TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE machine_log ADD COLUMN odo_start REAL NOT NULL DEFAULT 0",
            "ALTER TABLE machine_log ADD COLUMN odo_end REAL NOT NULL DEFAULT 0",
            "ALTER TABLE machine_log ADD COLUMN trips INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE machine_log ADD COLUMN cargo REAL NOT NULL DEFAULT 0",
            "ALTER TABLE quality_check ADD COLUMN fixed_at TEXT",
            "ALTER TABLE purchase ADD COLUMN section TEXT NOT NULL DEFAULT 'none'",
            "ALTER TABLE purchase ADD COLUMN task_id INTEGER",
            "ALTER TABLE purchase ADD COLUMN contract_id INTEGER",
            "ALTER TABLE purchase ADD COLUMN urgent INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE purchase ADD COLUMN buyer TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE timesheet ADD COLUMN kind TEXT NOT NULL DEFAULT 'work'",
            "ALTER TABLE timesheet ADD COLUMN shift TEXT NOT NULL DEFAULT 'day'",
            "ALTER TABLE stock_move ADD COLUMN batch_id INTEGER",
        ] {
            let _ = self.conn().execute(sql, []);
        }
        Ok(())
    }

    // ---------- XIX–XX. Sotuv ----------

    pub fn blocks(&self, pid: i64) -> Vec<Block> {
        self.list(
            "SELECT id,project_id,name,floors,first_floor,note
             FROM block WHERE project_id=?1 ORDER BY name,id",
            pid,
            |r| {
                Ok(Block {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    name: r.get(2)?,
                    floors: r.get(3)?,
                    first_floor: r.get(4)?,
                    note: r.get(5)?,
                })
            },
        )
    }

    pub fn insert_block(&self, b: &Block) -> i64 {
        self.ins(
            "INSERT INTO block (project_id,name,floors,first_floor,note)
             VALUES (?1,?2,?3,?4,?5)",
            params![b.project_id, b.name, b.floors, b.first_floor, b.note],
        )
    }

    pub fn update_block(&self, b: &Block) -> bool {
        self.upd(
            "UPDATE block SET name=?2,floors=?3,first_floor=?4,note=?5 WHERE id=?1",
            params![b.id, b.name, b.floors, b.first_floor, b.note],
        )
    }

    pub fn units(&self, pid: i64) -> Vec<Unit> {
        self.list(
            "SELECT id,project_id,block_id,number,floor,position,kind,rooms,area,area_living,
                    price_per_m2,status,layout,note
             FROM unit WHERE project_id=?1 ORDER BY block_id,floor,position,id",
            pid,
            |r| {
                Ok(Unit {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    block_id: r.get(2)?,
                    number: r.get(3)?,
                    floor: r.get(4)?,
                    position: r.get(5)?,
                    kind: UnitKind::parse(&r.get::<_, String>(6)?),
                    rooms: r.get(7)?,
                    area: r.get(8)?,
                    area_living: r.get(9)?,
                    price_per_m2: r.get(10)?,
                    status: UnitStatus::parse(&r.get::<_, String>(11)?),
                    layout: r.get(12)?,
                    note: r.get(13)?,
                })
            },
        )
    }

    pub fn insert_unit(&self, u: &Unit) -> i64 {
        self.ins(
            "INSERT INTO unit (project_id,block_id,number,floor,position,kind,rooms,area,
                               area_living,price_per_m2,status,layout,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                u.project_id,
                u.block_id,
                u.number,
                u.floor,
                u.position,
                u.kind.code(),
                u.rooms,
                u.area,
                u.area_living,
                u.price_per_m2,
                u.status.code(),
                u.layout,
                u.note
            ],
        )
    }

    pub fn update_unit(&self, u: &Unit) -> bool {
        self.upd(
            "UPDATE unit SET block_id=?2,number=?3,floor=?4,position=?5,kind=?6,rooms=?7,area=?8,
                             area_living=?9,price_per_m2=?10,status=?11,layout=?12,note=?13
             WHERE id=?1",
            params![
                u.id,
                u.block_id,
                u.number,
                u.floor,
                u.position,
                u.kind.code(),
                u.rooms,
                u.area,
                u.area_living,
                u.price_per_m2,
                u.status.code(),
                u.layout,
                u.note
            ],
        )
    }

    pub fn deals(&self, pid: i64) -> Vec<Deal> {
        self.list(
            "SELECT id,project_id,unit_id,number,date,client,phone,client_doc,pay_kind,price,
                    discount,prepayment,months,status,manager,note
             FROM deal WHERE project_id=?1 ORDER BY date DESC,id DESC",
            pid,
            |r| {
                Ok(Deal {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    unit_id: r.get(2)?,
                    number: r.get(3)?,
                    date: date(&r.get::<_, String>(4)?),
                    client: r.get(5)?,
                    phone: r.get(6)?,
                    client_doc: r.get(7)?,
                    pay_kind: PayKind::parse(&r.get::<_, String>(8)?),
                    price: r.get(9)?,
                    discount: r.get(10)?,
                    prepayment: r.get(11)?,
                    months: r.get(12)?,
                    status: DealStatus::parse(&r.get::<_, String>(13)?),
                    manager: r.get(14)?,
                    note: r.get(15)?,
                })
            },
        )
    }

    pub fn insert_deal(&self, d: &Deal) -> i64 {
        self.ins(
            "INSERT INTO deal (project_id,unit_id,number,date,client,phone,client_doc,pay_kind,
                               price,discount,prepayment,months,status,manager,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![
                d.project_id,
                d.unit_id,
                d.number,
                d.date.to_string(),
                d.client,
                d.phone,
                d.client_doc,
                d.pay_kind.code(),
                d.price,
                d.discount,
                d.prepayment,
                d.months,
                d.status.code(),
                d.manager,
                d.note
            ],
        )
    }

    pub fn update_deal(&self, d: &Deal) -> bool {
        self.upd(
            "UPDATE deal SET unit_id=?2,number=?3,date=?4,client=?5,phone=?6,client_doc=?7,
                             pay_kind=?8,price=?9,discount=?10,prepayment=?11,months=?12,
                             status=?13,manager=?14,note=?15
             WHERE id=?1",
            params![
                d.id,
                d.unit_id,
                d.number,
                d.date.to_string(),
                d.client,
                d.phone,
                d.client_doc,
                d.pay_kind.code(),
                d.price,
                d.discount,
                d.prepayment,
                d.months,
                d.status.code(),
                d.manager,
                d.note
            ],
        )
    }

    pub fn payments(&self, pid: i64) -> Vec<Payment> {
        self.list(
            "SELECT id,project_id,deal_id,due,planned,paid,paid_date,kind,document,note
             FROM payment WHERE project_id=?1 ORDER BY due,id",
            pid,
            |r| {
                Ok(Payment {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    deal_id: r.get(2)?,
                    due: date(&r.get::<_, String>(3)?),
                    planned: r.get(4)?,
                    paid: r.get(5)?,
                    paid_date: odate(r.get(6)?),
                    kind: PayKind::parse(&r.get::<_, String>(7)?),
                    document: r.get(8)?,
                    note: r.get(9)?,
                })
            },
        )
    }

    pub fn insert_payment(&self, p: &Payment) -> i64 {
        self.ins(
            "INSERT INTO payment (project_id,deal_id,due,planned,paid,paid_date,kind,document,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                p.project_id,
                p.deal_id,
                p.due.to_string(),
                p.planned,
                p.paid,
                p.paid_date.map(|d| d.to_string()),
                p.kind.code(),
                p.document,
                p.note
            ],
        )
    }

    pub fn update_payment(&self, p: &Payment) -> bool {
        self.upd(
            "UPDATE payment SET due=?2,planned=?3,paid=?4,paid_date=?5,kind=?6,document=?7,note=?8
             WHERE id=?1",
            params![
                p.id,
                p.due.to_string(),
                p.planned,
                p.paid,
                p.paid_date.map(|d| d.to_string()),
                p.kind.code(),
                p.document,
                p.note
            ],
        )
    }

    /// Bitta shartnomaning to'lov grafigini o'chiradi — qayta yaratishdan oldin.
    pub fn clear_payments(&self, deal_id: i64) -> bool {
        self.upd("DELETE FROM payment WHERE deal_id=?1", params![deal_id])
    }

    // ---------- Rollar ----------

    pub fn users(&self) -> Vec<User> {
        let Ok(mut st) = self
            .conn()
            .prepare("SELECT id,name,role,note FROM app_user ORDER BY id")
        else {
            return Vec::new();
        };
        let Ok(rows) = st.query_map([], |r| {
            Ok(User {
                id: r.get(0)?,
                name: r.get(1)?,
                role: Role::parse(&r.get::<_, String>(2)?),
                note: r.get(3)?,
            })
        }) else {
            return Vec::new();
        };
        rows.filter_map(|x| x.ok()).collect()
    }

    pub fn insert_user(&self, u: &User) -> i64 {
        self.ins(
            "INSERT INTO app_user (name,role,note) VALUES (?1,?2,?3)",
            params![u.name, u.role.code(), u.note],
        )
    }

    pub fn update_user(&self, u: &User) -> bool {
        self.upd(
            "UPDATE app_user SET name=?2,role=?3,note=?4 WHERE id=?1",
            params![u.id, u.name, u.role.code(), u.note],
        )
    }

    // ---------- XI. Omborlar, partiyalar, rezerv, inventarizatsiya ----------

    pub fn warehouses(&self, pid: i64) -> Vec<Warehouse> {
        self.list(
            "SELECT id,project_id,name,kind,responsible,note
             FROM warehouse WHERE project_id=?1 ORDER BY id",
            pid,
            |r| {
                Ok(Warehouse {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    name: r.get(2)?,
                    kind: WarehouseKind::parse(&r.get::<_, String>(3)?),
                    responsible: r.get(4)?,
                    note: r.get(5)?,
                })
            },
        )
    }

    pub fn insert_warehouse(&self, w: &Warehouse) -> i64 {
        self.ins(
            "INSERT INTO warehouse (project_id,name,kind,responsible,note)
             VALUES (?1,?2,?3,?4,?5)",
            params![w.project_id, w.name, w.kind.code(), w.responsible, w.note],
        )
    }

    pub fn update_warehouse(&self, w: &Warehouse) -> bool {
        self.upd(
            "UPDATE warehouse SET name=?2,kind=?3,responsible=?4,note=?5 WHERE id=?1",
            params![w.id, w.name, w.kind.code(), w.responsible, w.note],
        )
    }

    pub fn batches(&self, pid: i64) -> Vec<Batch> {
        self.list(
            "SELECT id,project_id,material_id,number,received,supplier,cert_no,cert_until,
                    expires,note
             FROM batch WHERE project_id=?1 ORDER BY received,id",
            pid,
            |r| {
                Ok(Batch {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    material_id: r.get(2)?,
                    number: r.get(3)?,
                    received: date(&r.get::<_, String>(4)?),
                    supplier: r.get(5)?,
                    cert_no: r.get(6)?,
                    cert_until: odate(r.get(7)?),
                    expires: odate(r.get(8)?),
                    note: r.get(9)?,
                })
            },
        )
    }

    pub fn insert_batch(&self, b: &Batch) -> i64 {
        self.ins(
            "INSERT INTO batch (project_id,material_id,number,received,supplier,cert_no,
                                cert_until,expires,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                b.project_id,
                b.material_id,
                b.number,
                b.received.to_string(),
                b.supplier,
                b.cert_no,
                b.cert_until.map(|d| d.to_string()),
                b.expires.map(|d| d.to_string()),
                b.note
            ],
        )
    }

    pub fn update_batch(&self, b: &Batch) -> bool {
        self.upd(
            "UPDATE batch SET material_id=?2,number=?3,received=?4,supplier=?5,cert_no=?6,
                              cert_until=?7,expires=?8,note=?9
             WHERE id=?1",
            params![
                b.id,
                b.material_id,
                b.number,
                b.received.to_string(),
                b.supplier,
                b.cert_no,
                b.cert_until.map(|d| d.to_string()),
                b.expires.map(|d| d.to_string()),
                b.note
            ],
        )
    }

    pub fn reservations(&self, pid: i64) -> Vec<Reservation> {
        self.list(
            "SELECT id,project_id,material_id,task_id,qty,date,until,note
             FROM reservation WHERE project_id=?1 ORDER BY date,id",
            pid,
            |r| {
                Ok(Reservation {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    material_id: r.get(2)?,
                    task_id: r.get(3)?,
                    qty: r.get(4)?,
                    date: date(&r.get::<_, String>(5)?),
                    until: odate(r.get(6)?),
                    note: r.get(7)?,
                })
            },
        )
    }

    pub fn insert_reservation(&self, r: &Reservation) -> i64 {
        self.ins(
            "INSERT INTO reservation (project_id,material_id,task_id,qty,date,until,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                r.project_id,
                r.material_id,
                r.task_id,
                r.qty,
                r.date.to_string(),
                r.until.map(|d| d.to_string()),
                r.note
            ],
        )
    }

    pub fn update_reservation(&self, r: &Reservation) -> bool {
        self.upd(
            "UPDATE reservation SET material_id=?2,task_id=?3,qty=?4,date=?5,until=?6,note=?7
             WHERE id=?1",
            params![
                r.id,
                r.material_id,
                r.task_id,
                r.qty,
                r.date.to_string(),
                r.until.map(|d| d.to_string()),
                r.note
            ],
        )
    }

    pub fn material_norms(&self, pid: i64) -> Vec<MaterialNorm> {
        self.list(
            "SELECT id,project_id,task_id,material_id,per_unit,tolerance,note
             FROM material_norm WHERE project_id=?1 ORDER BY id",
            pid,
            |r| {
                Ok(MaterialNorm {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    task_id: r.get(2)?,
                    material_id: r.get(3)?,
                    per_unit: r.get(4)?,
                    tolerance: r.get(5)?,
                    note: r.get(6)?,
                })
            },
        )
    }

    pub fn insert_material_norm(&self, n: &MaterialNorm) -> i64 {
        self.ins(
            "INSERT INTO material_norm (project_id,task_id,material_id,per_unit,tolerance,note)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                n.project_id,
                n.task_id,
                n.material_id,
                n.per_unit,
                n.tolerance,
                n.note
            ],
        )
    }

    pub fn update_material_norm(&self, n: &MaterialNorm) -> bool {
        self.upd(
            "UPDATE material_norm SET task_id=?2,material_id=?3,per_unit=?4,tolerance=?5,note=?6
             WHERE id=?1",
            params![
                n.id,
                n.task_id,
                n.material_id,
                n.per_unit,
                n.tolerance,
                n.note
            ],
        )
    }

    pub fn inventories(&self, pid: i64) -> Vec<Inventory> {
        self.list(
            "SELECT id,project_id,warehouse_id,date,responsible,closed,note
             FROM inventory WHERE project_id=?1 ORDER BY date DESC,id DESC",
            pid,
            |r| {
                Ok(Inventory {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    warehouse_id: r.get(2)?,
                    date: date(&r.get::<_, String>(3)?),
                    responsible: r.get(4)?,
                    closed: r.get::<_, i64>(5)? != 0,
                    note: r.get(6)?,
                })
            },
        )
    }

    pub fn insert_inventory(&self, v: &Inventory) -> i64 {
        self.ins(
            "INSERT INTO inventory (project_id,warehouse_id,date,responsible,closed,note)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                v.project_id,
                v.warehouse_id,
                v.date.to_string(),
                v.responsible,
                i64::from(v.closed),
                v.note
            ],
        )
    }

    pub fn update_inventory(&self, v: &Inventory) -> bool {
        self.upd(
            "UPDATE inventory SET warehouse_id=?2,date=?3,responsible=?4,closed=?5,note=?6
             WHERE id=?1",
            params![
                v.id,
                v.warehouse_id,
                v.date.to_string(),
                v.responsible,
                i64::from(v.closed),
                v.note
            ],
        )
    }

    pub fn inventory_lines(&self, pid: i64) -> Vec<InventoryLine> {
        self.list(
            "SELECT l.id,l.inventory_id,l.material_id,l.book,l.fact,l.note
             FROM inventory_line l
             JOIN inventory v ON v.id = l.inventory_id
             WHERE v.project_id=?1 ORDER BY l.id",
            pid,
            |r| {
                Ok(InventoryLine {
                    id: r.get(0)?,
                    inventory_id: r.get(1)?,
                    material_id: r.get(2)?,
                    book: r.get(3)?,
                    fact: r.get(4)?,
                    note: r.get(5)?,
                })
            },
        )
    }

    pub fn insert_inventory_line(&self, l: &InventoryLine) -> i64 {
        self.ins(
            "INSERT INTO inventory_line (inventory_id,material_id,book,fact,note)
             VALUES (?1,?2,?3,?4,?5)",
            params![l.inventory_id, l.material_id, l.book, l.fact, l.note],
        )
    }

    pub fn update_inventory_line(&self, l: &InventoryLine) -> bool {
        self.upd(
            "UPDATE inventory_line SET material_id=?2,book=?3,fact=?4,note=?5 WHERE id=?1",
            params![l.id, l.material_id, l.book, l.fact, l.note],
        )
    }

    // ---------- XV.4-12. Ruxsatlar, SIZ, naryad-dopusk ----------

    pub fn worker_permits(&self, pid: i64) -> Vec<WorkerPermit> {
        self.list(
            "SELECT id,project_id,worker_id,kind,number,issued,valid_until,note
             FROM worker_permit WHERE project_id=?1 ORDER BY worker_id,kind",
            pid,
            |r| {
                Ok(WorkerPermit {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    worker_id: r.get(2)?,
                    kind: PermitKind::parse(&r.get::<_, String>(3)?),
                    number: r.get(4)?,
                    issued: date(&r.get::<_, String>(5)?),
                    valid_until: date(&r.get::<_, String>(6)?),
                    note: r.get(7)?,
                })
            },
        )
    }

    pub fn insert_worker_permit(&self, p: &WorkerPermit) -> i64 {
        self.ins(
            "INSERT INTO worker_permit (project_id,worker_id,kind,number,issued,valid_until,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                p.project_id,
                p.worker_id,
                p.kind.code(),
                p.number,
                p.issued.to_string(),
                p.valid_until.to_string(),
                p.note
            ],
        )
    }

    pub fn update_worker_permit(&self, p: &WorkerPermit) -> bool {
        self.upd(
            "UPDATE worker_permit SET worker_id=?2,kind=?3,number=?4,issued=?5,valid_until=?6,
                    note=?7 WHERE id=?1",
            params![
                p.id,
                p.worker_id,
                p.kind.code(),
                p.number,
                p.issued.to_string(),
                p.valid_until.to_string(),
                p.note
            ],
        )
    }

    pub fn ppe_issues(&self, pid: i64) -> Vec<PpeIssue> {
        self.list(
            "SELECT id,project_id,worker_id,item,issued,months,note
             FROM ppe_issue WHERE project_id=?1 ORDER BY worker_id,item",
            pid,
            |r| {
                Ok(PpeIssue {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    worker_id: r.get(2)?,
                    item: PpeItem::parse(&r.get::<_, String>(3)?),
                    issued: date(&r.get::<_, String>(4)?),
                    months: r.get(5)?,
                    note: r.get(6)?,
                })
            },
        )
    }

    pub fn insert_ppe_issue(&self, p: &PpeIssue) -> i64 {
        self.ins(
            "INSERT INTO ppe_issue (project_id,worker_id,item,issued,months,note)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                p.project_id,
                p.worker_id,
                p.item.code(),
                p.issued.to_string(),
                p.months,
                p.note
            ],
        )
    }

    pub fn update_ppe_issue(&self, p: &PpeIssue) -> bool {
        self.upd(
            "UPDATE ppe_issue SET worker_id=?2,item=?3,issued=?4,months=?5,note=?6 WHERE id=?1",
            params![
                p.id,
                p.worker_id,
                p.item.code(),
                p.issued.to_string(),
                p.months,
                p.note
            ],
        )
    }

    pub fn work_permits(&self, pid: i64) -> Vec<WorkPermit> {
        self.list(
            "SELECT id,project_id,number,kind,task_id,place,date_from,date_to,issuer,supervisor,
                    workers,measures,status,note
             FROM work_permit WHERE project_id=?1 ORDER BY date_from DESC,id DESC",
            pid,
            |r| {
                Ok(WorkPermit {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    number: r.get(2)?,
                    kind: PermitKind::parse(&r.get::<_, String>(3)?),
                    task_id: r.get(4)?,
                    place: r.get(5)?,
                    date_from: date(&r.get::<_, String>(6)?),
                    date_to: date(&r.get::<_, String>(7)?),
                    issuer: r.get(8)?,
                    supervisor: r.get(9)?,
                    workers: r.get(10)?,
                    measures: r.get(11)?,
                    status: PermitStatus::parse(&r.get::<_, String>(12)?),
                    note: r.get(13)?,
                })
            },
        )
    }

    pub fn insert_work_permit(&self, p: &WorkPermit) -> i64 {
        self.ins(
            "INSERT INTO work_permit (project_id,number,kind,task_id,place,date_from,date_to,
                                      issuer,supervisor,workers,measures,status,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                p.project_id,
                p.number,
                p.kind.code(),
                p.task_id,
                p.place,
                p.date_from.to_string(),
                p.date_to.to_string(),
                p.issuer,
                p.supervisor,
                p.workers,
                p.measures,
                p.status.code(),
                p.note
            ],
        )
    }

    pub fn update_work_permit(&self, p: &WorkPermit) -> bool {
        self.upd(
            "UPDATE work_permit SET number=?2,kind=?3,task_id=?4,place=?5,date_from=?6,date_to=?7,
                    issuer=?8,supervisor=?9,workers=?10,measures=?11,status=?12,note=?13
             WHERE id=?1",
            params![
                p.id,
                p.number,
                p.kind.code(),
                p.task_id,
                p.place,
                p.date_from.to_string(),
                p.date_to.to_string(),
                p.issuer,
                p.supervisor,
                p.workers,
                p.measures,
                p.status.code(),
                p.note
            ],
        )
    }

    // ---------- XIV.8. Chek-listlar va nazorat nuqtalari ----------

    pub fn checklists(&self, pid: i64) -> Vec<Checklist> {
        self.list(
            "SELECT id,project_id,name,section,kind,note
             FROM checklist WHERE project_id=?1 ORDER BY name",
            pid,
            |r| {
                Ok(Checklist {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    name: r.get(2)?,
                    section: Section::parse(&r.get::<_, String>(3)?),
                    kind: QualityKind::parse(&r.get::<_, String>(4)?),
                    note: r.get(5)?,
                })
            },
        )
    }

    pub fn insert_checklist(&self, c: &Checklist) -> i64 {
        self.ins(
            "INSERT INTO checklist (project_id,name,section,kind,note) VALUES (?1,?2,?3,?4,?5)",
            params![
                c.project_id,
                c.name,
                c.section.code(),
                c.kind.code(),
                c.note
            ],
        )
    }

    pub fn update_checklist(&self, c: &Checklist) -> bool {
        self.upd(
            "UPDATE checklist SET name=?2,section=?3,kind=?4,note=?5 WHERE id=?1",
            params![c.id, c.name, c.section.code(), c.kind.code(), c.note],
        )
    }

    pub fn checklist_items(&self, pid: i64) -> Vec<ChecklistItem> {
        self.list(
            "SELECT i.id,i.checklist_id,i.pos,i.text,i.norm_doc,i.norm_clause
             FROM checklist_item i
             JOIN checklist c ON c.id = i.checklist_id
             WHERE c.project_id=?1 ORDER BY i.checklist_id,i.pos,i.id",
            pid,
            |r| {
                Ok(ChecklistItem {
                    id: r.get(0)?,
                    checklist_id: r.get(1)?,
                    pos: r.get(2)?,
                    text: r.get(3)?,
                    norm_doc: r.get(4)?,
                    norm_clause: r.get(5)?,
                })
            },
        )
    }

    pub fn insert_checklist_item(&self, i: &ChecklistItem) -> i64 {
        self.ins(
            "INSERT INTO checklist_item (checklist_id,pos,text,norm_doc,norm_clause)
             VALUES (?1,?2,?3,?4,?5)",
            params![i.checklist_id, i.pos, i.text, i.norm_doc, i.norm_clause],
        )
    }

    pub fn update_checklist_item(&self, i: &ChecklistItem) -> bool {
        self.upd(
            "UPDATE checklist_item SET pos=?2,text=?3,norm_doc=?4,norm_clause=?5 WHERE id=?1",
            params![i.id, i.pos, i.text, i.norm_doc, i.norm_clause],
        )
    }

    pub fn check_points(&self, pid: i64) -> Vec<CheckPoint> {
        self.list(
            "SELECT p.id,p.check_id,p.pos,p.text,p.norm_doc,p.norm_clause,p.result,p.note
             FROM check_point p
             JOIN quality_check q ON q.id = p.check_id
             WHERE q.project_id=?1 ORDER BY p.check_id,p.pos,p.id",
            pid,
            |r| {
                Ok(CheckPoint {
                    id: r.get(0)?,
                    check_id: r.get(1)?,
                    pos: r.get(2)?,
                    text: r.get(3)?,
                    norm_doc: r.get(4)?,
                    norm_clause: r.get(5)?,
                    result: PointResult::parse(&r.get::<_, String>(6)?),
                    note: r.get(7)?,
                })
            },
        )
    }

    pub fn insert_check_point(&self, p: &CheckPoint) -> i64 {
        self.ins(
            "INSERT INTO check_point (check_id,pos,text,norm_doc,norm_clause,result,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                p.check_id,
                p.pos,
                p.text,
                p.norm_doc,
                p.norm_clause,
                p.result.code(),
                p.note
            ],
        )
    }

    pub fn update_check_point(&self, p: &CheckPoint) -> bool {
        self.upd(
            "UPDATE check_point SET pos=?2,text=?3,norm_doc=?4,norm_clause=?5,result=?6,note=?7
             WHERE id=?1",
            params![
                p.id,
                p.pos,
                p.text,
                p.norm_doc,
                p.norm_clause,
                p.result.code(),
                p.note
            ],
        )
    }

    /// Chek-list bandlarini tekshiruvga ko'chiradi (TZ XIV.8).
    ///
    /// Ko'chirib olinadi, havola qilinmaydi: namuna keyin o'zgarsa ham
    /// o'tkazilgan tekshiruv o'zgarmaydi.
    pub fn apply_checklist(&self, check_id: i64, checklist_id: i64) -> usize {
        let _ = self.conn().execute(
            "DELETE FROM check_point WHERE check_id=?1",
            params![check_id],
        );
        let items: Vec<ChecklistItem> = self
            .conn()
            .prepare(
                "SELECT id,checklist_id,pos,text,norm_doc,norm_clause
                 FROM checklist_item WHERE checklist_id=?1 ORDER BY pos,id",
            )
            .and_then(|mut st| {
                st.query_map(params![checklist_id], |r| {
                    Ok(ChecklistItem {
                        id: r.get(0)?,
                        checklist_id: r.get(1)?,
                        pos: r.get(2)?,
                        text: r.get(3)?,
                        norm_doc: r.get(4)?,
                        norm_clause: r.get(5)?,
                    })
                })
                .and_then(|rows| rows.collect())
            })
            .unwrap_or_default();

        for (n, i) in items.iter().enumerate() {
            self.insert_check_point(&CheckPoint {
                id: 0,
                check_id,
                pos: n as i64 + 1,
                text: i.text.clone(),
                norm_doc: i.norm_doc.clone(),
                norm_clause: i.norm_clause.clone(),
                result: PointResult::Pending,
                note: String::new(),
            });
        }
        items.len()
    }

    // ---------- IX.8-10. Kelishuv marshruti ----------

    pub fn approvals(&self, pid: i64) -> Vec<Approval> {
        self.list(
            "SELECT id,project_id,request_id,step,role,approver,decision,decided_at,comment
             FROM approval WHERE project_id=?1 ORDER BY request_id,step",
            pid,
            |r| {
                Ok(Approval {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    request_id: r.get(2)?,
                    step: r.get(3)?,
                    role: r.get(4)?,
                    approver: r.get(5)?,
                    decision: ApprovalDecision::parse(&r.get::<_, String>(6)?),
                    decided_at: odate(r.get(7)?),
                    comment: r.get(8)?,
                })
            },
        )
    }

    pub fn insert_approval(&self, a: &Approval) -> i64 {
        self.ins(
            "INSERT INTO approval (project_id,request_id,step,role,approver,decision,decided_at,
                                   comment)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                a.project_id,
                a.request_id,
                a.step,
                a.role,
                a.approver,
                a.decision.code(),
                a.decided_at.map(|d| d.to_string()),
                a.comment
            ],
        )
    }

    pub fn update_approval(&self, a: &Approval) -> bool {
        self.upd(
            "UPDATE approval SET step=?2,role=?3,approver=?4,decision=?5,decided_at=?6,comment=?7
             WHERE id=?1",
            params![
                a.id,
                a.step,
                a.role,
                a.approver,
                a.decision.code(),
                a.decided_at.map(|d| d.to_string()),
                a.comment
            ],
        )
    }

    /// Arizaning barcha kelishuv bosqichlarini o'chiradi — marshrut qayta quriladi.
    pub fn clear_approvals(&self, request_id: i64) -> bool {
        self.conn()
            .execute(
                "DELETE FROM approval WHERE request_id=?1",
                params![request_id],
            )
            .is_ok()
    }

    // ---------- X.7-15, 34-35. Yetkazib beruvchilar, KP, byudjet ----------

    pub fn suppliers(&self, pid: i64) -> Vec<Supplier> {
        self.list(
            "SELECT id,project_id,name,inn,contact,phone,blocked,note
             FROM supplier WHERE project_id=?1 ORDER BY name",
            pid,
            |r| {
                Ok(Supplier {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    name: r.get(2)?,
                    inn: r.get(3)?,
                    contact: r.get(4)?,
                    phone: r.get(5)?,
                    blocked: r.get::<_, i64>(6)? != 0,
                    note: r.get(7)?,
                })
            },
        )
    }

    pub fn insert_supplier(&self, x: &Supplier) -> i64 {
        self.ins(
            "INSERT INTO supplier (project_id,name,inn,contact,phone,blocked,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                x.project_id,
                x.name,
                x.inn,
                x.contact,
                x.phone,
                i64::from(x.blocked),
                x.note
            ],
        )
    }

    pub fn update_supplier(&self, x: &Supplier) -> bool {
        self.upd(
            "UPDATE supplier SET name=?2,inn=?3,contact=?4,phone=?5,blocked=?6,note=?7 WHERE id=?1",
            params![
                x.id,
                x.name,
                x.inn,
                x.contact,
                x.phone,
                i64::from(x.blocked),
                x.note
            ],
        )
    }

    pub fn quotes(&self, pid: i64) -> Vec<Quote> {
        self.list(
            "SELECT id,project_id,request_id,supplier,title,qty,unit,price,currency,
                    delivery_days,valid_until,chosen,date,note
             FROM quote WHERE project_id=?1 ORDER BY request_id,price",
            pid,
            |r| {
                Ok(Quote {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    request_id: r.get(2)?,
                    supplier: r.get(3)?,
                    title: r.get(4)?,
                    qty: r.get(5)?,
                    unit: r.get(6)?,
                    price: r.get(7)?,
                    currency: r.get(8)?,
                    delivery_days: r.get(9)?,
                    valid_until: odate(r.get(10)?),
                    chosen: r.get::<_, i64>(11)? != 0,
                    date: date(&r.get::<_, String>(12)?),
                    note: r.get(13)?,
                })
            },
        )
    }

    pub fn insert_quote(&self, q: &Quote) -> i64 {
        self.ins(
            "INSERT INTO quote (project_id,request_id,supplier,title,qty,unit,price,currency,
                                delivery_days,valid_until,chosen,date,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                q.project_id,
                q.request_id,
                q.supplier,
                q.title,
                q.qty,
                q.unit,
                q.price,
                q.currency,
                q.delivery_days,
                q.valid_until.map(|d| d.to_string()),
                i64::from(q.chosen),
                q.date.to_string(),
                q.note
            ],
        )
    }

    pub fn update_quote(&self, q: &Quote) -> bool {
        self.upd(
            "UPDATE quote SET request_id=?2,supplier=?3,title=?4,qty=?5,unit=?6,price=?7,
                    currency=?8,delivery_days=?9,valid_until=?10,chosen=?11,date=?12,note=?13
             WHERE id=?1",
            params![
                q.id,
                q.request_id,
                q.supplier,
                q.title,
                q.qty,
                q.unit,
                q.price,
                q.currency,
                q.delivery_days,
                q.valid_until.map(|d| d.to_string()),
                i64::from(q.chosen),
                q.date.to_string(),
                q.note
            ],
        )
    }

    /// Bitta arizada faqat bitta taklif tanlangan bo'ladi (TZ X.12).
    pub fn choose_quote(&self, id: i64) -> bool {
        let Ok(conn) = self.conn().execute(
            "UPDATE quote SET chosen = 0
             WHERE request_id IS NOT NULL
               AND request_id = (SELECT request_id FROM quote WHERE id=?1)",
            params![id],
        ) else {
            return false;
        };
        let _ = conn;
        self.upd("UPDATE quote SET chosen = 1 WHERE id=?1", params![id])
    }

    pub fn purchase_budgets(&self, pid: i64) -> Vec<PurchaseBudget> {
        self.list(
            "SELECT id,project_id,section,planned,note
             FROM purchase_budget WHERE project_id=?1 ORDER BY id",
            pid,
            |r| {
                Ok(PurchaseBudget {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    section: Section::parse(&r.get::<_, String>(2)?),
                    planned: r.get(3)?,
                    note: r.get(4)?,
                })
            },
        )
    }

    pub fn insert_purchase_budget(&self, b: &PurchaseBudget) -> i64 {
        self.ins(
            "INSERT INTO purchase_budget (project_id,section,planned,note) VALUES (?1,?2,?3,?4)",
            params![b.project_id, b.section.code(), b.planned, b.note],
        )
    }

    pub fn update_purchase_budget(&self, b: &PurchaseBudget) -> bool {
        self.upd(
            "UPDATE purchase_budget SET section=?2,planned=?3,note=?4 WHERE id=?1",
            params![b.id, b.section.code(), b.planned, b.note],
        )
    }

    // ---------- Umumiy yordamchilar ----------

    fn list<T>(&self, sql: &str, pid: i64, f: impl Fn(&Row) -> rusqlite::Result<T>) -> Vec<T> {
        let Ok(mut st) = self.conn().prepare(sql) else {
            return Vec::new();
        };
        let Ok(rows) = st.query_map([pid], |r| f(r)) else {
            return Vec::new();
        };
        rows.filter_map(|x| x.ok()).collect()
    }

    fn ins(&self, sql: &str, p: &[&dyn rusqlite::ToSql]) -> i64 {
        match self.conn().execute(sql, p) {
            Ok(_) => {
                let id = self.conn().last_insert_rowid();
                self.audit(AuditAction::Insert, table_of_sql(sql), id);
                id
            }
            Err(_) => 0,
        }
    }

    fn upd(&self, sql: &str, p: &[&dyn rusqlite::ToSql]) -> bool {
        let ok = self.conn().execute(sql, p).is_ok();
        if ok {
            // `UPDATE ... WHERE id=?1` — birinchi parametr doim id bo'ladi;
            // `INSERT ... ON CONFLICT` da esa id ma'lum emas, nol qoladi.
            self.audit(AuditAction::Update, table_of_sql(sql), 0);
        }
        ok
    }

    pub fn del(&self, table: &str, id: i64) -> bool {
        // Jadval nomi kod ichidan keladi, foydalanuvchidan emas.
        let ok = self
            .conn()
            .execute(&format!("DELETE FROM {table} WHERE id=?1"), [id])
            .is_ok();
        if ok {
            self.audit(AuditAction::Delete, table, id);
        }
        ok
    }

    // ---------- Amallar tarixi (umumiy talab) ----------

    /// Bitta o'zgarishni jurnalga yozadi.
    ///
    /// Jurnalning o'zi jurnalga tushmaydi va bo'sh jadval nomi yozilmaydi:
    /// aks holda sxema migratsiyasi ham «o'zgarish» bo'lib ko'rinardi.
    fn audit(&self, action: AuditAction, table: &str, row_id: i64) {
        if table.is_empty() || table == "audit_log" {
            return;
        }
        // Har bir yozuv o'zgarishi shu yerdan o'tadi — hisoblagich ham
        // shu yerda oshadi, aks holda biror joyda unutilardi.
        self.bump_revision();
        let _ = self.conn().execute(
            "INSERT INTO audit_log (at,user,action,table_name,row_id) VALUES (?1,?2,?3,?4,?5)",
            params![
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                self.audit_user(),
                action.code(),
                table,
                row_id
            ],
        );
    }

    /// Oxirgi amallar, yangisi yuqorida.
    pub fn audit_log(&self, limit: i64) -> Vec<AuditEntry> {
        let Ok(mut st) = self.conn().prepare(
            "SELECT id,at,user,action,table_name,row_id
             FROM audit_log ORDER BY id DESC LIMIT ?1",
        ) else {
            return Vec::new();
        };
        st.query_map(params![limit], |r| {
            Ok(AuditEntry {
                id: r.get(0)?,
                at: r.get(1)?,
                user: r.get(2)?,
                action: AuditAction::parse(&r.get::<_, String>(3)?),
                table_name: r.get(4)?,
                row_id: r.get(5)?,
            })
        })
        .and_then(|rows| rows.collect())
        .unwrap_or_default()
    }

    /// Jurnaldagi yozuvlar soni.
    pub fn audit_count(&self) -> i64 {
        self.conn()
            .query_row("SELECT COUNT(*) FROM audit_log", [], |r| r.get(0))
            .unwrap_or(0)
    }

    /// Jurnalni belgilangan hajmda ushlab turadi.
    ///
    /// Cheksiz o'sadigan jurnal bazani shishiradi, shuning uchun eng eski
    /// yozuvlar o'chiriladi. Bu ilova ochilganda bir marta bajariladi.
    pub fn trim_audit_log(&self, keep: i64) -> usize {
        let extra = (self.audit_count() - keep).max(0);
        if extra == 0 {
            return 0;
        }
        self.conn()
            .execute(
                "DELETE FROM audit_log WHERE id IN
                 (SELECT id FROM audit_log ORDER BY id ASC LIMIT ?1)",
                params![extra],
            )
            .unwrap_or(0)
    }

    // ---------- II–III. Nomuvofiqliklar ----------

    pub fn issues(&self, pid: i64) -> Vec<Issue> {
        self.list(
            "SELECT id,project_id,module,section,code,sheet,location,element,title,description,
                    severity,norm_doc,norm_clause,norm_text,recommendation,responsible,status,auto,
                    created_at,deadline
             FROM issue WHERE project_id=?1 ORDER BY id",
            pid,
            |r| {
                Ok(Issue {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    module: IssueModule::parse(&r.get::<_, String>(2)?),
                    section: Section::parse(&r.get::<_, String>(3)?),
                    code: r.get(4)?,
                    sheet: r.get(5)?,
                    location: r.get(6)?,
                    element: r.get(7)?,
                    title: r.get(8)?,
                    description: r.get(9)?,
                    severity: Severity::parse(&r.get::<_, String>(10)?),
                    norm_doc: r.get(11)?,
                    norm_clause: r.get(12)?,
                    norm_text: r.get(13)?,
                    recommendation: r.get(14)?,
                    responsible: r.get(15)?,
                    status: IssueStatus::parse(&r.get::<_, String>(16)?),
                    auto: r.get::<_, i64>(17)? != 0,
                    created_at: r.get(18)?,
                    deadline: odate(r.get::<_, Option<String>>(19)?),
                })
            },
        )
    }

    pub fn insert_issue(&self, i: &Issue) -> i64 {
        self.ins(
            "INSERT INTO issue (project_id,module,section,code,sheet,location,element,title,
                                description,severity,norm_doc,norm_clause,norm_text,recommendation,
                                responsible,status,auto,deadline)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
            params![
                i.project_id,
                i.module.code(),
                i.section.code(),
                i.code,
                i.sheet,
                i.location,
                i.element,
                i.title,
                i.description,
                i.severity.code(),
                i.norm_doc,
                i.norm_clause,
                i.norm_text,
                i.recommendation,
                i.responsible,
                i.status.code(),
                i.auto as i64,
                ods(i.deadline)
            ],
        )
    }

    pub fn update_issue(&self, i: &Issue) -> bool {
        self.upd(
            "UPDATE issue SET module=?2,section=?3,code=?4,sheet=?5,location=?6,element=?7,
                    title=?8,description=?9,severity=?10,norm_doc=?11,norm_clause=?12,norm_text=?13,
                    recommendation=?14,responsible=?15,status=?16,deadline=?17 WHERE id=?1",
            params![
                i.id,
                i.module.code(),
                i.section.code(),
                i.code,
                i.sheet,
                i.location,
                i.element,
                i.title,
                i.description,
                i.severity.code(),
                i.norm_doc,
                i.norm_clause,
                i.norm_text,
                i.recommendation,
                i.responsible,
                i.status.code(),
                ods(i.deadline)
            ],
        )
    }

    /// Avtomatik topilgan nomuvofiqliklarni almashtiradi: qo'lda kiritilganlar
    /// va holati o'zgartirilganlar saqlanib qoladi.
    pub fn replace_auto_issues(&self, pid: i64, module: IssueModule, found: &[Issue]) {
        let _ = self.conn().execute(
            "DELETE FROM issue WHERE project_id=?1 AND module=?2 AND auto=1 AND status='open'",
            params![pid, module.code()],
        );
        for i in found {
            // Foydalanuvchi allaqachon yopgan xato qayta paydo bo'lmasin.
            let seen: i64 = self
                .conn()
                .query_row(
                    "SELECT COUNT(*) FROM issue WHERE project_id=?1 AND code=?2 AND status<>'open'",
                    params![pid, i.code],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            if seen == 0 {
                self.insert_issue(i);
            }
            // Ochiq yozuvga qo'yilgan muddat qayta tekshiruvda yo'qolmasin.
            let _ = self.conn().execute(
                "UPDATE issue SET deadline=(SELECT deadline FROM issue old
                     WHERE old.project_id=?1 AND old.code=?2 AND old.deadline IS NOT NULL
                     ORDER BY old.id LIMIT 1)
                 WHERE project_id=?1 AND code=?2 AND deadline IS NULL",
                params![pid, i.code],
            );
        }
    }

    // ---------- I.3. PPR ----------

    pub fn ppr_docs(&self, pid: i64) -> Vec<PprDoc> {
        self.list(
            "SELECT id,project_id,kind,number,name,section,task_id,workers,machines,path,approved,note,
                    author,approved_at
             FROM ppr WHERE project_id=?1 ORDER BY id",
            pid,
            |r| {
                Ok(PprDoc {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    kind: PprKind::parse(&r.get::<_, String>(2)?),
                    number: r.get(3)?,
                    name: r.get(4)?,
                    section: Section::parse(&r.get::<_, String>(5)?),
                    task_id: r.get(6)?,
                    workers: r.get(7)?,
                    machines: r.get(8)?,
                    path: r.get(9)?,
                    approved: r.get::<_, i64>(10)? != 0,
                    note: r.get(11)?,
                    author: r.get(12)?,
                    approved_at: odate(r.get::<_, Option<String>>(13)?),
                })
            },
        )
    }

    pub fn insert_ppr(&self, d: &PprDoc) -> i64 {
        self.ins(
            "INSERT INTO ppr (project_id,kind,number,name,section,task_id,workers,machines,path,
                              approved,note,author,approved_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                d.project_id,
                d.kind.code(),
                d.number,
                d.name,
                d.section.code(),
                d.task_id,
                d.workers,
                d.machines,
                d.path,
                d.approved as i64,
                d.note,
                d.author,
                ods(d.approved_at)
            ],
        )
    }

    pub fn update_ppr(&self, d: &PprDoc) -> bool {
        self.upd(
            "UPDATE ppr SET kind=?2,number=?3,name=?4,section=?5,task_id=?6,workers=?7,
                    machines=?8,path=?9,approved=?10,note=?11,author=?12,approved_at=?13
             WHERE id=?1",
            params![
                d.id,
                d.kind.code(),
                d.number,
                d.name,
                d.section.code(),
                d.task_id,
                d.workers,
                d.machines,
                d.path,
                d.approved as i64,
                d.note,
                d.author,
                ods(d.approved_at)
            ],
        )
    }

    // ---------- II. Elementlar ----------

    pub fn elements(&self, pid: i64) -> Vec<Element> {
        self.list(
            "SELECT id,project_id,section,kind,mark,room,axis,level,size,unit,value,value_name,sheet,note
             FROM element WHERE project_id=?1 ORDER BY id",
            pid,
            |r| {
                Ok(Element {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    section: Section::parse(&r.get::<_, String>(2)?),
                    kind: ElementKind::parse(&r.get::<_, String>(3)?),
                    mark: r.get(4)?,
                    room: r.get(5)?,
                    axis: r.get(6)?,
                    level: r.get(7)?,
                    size: r.get(8)?,
                    unit: r.get(9)?,
                    value: r.get(10)?,
                    value_name: r.get(11)?,
                    sheet: r.get(12)?,
                    note: r.get(13)?,
                })
            },
        )
    }

    pub fn insert_element(&self, e: &Element) -> i64 {
        self.ins(
            "INSERT INTO element (project_id,section,kind,mark,room,axis,level,size,unit,value,value_name,sheet,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                e.project_id, e.section.code(), e.kind.code(), e.mark, e.room, e.axis, e.level,
                e.size, e.unit, e.value, e.value_name, e.sheet, e.note
            ],
        )
    }

    pub fn update_element(&self, e: &Element) -> bool {
        self.upd(
            "UPDATE element SET section=?2,kind=?3,mark=?4,room=?5,axis=?6,level=?7,size=?8,
                    unit=?9,value=?10,value_name=?11,sheet=?12,note=?13 WHERE id=?1",
            params![
                e.id,
                e.section.code(),
                e.kind.code(),
                e.mark,
                e.room,
                e.axis,
                e.level,
                e.size,
                e.unit,
                e.value,
                e.value_name,
                e.sheet,
                e.note
            ],
        )
    }

    pub fn element_links(&self, pid: i64) -> Vec<ElementLink> {
        self.list(
            "SELECT l.id,l.from_el,l.to_el,l.relation FROM element_link l
             JOIN element e ON e.id=l.from_el WHERE e.project_id=?1",
            pid,
            |r| {
                Ok(ElementLink {
                    id: r.get(0)?,
                    from_el: r.get(1)?,
                    to_el: r.get(2)?,
                    relation: Relation::parse(&r.get::<_, String>(3)?),
                })
            },
        )
    }

    pub fn insert_element_link(&self, l: &ElementLink) -> i64 {
        self.ins(
            "INSERT OR IGNORE INTO element_link (from_el,to_el,relation) VALUES (?1,?2,?3)",
            params![l.from_el, l.to_el, l.relation.code()],
        )
    }

    // ---------- II. Hujjatlar ----------

    pub fn documents(&self, pid: i64) -> Vec<Document> {
        self.list(
            "SELECT id,project_id,section,name,format,path,sheets,added_at,
                    revision,version,replaces,change_note,issued
             FROM document WHERE project_id=?1 ORDER BY id",
            pid,
            |r| {
                Ok(Document {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    section: Section::parse(&r.get::<_, String>(2)?),
                    name: r.get(3)?,
                    format: r.get(4)?,
                    path: r.get(5)?,
                    sheets: r.get(6)?,
                    added_at: r.get(7)?,
                    revision: r.get(8)?,
                    version: r.get(9)?,
                    replaces: r.get(10)?,
                    change_note: r.get(11)?,
                    issued: r.get::<_, Option<String>>(12)?.as_deref().map(date),
                })
            },
        )
    }

    pub fn insert_document(&self, d: &Document) -> i64 {
        self.ins(
            "INSERT INTO document (project_id,section,name,format,path,sheets,revision,version,
                                   replaces,change_note,issued)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                d.project_id,
                d.section.code(),
                d.name,
                d.format,
                d.path,
                d.sheets,
                d.revision,
                d.version,
                d.replaces,
                d.change_note,
                ods(d.issued)
            ],
        )
    }

    pub fn update_document(&self, d: &Document) -> bool {
        self.upd(
            "UPDATE document SET section=?2,name=?3,format=?4,path=?5,sheets=?6,revision=?7,
                    version=?8,replaces=?9,change_note=?10,issued=?11 WHERE id=?1",
            params![
                d.id,
                d.section.code(),
                d.name,
                d.format,
                d.path,
                d.sheets,
                d.revision,
                d.version,
                d.replaces,
                d.change_note,
                ods(d.issued)
            ],
        )
    }

    // ---------- III. Smeta ----------

    pub fn estimates(&self, pid: i64) -> Vec<Estimate> {
        self.list(
            "SELECT id,project_id,name,currency,declared_total,overhead_pct,profit_pct,vat_pct,
                    added_at
             FROM estimate WHERE project_id=?1 ORDER BY id",
            pid,
            |r| {
                Ok(Estimate {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    name: r.get(2)?,
                    currency: r.get(3)?,
                    declared_total: r.get(4)?,
                    overhead_pct: r.get(5)?,
                    profit_pct: r.get(6)?,
                    vat_pct: r.get(7)?,
                    added_at: r.get(8)?,
                })
            },
        )
    }

    pub fn insert_estimate(&self, e: &Estimate) -> i64 {
        self.ins(
            "INSERT INTO estimate (project_id,name,currency,declared_total,overhead_pct,
                                   profit_pct,vat_pct)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                e.project_id,
                e.name,
                e.currency,
                e.declared_total,
                e.overhead_pct,
                e.profit_pct,
                e.vat_pct
            ],
        )
    }

    pub fn update_estimate(&self, e: &Estimate) -> bool {
        self.upd(
            "UPDATE estimate SET name=?2,currency=?3,declared_total=?4,overhead_pct=?5,
                    profit_pct=?6,vat_pct=?7 WHERE id=?1",
            params![
                e.id,
                e.name,
                e.currency,
                e.declared_total,
                e.overhead_pct,
                e.profit_pct,
                e.vat_pct
            ],
        )
    }

    pub fn estimate_items(&self, eid: i64) -> Vec<EstimateItem> {
        self.list(
            "SELECT id,estimate_id,pos,section,code,name,unit,qty,price,cost,task_id,note
             FROM estimate_item WHERE estimate_id=?1 ORDER BY pos,id",
            eid,
            |r| {
                Ok(EstimateItem {
                    id: r.get(0)?,
                    estimate_id: r.get(1)?,
                    pos: r.get(2)?,
                    section: Section::parse(&r.get::<_, String>(3)?),
                    code: r.get(4)?,
                    name: r.get(5)?,
                    unit: r.get(6)?,
                    qty: r.get(7)?,
                    price: r.get(8)?,
                    cost: r.get(9)?,
                    task_id: r.get(10)?,
                    note: r.get(11)?,
                })
            },
        )
    }

    pub fn insert_estimate_item(&self, i: &EstimateItem) -> i64 {
        self.ins(
            "INSERT INTO estimate_item (estimate_id,pos,section,code,name,unit,qty,price,cost,
                                        task_id,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                i.estimate_id,
                i.pos,
                i.section.code(),
                i.code,
                i.name,
                i.unit,
                i.qty,
                i.price,
                i.cost,
                i.task_id,
                i.note
            ],
        )
    }

    /// Import uchun: barcha pozitsiyani bitta tranzaksiyada yozadi.
    /// Har bir qator alohida yozilsa, WAL ga yuzlab fsync tushardi.
    pub fn insert_estimate_items(&self, eid: i64, items: &[EstimateItem]) -> usize {
        let _ = self.conn().execute_batch("BEGIN");
        let mut n = 0;
        for it in items {
            let mut it = it.clone();
            it.estimate_id = eid;
            if self.insert_estimate_item(&it) > 0 {
                n += 1;
            }
        }
        let _ = self.conn().execute_batch("COMMIT");
        n
    }

    pub fn update_estimate_item(&self, i: &EstimateItem) -> bool {
        self.upd(
            "UPDATE estimate_item SET pos=?2,section=?3,code=?4,name=?5,unit=?6,qty=?7,price=?8,
                    cost=?9,task_id=?10,note=?11 WHERE id=?1",
            params![
                i.id,
                i.pos,
                i.section.code(),
                i.code,
                i.name,
                i.unit,
                i.qty,
                i.price,
                i.cost,
                i.task_id,
                i.note
            ],
        )
    }

    // ---------- XVI.28. Texnikaning kunlik ko'rigi ----------

    pub fn machine_checks(&self, pid: i64) -> Vec<MachineCheck> {
        self.list(
            "SELECT id,project_id,machine_id,date,by_whom,items_ok,items_total,fault,allowed,note
             FROM machine_check WHERE project_id=?1 ORDER BY date DESC,id DESC",
            pid,
            |r| {
                Ok(MachineCheck {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    machine_id: r.get(2)?,
                    date: date(&r.get::<_, String>(3)?),
                    by: r.get(4)?,
                    items_ok: r.get(5)?,
                    items_total: r.get(6)?,
                    fault: r.get(7)?,
                    allowed: r.get::<_, i64>(8)? != 0,
                    note: r.get(9)?,
                })
            },
        )
    }

    pub fn insert_machine_check(&self, c: &MachineCheck) -> i64 {
        self.ins(
            "INSERT INTO machine_check (project_id,machine_id,date,by_whom,items_ok,items_total,
                                        fault,allowed,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                c.project_id,
                c.machine_id,
                c.date.to_string(),
                c.by,
                c.items_ok,
                c.items_total,
                c.fault,
                c.allowed as i64,
                c.note
            ],
        )
    }

    pub fn update_machine_check(&self, c: &MachineCheck) -> bool {
        self.upd(
            "UPDATE machine_check SET machine_id=?2,date=?3,by_whom=?4,items_ok=?5,
                    items_total=?6,fault=?7,allowed=?8,note=?9 WHERE id=?1",
            params![
                c.id,
                c.machine_id,
                c.date.to_string(),
                c.by,
                c.items_ok,
                c.items_total,
                c.fault,
                c.allowed as i64,
                c.note
            ],
        )
    }

    // ---------- Umumiy: izoh va biriktirma ----------

    /// Obyektning barcha izohlari. Filtrlash ekranda qilinadi — izohlar
    /// kam bo'ladi va bir marta o'qilgani tezroq.
    pub fn notes(&self, pid: i64) -> Vec<Note> {
        self.list(
            "SELECT id,project_id,target,target_id,author,at,text,parent,resolved
             FROM note WHERE project_id=?1 ORDER BY at,id",
            pid,
            |r| {
                Ok(Note {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    target: NoteTarget::parse(&r.get::<_, String>(2)?),
                    target_id: r.get(3)?,
                    author: r.get(4)?,
                    at: r.get(5)?,
                    text: r.get(6)?,
                    parent: r.get(7)?,
                    resolved: r.get::<_, i64>(8)? != 0,
                })
            },
        )
    }

    pub fn insert_note(&self, n: &Note) -> i64 {
        self.ins(
            "INSERT INTO note (project_id,target,target_id,author,at,text,parent,resolved)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                n.project_id,
                n.target.code(),
                n.target_id,
                n.author,
                n.at,
                n.text,
                n.parent,
                n.resolved as i64
            ],
        )
    }

    /// Izohning **faqat holati** o'zgaradi: matn tahrirlanmaydi, chunki
    /// muhokama tarixi o'zgarsa uning ma'nosi qolmaydi.
    pub fn set_note_resolved(&self, id: i64, resolved: bool) -> bool {
        self.upd(
            "UPDATE note SET resolved=?2 WHERE id=?1",
            params![id, resolved as i64],
        )
    }

    pub fn attachments(&self, pid: i64) -> Vec<Attachment> {
        self.list(
            "SELECT id,project_id,target,target_id,path,stage,caption,author,at
             FROM attachment WHERE project_id=?1 ORDER BY id",
            pid,
            |r| {
                Ok(Attachment {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    target: NoteTarget::parse(&r.get::<_, String>(2)?),
                    target_id: r.get(3)?,
                    path: r.get(4)?,
                    stage: PhotoStage::parse(&r.get::<_, String>(5)?),
                    caption: r.get(6)?,
                    author: r.get(7)?,
                    at: r.get(8)?,
                })
            },
        )
    }

    pub fn insert_attachment(&self, a: &Attachment) -> i64 {
        self.ins(
            "INSERT INTO attachment (project_id,target,target_id,path,stage,caption,author,at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                a.project_id,
                a.target.code(),
                a.target_id,
                a.path,
                a.stage.code(),
                a.caption,
                a.author,
                a.at
            ],
        )
    }

    pub fn update_attachment(&self, a: &Attachment) -> bool {
        self.upd(
            "UPDATE attachment SET stage=?2,caption=?3 WHERE id=?1",
            params![a.id, a.stage.code(), a.caption],
        )
    }

    /// Biriktirmani ro'yxatdan olib tashlaydi. **Fayl o'chirilmaydi** —
    /// dastur o'zi joylashtirmagan faylni o'chirishi noto'g'ri bo'lardi.
    pub fn delete_attachment(&self, id: i64) -> bool {
        self.del("attachment", id)
    }

    // ---------- IV. Ijro hujjatlari ----------

    pub fn exec_docs(&self, pid: i64) -> Vec<ExecDoc> {
        self.list(
            "SELECT id,project_id,kind,number,name,date,task_id,status,responsible,
                    version,replaces,note
             FROM exec_doc WHERE project_id=?1 ORDER BY date DESC,id DESC",
            pid,
            |r| {
                Ok(ExecDoc {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    kind: ExecDocKind::parse(&r.get::<_, String>(2)?),
                    number: r.get(3)?,
                    name: r.get(4)?,
                    date: date(&r.get::<_, String>(5)?),
                    task_id: r.get(6)?,
                    status: ExecDocStatus::parse(&r.get::<_, String>(7)?),
                    responsible: r.get(8)?,
                    version: r.get(9)?,
                    replaces: r.get(10)?,
                    note: r.get(11)?,
                })
            },
        )
    }

    pub fn insert_exec_doc(&self, d: &ExecDoc) -> i64 {
        self.ins(
            "INSERT INTO exec_doc (project_id,kind,number,name,date,task_id,status,responsible,
                                   version,replaces,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                d.project_id,
                d.kind.code(),
                d.number,
                d.name,
                d.date.to_string(),
                d.task_id,
                d.status.code(),
                d.responsible,
                d.version,
                d.replaces,
                d.note
            ],
        )
    }

    pub fn update_exec_doc(&self, d: &ExecDoc) -> bool {
        self.upd(
            "UPDATE exec_doc SET kind=?2,number=?3,name=?4,date=?5,task_id=?6,status=?7,
                    responsible=?8,version=?9,replaces=?10,note=?11 WHERE id=?1",
            params![
                d.id,
                d.kind.code(),
                d.number,
                d.name,
                d.date.to_string(),
                d.task_id,
                d.status.code(),
                d.responsible,
                d.version,
                d.replaces,
                d.note
            ],
        )
    }

    // ---------- V. Jurnal ----------

    pub fn journal(&self, pid: i64) -> Vec<JournalEntry> {
        self.list(
            "SELECT id,project_id,date,author,weather,temperature,workers,machines,task_id,volume,
                    unit,text,remarks,photos,gps
             FROM journal WHERE project_id=?1 ORDER BY date DESC,id DESC",
            pid,
            |r| {
                Ok(JournalEntry {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    date: date(&r.get::<_, String>(2)?),
                    author: r.get(3)?,
                    weather: r.get(4)?,
                    temperature: r.get(5)?,
                    workers: r.get(6)?,
                    machines: r.get(7)?,
                    task_id: r.get(8)?,
                    volume: r.get(9)?,
                    unit: r.get(10)?,
                    text: r.get(11)?,
                    remarks: r.get(12)?,
                    photos: r.get(13)?,
                    gps: r.get(14)?,
                })
            },
        )
    }

    pub fn insert_journal(&self, j: &JournalEntry) -> i64 {
        self.ins(
            "INSERT INTO journal (project_id,date,author,weather,temperature,workers,machines,
                                 task_id,volume,unit,text,remarks,photos,gps)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                j.project_id,
                j.date.to_string(),
                j.author,
                j.weather,
                j.temperature,
                j.workers,
                j.machines,
                j.task_id,
                j.volume,
                j.unit,
                j.text,
                j.remarks,
                j.photos,
                j.gps
            ],
        )
    }

    pub fn update_journal(&self, j: &JournalEntry) -> bool {
        self.upd(
            "UPDATE journal SET date=?2,author=?3,weather=?4,temperature=?5,workers=?6,machines=?7,
                    task_id=?8,volume=?9,unit=?10,text=?11,remarks=?12,photos=?13,gps=?14
             WHERE id=?1",
            params![
                j.id,
                j.date.to_string(),
                j.author,
                j.weather,
                j.temperature,
                j.workers,
                j.machines,
                j.task_id,
                j.volume,
                j.unit,
                j.text,
                j.remarks,
                j.photos,
                j.gps
            ],
        )
    }

    // ---------- IX. Arizalar ----------

    pub fn requests(&self, pid: i64) -> Vec<Request> {
        self.list(
            "SELECT id,project_id,number,date,kind,title,material_id,qty,unit,requester,need_date,
                    priority,status,task_id,reject_reason,note
             FROM request WHERE project_id=?1 ORDER BY date DESC,id DESC",
            pid,
            |r| {
                Ok(Request {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    number: r.get(2)?,
                    date: date(&r.get::<_, String>(3)?),
                    kind: RequestKind::parse(&r.get::<_, String>(4)?),
                    title: r.get(5)?,
                    material_id: r.get(6)?,
                    qty: r.get(7)?,
                    unit: r.get(8)?,
                    requester: r.get(9)?,
                    need_date: date(&r.get::<_, String>(10)?),
                    priority: Priority::parse(&r.get::<_, String>(11)?),
                    status: RequestStatus::parse(&r.get::<_, String>(12)?),
                    task_id: r.get(13)?,
                    reject_reason: r.get(14)?,
                    note: r.get(15)?,
                })
            },
        )
    }

    pub fn insert_request(&self, q: &Request) -> i64 {
        self.ins(
            "INSERT INTO request (project_id,number,date,kind,title,material_id,qty,unit,requester,
                                  need_date,priority,status,task_id,reject_reason,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![
                q.project_id,
                q.number,
                q.date.to_string(),
                q.kind.code(),
                q.title,
                q.material_id,
                q.qty,
                q.unit,
                q.requester,
                q.need_date.to_string(),
                q.priority.code(),
                q.status.code(),
                q.task_id,
                q.reject_reason,
                q.note
            ],
        )
    }

    pub fn update_request(&self, q: &Request) -> bool {
        self.upd(
            "UPDATE request SET number=?2,date=?3,kind=?4,title=?5,material_id=?6,qty=?7,unit=?8,
                    requester=?9,need_date=?10,priority=?11,status=?12,task_id=?13,
                    reject_reason=?14,note=?15 WHERE id=?1",
            params![
                q.id,
                q.number,
                q.date.to_string(),
                q.kind.code(),
                q.title,
                q.material_id,
                q.qty,
                q.unit,
                q.requester,
                q.need_date.to_string(),
                q.priority.code(),
                q.status.code(),
                q.task_id,
                q.reject_reason,
                q.note
            ],
        )
    }

    // ---------- X. Xaridlar ----------

    pub fn purchases(&self, pid: i64) -> Vec<Purchase> {
        self.list(
            "SELECT id,project_id,request_id,number,date,supplier,title,qty,unit,price,currency,
                    delivery_date,status,delivered_qty,section,task_id,contract_id,urgent,buyer,
                    material_id,substitute_for,tech_ok,tech_by,paid,pay_due,note
             FROM purchase WHERE project_id=?1 ORDER BY date DESC,id DESC",
            pid,
            |r| {
                Ok(Purchase {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    request_id: r.get(2)?,
                    number: r.get(3)?,
                    date: date(&r.get::<_, String>(4)?),
                    supplier: r.get(5)?,
                    title: r.get(6)?,
                    qty: r.get(7)?,
                    unit: r.get(8)?,
                    price: r.get(9)?,
                    currency: r.get(10)?,
                    delivery_date: date(&r.get::<_, String>(11)?),
                    status: PurchaseStatus::parse(&r.get::<_, String>(12)?),
                    delivered_qty: r.get(13)?,
                    section: Section::parse(&r.get::<_, String>(14)?),
                    task_id: r.get(15)?,
                    contract_id: r.get(16)?,
                    urgent: r.get::<_, i64>(17)? != 0,
                    buyer: r.get(18)?,
                    material_id: r.get(19)?,
                    substitute_for: r.get(20)?,
                    tech_ok: r.get::<_, i64>(21)? != 0,
                    tech_by: r.get(22)?,
                    paid: r.get(23)?,
                    pay_due: r.get::<_, Option<String>>(24)?.as_deref().map(date),
                    note: r.get(25)?,
                })
            },
        )
    }

    pub fn insert_purchase(&self, p: &Purchase) -> i64 {
        self.ins(
            "INSERT INTO purchase (project_id,request_id,number,date,supplier,title,qty,unit,price,
                                   currency,delivery_date,status,delivered_qty,section,task_id,
                                   contract_id,urgent,buyer,material_id,substitute_for,
                                   tech_ok,tech_by,paid,pay_due,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,
                     ?22,?23,?24,?25)",
            params![
                p.project_id,
                p.request_id,
                p.number,
                p.date.to_string(),
                p.supplier,
                p.title,
                p.qty,
                p.unit,
                p.price,
                p.currency,
                p.delivery_date.to_string(),
                p.status.code(),
                p.delivered_qty,
                p.section.code(),
                p.task_id,
                p.contract_id,
                p.urgent as i64,
                p.buyer,
                p.material_id,
                p.substitute_for,
                p.tech_ok as i64,
                p.tech_by,
                p.paid,
                ods(p.pay_due),
                p.note
            ],
        )
    }

    pub fn update_purchase(&self, p: &Purchase) -> bool {
        self.upd(
            "UPDATE purchase SET request_id=?2,number=?3,date=?4,supplier=?5,title=?6,qty=?7,
                    unit=?8,price=?9,currency=?10,delivery_date=?11,status=?12,delivered_qty=?13,
                    section=?14,task_id=?15,contract_id=?16,urgent=?17,buyer=?18,
                    material_id=?19,substitute_for=?20,tech_ok=?21,tech_by=?22,paid=?23,
                    pay_due=?24,note=?25
             WHERE id=?1",
            params![
                p.id,
                p.request_id,
                p.number,
                p.date.to_string(),
                p.supplier,
                p.title,
                p.qty,
                p.unit,
                p.price,
                p.currency,
                p.delivery_date.to_string(),
                p.status.code(),
                p.delivered_qty,
                p.section.code(),
                p.task_id,
                p.contract_id,
                p.urgent as i64,
                p.buyer,
                p.material_id,
                p.substitute_for,
                p.tech_ok as i64,
                p.tech_by,
                p.paid,
                ods(p.pay_due),
                p.note
            ],
        )
    }

    // ---------- XI–XII. Materiallar va ombor ----------

    pub fn materials(&self, pid: i64) -> Vec<Material> {
        self.list(
            "SELECT id,project_id,code,name,unit,section,spec,cert_no,cert_until,min_stock,price,
                    estimate_code,spec_ref,special,banned,ban_reason,note
             FROM material WHERE project_id=?1 ORDER BY name",
            pid,
            |r| {
                Ok(Material {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    code: r.get(2)?,
                    name: r.get(3)?,
                    unit: r.get(4)?,
                    section: Section::parse(&r.get::<_, String>(5)?),
                    spec: r.get(6)?,
                    cert_no: r.get(7)?,
                    cert_until: odate(r.get(8)?),
                    min_stock: r.get(9)?,
                    price: r.get(10)?,
                    estimate_code: r.get(11)?,
                    spec_ref: r.get(12)?,
                    special: r.get(13)?,
                    banned: r.get::<_, i64>(14)? != 0,
                    ban_reason: r.get(15)?,
                    note: r.get(16)?,
                })
            },
        )
    }

    pub fn insert_material(&self, m: &Material) -> i64 {
        self.ins(
            "INSERT INTO material (project_id,code,name,unit,section,spec,cert_no,cert_until,
                                   min_stock,price,estimate_code,spec_ref,special,banned,
                                   ban_reason,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
            params![
                m.project_id,
                m.code,
                m.name,
                m.unit,
                m.section.code(),
                m.spec,
                m.cert_no,
                ods(m.cert_until),
                m.min_stock,
                m.price,
                m.estimate_code,
                m.spec_ref,
                m.special,
                i64::from(m.banned),
                m.ban_reason,
                m.note
            ],
        )
    }

    pub fn update_material(&self, m: &Material) -> bool {
        self.upd(
            "UPDATE material SET code=?2,name=?3,unit=?4,section=?5,spec=?6,cert_no=?7,
                    cert_until=?8,min_stock=?9,price=?10,estimate_code=?11,spec_ref=?12,
                    special=?13,banned=?14,ban_reason=?15,note=?16 WHERE id=?1",
            params![
                m.id,
                m.code,
                m.name,
                m.unit,
                m.section.code(),
                m.spec,
                m.cert_no,
                ods(m.cert_until),
                m.min_stock,
                m.price,
                m.estimate_code,
                m.spec_ref,
                m.special,
                i64::from(m.banned),
                m.ban_reason,
                m.note
            ],
        )
    }

    // ---------- XII.9-11. Analoglar ----------

    pub fn material_alts(&self, pid: i64) -> Vec<MaterialAlt> {
        self.list(
            "SELECT id,project_id,material_id,alt_id,approved_by,approved_at,note
             FROM material_alt WHERE project_id=?1 ORDER BY material_id,id",
            pid,
            |r| {
                Ok(MaterialAlt {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    material_id: r.get(2)?,
                    alt_id: r.get(3)?,
                    approved_by: r.get(4)?,
                    approved_at: odate(r.get(5)?),
                    note: r.get(6)?,
                })
            },
        )
    }

    pub fn insert_material_alt(&self, a: &MaterialAlt) -> i64 {
        self.ins(
            "INSERT INTO material_alt (project_id,material_id,alt_id,approved_by,approved_at,note)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                a.project_id,
                a.material_id,
                a.alt_id,
                a.approved_by,
                ods(a.approved_at),
                a.note
            ],
        )
    }

    pub fn update_material_alt(&self, a: &MaterialAlt) -> bool {
        self.upd(
            "UPDATE material_alt SET material_id=?2,alt_id=?3,approved_by=?4,approved_at=?5,
                    note=?6 WHERE id=?1",
            params![
                a.id,
                a.material_id,
                a.alt_id,
                a.approved_by,
                ods(a.approved_at),
                a.note
            ],
        )
    }

    pub fn stock_moves(&self, pid: i64) -> Vec<StockMove> {
        self.list(
            "SELECT id,project_id,material_id,date,kind,qty,price,document,counterparty,task_id,
                    note,warehouse_id,batch_id
             FROM stock_move WHERE project_id=?1 ORDER BY date DESC,id DESC",
            pid,
            |r| {
                Ok(StockMove {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    material_id: r.get(2)?,
                    date: date(&r.get::<_, String>(3)?),
                    kind: MoveKind::parse(&r.get::<_, String>(4)?),
                    qty: r.get(5)?,
                    price: r.get(6)?,
                    document: r.get(7)?,
                    counterparty: r.get(8)?,
                    task_id: r.get(9)?,
                    note: r.get(10)?,
                    warehouse_id: r.get(11)?,
                    batch_id: r.get(12)?,
                })
            },
        )
    }

    pub fn insert_stock_move(&self, m: &StockMove) -> i64 {
        self.ins(
            "INSERT INTO stock_move (project_id,material_id,date,kind,qty,price,document,
                                     counterparty,task_id,note,warehouse_id,batch_id)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                m.project_id,
                m.material_id,
                m.date.to_string(),
                m.kind.code(),
                m.qty,
                m.price,
                m.document,
                m.counterparty,
                m.task_id,
                m.note,
                m.warehouse_id,
                m.batch_id
            ],
        )
    }

    pub fn update_stock_move(&self, m: &StockMove) -> bool {
        self.upd(
            "UPDATE stock_move SET material_id=?2,date=?3,kind=?4,qty=?5,price=?6,document=?7,
                    counterparty=?8,task_id=?9,note=?10,warehouse_id=?11,batch_id=?12
             WHERE id=?1",
            params![
                m.id,
                m.material_id,
                m.date.to_string(),
                m.kind.code(),
                m.qty,
                m.price,
                m.document,
                m.counterparty,
                m.task_id,
                m.note,
                m.warehouse_id,
                m.batch_id
            ],
        )
    }

    // ---------- XIII. Tabel ----------

    pub fn workers(&self, pid: i64) -> Vec<Worker> {
        self.list(
            "SELECT id,project_id,name,position,org,hourly_rate,active,brigade_id
             FROM worker WHERE project_id=?1 ORDER BY name",
            pid,
            |r| {
                Ok(Worker {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    name: r.get(2)?,
                    position: r.get(3)?,
                    org: r.get(4)?,
                    hourly_rate: r.get(5)?,
                    active: r.get::<_, i64>(6)? != 0,
                    brigade_id: r.get(7)?,
                })
            },
        )
    }

    pub fn insert_worker(&self, w: &Worker) -> i64 {
        self.ins(
            "INSERT INTO worker (project_id,name,position,org,hourly_rate,active,brigade_id)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                w.project_id,
                w.name,
                w.position,
                w.org,
                w.hourly_rate,
                w.active as i64,
                w.brigade_id
            ],
        )
    }

    pub fn update_worker(&self, w: &Worker) -> bool {
        self.upd(
            "UPDATE worker SET name=?2,position=?3,org=?4,hourly_rate=?5,active=?6,brigade_id=?7
             WHERE id=?1",
            params![
                w.id,
                w.name,
                w.position,
                w.org,
                w.hourly_rate,
                w.active as i64,
                w.brigade_id
            ],
        )
    }

    /// Xodimni boshqa obyektga ko'chiradi (TZ XIII.28).
    ///
    /// Alohida amal: `update_worker` obyektga tegmaydi, chunki oddiy
    /// tahrirlashda xodim tasodifan boshqa obyektga o'tib ketmasligi kerak.
    /// Brigada obyektga bog'langan, shuning uchun bog'lanish tushadi.
    pub fn move_worker(&self, worker_id: i64, to_project: i64) -> bool {
        self.upd(
            "UPDATE worker SET project_id=?2, brigade_id=NULL WHERE id=?1",
            params![worker_id, to_project],
        )
    }

    // ---------- VI.32. Yozishma ----------

    /// Obyekt bo'yicha xabarlar, eskisidan boshlab.
    pub fn messages(&self, pid: i64) -> Vec<ChatMessage> {
        self.list(
            "SELECT id,project_id,server_id,author,role,text,at
             FROM message WHERE project_id=?1 ORDER BY server_id, id",
            pid,
            |r| {
                Ok(ChatMessage {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    server_id: r.get(2)?,
                    author: r.get(3)?,
                    role: r.get(4)?,
                    text: r.get(5)?,
                    at: r.get(6)?,
                })
            },
        )
    }

    /// Serverdan kelgan xabarni saqlaydi. Takror kelsa yozilmaydi.
    pub fn insert_message(&self, m: &ChatMessage) -> i64 {
        self.ins(
            "INSERT INTO message (project_id,server_id,author,role,text,at)
             VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(project_id,server_id) DO NOTHING",
            params![m.project_id, m.server_id, m.author, m.role, m.text, m.at],
        )
    }

    /// Oxirgi olingan xabar raqami — keyingi so'rov shundan boshlanadi.
    pub fn last_message_id(&self, pid: i64) -> i64 {
        self.conn()
            .query_row(
                "SELECT COALESCE(MAX(server_id),0) FROM message WHERE project_id=?1",
                params![pid],
                |r| r.get(0),
            )
            .unwrap_or(0)
    }

    // ---------- XIII.4–6. Kirish/chiqish ----------

    /// Maydonchaga kirish-chiqish belgilari, oxirgisi oldinda.
    pub fn attendance(&self, pid: i64) -> Vec<Attendance> {
        self.list(
            "SELECT id,project_id,worker,at,kind,gps,source
             FROM attendance WHERE project_id=?1 ORDER BY at DESC, id DESC",
            pid,
            |r| {
                Ok(Attendance {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    worker: r.get(2)?,
                    at: parse_stamp(&r.get::<_, String>(3)?),
                    kind: InOut::parse(&r.get::<_, String>(4)?),
                    gps: r.get(5)?,
                    source: r.get(6)?,
                })
            },
        )
    }

    /// Belgini qo'shadi.
    ///
    /// Bir xil belgi ikki marta kelishi mumkin (paket qayta olinsa),
    /// shuning uchun ishchi + payt + tur bo'yicha takrorlanmaydi.
    pub fn insert_attendance(&self, a: &Attendance) -> i64 {
        self.ins(
            "INSERT INTO attendance (project_id,worker,at,kind,gps,source)
             VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(project_id,worker,at,kind) DO NOTHING",
            params![
                a.project_id,
                a.worker,
                a.at.format("%Y-%m-%dT%H:%M:%S").to_string(),
                a.kind.code(),
                a.gps,
                a.source
            ],
        )
    }

    pub fn timesheet(&self, pid: i64) -> Vec<TimesheetEntry> {
        self.list(
            "SELECT id,project_id,worker_id,date,hours,task_id,kind,shift,note
             FROM timesheet WHERE project_id=?1 ORDER BY date",
            pid,
            |r| {
                Ok(TimesheetEntry {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    worker_id: r.get(2)?,
                    date: date(&r.get::<_, String>(3)?),
                    hours: r.get(4)?,
                    task_id: r.get(5)?,
                    kind: DayKind::parse(&r.get::<_, String>(6)?),
                    shift: Shift::parse(&r.get::<_, String>(7)?),
                    note: r.get(8)?,
                })
            },
        )
    }

    /// Tabel katakchasi: bir ishchining bir kunlik soati. Nol soat va oddiy ish
    /// kuni — yozuv o'chiriladi (bo'sh katak «ishlamagan» degani).
    pub fn set_timesheet(&self, pid: i64, worker: i64, day: NaiveDate, hours: f64) -> bool {
        let kind = self.timesheet_kind(worker, day);
        if hours <= 0.0 && kind == DayKind::Work {
            return self.delete_timesheet(worker, day);
        }
        self.upd(
            "INSERT INTO timesheet (project_id,worker_id,date,hours) VALUES (?1,?2,?3,?4)
             ON CONFLICT(worker_id,date) DO UPDATE SET hours=excluded.hours",
            params![pid, worker, day.to_string(), hours],
        )
    }

    /// Katakning turi: yo'qlik, bo'sh turish yoki oddiy ish kuni (TZ XIII.16–22).
    ///
    /// Oddiy ish kuniga qaytarilganda va soat nol bo'lsa yozuv o'chiriladi —
    /// bo'sh katak bazada ham bo'sh qoladi.
    pub fn set_timesheet_kind(&self, pid: i64, worker: i64, day: NaiveDate, kind: DayKind) -> bool {
        if kind == DayKind::Work && self.timesheet_hours(worker, day) <= 0.0 {
            return self.delete_timesheet(worker, day);
        }
        self.upd(
            "INSERT INTO timesheet (project_id,worker_id,date,hours,kind) VALUES (?1,?2,?3,0,?4)
             ON CONFLICT(worker_id,date) DO UPDATE SET kind=excluded.kind",
            params![pid, worker, day.to_string(), kind.code()],
        )
    }

    /// Katakning smenasi (TZ XIII.11).
    pub fn set_timesheet_shift(&self, pid: i64, worker: i64, day: NaiveDate, shift: Shift) -> bool {
        self.upd(
            "INSERT INTO timesheet (project_id,worker_id,date,hours,shift) VALUES (?1,?2,?3,0,?4)
             ON CONFLICT(worker_id,date) DO UPDATE SET shift=excluded.shift",
            params![pid, worker, day.to_string(), shift.code()],
        )
    }

    /// Katak qaysi ishga tegishli (TZ XIII.10) — tannarx shundan yig'iladi.
    pub fn set_timesheet_task(
        &self,
        pid: i64,
        worker: i64,
        day: NaiveDate,
        task: Option<i64>,
    ) -> bool {
        self.upd(
            "INSERT INTO timesheet (project_id,worker_id,date,hours,task_id) VALUES (?1,?2,?3,0,?4)
             ON CONFLICT(worker_id,date) DO UPDATE SET task_id=excluded.task_id",
            params![pid, worker, day.to_string(), task],
        )
    }

    fn delete_timesheet(&self, worker: i64, day: NaiveDate) -> bool {
        self.conn()
            .execute(
                "DELETE FROM timesheet WHERE worker_id=?1 AND date=?2",
                params![worker, day.to_string()],
            )
            .is_ok()
    }

    fn timesheet_hours(&self, worker: i64, day: NaiveDate) -> f64 {
        self.conn()
            .query_row(
                "SELECT hours FROM timesheet WHERE worker_id=?1 AND date=?2",
                params![worker, day.to_string()],
                |r| r.get(0),
            )
            .unwrap_or(0.0)
    }

    fn timesheet_kind(&self, worker: i64, day: NaiveDate) -> DayKind {
        self.conn()
            .query_row(
                "SELECT kind FROM timesheet WHERE worker_id=?1 AND date=?2",
                params![worker, day.to_string()],
                |r| r.get::<_, String>(0),
            )
            .map(|s| DayKind::parse(&s))
            .unwrap_or(DayKind::Work)
    }

    // ---------- XIII.8. Brigadalar ----------

    pub fn brigades(&self, pid: i64) -> Vec<Brigade> {
        self.list(
            "SELECT id,project_id,name,foreman,task_id,note
             FROM brigade WHERE project_id=?1 ORDER BY name",
            pid,
            |r| {
                Ok(Brigade {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    name: r.get(2)?,
                    foreman: r.get(3)?,
                    task_id: r.get(4)?,
                    note: r.get(5)?,
                })
            },
        )
    }

    pub fn insert_brigade(&self, b: &Brigade) -> i64 {
        self.ins(
            "INSERT INTO brigade (project_id,name,foreman,task_id,note) VALUES (?1,?2,?3,?4,?5)",
            params![b.project_id, b.name, b.foreman, b.task_id, b.note],
        )
    }

    pub fn update_brigade(&self, b: &Brigade) -> bool {
        self.upd(
            "UPDATE brigade SET name=?2,foreman=?3,task_id=?4,note=?5 WHERE id=?1",
            params![b.id, b.name, b.foreman, b.task_id, b.note],
        )
    }

    // ---------- XIV. Sifat ----------

    pub fn quality_checks(&self, pid: i64) -> Vec<QualityCheck> {
        self.list(
            "SELECT id,project_id,kind,date,task_id,material_id,subject,inspector,result,defect,
                    deadline,checklist_id,fixed_at,note
             FROM quality_check WHERE project_id=?1 ORDER BY date DESC,id DESC",
            pid,
            |r| {
                Ok(QualityCheck {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    kind: QualityKind::parse(&r.get::<_, String>(2)?),
                    date: date(&r.get::<_, String>(3)?),
                    task_id: r.get(4)?,
                    material_id: r.get(5)?,
                    subject: r.get(6)?,
                    inspector: r.get(7)?,
                    result: QualityResult::parse(&r.get::<_, String>(8)?),
                    defect: r.get(9)?,
                    deadline: odate(r.get(10)?),
                    checklist_id: r.get(11)?,
                    fixed_at: odate(r.get(12)?),
                    note: r.get(13)?,
                })
            },
        )
    }

    pub fn insert_quality(&self, q: &QualityCheck) -> i64 {
        self.ins(
            "INSERT INTO quality_check (project_id,kind,date,task_id,material_id,subject,inspector,
                                        result,defect,deadline,checklist_id,fixed_at,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                q.project_id,
                q.kind.code(),
                q.date.to_string(),
                q.task_id,
                q.material_id,
                q.subject,
                q.inspector,
                q.result.code(),
                q.defect,
                ods(q.deadline),
                q.checklist_id,
                ods(q.fixed_at),
                q.note
            ],
        )
    }

    pub fn update_quality(&self, q: &QualityCheck) -> bool {
        self.upd(
            "UPDATE quality_check SET kind=?2,date=?3,task_id=?4,material_id=?5,subject=?6,
                    inspector=?7,result=?8,defect=?9,deadline=?10,checklist_id=?11,fixed_at=?12,
                    note=?13 WHERE id=?1",
            params![
                q.id,
                q.kind.code(),
                q.date.to_string(),
                q.task_id,
                q.material_id,
                q.subject,
                q.inspector,
                q.result.code(),
                q.defect,
                ods(q.deadline),
                q.checklist_id,
                ods(q.fixed_at),
                q.note
            ],
        )
    }

    // ---------- VII. Texnik nazorat ----------

    pub fn inspections(&self, pid: i64) -> Vec<Inspection> {
        self.list(
            "SELECT id,project_id,task_id,kind,number,planned,done,requested_by,inspector,place,
                    result,deadline,fixed_at,note
             FROM inspection WHERE project_id=?1 ORDER BY planned DESC,id DESC",
            pid,
            |r| {
                Ok(Inspection {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    task_id: r.get(2)?,
                    kind: InspectionKind::parse(&r.get::<_, String>(3)?),
                    number: r.get(4)?,
                    planned: date(&r.get::<_, String>(5)?),
                    done: odate(r.get(6)?),
                    requested_by: r.get(7)?,
                    inspector: r.get(8)?,
                    place: r.get(9)?,
                    result: InspectionResult::parse(&r.get::<_, String>(10)?),
                    deadline: odate(r.get(11)?),
                    fixed_at: odate(r.get(12)?),
                    note: r.get(13)?,
                })
            },
        )
    }

    pub fn insert_inspection(&self, x: &Inspection) -> i64 {
        self.ins(
            "INSERT INTO inspection (project_id,task_id,kind,number,planned,done,requested_by,
                                     inspector,place,result,deadline,fixed_at,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                x.project_id,
                x.task_id,
                x.kind.code(),
                x.number,
                x.planned.to_string(),
                ods(x.done),
                x.requested_by,
                x.inspector,
                x.place,
                x.result.code(),
                ods(x.deadline),
                ods(x.fixed_at),
                x.note
            ],
        )
    }

    pub fn update_inspection(&self, x: &Inspection) -> bool {
        self.upd(
            "UPDATE inspection SET task_id=?2,kind=?3,number=?4,planned=?5,done=?6,requested_by=?7,
                    inspector=?8,place=?9,result=?10,deadline=?11,fixed_at=?12,note=?13
             WHERE id=?1",
            params![
                x.id,
                x.task_id,
                x.kind.code(),
                x.number,
                x.planned.to_string(),
                ods(x.done),
                x.requested_by,
                x.inspector,
                x.place,
                x.result.code(),
                ods(x.deadline),
                ods(x.fixed_at),
                x.note
            ],
        )
    }

    pub fn delete_inspection(&self, id: i64) -> bool {
        self.del("inspection", id)
    }

    pub fn concrete_tests(&self, pid: i64) -> Vec<ConcreteTest> {
        self.list(
            "SELECT id,project_id,inspection_id,task_id,sample,grade,structure,poured,age_days,
                    required,actual,lab,note
             FROM concrete_test WHERE project_id=?1 ORDER BY poured DESC,id DESC",
            pid,
            |r| {
                Ok(ConcreteTest {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    inspection_id: r.get(2)?,
                    task_id: r.get(3)?,
                    sample: r.get(4)?,
                    grade: r.get(5)?,
                    structure: r.get(6)?,
                    poured: date(&r.get::<_, String>(7)?),
                    age_days: r.get(8)?,
                    required: r.get(9)?,
                    actual: r.get(10)?,
                    lab: r.get(11)?,
                    note: r.get(12)?,
                })
            },
        )
    }

    pub fn insert_concrete_test(&self, x: &ConcreteTest) -> i64 {
        self.ins(
            "INSERT INTO concrete_test (project_id,inspection_id,task_id,sample,grade,structure,
                                        poured,age_days,required,actual,lab,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                x.project_id,
                x.inspection_id,
                x.task_id,
                x.sample,
                x.grade,
                x.structure,
                x.poured.to_string(),
                x.age_days,
                x.required,
                x.actual,
                x.lab,
                x.note
            ],
        )
    }

    pub fn update_concrete_test(&self, x: &ConcreteTest) -> bool {
        self.upd(
            "UPDATE concrete_test SET inspection_id=?2,task_id=?3,sample=?4,grade=?5,structure=?6,
                    poured=?7,age_days=?8,required=?9,actual=?10,lab=?11,note=?12 WHERE id=?1",
            params![
                x.id,
                x.inspection_id,
                x.task_id,
                x.sample,
                x.grade,
                x.structure,
                x.poured.to_string(),
                x.age_days,
                x.required,
                x.actual,
                x.lab,
                x.note
            ],
        )
    }

    pub fn delete_concrete_test(&self, id: i64) -> bool {
        self.del("concrete_test", id)
    }

    pub fn geodesy_points(&self, pid: i64) -> Vec<GeodesyPoint> {
        self.list(
            "SELECT id,project_id,inspection_id,mark,axis,level,design,fact,tolerance,unit,
                    measured,surveyor,note
             FROM geodesy_point WHERE project_id=?1 ORDER BY measured DESC,id DESC",
            pid,
            |r| {
                Ok(GeodesyPoint {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    inspection_id: r.get(2)?,
                    mark: r.get(3)?,
                    axis: r.get(4)?,
                    level: r.get(5)?,
                    design: r.get(6)?,
                    fact: r.get(7)?,
                    tolerance: r.get(8)?,
                    unit: r.get(9)?,
                    measured: date(&r.get::<_, String>(10)?),
                    surveyor: r.get(11)?,
                    note: r.get(12)?,
                })
            },
        )
    }

    pub fn insert_geodesy_point(&self, x: &GeodesyPoint) -> i64 {
        self.ins(
            "INSERT INTO geodesy_point (project_id,inspection_id,mark,axis,level,design,fact,
                                        tolerance,unit,measured,surveyor,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                x.project_id,
                x.inspection_id,
                x.mark,
                x.axis,
                x.level,
                x.design,
                x.fact,
                x.tolerance,
                x.unit,
                x.measured.to_string(),
                x.surveyor,
                x.note
            ],
        )
    }

    pub fn update_geodesy_point(&self, x: &GeodesyPoint) -> bool {
        self.upd(
            "UPDATE geodesy_point SET inspection_id=?2,mark=?3,axis=?4,level=?5,design=?6,fact=?7,
                    tolerance=?8,unit=?9,measured=?10,surveyor=?11,note=?12 WHERE id=?1",
            params![
                x.id,
                x.inspection_id,
                x.mark,
                x.axis,
                x.level,
                x.design,
                x.fact,
                x.tolerance,
                x.unit,
                x.measured.to_string(),
                x.surveyor,
                x.note
            ],
        )
    }

    pub fn delete_geodesy_point(&self, id: i64) -> bool {
        self.del("geodesy_point", id)
    }

    // ---------- XI.32-34. Asboblar ----------

    pub fn tools(&self, pid: i64) -> Vec<Tool> {
        self.list(
            "SELECT id,project_id,code,name,kind,inventory_no,price,condition,check_due,note
             FROM tool WHERE project_id=?1 ORDER BY name,id",
            pid,
            |r| {
                Ok(Tool {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    code: r.get(2)?,
                    name: r.get(3)?,
                    kind: ToolKind::parse(&r.get::<_, String>(4)?),
                    inventory_no: r.get(5)?,
                    price: r.get(6)?,
                    condition: ToolCondition::parse(&r.get::<_, String>(7)?),
                    check_due: odate(r.get(8)?),
                    note: r.get(9)?,
                })
            },
        )
    }

    pub fn insert_tool(&self, x: &Tool) -> i64 {
        self.ins(
            "INSERT INTO tool (project_id,code,name,kind,inventory_no,price,condition,check_due,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                x.project_id,
                x.code,
                x.name,
                x.kind.code(),
                x.inventory_no,
                x.price,
                x.condition.code(),
                ods(x.check_due),
                x.note
            ],
        )
    }

    pub fn update_tool(&self, x: &Tool) -> bool {
        self.upd(
            "UPDATE tool SET code=?2,name=?3,kind=?4,inventory_no=?5,price=?6,condition=?7,
                    check_due=?8,note=?9 WHERE id=?1",
            params![
                x.id,
                x.code,
                x.name,
                x.kind.code(),
                x.inventory_no,
                x.price,
                x.condition.code(),
                ods(x.check_due),
                x.note
            ],
        )
    }

    pub fn delete_tool(&self, id: i64) -> bool {
        self.del("tool", id)
    }

    pub fn tool_issues(&self, pid: i64) -> Vec<ToolIssue> {
        self.list(
            "SELECT id,project_id,tool_id,worker_id,issued,due,returned,note
             FROM tool_issue WHERE project_id=?1 ORDER BY issued DESC,id DESC",
            pid,
            |r| {
                Ok(ToolIssue {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    tool_id: r.get(2)?,
                    worker_id: r.get(3)?,
                    issued: date(&r.get::<_, String>(4)?),
                    due: odate(r.get(5)?),
                    returned: odate(r.get(6)?),
                    note: r.get(7)?,
                })
            },
        )
    }

    pub fn insert_tool_issue(&self, x: &ToolIssue) -> i64 {
        self.ins(
            "INSERT INTO tool_issue (project_id,tool_id,worker_id,issued,due,returned,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                x.project_id,
                x.tool_id,
                x.worker_id,
                x.issued.to_string(),
                ods(x.due),
                ods(x.returned),
                x.note
            ],
        )
    }

    pub fn update_tool_issue(&self, x: &ToolIssue) -> bool {
        self.upd(
            "UPDATE tool_issue SET tool_id=?2,worker_id=?3,issued=?4,due=?5,returned=?6,note=?7
             WHERE id=?1",
            params![
                x.id,
                x.tool_id,
                x.worker_id,
                x.issued.to_string(),
                ods(x.due),
                ods(x.returned),
                x.note
            ],
        )
    }

    pub fn delete_tool_issue(&self, id: i64) -> bool {
        self.del("tool_issue", id)
    }

    // ---------- XIII.35-36. Tabel davri ----------

    pub fn timesheet_periods(&self, pid: i64) -> Vec<TimesheetPeriod> {
        self.list(
            "SELECT id,project_id,month,closed,closed_at,closed_by,reopen_reason,note
             FROM timesheet_period WHERE project_id=?1 ORDER BY month DESC",
            pid,
            |r| {
                Ok(TimesheetPeriod {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    month: date(&r.get::<_, String>(2)?),
                    closed: r.get::<_, i64>(3)? != 0,
                    closed_at: odate(r.get(4)?),
                    closed_by: r.get(5)?,
                    reopen_reason: r.get(6)?,
                    note: r.get(7)?,
                })
            },
        )
    }

    pub fn insert_timesheet_period(&self, x: &TimesheetPeriod) -> i64 {
        self.ins(
            "INSERT INTO timesheet_period (project_id,month,closed,closed_at,closed_by,
                                           reopen_reason,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                x.project_id,
                x.month.to_string(),
                x.closed as i64,
                ods(x.closed_at),
                x.closed_by,
                x.reopen_reason,
                x.note
            ],
        )
    }

    pub fn update_timesheet_period(&self, x: &TimesheetPeriod) -> bool {
        self.upd(
            "UPDATE timesheet_period SET month=?2,closed=?3,closed_at=?4,closed_by=?5,
                    reopen_reason=?6,note=?7 WHERE id=?1",
            params![
                x.id,
                x.month.to_string(),
                x.closed as i64,
                ods(x.closed_at),
                x.closed_by,
                x.reopen_reason,
                x.note
            ],
        )
    }

    // ---------- XIV.22-25. Sinovlar ----------

    pub fn lab_tests(&self, pid: i64) -> Vec<LabTest> {
        self.list(
            "SELECT id,project_id,task_id,kind,number,subject,date,value,required,unit,result,
                    lab,retest,note
             FROM lab_test WHERE project_id=?1 ORDER BY date DESC,id DESC",
            pid,
            |r| {
                Ok(LabTest {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    task_id: r.get(2)?,
                    kind: LabTestKind::parse(&r.get::<_, String>(3)?),
                    number: r.get(4)?,
                    subject: r.get(5)?,
                    date: date(&r.get::<_, String>(6)?),
                    value: r.get(7)?,
                    required: r.get(8)?,
                    unit: r.get(9)?,
                    result: LabTestResult::parse(&r.get::<_, String>(10)?),
                    lab: r.get(11)?,
                    retest: odate(r.get(12)?),
                    note: r.get(13)?,
                })
            },
        )
    }

    pub fn insert_lab_test(&self, x: &LabTest) -> i64 {
        self.ins(
            "INSERT INTO lab_test (project_id,task_id,kind,number,subject,date,value,required,
                                   unit,result,lab,retest,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                x.project_id,
                x.task_id,
                x.kind.code(),
                x.number,
                x.subject,
                x.date.to_string(),
                x.value,
                x.required,
                x.unit,
                x.result.code(),
                x.lab,
                ods(x.retest),
                x.note
            ],
        )
    }

    pub fn update_lab_test(&self, x: &LabTest) -> bool {
        self.upd(
            "UPDATE lab_test SET task_id=?2,kind=?3,number=?4,subject=?5,date=?6,value=?7,
                    required=?8,unit=?9,result=?10,lab=?11,retest=?12,note=?13 WHERE id=?1",
            params![
                x.id,
                x.task_id,
                x.kind.code(),
                x.number,
                x.subject,
                x.date.to_string(),
                x.value,
                x.required,
                x.unit,
                x.result.code(),
                x.lab,
                ods(x.retest),
                x.note
            ],
        )
    }

    pub fn delete_lab_test(&self, id: i64) -> bool {
        self.del("lab_test", id)
    }

    // ---------- XVI.10-12, 23-24. Texnika bandligi va ta'miri ----------

    pub fn machine_bookings(&self, pid: i64) -> Vec<MachineBooking> {
        self.list(
            "SELECT id,project_id,machine_id,task_id,start,finish,shifts,note
             FROM machine_booking WHERE project_id=?1 ORDER BY start,id",
            pid,
            |r| {
                Ok(MachineBooking {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    machine_id: r.get(2)?,
                    task_id: r.get(3)?,
                    from: date(&r.get::<_, String>(4)?),
                    to: date(&r.get::<_, String>(5)?),
                    shifts: r.get(6)?,
                    note: r.get(7)?,
                })
            },
        )
    }

    pub fn insert_machine_booking(&self, x: &MachineBooking) -> i64 {
        self.ins(
            "INSERT INTO machine_booking (project_id,machine_id,task_id,start,finish,shifts,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                x.project_id,
                x.machine_id,
                x.task_id,
                x.from.to_string(),
                x.to.to_string(),
                x.shifts,
                x.note
            ],
        )
    }

    pub fn update_machine_booking(&self, x: &MachineBooking) -> bool {
        self.upd(
            "UPDATE machine_booking SET machine_id=?2,task_id=?3,start=?4,finish=?5,shifts=?6,
                    note=?7 WHERE id=?1",
            params![
                x.id,
                x.machine_id,
                x.task_id,
                x.from.to_string(),
                x.to.to_string(),
                x.shifts,
                x.note
            ],
        )
    }

    pub fn delete_machine_booking(&self, id: i64) -> bool {
        self.del("machine_booking", id)
    }

    pub fn machine_repairs(&self, pid: i64) -> Vec<MachineRepair> {
        self.list(
            "SELECT id,project_id,machine_id,kind,started,finished,reason,cost,hours_at,note
             FROM machine_repair WHERE project_id=?1 ORDER BY started DESC,id DESC",
            pid,
            |r| {
                Ok(MachineRepair {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    machine_id: r.get(2)?,
                    kind: RepairKind::parse(&r.get::<_, String>(3)?),
                    started: date(&r.get::<_, String>(4)?),
                    finished: odate(r.get(5)?),
                    reason: r.get(6)?,
                    cost: r.get(7)?,
                    hours_at: r.get(8)?,
                    note: r.get(9)?,
                })
            },
        )
    }

    pub fn insert_machine_repair(&self, x: &MachineRepair) -> i64 {
        self.ins(
            "INSERT INTO machine_repair (project_id,machine_id,kind,started,finished,reason,cost,
                                         hours_at,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                x.project_id,
                x.machine_id,
                x.kind.code(),
                x.started.to_string(),
                ods(x.finished),
                x.reason,
                x.cost,
                x.hours_at,
                x.note
            ],
        )
    }

    pub fn update_machine_repair(&self, x: &MachineRepair) -> bool {
        self.upd(
            "UPDATE machine_repair SET machine_id=?2,kind=?3,started=?4,finished=?5,reason=?6,
                    cost=?7,hours_at=?8,note=?9 WHERE id=?1",
            params![
                x.id,
                x.machine_id,
                x.kind.code(),
                x.started.to_string(),
                ods(x.finished),
                x.reason,
                x.cost,
                x.hours_at,
                x.note
            ],
        )
    }

    pub fn delete_machine_repair(&self, id: i64) -> bool {
        self.del("machine_repair", id)
    }

    // ---------- XV.15, 22-24. Xavfli zonalar va inventar ----------

    pub fn safety_zones(&self, pid: i64) -> Vec<SafetyZone> {
        self.list(
            "SELECT id,project_id,kind,name,place,measure,responsible,check_due,checked_at,
                    ready,note
             FROM safety_zone WHERE project_id=?1 ORDER BY kind,id",
            pid,
            |r| {
                Ok(SafetyZone {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    kind: ZoneKind::parse(&r.get::<_, String>(2)?),
                    name: r.get(3)?,
                    place: r.get(4)?,
                    measure: r.get(5)?,
                    responsible: r.get(6)?,
                    check_due: odate(r.get(7)?),
                    checked_at: odate(r.get(8)?),
                    ready: r.get::<_, i64>(9)? != 0,
                    note: r.get(10)?,
                })
            },
        )
    }

    pub fn insert_safety_zone(&self, x: &SafetyZone) -> i64 {
        self.ins(
            "INSERT INTO safety_zone (project_id,kind,name,place,measure,responsible,check_due,
                                      checked_at,ready,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                x.project_id,
                x.kind.code(),
                x.name,
                x.place,
                x.measure,
                x.responsible,
                ods(x.check_due),
                ods(x.checked_at),
                x.ready as i64,
                x.note
            ],
        )
    }

    pub fn update_safety_zone(&self, x: &SafetyZone) -> bool {
        self.upd(
            "UPDATE safety_zone SET kind=?2,name=?3,place=?4,measure=?5,responsible=?6,
                    check_due=?7,checked_at=?8,ready=?9,note=?10 WHERE id=?1",
            params![
                x.id,
                x.kind.code(),
                x.name,
                x.place,
                x.measure,
                x.responsible,
                ods(x.check_due),
                ods(x.checked_at),
                x.ready as i64,
                x.note
            ],
        )
    }

    pub fn delete_safety_zone(&self, id: i64) -> bool {
        self.del("safety_zone", id)
    }

    // ---------- VIII. Buyurtmachi: shartnomalar va to'lovlar ----------

    pub fn contracts(&self, pid: i64) -> Vec<Contract> {
        self.list(
            "SELECT id,project_id,number,name,kind,party_id,signed,start,finish,sum,
                    advance_pct,retention_pct,currency,status,note
             FROM contract WHERE project_id=?1 ORDER BY signed DESC,id DESC",
            pid,
            |r| {
                Ok(Contract {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    number: r.get(2)?,
                    name: r.get(3)?,
                    kind: ContractKind::parse(&r.get::<_, String>(4)?),
                    party_id: r.get(5)?,
                    signed: date(&r.get::<_, String>(6)?),
                    start: date(&r.get::<_, String>(7)?),
                    end: date(&r.get::<_, String>(8)?),
                    sum: r.get(9)?,
                    advance_pct: r.get(10)?,
                    retention_pct: r.get(11)?,
                    currency: r.get(12)?,
                    status: ContractStatus::parse(&r.get::<_, String>(13)?),
                    note: r.get(14)?,
                })
            },
        )
    }

    pub fn insert_contract(&self, x: &Contract) -> i64 {
        self.ins(
            "INSERT INTO contract (project_id,number,name,kind,party_id,signed,start,finish,sum,
                                   advance_pct,retention_pct,currency,status,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                x.project_id,
                x.number,
                x.name,
                x.kind.code(),
                x.party_id,
                x.signed.to_string(),
                x.start.to_string(),
                x.end.to_string(),
                x.sum,
                x.advance_pct,
                x.retention_pct,
                x.currency,
                x.status.code(),
                x.note
            ],
        )
    }

    pub fn update_contract(&self, x: &Contract) -> bool {
        self.upd(
            "UPDATE contract SET number=?2,name=?3,kind=?4,party_id=?5,signed=?6,start=?7,
                    finish=?8,sum=?9,advance_pct=?10,retention_pct=?11,currency=?12,status=?13,
                    note=?14 WHERE id=?1",
            params![
                x.id,
                x.number,
                x.name,
                x.kind.code(),
                x.party_id,
                x.signed.to_string(),
                x.start.to_string(),
                x.end.to_string(),
                x.sum,
                x.advance_pct,
                x.retention_pct,
                x.currency,
                x.status.code(),
                x.note
            ],
        )
    }

    pub fn delete_contract(&self, id: i64) -> bool {
        self.del("contract", id)
    }

    pub fn contract_changes(&self, pid: i64) -> Vec<ContractChange> {
        self.list(
            "SELECT id,project_id,contract_id,number,kind,date,description,amount,days,reason,
                    status,decided_at,decided_by,note
             FROM contract_change WHERE project_id=?1 ORDER BY date DESC,id DESC",
            pid,
            |r| {
                Ok(ContractChange {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    contract_id: r.get(2)?,
                    number: r.get(3)?,
                    kind: ChangeKind::parse(&r.get::<_, String>(4)?),
                    date: date(&r.get::<_, String>(5)?),
                    description: r.get(6)?,
                    amount: r.get(7)?,
                    days: r.get(8)?,
                    reason: r.get(9)?,
                    status: ChangeStatus::parse(&r.get::<_, String>(10)?),
                    decided_at: odate(r.get(11)?),
                    decided_by: r.get(12)?,
                    note: r.get(13)?,
                })
            },
        )
    }

    pub fn insert_contract_change(&self, x: &ContractChange) -> i64 {
        self.ins(
            "INSERT INTO contract_change (project_id,contract_id,number,kind,date,description,
                                          amount,days,reason,status,decided_at,decided_by,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                x.project_id,
                x.contract_id,
                x.number,
                x.kind.code(),
                x.date.to_string(),
                x.description,
                x.amount,
                x.days,
                x.reason,
                x.status.code(),
                ods(x.decided_at),
                x.decided_by,
                x.note
            ],
        )
    }

    pub fn update_contract_change(&self, x: &ContractChange) -> bool {
        self.upd(
            "UPDATE contract_change SET contract_id=?2,number=?3,kind=?4,date=?5,description=?6,
                    amount=?7,days=?8,reason=?9,status=?10,decided_at=?11,decided_by=?12,note=?13
             WHERE id=?1",
            params![
                x.id,
                x.contract_id,
                x.number,
                x.kind.code(),
                x.date.to_string(),
                x.description,
                x.amount,
                x.days,
                x.reason,
                x.status.code(),
                ods(x.decided_at),
                x.decided_by,
                x.note
            ],
        )
    }

    pub fn delete_contract_change(&self, id: i64) -> bool {
        self.del("contract_change", id)
    }

    pub fn payment_stages(&self, pid: i64) -> Vec<PaymentStage> {
        self.list(
            "SELECT id,project_id,contract_id,number,basis,due,amount,paid,paid_at,note
             FROM payment_stage WHERE project_id=?1 ORDER BY due,id",
            pid,
            |r| {
                Ok(PaymentStage {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    contract_id: r.get(2)?,
                    number: r.get(3)?,
                    basis: r.get(4)?,
                    due: date(&r.get::<_, String>(5)?),
                    amount: r.get(6)?,
                    paid: r.get(7)?,
                    paid_at: odate(r.get(8)?),
                    note: r.get(9)?,
                })
            },
        )
    }

    pub fn insert_payment_stage(&self, x: &PaymentStage) -> i64 {
        self.ins(
            "INSERT INTO payment_stage (project_id,contract_id,number,basis,due,amount,paid,
                                        paid_at,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                x.project_id,
                x.contract_id,
                x.number,
                x.basis,
                x.due.to_string(),
                x.amount,
                x.paid,
                ods(x.paid_at),
                x.note
            ],
        )
    }

    pub fn update_payment_stage(&self, x: &PaymentStage) -> bool {
        self.upd(
            "UPDATE payment_stage SET contract_id=?2,number=?3,basis=?4,due=?5,amount=?6,paid=?7,
                    paid_at=?8,note=?9 WHERE id=?1",
            params![
                x.id,
                x.contract_id,
                x.number,
                x.basis,
                x.due.to_string(),
                x.amount,
                x.paid,
                ods(x.paid_at),
                x.note
            ],
        )
    }

    pub fn delete_payment_stage(&self, id: i64) -> bool {
        self.del("payment_stage", id)
    }

    pub fn work_acceptances(&self, pid: i64) -> Vec<WorkAcceptance> {
        self.list(
            "SELECT id,project_id,task_id,number,date,volume,unit,amount,state,decided_at,
                    decided_by,comment
             FROM work_acceptance WHERE project_id=?1 ORDER BY date DESC,id DESC",
            pid,
            |r| {
                Ok(WorkAcceptance {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    task_id: r.get(2)?,
                    number: r.get(3)?,
                    date: date(&r.get::<_, String>(4)?),
                    volume: r.get(5)?,
                    unit: r.get(6)?,
                    amount: r.get(7)?,
                    state: AcceptState::parse(&r.get::<_, String>(8)?),
                    decided_at: odate(r.get(9)?),
                    decided_by: r.get(10)?,
                    comment: r.get(11)?,
                })
            },
        )
    }

    pub fn insert_work_acceptance(&self, x: &WorkAcceptance) -> i64 {
        self.ins(
            "INSERT INTO work_acceptance (project_id,task_id,number,date,volume,unit,amount,state,
                                          decided_at,decided_by,comment)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                x.project_id,
                x.task_id,
                x.number,
                x.date.to_string(),
                x.volume,
                x.unit,
                x.amount,
                x.state.code(),
                ods(x.decided_at),
                x.decided_by,
                x.comment
            ],
        )
    }

    pub fn update_work_acceptance(&self, x: &WorkAcceptance) -> bool {
        self.upd(
            "UPDATE work_acceptance SET task_id=?2,number=?3,date=?4,volume=?5,unit=?6,amount=?7,
                    state=?8,decided_at=?9,decided_by=?10,comment=?11 WHERE id=?1",
            params![
                x.id,
                x.task_id,
                x.number,
                x.date.to_string(),
                x.volume,
                x.unit,
                x.amount,
                x.state.code(),
                ods(x.decided_at),
                x.decided_by,
                x.comment
            ],
        )
    }

    pub fn delete_work_acceptance(&self, id: i64) -> bool {
        self.del("work_acceptance", id)
    }

    // ---------- XV. Xavfsizlik ----------

    pub fn safety_events(&self, pid: i64) -> Vec<SafetyEvent> {
        self.list(
            "SELECT id,project_id,date,kind,severity,place,description,responsible,measure,deadline,
                    status,root_cause
             FROM safety_event WHERE project_id=?1 ORDER BY date DESC,id DESC",
            pid,
            |r| {
                Ok(SafetyEvent {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    date: date(&r.get::<_, String>(2)?),
                    kind: SafetyKind::parse(&r.get::<_, String>(3)?),
                    severity: Severity::parse(&r.get::<_, String>(4)?),
                    place: r.get(5)?,
                    description: r.get(6)?,
                    responsible: r.get(7)?,
                    measure: r.get(8)?,
                    deadline: odate(r.get(9)?),
                    status: IssueStatus::parse(&r.get::<_, String>(10)?),
                    root_cause: RootCause::parse(&r.get::<_, String>(11)?),
                })
            },
        )
    }

    pub fn insert_safety(&self, s: &SafetyEvent) -> i64 {
        self.ins(
            "INSERT INTO safety_event (project_id,date,kind,severity,place,description,responsible,
                                       measure,deadline,status,root_cause)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                s.project_id,
                s.date.to_string(),
                s.kind.code(),
                s.severity.code(),
                s.place,
                s.description,
                s.responsible,
                s.measure,
                ods(s.deadline),
                s.status.code(),
                s.root_cause.code()
            ],
        )
    }

    pub fn update_safety(&self, s: &SafetyEvent) -> bool {
        self.upd(
            "UPDATE safety_event SET date=?2,kind=?3,severity=?4,place=?5,description=?6,
                    responsible=?7,measure=?8,deadline=?9,status=?10,root_cause=?11
             WHERE id=?1",
            params![
                s.id,
                s.date.to_string(),
                s.kind.code(),
                s.severity.code(),
                s.place,
                s.description,
                s.responsible,
                s.measure,
                ods(s.deadline),
                s.status.code(),
                s.root_cause.code()
            ],
        )
    }

    // ---------- XVI. Mashinalar ----------

    pub fn machines(&self, pid: i64) -> Vec<Machine> {
        self.list(
            "SELECT id,project_id,name,kind,reg_no,owner,status,hour_rate,operator,inspection_until,
                    fuel_norm,service_hours,service_done,rented,price
             FROM machine WHERE project_id=?1 ORDER BY name",
            pid,
            |r| {
                Ok(Machine {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    name: r.get(2)?,
                    kind: MachineKind::parse(&r.get::<_, String>(3)?),
                    reg_no: r.get(4)?,
                    owner: r.get(5)?,
                    status: MachineStatus::parse(&r.get::<_, String>(6)?),
                    hour_rate: r.get(7)?,
                    operator: r.get(8)?,
                    inspection_until: odate(r.get(9)?),
                    fuel_norm: r.get(10)?,
                    service_hours: r.get(11)?,
                    service_done: r.get(12)?,
                    rented: r.get::<_, i64>(13)? != 0,
                    price: r.get(14)?,
                })
            },
        )
    }

    pub fn insert_machine(&self, m: &Machine) -> i64 {
        self.ins(
            "INSERT INTO machine (project_id,name,kind,reg_no,owner,status,hour_rate,operator,
                                  inspection_until,fuel_norm,service_hours,service_done,rented,
                                  price)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                m.project_id,
                m.name,
                m.kind.code(),
                m.reg_no,
                m.owner,
                m.status.code(),
                m.hour_rate,
                m.operator,
                ods(m.inspection_until),
                m.fuel_norm,
                m.service_hours,
                m.service_done,
                i64::from(m.rented),
                m.price
            ],
        )
    }

    pub fn update_machine(&self, m: &Machine) -> bool {
        self.upd(
            "UPDATE machine SET name=?2,kind=?3,reg_no=?4,owner=?5,status=?6,hour_rate=?7,
                    operator=?8,inspection_until=?9,fuel_norm=?10,service_hours=?11,
                    service_done=?12,rented=?13,price=?14 WHERE id=?1",
            params![
                m.id,
                m.name,
                m.kind.code(),
                m.reg_no,
                m.owner,
                m.status.code(),
                m.hour_rate,
                m.operator,
                ods(m.inspection_until),
                m.fuel_norm,
                m.service_hours,
                m.service_done,
                i64::from(m.rented),
                m.price
            ],
        )
    }

    pub fn machine_logs(&self, pid: i64) -> Vec<MachineLog> {
        self.list(
            "SELECT id,project_id,machine_id,date,hours,fuel,task_id,number,driver,route,
                    odo_start,odo_end,trips,cargo,note
             FROM machine_log WHERE project_id=?1 ORDER BY date DESC,id DESC",
            pid,
            |r| {
                Ok(MachineLog {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    machine_id: r.get(2)?,
                    date: date(&r.get::<_, String>(3)?),
                    hours: r.get(4)?,
                    fuel: r.get(5)?,
                    task_id: r.get(6)?,
                    number: r.get(7)?,
                    driver: r.get(8)?,
                    route: r.get(9)?,
                    odo_start: r.get(10)?,
                    odo_end: r.get(11)?,
                    trips: r.get(12)?,
                    cargo: r.get(13)?,
                    note: r.get(14)?,
                })
            },
        )
    }

    pub fn insert_machine_log(&self, l: &MachineLog) -> i64 {
        self.ins(
            "INSERT INTO machine_log (project_id,machine_id,date,hours,fuel,task_id,number,driver,
                                      route,odo_start,odo_end,trips,cargo,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                l.project_id,
                l.machine_id,
                l.date.to_string(),
                l.hours,
                l.fuel,
                l.task_id,
                l.number,
                l.driver,
                l.route,
                l.odo_start,
                l.odo_end,
                l.trips,
                l.cargo,
                l.note
            ],
        )
    }

    pub fn update_machine_log(&self, l: &MachineLog) -> bool {
        self.upd(
            "UPDATE machine_log SET machine_id=?2,date=?3,hours=?4,fuel=?5,task_id=?6,number=?7,
                    driver=?8,route=?9,odo_start=?10,odo_end=?11,trips=?12,cargo=?13,note=?14
             WHERE id=?1",
            params![
                l.id,
                l.machine_id,
                l.date.to_string(),
                l.hours,
                l.fuel,
                l.task_id,
                l.number,
                l.driver,
                l.route,
                l.odo_start,
                l.odo_end,
                l.trips,
                l.cargo,
                l.note
            ],
        )
    }

    // ---------- Namoyish ma'lumoti: II-III modullar ----------

    /// Loyiha elementlari grafi va smeta namunasi.
    ///
    /// Ma'lumot ataylab bir nechta tipik nomuvofiqlik bilan tuzilgan — tekshiruv
    /// ishlayotganini ko'rsatish uchun: teshiksiz deraza, ta'minotsiz xona,
    /// rigelni kesib o'tuvchi quvur, uklon yetishmovchiligi, arifmetik xato,
    /// dublikat va narx anomaliyasi.
    pub fn seed_demo_modules(&self, pid: i64) {
        let ru = crate::i18n::lang() == crate::i18n::Lang::Ru;
        let s = |uz: &'static str, r: &'static str| -> String {
            if ru {
                r.to_string()
            } else {
                uz.to_string()
            }
        };

        // (bo'lim, tur, marka, xona, o'q, qavat, o'lcham, birlik, qiymat, qiymat nomi, varaq)
        let el = |section: Section,
                  kind: ElementKind,
                  mark: &str,
                  room: String,
                  axis: &str,
                  level: &str,
                  size: f64,
                  unit: &str,
                  value: f64,
                  value_name: &str,
                  sheet: &str|
         -> i64 {
            self.insert_element(&Element {
                id: 0,
                project_id: pid,
                section,
                kind,
                mark: mark.into(),
                room,
                axis: axis.into(),
                level: level.into(),
                size,
                unit: unit.into(),
                value,
                value_name: value_name.into(),
                sheet: sheet.into(),
                note: String::new(),
            })
        };

        let r101 = el(
            Section::Ar,
            ElementKind::Room,
            "101",
            s("101-xona, kvartira 1", "Помещение 101, кв. 1"),
            "A-B/1-2",
            "1",
            0.0,
            "",
            42.5,
            s("maydon", "площадь").as_str(),
            "AR-04",
        );
        let r102 = el(
            Section::Ar,
            ElementKind::Room,
            "102",
            s("102-xona, kvartira 2", "Помещение 102, кв. 2"),
            "B-V/1-2",
            "1",
            0.0,
            "",
            38.0,
            s("maydon", "площадь").as_str(),
            "AR-04",
        );
        let r103 = el(
            Section::Ar,
            ElementKind::Room,
            "103",
            s("103-xona, omborxona", "Помещение 103, кладовая"),
            "V-G/1-2",
            "1",
            0.0,
            "",
            12.0,
            s("maydon", "площадь").as_str(),
            "AR-04",
        );

        // OK-1 uchun teshik bor, OK-2 uchun yo'q -> AR/KJ nomuvofiqligi.
        let ok1 = el(
            Section::Ar,
            ElementKind::Window,
            "OK-1",
            s("101-xona", "Помещение 101"),
            "A/1",
            "1",
            1500.0,
            "mm",
            0.0,
            "",
            "AR-06",
        );
        let ok2 = el(
            Section::Ar,
            ElementKind::Window,
            "OK-2",
            s("102-xona", "Помещение 102"),
            "B/1",
            "1",
            1500.0,
            "mm",
            0.0,
            "",
            "AR-06",
        );
        // Marka takrorlangan: bir bo'limda ikkita OK-1.
        let _ok1b = el(
            Section::Ar,
            ElementKind::Window,
            "OK-1",
            s("103-xona", "Помещение 103"),
            "V/1",
            "1",
            900.0,
            "mm",
            0.0,
            "",
            "AR-06",
        );
        let d1 = el(
            Section::Ar,
            ElementKind::Door,
            "D-1",
            s("101-xona", "Помещение 101"),
            "A/2",
            "1",
            900.0,
            "mm",
            0.0,
            "",
            "AR-06",
        );

        let pr1 = el(
            Section::Kj,
            ElementKind::Opening,
            "PR-1",
            s("101-xona", "Помещение 101"),
            "A/1",
            "1",
            1500.0,
            "mm",
            0.0,
            "",
            "KJ-12",
        );
        let pr2 = el(
            Section::Kj,
            ElementKind::Opening,
            "PR-2",
            s("101-xona", "Помещение 101"),
            "A/2",
            "1",
            900.0,
            "mm",
            0.0,
            "",
            "KJ-12",
        );
        let b1 = el(
            Section::Kj,
            ElementKind::Beam,
            "B-1",
            s("Koridor", "Коридор"),
            "B/1-2",
            "1",
            400.0,
            "mm",
            0.0,
            "",
            "KJ-08",
        );
        let k1 = el(
            Section::Kj,
            ElementKind::Column,
            "K-1",
            s("101-xona", "Помещение 101"),
            "A/1",
            "1",
            400.0,
            "mm",
            0.0,
            "",
            "KJ-05",
        );

        // Uklon 0.008 — minimal 0.02 dan past.
        let p1 = el(
            Section::Vk,
            ElementKind::Pipe,
            "K1-1",
            s("101-xona", "Помещение 101"),
            "A/1",
            "1",
            110.0,
            "mm",
            0.008,
            s("uklon", "уклон").as_str(),
            "VK-03",
        );
        // Rigelni kesib o'tadi, teshik ko'zda tutilmagan.
        let p2 = el(
            Section::Vk,
            ElementKind::Pipe,
            "K1-2",
            s("Koridor", "Коридор"),
            "B/1-2",
            "1",
            100.0,
            "mm",
            0.025,
            s("uklon", "уклон").as_str(),
            "VK-03",
        );
        let v1 = el(
            Section::Ov,
            ElementKind::Duct,
            "V-1",
            s("Koridor", "Коридор"),
            "B/1-2",
            "1",
            200.0,
            "mm",
            0.0,
            "",
            "OV-02",
        );
        // O'lcham nol va varaq ko'rsatilmagan — ikkita alohida qoida.
        let w1 = el(
            Section::Eom,
            ElementKind::Cable,
            "W-1",
            s("101-xona", "Помещение 101"),
            "A/1",
            "1",
            0.0,
            "mm2",
            0.0,
            "",
            "",
        );
        let w2 = el(
            Section::Eom,
            ElementKind::Cable,
            "W-2",
            s("103-xona", "Помещение 103"),
            "V/1",
            "1",
            4.0,
            "mm2",
            0.0,
            "",
            "EOM-01",
        );
        // Grafda yolg'iz turgan qurilma.
        let _sh1 = el(
            Section::Eom,
            ElementKind::Device,
            "SH-1",
            s("Elektr xonasi", "Электрощитовая"),
            "G/1",
            "1",
            0.0,
            "",
            25.0,
            s("quvvat", "мощность").as_str(),
            "EOM-01",
        );
        // Elektr ta'minoti ko'zda tutilmagan ventilyator va yong'in izvestchateli.
        let vn1 = el(
            Section::Ov,
            ElementKind::Device,
            "VN-1",
            s("Koridor", "Коридор"),
            "B/2",
            "1",
            0.0,
            "",
            1.5,
            s("quvvat", "мощность").as_str(),
            "OV-02",
        );
        let ip1 = el(
            Section::Pb,
            ElementKind::Device,
            "IP-1",
            s("103-xona", "Помещение 103"),
            "V/1",
            "1",
            0.0,
            "",
            0.02,
            s("quvvat", "мощность").as_str(),
            "PB-03",
        );
        // Nasos esa to'g'ri ulangan — ijobiy misol.
        let n1 = el(
            Section::Vk,
            ElementKind::Device,
            "N-1",
            s("Nasos xonasi", "Насосная"),
            "G/2",
            "0",
            0.0,
            "",
            4.0,
            s("quvvat", "мощность").as_str(),
            "VK-01",
        );
        // Metall rigel: KJ bilan tayanch tuguni kelishilmagan.
        let mb1 = el(
            Section::Km,
            ElementKind::Beam,
            "MB-1",
            s("Koridor", "Коридор"),
            "B/1-2",
            "9",
            300.0,
            "mm",
            0.0,
            "",
            "KM-02",
        );
        // Metall ustun: tayanchi KJ ustuniga bog'langan.
        let mk1 = el(
            Section::Km,
            ElementKind::Column,
            "MK-1",
            s("101-xona", "Помещение 101"),
            "A/1",
            "9",
            200.0,
            "mm",
            0.0,
            "",
            "KM-02",
        );

        let link = |from_el: i64, to_el: i64, relation: Relation| {
            self.insert_element_link(&ElementLink {
                id: 0,
                from_el,
                to_el,
                relation,
            });
        };
        link(ok1, pr1, Relation::Contains);
        link(d1, pr2, Relation::Contains);
        link(r101, k1, Relation::Contains);
        link(r101, ok1, Relation::Contains);
        link(r102, ok2, Relation::Contains);
        // 101-xona to'liq ta'minlangan, 102 — faqat VK va OV, 103 — faqat EOM.
        link(p1, r101, Relation::Serves);
        link(v1, r101, Relation::Serves);
        link(w1, r101, Relation::Serves);
        link(p2, r102, Relation::Serves);
        link(v1, r102, Relation::Serves);
        link(w2, r103, Relation::Serves);
        link(p2, b1, Relation::Crosses);
        link(b1, k1, Relation::SupportedBy);
        // Qurilmalar xonalarga tegishli, lekin quvvat manbai faqat nasosda bor.
        link(r102, vn1, Relation::Contains);
        link(r103, ip1, Relation::Contains);
        link(r101, mb1, Relation::Contains);
        link(n1, w2, Relation::PoweredBy);
        link(mk1, k1, Relation::SupportedBy);

        self.seed_demo_estimate(pid, ru);
        self.seed_demo_execution(pid, ru);
        self.seed_demo_stock(pid, ru);
        self.seed_demo_supply(pid, ru);
        self.seed_demo_sales(pid, ru);
        self.seed_demo_resources(pid, ru);
        self.seed_demo_supervision(pid, ru);
        self.seed_demo_client(pid, ru);
        self.seed_demo_docs(pid, ru);
        self.seed_demo_history(pid, ru);
        self.seed_demo_supply_extra(pid, ru);
        self.seed_demo_tools(pid, ru);
        self.seed_demo_lab(pid, ru);
        self.seed_demo_machine_plan(pid, ru);
        self.seed_demo_zones(pid, ru);
        self.seed_demo_notes(pid, ru);
        self.seed_demo_machine_checks(pid, ru);
        self.seed_demo_estimate_alt(pid, ru);
    }

    /// Resurs va nazorat modullari namunasi (TZ XIII-XVI).
    ///
    /// Ataylab bir nechta muammoli holat qoldirilgan: bitta materialning kirish
    /// nazorati «mos emas», bitta xavfsizlik yozuvi muddati o'tgan, bitta
    /// texnikaning texnik ko'rigi tugagan.
    /// VII. Texnik nazorat: tekshiruvlar, beton sinovlari va geodeziya.
    pub fn seed_demo_supervision(&self, pid: i64, ru: bool) {
        if !self.inspections(pid).is_empty() {
            return;
        }
        let today = chrono::Local::now().date_naive();
        let tasks = self.tasks(pid).unwrap_or_default();
        let by_wbs = |w: &str| tasks.iter().find(|t| t.wbs == w).map(|t| t.id);
        let d = |back: i64| today - chrono::Duration::days(back);

        let tech = if ru {
            "Собиров Р.Х."
        } else {
            "Sobirov R.X."
        };
        let foreman = if ru {
            "Юсупов Б.Р."
        } else {
            "Yusupov B.R."
        };

        let list: [InspectionDef; 9] = [
            (
                InspectionKind::Hidden,
                "TN-011",
                40,
                Some(40),
                "3-qavat, A-D o'qlari, armatura",
                "3 этаж, оси А-Д, арматура",
                InspectionResult::Pass,
                None,
                None,
                "6",
            ),
            (
                InspectionKind::Concrete,
                "TN-012",
                38,
                Some(38),
                "3-qavat plita, beton quyish",
                "Плита 3 этажа, бетонирование",
                InspectionResult::Pass,
                None,
                None,
                "6",
            ),
            (
                InspectionKind::Geodesy,
                "TN-013",
                31,
                Some(31),
                "4-qavat, ustunlar o'qi",
                "4 этаж, оси колонн",
                InspectionResult::Conditional,
                Some(24),
                Some(22),
                "7",
            ),
            (
                InspectionKind::Hidden,
                "TN-014",
                18,
                Some(17),
                "5-qavat, A-D o'qlari, armatura",
                "5 этаж, оси А-Д, арматура",
                InspectionResult::Fail,
                Some(11),
                None,
                "8",
            ),
            (
                InspectionKind::Material,
                "TN-015",
                12,
                Some(12),
                "Kirish nazorati: g'isht partiyasi",
                "Входной контроль: партия кирпича",
                InspectionResult::Pass,
                None,
                None,
                "9",
            ),
            (
                InspectionKind::Volume,
                "TN-016",
                6,
                Some(6),
                "G'ishtin devor hajmi, 4-qavat",
                "Объём кирпичной кладки, 4 этаж",
                InspectionResult::Conditional,
                Some(1),
                None,
                "9",
            ),
            (
                InspectionKind::Hidden,
                "TN-017",
                0,
                None,
                "6-qavat, A-D o'qlari, armatura",
                "6 этаж, оси А-Д, арматура",
                InspectionResult::Waiting,
                None,
                None,
                "10",
            ),
            (
                InspectionKind::Physical,
                "TN-018",
                -3,
                None,
                "Fasad panellarini mahkamlash",
                "Крепление фасадных панелей",
                InspectionResult::Waiting,
                None,
                None,
                "12",
            ),
            (
                InspectionKind::Concrete,
                "TN-019",
                -6,
                None,
                "6-qavat plita, beton quyish",
                "Плита 6 этажа, бетонирование",
                InspectionResult::Waiting,
                None,
                None,
                "10",
            ),
        ];

        let mut ids: Vec<i64> = Vec::new();
        for (kind, number, plan_back, done_back, place_uz, place_ru, result, dl, fixed, wbs) in list
        {
            let id = self.insert_inspection(&Inspection {
                id: 0,
                project_id: pid,
                task_id: by_wbs(wbs),
                kind,
                number: number.into(),
                planned: d(plan_back),
                done: done_back.map(d),
                requested_by: foreman.into(),
                inspector: if done_back.is_some() {
                    tech.to_string()
                } else {
                    String::new()
                },
                place: if ru { place_ru } else { place_uz }.into(),
                result,
                deadline: dl.map(d),
                fixed_at: fixed.map(d),
                note: String::new(),
            });
            ids.push(id);
        }

        // Beton sinovlari: 7 va 28 kunlik namunalar.
        let concrete: [ConcreteDef; 6] = [
            (
                "N-101",
                "B25",
                "3-qavat plitasi",
                "Плита 3 этажа",
                38,
                7,
                17.5,
                Some(19.2),
            ),
            (
                "N-101",
                "B25",
                "3-qavat plitasi",
                "Плита 3 этажа",
                38,
                28,
                25.0,
                Some(27.8),
            ),
            (
                "N-102",
                "B25",
                "4-qavat plitasi",
                "Плита 4 этажа",
                24,
                7,
                17.5,
                Some(16.1),
            ),
            (
                "N-102",
                "B25",
                "4-qavat plitasi",
                "Плита 4 этажа",
                24,
                28,
                25.0,
                Some(24.1),
            ),
            (
                "N-103",
                "B30",
                "5-qavat ustunlari",
                "Колонны 5 этажа",
                12,
                7,
                21.0,
                Some(22.4),
            ),
            (
                "N-103",
                "B30",
                "5-qavat ustunlari",
                "Колонны 5 этажа",
                12,
                28,
                30.0,
                None,
            ),
        ];
        for (sample, grade, uz, rux, back, age, req, act) in concrete {
            self.insert_concrete_test(&ConcreteTest {
                id: 0,
                project_id: pid,
                inspection_id: None,
                task_id: by_wbs("6"),
                sample: sample.into(),
                grade: grade.into(),
                structure: if ru { rux } else { uz }.into(),
                poured: d(back),
                age_days: age,
                required: req,
                actual: act,
                lab: if ru {
                    "Стройлаборатория №4"
                } else {
                    "4-son qurilish laboratoriyasi"
                }
                .into(),
                note: String::new(),
            });
        }

        // Geodeziya: ustun o'qlari va qavat belgilari, mm da.
        // (marka, o'q, qavat, loyiha, fakt, dopusk)
        let geo: [(&str, &str, &str, f64, f64, f64); 7] = [
            ("K-1", "A/1", "4", 0.0, 4.0, 8.0),
            ("K-2", "A/3", "4", 0.0, -6.0, 8.0),
            ("K-3", "B/1", "4", 0.0, 11.0, 8.0),
            ("K-4", "B/3", "4", 0.0, 3.0, 8.0),
            ("O-1", "A/1", "5", 14_400.0, 14_403.0, 10.0),
            ("O-2", "B/3", "5", 14_400.0, 14_386.0, 10.0),
            ("O-3", "D/3", "5", 14_400.0, 14_398.0, 10.0),
        ];
        for (mark, axis, level, design, fact, tol) in geo {
            self.insert_geodesy_point(&GeodesyPoint {
                id: 0,
                project_id: pid,
                inspection_id: ids.get(2).copied(),
                mark: mark.into(),
                axis: axis.into(),
                level: level.into(),
                design,
                fact,
                tolerance: tol,
                unit: "mm".into(),
                measured: d(31),
                surveyor: if ru {
                    "Эргашев Ж.Т."
                } else {
                    "Ergashev J.T."
                }
                .into(),
                note: String::new(),
            });
        }
    }

    /// VIII. Buyurtmachi: shartnomalar, o'zgarishlar, to'lovlar va qabul.
    pub fn seed_demo_client(&self, pid: i64, ru: bool) {
        if !self.contracts(pid).is_empty() {
            return;
        }
        let today = chrono::Local::now().date_naive();
        let d = |back: i64| today - chrono::Duration::days(back);
        let tasks = self.tasks(pid).unwrap_or_default();
        let by_wbs = |w: &str| tasks.iter().find(|t| t.wbs == w).map(|t| t.id);
        let parties = self.parties(pid).unwrap_or_default();
        let party =
            |role: crate::model::PartyRole| parties.iter().find(|p| p.role == role).map(|p| p.id);

        // ---------- Shartnomalar ----------
        let general = self.insert_contract(&Contract {
            id: 0,
            project_id: pid,
            number: "P-2026/14".into(),
            name: if ru {
                "Генеральный подряд: строительство блока №15"
            } else {
                "Bosh pudrat: 15-blok qurilishi"
            }
            .into(),
            kind: ContractKind::General,
            party_id: party(crate::model::PartyRole::Contractor),
            signed: d(140),
            start: d(126),
            end: d(126) + chrono::Duration::days(400),
            sum: 48_500_000_000.0,
            advance_pct: 15.0,
            retention_pct: 5.0,
            currency: "UZS".into(),
            status: ContractStatus::Active,
            note: String::new(),
        });

        let sub = self.insert_contract(&Contract {
            id: 0,
            project_id: pid,
            number: "S-2026/07".into(),
            name: if ru {
                "Субподряд: инженерные сети"
            } else {
                "Subpudrat: muhandislik tarmoqlari"
            }
            .into(),
            kind: ContractKind::Sub,
            party_id: party(crate::model::PartyRole::Subcontractor),
            signed: d(96),
            start: d(90),
            end: d(90) + chrono::Duration::days(210),
            sum: 6_400_000_000.0,
            advance_pct: 20.0,
            retention_pct: 5.0,
            currency: "UZS".into(),
            status: ContractStatus::Active,
            note: String::new(),
        });

        // ---------- Qiymat va muddat o'zgarishlari ----------
        // (shartnoma, raqam, tur, kun oldin, tavsif uz/ru, summa, kun, holat)
        let changes: [ChangeDef; 5] = [
            (
                general,
                "DS-1",
                ChangeKind::Extra,
                74,
                "Yerto'la devorlariga qo'shimcha gidroizolyatsiya",
                "Дополнительная гидроизоляция стен подвала",
                310_000_000.0,
                0,
                ChangeStatus::Approved,
            ),
            (
                general,
                "DS-2",
                ChangeKind::Price,
                52,
                "Armatura narxining o'zgarishi bo'yicha qayta hisob",
                "Перерасчёт по изменению цены арматуры",
                186_000_000.0,
                0,
                ChangeStatus::Approved,
            ),
            (
                general,
                "DS-3",
                ChangeKind::Term,
                33,
                "Yog'ingarchilik tufayli muddatni surish",
                "Перенос срока из-за осадков",
                0.0,
                14,
                ChangeStatus::Approved,
            ),
            (
                general,
                "DS-4",
                ChangeKind::Extra,
                12,
                "Kirish guruhi fasadini o'zgartirish",
                "Изменение фасада входной группы",
                240_000_000.0,
                7,
                ChangeStatus::Sent,
            ),
            (
                sub,
                "DS-5",
                ChangeKind::Reduce,
                20,
                "Ikkinchi bosqich shamollatishi shartnomadan chiqarildi",
                "Вентиляция второй очереди исключена из договора",
                -420_000_000.0,
                0,
                ChangeStatus::Approved,
            ),
        ];
        for (cid, number, kind, back, uz, rux, amount, days, status) in changes {
            self.insert_contract_change(&ContractChange {
                id: 0,
                project_id: pid,
                contract_id: Some(cid),
                number: number.into(),
                kind,
                date: d(back),
                description: if ru { rux } else { uz }.into(),
                amount,
                days,
                reason: String::new(),
                status,
                decided_at: (status != ChangeStatus::Sent).then(|| d(back - 4)),
                decided_by: if status == ChangeStatus::Sent {
                    String::new()
                } else if ru {
                    "Тошматов А.А.".into()
                } else {
                    "Toshmatov A.A.".into()
                },
                note: String::new(),
            });
        }

        // ---------- To'lov jadvali ----------
        // (shartnoma, raqam, asos uz/ru, muddat kuni, summa, to'langan, to'langan kun)
        let stages: [StageDef; 7] = [
            (
                general,
                "T-1",
                "Avans",
                "Аванс",
                120,
                7_275_000_000.0,
                7_275_000_000.0,
                Some(118),
            ),
            (
                general,
                "T-2",
                "1-oy bajarilgan ish",
                "Работы за 1 месяц",
                96,
                5_200_000_000.0,
                5_200_000_000.0,
                Some(92),
            ),
            (
                general,
                "T-3",
                "2-oy bajarilgan ish",
                "Работы за 2 месяц",
                66,
                6_100_000_000.0,
                6_100_000_000.0,
                Some(51),
            ),
            (
                general,
                "T-4",
                "3-oy bajarilgan ish",
                "Работы за 3 месяц",
                36,
                5_900_000_000.0,
                5_900_000_000.0,
                Some(28),
            ),
            (
                general,
                "T-5",
                "4-oy bajarilgan ish",
                "Работы за 4 месяц",
                6,
                4_800_000_000.0,
                2_825_000_000.0,
                None,
            ),
            (
                general,
                "T-6",
                "5-oy bajarilgan ish",
                "Работы за 5 месяц",
                -24,
                5_100_000_000.0,
                0.0,
                None,
            ),
            (
                sub,
                "S-1",
                "Avans",
                "Аванс",
                88,
                1_280_000_000.0,
                1_280_000_000.0,
                Some(85),
            ),
        ];
        for (cid, number, uz, rux, back, amount, paid, paid_back) in stages {
            self.insert_payment_stage(&PaymentStage {
                id: 0,
                project_id: pid,
                contract_id: Some(cid),
                number: number.into(),
                basis: if ru { rux } else { uz }.into(),
                due: d(back),
                amount,
                paid,
                paid_at: paid_back.map(d),
                note: String::new(),
            });
        }

        // ---------- Ishlarni topshirish ----------
        // (VBS, raqam, kun oldin, hajm, birlik, summa, holat)
        let acts: [AcceptDef; 4] = [
            (
                "5",
                "QT-1",
                58,
                1_150.0,
                "m3",
                2_760_000_000.0,
                AcceptState::Accepted,
            ),
            (
                "6",
                "QT-2",
                34,
                980.0,
                "m3",
                2_352_000_000.0,
                AcceptState::Accepted,
            ),
            (
                "7",
                "QT-3",
                15,
                860.0,
                "m3",
                2_064_000_000.0,
                AcceptState::Rejected,
            ),
            (
                "9",
                "QT-4",
                3,
                1_420.0,
                "m2",
                1_136_000_000.0,
                AcceptState::Submitted,
            ),
        ];
        for (wbs, number, back, volume, unit, amount, state) in acts {
            self.insert_work_acceptance(&WorkAcceptance {
                id: 0,
                project_id: pid,
                task_id: by_wbs(wbs),
                number: number.into(),
                date: d(back),
                volume,
                unit: unit.into(),
                amount,
                state,
                decided_at: (state != AcceptState::Submitted).then(|| d(back - 3)),
                decided_by: if state == AcceptState::Submitted {
                    String::new()
                } else if ru {
                    "Тошматов А.А.".into()
                } else {
                    "Toshmatov A.A.".into()
                },
                comment: if state == AcceptState::Rejected {
                    if ru {
                        "Объём не подтверждён исполнительной съёмкой".into()
                    } else {
                        "Hajm ijro syomkasi bilan tasdiqlanmagan".into()
                    }
                } else {
                    String::new()
                },
            });
        }
    }

    /// Loyiha hujjatlari, saqlangan nomuvofiqliklar va inventarizatsiya.
    ///
    /// AI tekshiruvi topgan nomuvofiqliklar har safar qaytadan hisoblanadi va
    /// almashadi. Bu yerdagilar esa **qaror qabul qilingan** yozuvlar: mas'ul,
    /// muddat va holat bilan — shuning uchun ular qo'lda kiritilgan deb
    /// belgilanadi va avtomatik tekshiruv ularni o'chirmaydi.
    pub fn seed_demo_docs(&self, pid: i64, ru: bool) {
        /// Hisob bo'yicha qoldiq: kirim va qaytarish qo'shiladi, chiqim ayriladi.
        fn balance(moves: &[StockMove], material: i64) -> f64 {
            moves
                .iter()
                .filter(|x| x.material_id == material)
                .map(|x| match x.kind {
                    MoveKind::In | MoveKind::Return => x.qty,
                    _ => -x.qty,
                })
                .sum()
        }

        if !self.documents(pid).is_empty() {
            return;
        }
        let today = chrono::Local::now().date_naive();
        let d = |back: i64| today - chrono::Duration::days(back);
        let stamp = |back: i64| format!("{} 09:00:00", d(back));

        // ---------- Loyiha hujjatlari (TZ II.2) ----------
        // (bo'lim, nomi uz/ru, format, varaq, kun oldin)
        let docs: [(Section, &str, &str, &str, i64, i64); 9] = [
            (
                Section::Ar,
                "AR — Arxitektura yechimlari",
                "АР — Архитектурные решения",
                "pdf",
                42,
                138,
            ),
            (
                Section::Kj,
                "KJ — Temir-beton konstruksiyalar",
                "КЖ — Железобетонные конструкции",
                "pdf",
                64,
                138,
            ),
            (
                Section::Km,
                "KM — Metall konstruksiyalar",
                "КМ — Металлические конструкции",
                "pdf",
                18,
                132,
            ),
            (
                Section::Vk,
                "VK — Suv va kanalizatsiya",
                "ВК — Водоснабжение и канализация",
                "pdf",
                26,
                126,
            ),
            (
                Section::Ov,
                "OV — Isitish va shamollatish",
                "ОВ — Отопление и вентиляция",
                "pdf",
                31,
                126,
            ),
            (
                Section::Eom,
                "EOM — Elektr jihozlari",
                "ЭОМ — Электрооборудование",
                "pdf",
                29,
                120,
            ),
            (
                Section::Ss,
                "SS — Kuchsiz tok tizimlari",
                "СС — Слаботочные системы",
                "pdf",
                14,
                118,
            ),
            (
                Section::Pb,
                "PB — Yong'in xavfsizligi",
                "ПБ — Пожарная безопасность",
                "pdf",
                12,
                118,
            ),
            (
                Section::None,
                "Bosh reja va obodonlashtirish",
                "Генплан и благоустройство",
                "pdf",
                9,
                140,
            ),
        ];
        for (section, uz, rux, fmt, sheets, back) in docs {
            self.insert_document(&Document {
                id: 0,
                project_id: pid,
                section,
                name: if ru { rux } else { uz }.into(),
                format: fmt.into(),
                // Fayl yo'li namunada bo'sh: ilova mavjud bo'lmagan faylga
                // havola ko'rsatmasligi kerak.
                path: String::new(),
                sheets,
                added_at: stamp(back),
                // Birinchi versiya loyihaning boshida topshirilgan.
                revision: String::new(),
                version: 1,
                replaces: None,
                change_note: String::new(),
                issued: Some(d(back)),
            });
        }

        // KJ chizmasining ikkinchi versiyasi (TZ VII.30, 32): u
        // qurilishga topshirilgan va undan **oldin** tugatilgan ishlar
        // bor — versiya nazorati aynan shu holatni ko'rsatishi kerak.
        if let Some(kj) = self
            .documents(pid)
            .into_iter()
            .find(|x| x.section == Section::Kj)
        {
            self.insert_document(&Document {
                id: 0,
                project_id: pid,
                section: Section::Kj,
                name: kj.name.clone(),
                format: kj.format.clone(),
                path: String::new(),
                sheets: kj.sheets + 3,
                added_at: stamp(20),
                revision: if ru {
                    "Изм. 2".into()
                } else {
                    "Izm. 2".to_string()
                },
                version: 2,
                replaces: Some(kj.id),
                change_note: if ru {
                    "Изменено армирование плиты 3 этажа, добавлены 3 листа".into()
                } else {
                    "3-qavat plitasi armaturasi o'zgardi, 3 varaq qo'shildi".to_string()
                },
                issued: Some(d(18)),
            });
        }

        // ---------- Nomuvofiqliklar (TZ II.16-20) ----------
        // (modul, bo'lim, kod, varaq, joy, element, sarlavha uz/ru, tavsif uz/ru,
        //  og'irlik, norma, band, tavsiya uz/ru, mas'ul, muddat, holat, avto)
        let issues: [IssueDef; 12] = [
            (
                IssueModule::Project,
                Section::Kj,
                "KJ-00118",
                "KJ-14",
                "3-qavat, A/2 o'qi",
                "K-2",
                "Ustun armaturasi qoplamasi yetarli emas",
                "Недостаточный защитный слой арматуры колонны",
                "Chizmada qoplama 20 mm, konstruktiv talab 30 mm",
                "На чертеже слой 20 мм, конструктивное требование 30 мм",
                Severity::Critical,
                "ShNQ 2.03.01-96",
                "5.12",
                "Qoplamani 30 mm ga oshirish yoki hisobni qayta ko'rish",
                "Увеличить слой до 30 мм либо пересмотреть расчёт",
                "Rahimov O.S.",
                Some(96),
                IssueStatus::Fixed,
                false,
            ),
            (
                IssueModule::Project,
                Section::Vk,
                "VK-00452",
                "VK-07",
                "Yerto'la, nasos xonasi",
                "N-1",
                "Nasos quvuri diametri hisobga mos emas",
                "Диаметр трубы насоса не соответствует расчёту",
                "Chizmada DN80, gidravlik hisobda DN100 talab qilingan",
                "На чертеже DN80, в гидравлическом расчёте требуется DN100",
                Severity::Major,
                "ShNQ 2.04.01-97",
                "8.4",
                "Quvurni DN100 ga almashtirish",
                "Заменить трубу на DN100",
                "Elektromontaj-Servis",
                Some(64),
                IssueStatus::Fixed,
                false,
            ),
            (
                IssueModule::Project,
                Section::Ar,
                "AR-00231",
                "AR-22",
                "1-qavat, kirish guruhi",
                "D-3",
                "Evakuatsiya eshigi eni normadan kichik",
                "Ширина эвакуационного выхода меньше нормы",
                "Chizmada 900 mm, yong'in normasi bo'yicha 1200 mm kerak",
                "На чертеже 900 мм, по пожарной норме требуется 1200 мм",
                Severity::Critical,
                "ShNQ 2.01.02-94",
                "4.7",
                "Eshik enini 1200 mm ga oshirish",
                "Увеличить ширину двери до 1200 мм",
                "Loyiha instituti",
                Some(12),
                IssueStatus::InWork,
                false,
            ),
            (
                IssueModule::Project,
                Section::Eom,
                "EOM-00087",
                "EOM-05",
                "Elektr shchiti xonasi",
                "SHCH-1",
                "Yerga ulash konturi ko'rsatilmagan",
                "Не показан контур заземления",
                "Shchit xonasi chizmasida yerga ulash sxemasi yo'q",
                "На чертеже щитовой отсутствует схема заземления",
                Severity::Major,
                "PUE",
                "1.7.62",
                "Yerga ulash konturi chizmasini qo'shish",
                "Дополнить чертёж контуром заземления",
                "Loyiha instituti",
                Some(5),
                IssueStatus::InWork,
                false,
            ),
            (
                IssueModule::Project,
                Section::Ov,
                "OV-00164",
                "OV-11",
                "6-qavat, shamollatish kamerasi",
                "V-4",
                "Havo sarfi xonalar bo'yicha mos kelmaydi",
                "Расход воздуха не сходится по помещениям",
                "Tarmoq bo'yicha yig'indi 12 % ga farq qiladi",
                "Сумма по сети расходится на 12 %",
                Severity::Warning,
                "ShNQ 2.04.05-97",
                "6.2",
                "Aeraulik hisobni qayta bajarish",
                "Пересчитать аэравлический расчёт",
                "Loyiha instituti",
                Some(-4),
                IssueStatus::Open,
                false,
            ),
            (
                IssueModule::Project,
                Section::Km,
                "KM-00042",
                "KM-03",
                "Tom, ferma tayanchi",
                "F-2",
                "Payvand chokining o'lchami ko'rsatilmagan",
                "Не указан размер сварного шва",
                "Tayanch tugunida chok belgisi yo'q",
                "В узле опирания отсутствует обозначение шва",
                Severity::Major,
                "ShNQ 2.03.05-97",
                "12.3",
                "Tugun chizmasiga chok belgisini qo'yish",
                "Проставить обозначение шва на чертеже узла",
                "Loyiha instituti",
                Some(-9),
                IssueStatus::Open,
                false,
            ),
            (
                IssueModule::Estimate,
                Section::Kj,
                "SM-00071",
                "",
                "Smeta, 3-bo'lim",
                "",
                "Beton hajmi chizmadagidan katta",
                "Объём бетона больше, чем на чертеже",
                "Smetada 1 240 m3, chizma bo'yicha 1 186 m3",
                "В смете 1 240 м3, по чертежу 1 186 м3",
                Severity::Major,
                "",
                "",
                "Hajmni chizma bo'yicha aniqlashtirish",
                "Уточнить объём по чертежу",
                "Karimova N.A.",
                Some(20),
                IssueStatus::InWork,
                false,
            ),
            (
                IssueModule::Estimate,
                Section::Ar,
                "SM-00089",
                "",
                "Smeta, 5-bo'lim",
                "",
                "Pardozlash narxi bozordan yuqori",
                "Цена отделки выше рыночной",
                "Pozitsiya narxi o'rtachadan 34 % yuqori",
                "Цена позиции выше средней на 34 %",
                Severity::Warning,
                "",
                "",
                "Narxni yetkazib beruvchi takliflari bilan solishtirish",
                "Сверить цену с предложениями поставщиков",
                "Karimova N.A.",
                Some(-2),
                IssueStatus::Open,
                false,
            ),
            (
                IssueModule::Ppr,
                Section::Kj,
                "PPR-00013",
                "",
                "PPR, beton ishlari",
                "",
                "Kran yuk ko'tarish qobiliyati yetarli emas",
                "Грузоподъёмности крана недостаточно",
                "Eng og'ir element 4,2 t, kran chekkada 3,8 t ko'taradi",
                "Самый тяжёлый элемент 4,2 т, кран на вылете поднимает 3,8 т",
                Severity::Major,
                "ShNQ 3.01.01-85",
                "3.9",
                "Kran turini o'zgartirish yoki montaj sxemasini qayta ko'rish",
                "Сменить тип крана либо пересмотреть схему монтажа",
                "Yusupov B.R.",
                Some(-1),
                IssueStatus::Open,
                false,
            ),
            (
                IssueModule::Quality,
                Section::Ar,
                "SF-00027",
                "",
                "4-qavat, 12-xonadon",
                "",
                "Suvoq sirti tekisligi dopuskdan tashqarida",
                "Ровность штукатурки вне допуска",
                "2 m reyka ostida tirqish 6 mm, dopusk 3 mm",
                "Просвет под 2-метровой рейкой 6 мм, допуск 3 мм",
                Severity::Major,
                "ShNQ 3.04.01-87",
                "3.12",
                "Uchastkani qayta suvoqlash",
                "Перештукатурить участок",
                "Umarov D.K.",
                Some(3),
                IssueStatus::InWork,
                false,
            ),
            (
                IssueModule::Safety,
                Section::None,
                "XV-00009",
                "",
                "5-qavat, chekka kontur",
                "",
                "Chekkada to'siq o'rnatilmagan",
                "Не установлено ограждение по краю",
                "Perimetrning 18 m qismida to'siq yo'q",
                "На участке периметра 18 м ограждение отсутствует",
                Severity::Critical,
                "ShNQ 3.01.03-85",
                "6.2",
                "To'siqni darhol o'rnatish, ishni to'xtatish",
                "Немедленно установить ограждение, работы приостановить",
                "Yusupov B.R.",
                Some(8),
                IssueStatus::Fixed,
                false,
            ),
            (
                IssueModule::Project,
                Section::Ss,
                "SS-00018",
                "SS-04",
                "Server xonasi",
                "",
                "Kabel yo'llari yong'in bo'limlari bilan kesishadi",
                "Кабельные трассы пересекают противопожарные отсеки",
                "O'tish joylarida yong'in to'siqlari ko'rsatilmagan",
                "В местах прохода не показаны противопожарные заделки",
                Severity::Warning,
                "ShNQ 2.01.02-94",
                "5.14",
                "O'tish joylarida yong'in to'sig'ini ko'rsatish",
                "Показать противопожарную заделку в местах прохода",
                "Loyiha instituti",
                None,
                IssueStatus::Rejected,
                false,
            ),
        ];
        for (
            module,
            section,
            code,
            sheet,
            location,
            element,
            title_uz,
            title_ru,
            desc_uz,
            desc_ru,
            severity,
            norm_doc,
            norm_clause,
            rec_uz,
            rec_ru,
            responsible,
            deadline,
            status,
            auto,
        ) in issues
        {
            self.insert_issue(&Issue {
                id: 0,
                project_id: pid,
                module,
                section,
                code: code.into(),
                sheet: sheet.into(),
                location: location.into(),
                element: element.into(),
                title: if ru { title_ru } else { title_uz }.into(),
                description: if ru { desc_ru } else { desc_uz }.into(),
                severity,
                norm_doc: norm_doc.into(),
                norm_clause: norm_clause.into(),
                // Norma matni namunada saqlanmaydi: uni o'ylab topib bo'lmaydi,
                // haqiqiy matn reyestrdan olinadi.
                norm_text: String::new(),
                recommendation: if ru { rec_ru } else { rec_uz }.into(),
                responsible: responsible.into(),
                deadline: deadline.map(d),
                status,
                auto,
                created_at: stamp(deadline.map(|x| x + 14).unwrap_or(30)),
            });
        }

        // ---------- Inventarizatsiya (TZ XI.24-25) ----------
        let warehouses = self.warehouses(pid);
        let materials = self.materials(pid);
        if let (Some(wh), false) = (warehouses.first(), materials.is_empty()) {
            let inv = self.insert_inventory(&Inventory {
                id: 0,
                project_id: pid,
                warehouse_id: Some(wh.id),
                date: d(21),
                responsible: if ru {
                    "Хасанов Ф.М."
                } else {
                    "Xasanov F.M."
                }
                .into(),
                closed: true,
                note: if ru {
                    "Ежеквартальная инвентаризация".into()
                } else {
                    "Choraklik inventarizatsiya".to_string()
                },
            });
            // Farqlar ataylab uchta: kam chiqqan, ortiqcha chiqqan va mos kelgan.
            let moves = self.stock_moves(pid);
            for (i, m) in materials.iter().take(5).enumerate() {
                let book = balance(&moves, m.id);
                let fact = match i {
                    0 => book * 0.98,
                    2 => book * 1.03,
                    _ => book,
                };
                self.insert_inventory_line(&InventoryLine {
                    id: 0,
                    inventory_id: inv,
                    material_id: m.id,
                    book,
                    fact,
                    note: String::new(),
                });
            }

            // Ikkinchisi hali ochiq: prorab uni to'ldirayotgan bo'ladi.
            let open = self.insert_inventory(&Inventory {
                id: 0,
                project_id: pid,
                warehouse_id: warehouses.get(1).map(|w| w.id).or(Some(wh.id)),
                date: d(2),
                responsible: if ru {
                    "Хасанов Ф.М."
                } else {
                    "Xasanov F.M."
                }
                .into(),
                closed: false,
                note: if ru {
                    "Выборочная проверка".into()
                } else {
                    "Tanlab tekshirish".to_string()
                },
            });
            for m in materials.iter().skip(2).take(3) {
                let book = balance(&moves, m.id);
                self.insert_inventory_line(&InventoryLine {
                    id: 0,
                    inventory_id: open,
                    material_id: m.id,
                    book,
                    fact: book,
                    note: String::new(),
                });
            }
        }
    }

    /// Kunlik ish tarixi: jurnal, ijro hujjatlari, sifat va xavfsizlik.
    ///
    /// Namunada bir necha yozuv emas, **ikki oylik hayot** bo'lishi kerak:
    /// grafiklar, o'rtachalar va «jurnal to'ldirilgan kunlar» kabi
    /// ko'rsatkichlar faqat shunday ma'noli chiqadi.
    pub fn seed_demo_history(&self, pid: i64, ru: bool) {
        // Jurnalda uchdan ko'p yozuv bo'lsa — tarix allaqachon to'ldirilgan.
        if self.journal(pid).len() > 3 {
            return;
        }
        let today = chrono::Local::now().date_naive();
        let d = |back: i64| today - chrono::Duration::days(back);
        let tasks = self.tasks(pid).unwrap_or_default();
        let by_wbs = |w: &str| tasks.iter().find(|t| t.wbs == w).map(|t| t.id);

        // ---------- Jurnal: 60 kunlik tarix ----------
        // Yozuvlar dam olish kunlarisiz, ish mazmuni davrga qarab o'zgaradi.
        let works: [(&str, &str, &str, &str, f64); 6] = [
            (
                "5",
                "Poydevor plitasini betonlash",
                "Бетонирование фундаментной плиты",
                "m3",
                96.0,
            ),
            (
                "6",
                "3-qavat ustunlarini armaturalash",
                "Армирование колонн 3 этажа",
                "t",
                4.2,
            ),
            (
                "7",
                "4-qavat oralig'ini betonlash",
                "Бетонирование перекрытия 4 этажа",
                "m3",
                78.0,
            ),
            (
                "8",
                "5-qavat karkasini montaj qilish",
                "Монтаж каркаса 5 этажа",
                "m3",
                64.0,
            ),
            (
                "9",
                "G'ishtin devor terish",
                "Кирпичная кладка стен",
                "m3",
                42.0,
            ),
            (
                "10",
                "6-qavat ustunlarini armaturalash",
                "Армирование колонн 6 этажа",
                "t",
                3.8,
            ),
        ];
        // Ob-havo: qurilish jurnalida u har kuni qayd etiladi.
        let weather: [(&str, &str, f64); 5] = [
            ("Ochiq", "Ясно", 27.0),
            ("Bulutli", "Облачно", 23.0),
            ("Yomg'ir", "Дождь", 18.0),
            ("Ochiq", "Ясно", 31.0),
            ("Shamol", "Ветрено", 21.0),
        ];
        let foreman = if ru {
            "Юсупов Б.Р."
        } else {
            "Yusupov B.R."
        };

        for back in 4..64i64 {
            let day = d(back);
            // Yakshanba ish kuni emas.
            if day.weekday() == chrono::Weekday::Sun {
                continue;
            }
            // Ishlar davr bo'yicha almashadi: eski kunlar — quyi qavatlar.
            let w = &works[((63 - back) / 11).clamp(0, 5) as usize];
            let (wu, wr, temp) = weather[(back % 5) as usize];
            // Yomg'irli kunda hajm kamayadi — bu jurnalda ko'rinishi kerak.
            let rain = wu == "Yomg'ir";
            let k = if rain {
                0.45
            } else {
                0.85 + (back % 7) as f64 * 0.05
            };

            self.insert_journal(&JournalEntry {
                id: 0,
                project_id: pid,
                date: day,
                author: foreman.into(),
                weather: if ru { wr } else { wu }.into(),
                temperature: temp,
                workers: if rain { 14 } else { 22 + back % 11 },
                machines: if rain { 1 } else { 2 + back % 3 },
                task_id: by_wbs(w.0),
                volume: (w.4 * k * 10.0).round() / 10.0,
                unit: w.3.into(),
                text: if ru { w.2 } else { w.1 }.into(),
                remarks: if rain {
                    if ru {
                        "Бетонные работы приостановлены из-за осадков".into()
                    } else {
                        "Yog'ingarchilik tufayli beton ishlari to'xtatildi".to_string()
                    }
                } else {
                    String::new()
                },
                photos: String::new(),
                gps: String::new(),
            });
        }

        // ---------- Ijro hujjatlari ----------
        // (tur, raqam, nomi uz/ru, VBS, kun oldin, holat)
        let docs: [ExecDocDef; 12] = [
            (
                ExecDocKind::Hidden,
                "AOSR-014",
                "Poydevor armaturasi",
                "Арматура фундамента",
                "5",
                118,
                ExecDocStatus::Signed,
            ),
            (
                ExecDocKind::Hidden,
                "AOSR-015",
                "Poydevor gidroizolyatsiyasi",
                "Гидроизоляция фундамента",
                "5",
                112,
                ExecDocStatus::Signed,
            ),
            (
                ExecDocKind::Test,
                "PR-004",
                "Beton mustahkamligi sinovi, poydevor",
                "Испытание прочности бетона, фундамент",
                "5",
                108,
                ExecDocStatus::Signed,
            ),
            (
                ExecDocKind::Hidden,
                "AOSR-016",
                "2-qavat ustun armaturasi",
                "Арматура колонн 2 этажа",
                "6",
                96,
                ExecDocStatus::Signed,
            ),
            (
                ExecDocKind::Acceptance,
                "AKT-007",
                "2-qavat oralig'ini qabul qilish",
                "Приёмка перекрытия 2 этажа",
                "6",
                88,
                ExecDocStatus::Signed,
            ),
            (
                ExecDocKind::Hidden,
                "AOSR-017",
                "3-qavat ustun armaturasi",
                "Арматура колонн 3 этажа",
                "6",
                74,
                ExecDocStatus::Signed,
            ),
            (
                ExecDocKind::Passport,
                "PS-011",
                "Beton B25 sifat pasporti",
                "Паспорт качества бетона B25",
                "7",
                66,
                ExecDocStatus::Signed,
            ),
            (
                ExecDocKind::Hidden,
                "AOSR-018",
                "4-qavat ustun armaturasi",
                "Арматура колонн 4 этажа",
                "7",
                52,
                ExecDocStatus::Signed,
            ),
            (
                ExecDocKind::Scheme,
                "IS-003",
                "4-qavat ijro syomkasi",
                "Исполнительная съёмка 4 этажа",
                "7",
                44,
                ExecDocStatus::Rejected,
            ),
            (
                ExecDocKind::Hidden,
                "AOSR-019",
                "5-qavat ustun armaturasi",
                "Арматура колонн 5 этажа",
                "8",
                32,
                ExecDocStatus::Signed,
            ),
            (
                ExecDocKind::Acceptance,
                "AKT-008",
                "5-qavat oralig'ini qabul qilish",
                "Приёмка перекрытия 5 этажа",
                "8",
                18,
                ExecDocStatus::OnReview,
            ),
            (
                ExecDocKind::Hidden,
                "AOSR-020",
                "6-qavat ustun armaturasi",
                "Арматура колонн 6 этажа",
                "10",
                4,
                ExecDocStatus::Draft,
            ),
        ];
        for (kind, number, uz, rux, wbs, back, status) in docs {
            self.insert_exec_doc(&ExecDoc {
                id: 0,
                project_id: pid,
                kind,
                number: number.into(),
                name: if ru { rux } else { uz }.into(),
                date: d(back),
                task_id: by_wbs(wbs),
                status,
                responsible: if ru {
                    "Собиров Р.Х."
                } else {
                    "Sobirov R.X."
                }
                .into(),
                version: 1,
                replaces: None,
                note: String::new(),
            });
        }

        // ---------- Sifat nazorati ----------
        // (tur, kun oldin, mavzu uz/ru, natija, nuqson uz/ru, muddat, yopilgan)
        let checks: [QualityDef; 11] = [
            (
                QualityKind::Input,
                104,
                "Sement partiyasi M400",
                "Партия цемента М400",
                QualityResult::Pass,
                "",
                "",
                None,
                None,
            ),
            (
                QualityKind::Operational,
                92,
                "Poydevor armaturasi qoplamasi",
                "Защитный слой арматуры фундамента",
                QualityResult::Conditional,
                "Ayrim joyda qoplama 22 mm",
                "Местами слой 22 мм",
                Some(85),
                Some(86),
            ),
            (
                QualityKind::Acceptance,
                78,
                "2-qavat oralig'i sirti",
                "Поверхность перекрытия 2 этажа",
                QualityResult::Pass,
                "",
                "",
                None,
                None,
            ),
            (
                QualityKind::Input,
                66,
                "G'isht partiyasi, 40 ming dona",
                "Партия кирпича, 40 тыс. шт.",
                QualityResult::Conditional,
                "Partiyaning 8 % i chetlangan, saralab qabul qilindi",
                "8 % партии с отколами, принято с отбраковкой",
                Some(58),
                Some(57),
            ),
            (
                QualityKind::Operational,
                54,
                "3-qavat ustunlari vertikalligi",
                "Вертикальность колонн 3 этажа",
                QualityResult::Pass,
                "",
                "",
                None,
                None,
            ),
            (
                QualityKind::Input,
                42,
                "Armatura A500S, 12 t",
                "Арматура А500С, 12 т",
                QualityResult::Pass,
                "",
                "",
                None,
                None,
            ),
            (
                QualityKind::Operational,
                34,
                "4-qavat oralig'i qalinligi",
                "Толщина перекрытия 4 этажа",
                QualityResult::Conditional,
                "Ikki joyda 8 mm og'ish",
                "В двух местах отклонение 8 мм",
                Some(27),
                Some(25),
            ),
            (
                QualityKind::Acceptance,
                22,
                "G'ishtin devor terimi, 4-qavat",
                "Кирпичная кладка, 4 этаж",
                QualityResult::Fail,
                "Chok qalinligi bir xil emas",
                "Неравномерная толщина шва",
                Some(15),
                None,
            ),
            (
                QualityKind::Input,
                16,
                "Issiqlik izolyatsiyasi, 40 m3",
                "Теплоизоляция, 40 м3",
                QualityResult::Pass,
                "",
                "",
                None,
                None,
            ),
            (
                QualityKind::Operational,
                9,
                "5-qavat oralig'i sirti",
                "Поверхность перекрытия 5 этажа",
                QualityResult::Conditional,
                "Sirt tekisligi dopusk chegarasida",
                "Ровность на границе допуска",
                Some(4),
                Some(2),
            ),
            (
                QualityKind::Input,
                3,
                "Beton B30, 60 m3",
                "Бетон B30, 60 м3",
                QualityResult::Pass,
                "",
                "",
                None,
                None,
            ),
        ];
        let inspector = if ru {
            "Назарова Г.А."
        } else {
            "Nazarova G.A."
        };
        for (kind, back, uz, rux, result, def_uz, def_ru, deadline, fixed) in checks {
            self.insert_quality(&QualityCheck {
                id: 0,
                project_id: pid,
                kind,
                date: d(back),
                task_id: None,
                material_id: None,
                subject: if ru { rux } else { uz }.into(),
                inspector: inspector.into(),
                result,
                defect: if ru { def_ru } else { def_uz }.into(),
                deadline: deadline.map(d),
                checklist_id: None,
                fixed_at: fixed.map(d),
                note: String::new(),
            });
        }

        // ---------- Mehnat xavfsizligi ----------
        // (tur, og'irlik, kun oldin, joy uz/ru, tavsif uz/ru, chora uz/ru, holat)
        let events: [SafetyDef; 10] = [
            (
                SafetyKind::Training,
                Severity::Info,
                112,
                "Uchastka",
                "Участок",
                "Kirish instruktaji, 12 ishchi",
                "Вводный инструктаж, 12 рабочих",
                "",
                "",
                IssueStatus::Fixed,
                RootCause::Unknown,
            ),
            (
                SafetyKind::Inspection,
                Severity::Info,
                96,
                "Butun obyekt",
                "Весь объект",
                "Rejali tekshiruv",
                "Плановая проверка",
                "",
                "",
                IssueStatus::Fixed,
                RootCause::Unknown,
            ),
            (
                SafetyKind::Training,
                Severity::Info,
                84,
                "3-qavat",
                "3 этаж",
                "Takroriy instruktaj, 18 ishchi",
                "Повторный инструктаж, 18 рабочих",
                "",
                "",
                IssueStatus::Fixed,
                RootCause::NoTraining,
            ),
            (
                SafetyKind::NearMiss,
                Severity::Major,
                68,
                "Kran zonasi",
                "Зона крана",
                "Yuk yo'lida odam bo'lgan",
                "Человек в зоне перемещения груза",
                "Zona to'sildi, signalchi tayinlandi",
                "Зона ограждена, назначен сигнальщик",
                IssueStatus::Fixed,
                RootCause::NoBarrier,
            ),
            (
                SafetyKind::Training,
                Severity::Info,
                60,
                "Uchastka",
                "Участок",
                "Balandlikda ishlash bo'yicha instruktaj",
                "Инструктаж по работе на высоте",
                "",
                "",
                IssueStatus::Fixed,
                RootCause::Unknown,
            ),
            (
                SafetyKind::Inspection,
                Severity::Info,
                44,
                "5-qavat",
                "5 этаж",
                "Chekka to'siqlarini tekshirish",
                "Проверка ограждений по краю",
                "Kamchilik topilmadi",
                "Замечаний не выявлено",
                IssueStatus::Fixed,
                RootCause::Unknown,
            ),
            (
                SafetyKind::Inspection,
                Severity::Info,
                30,
                "Butun obyekt",
                "Весь объект",
                "Naryad-dopusklar tekshiruvi",
                "Проверка нарядов-допусков",
                "",
                "",
                IssueStatus::Fixed,
                RootCause::Unknown,
            ),
            (
                SafetyKind::Violation,
                Severity::Warning,
                17,
                "Ombor",
                "Склад",
                "Yong'in o'chirgichning muddati o'tgan",
                "Просрочен огнетушитель",
                "Yangisiga almashtirildi",
                "Заменён на новый",
                IssueStatus::Fixed,
                RootCause::Organisation,
            ),
            (
                SafetyKind::NearMiss,
                Severity::Warning,
                8,
                "2-qavat",
                "2 этаж",
                "Vaqtinchalik kabel yo'lakda yotgan",
                "Временный кабель лежал в проходе",
                "Kabel osildi",
                "Кабель подвешен",
                IssueStatus::Fixed,
                RootCause::NoBarrier,
            ),
            (
                SafetyKind::Violation,
                Severity::Major,
                2,
                "6-qavat",
                "6 этаж",
                "Payvandchi ko'zoynaksiz ishlagan",
                "Сварщик работал без щитка",
                "Ish to'xtatildi, SIZ berildi",
                "Работы остановлены, выданы СИЗ",
                IssueStatus::Open,
                RootCause::NoPpe,
            ),
        ];
        for (kind, severity, back, pl_uz, pl_ru, de_uz, de_ru, me_uz, me_ru, status, cause) in
            events
        {
            self.insert_safety(&SafetyEvent {
                id: 0,
                project_id: pid,
                date: d(back),
                kind,
                severity,
                place: if ru { pl_ru } else { pl_uz }.into(),
                description: if ru { de_ru } else { de_uz }.into(),
                responsible: if ru {
                    "Юсупов Б.Р."
                } else {
                    "Yusupov B.R."
                }
                .into(),
                measure: if ru { me_ru } else { me_uz }.into(),
                deadline: (status != IssueStatus::Fixed).then(|| d(back - 10)),
                status,
                root_cause: cause,
            });
        }
    }

    /// Ta'minot va resurslar tarixi: ishchilar, materiallar, arizalar, xaridlar.
    ///
    /// Namunadagi ro'yxatlar mijozga ko'rsatish uchun yetarli bo'lishi kerak:
    /// bir necha qatorli jadval reytinglar, byudjet kesimlari va narx
    /// tarixining ma'nosini ochib bermaydi.
    pub fn seed_demo_supply_extra(&self, pid: i64, ru: bool) {
        // Arizalar to'rttadan ko'p bo'lsa — bu qism allaqachon to'ldirilgan.
        if self.requests(pid).len() > 4 {
            return;
        }
        let today = chrono::Local::now().date_naive();
        let d = |back: i64| today - chrono::Duration::days(back);
        let brigades = self.brigades(pid);
        let materials = self.materials(pid);
        let by_code = |c: &str| materials.iter().find(|m| m.code == c).map(|m| m.id);
        let tasks = self.tasks(pid).unwrap_or_default();
        let by_wbs = |w: &str| tasks.iter().find(|t| t.wbs == w).map(|t| t.id);

        // ---------- Ishchilar ----------
        // (ism uz/ru, lavozim uz/ru, tashkilot, soatlik stavka, brigada)
        let crew: [WorkerDef; 9] = [
            (
                "Sultonov J.B.",
                "Султонов Ж.Б.",
                "Beton quyuvchi",
                "Бетонщик",
                31_000.0,
                0,
            ),
            (
                "Qodirov M.T.",
                "Кодиров М.Т.",
                "Armaturachi",
                "Арматурщик",
                33_000.0,
                0,
            ),
            (
                "Ismoilov S.A.",
                "Исмоилов С.А.",
                "G'isht teruvchi",
                "Каменщик",
                30_000.0,
                1,
            ),
            (
                "Rustamov A.N.",
                "Рустамов А.Н.",
                "G'isht teruvchi",
                "Каменщик",
                29_000.0,
                1,
            ),
            (
                "Toshpo'latov E.R.",
                "Ташпулатов Э.Р.",
                "Payvandchi",
                "Сварщик",
                38_000.0,
                0,
            ),
            (
                "Ergashev N.M.",
                "Эргашев Н.М.",
                "Suvoqchi",
                "Штукатур",
                28_000.0,
                1,
            ),
            (
                "Abdullayev K.S.",
                "Абдуллаев К.С.",
                "Santexnik",
                "Сантехник",
                34_000.0,
                1,
            ),
            (
                "Yo'ldoshev B.T.",
                "Юлдашев Б.Т.",
                "Kranchi",
                "Крановщик",
                42_000.0,
                0,
            ),
            (
                "Mahmudov O'.A.",
                "Махмудов У.А.",
                "Yordamchi ishchi",
                "Разнорабочий",
                24_000.0,
                0,
            ),
        ];
        let org = if ru { "СМУ-7" } else { "QMB-7" };
        for (i, (uz, rux, pos_uz, pos_ru, rate, br)) in crew.iter().enumerate() {
            let wid = self.insert_worker(&Worker {
                id: 0,
                project_id: pid,
                name: if ru { *rux } else { *uz }.into(),
                position: if ru { *pos_ru } else { *pos_uz }.into(),
                org: org.into(),
                hourly_rate: *rate,
                active: true,
                brigade_id: brigades.get(*br).map(|b| b.id),
            });

            // Har bir ishchida kirish instruktaji va majburiy SIZ bo'lishi
            // kerak — usiz uni ishga qo'yib bo'lmaydi. Ularsiz qo'shilgan
            // ishchi xavfsizlik ballini asossiz tushirardi.
            self.insert_worker_permit(&WorkerPermit {
                id: 0,
                project_id: pid,
                worker_id: wid,
                kind: PermitKind::Induction,
                number: format!("IN-{:04}", 200 + i),
                issued: d(150),
                valid_until: d(-215),
                note: String::new(),
            });
            self.insert_worker_permit(&WorkerPermit {
                id: 0,
                project_id: pid,
                worker_id: wid,
                kind: PermitKind::Medical,
                number: format!("MD-{:04}", 300 + i),
                issued: d(140),
                valid_until: d(-225),
                note: String::new(),
            });
            // Balandlik ruxsati hammaga kerak emas — faqat shu ishni
            // bajaradiganlarga.
            if matches!(i, 2..=4) {
                self.insert_worker_permit(&WorkerPermit {
                    id: 0,
                    project_id: pid,
                    worker_id: wid,
                    kind: PermitKind::Height,
                    number: format!("HT-{:04}", 400 + i),
                    issued: d(120),
                    valid_until: d(-245),
                    note: String::new(),
                });
            }
            for item in PpeItem::REQUIRED {
                self.insert_ppe_issue(&PpeIssue {
                    id: 0,
                    project_id: pid,
                    worker_id: wid,
                    item: *item,
                    issued: d(150),
                    months: 12,
                    note: String::new(),
                });
            }
        }

        // ---------- Materiallar ----------
        // (kod, nomi uz/ru, birlik, bo'lim, minimal zaxira, narx, sertifikat)
        let mats: [MaterialDef; 11] = [
            (
                "M-104",
                "Beton B30 F150",
                "Бетон B30 F150",
                "m3",
                Section::Kj,
                20.0,
                780_000.0,
                "SF-2026/118",
            ),
            (
                "M-105",
                "Qum, yuvilgan",
                "Песок мытый",
                "m3",
                Section::Kj,
                40.0,
                96_000.0,
                "SF-2026/094",
            ),
            (
                "M-106",
                "Shag'al 5-20 mm",
                "Щебень 5-20 мм",
                "m3",
                Section::Kj,
                40.0,
                132_000.0,
                "SF-2026/095",
            ),
            (
                "M-107",
                "Sement M400",
                "Цемент М400",
                "t",
                Section::Kj,
                8.0,
                1_240_000.0,
                "SF-2026/101",
            ),
            (
                "M-203",
                "Armatura A500S d14",
                "Арматура А500С d14",
                "t",
                Section::Kj,
                5.0,
                8_900_000.0,
                "SF-2026/077",
            ),
            (
                "M-204",
                "Payvandlangan to'r 100x100",
                "Сетка сварная 100х100",
                "m2",
                Section::Kj,
                200.0,
                34_000.0,
                "SF-2026/079",
            ),
            (
                "M-305",
                "Issiqlik izolyatsiyasi, 100 mm",
                "Теплоизоляция 100 мм",
                "m3",
                Section::Ar,
                15.0,
                620_000.0,
                "SF-2026/133",
            ),
            (
                "M-306",
                "Gipskarton 12,5 mm",
                "Гипсокартон 12,5 мм",
                "m2",
                Section::Ar,
                300.0,
                42_000.0,
                "",
            ),
            (
                "M-403",
                "PP quvur d32",
                "Труба ПП d32",
                "m",
                Section::Vk,
                150.0,
                18_000.0,
                "SF-2026/142",
            ),
            (
                "M-501",
                "Kabel VVGng 3x2,5",
                "Кабель ВВГнг 3х2,5",
                "m",
                Section::Eom,
                400.0,
                21_000.0,
                "SF-2026/151",
            ),
            (
                "M-502",
                "Avtomat 25A",
                "Автомат 25А",
                "dona",
                Section::Eom,
                30.0,
                78_000.0,
                "",
            ),
        ];
        for (code, uz, rux, unit, section, min_stock, price, cert) in mats {
            self.insert_material(&Material {
                id: 0,
                project_id: pid,
                code: code.into(),
                name: if ru { rux } else { uz }.into(),
                unit: unit.into(),
                section,
                spec: String::new(),
                cert_no: cert.into(),
                // Sertifikatsiz material ham bor — ekranda u ajratib ko'rsatiladi.
                cert_until: (!cert.is_empty()).then(|| d(-180)),
                min_stock,
                price,
                estimate_code: String::new(),
                spec_ref: String::new(),
                special: String::new(),
                banned: false,
                ban_reason: String::new(),
                note: String::new(),
            });
        }

        // ---------- Arizalar ----------
        // (raqam, kun oldin, tur, mavzu uz/ru, material kodi, miqdor, birlik,
        //  muhimlik, holat, VBS, rad sababi uz/ru)
        let reqs: [RequestDef; 10] = [
            (
                "A-0031",
                62,
                RequestKind::Material,
                "Beton B25, 4-qavat oralig'i",
                "Бетон B25, перекрытие 4 этажа",
                "M-101",
                80.0,
                "m3",
                Priority::High,
                RequestStatus::Closed,
                "7",
                "",
                "",
            ),
            (
                "A-0032",
                54,
                RequestKind::Material,
                "Armatura A500S d12",
                "Арматура А500С d12",
                "M-201",
                6.0,
                "t",
                Priority::Normal,
                RequestStatus::Closed,
                "7",
                "",
                "",
            ),
            (
                "A-0033",
                41,
                RequestKind::Machine,
                "Avtokran 25 t, 3 kun",
                "Автокран 25 т, 3 дня",
                "",
                3.0,
                "smena",
                Priority::High,
                RequestStatus::Closed,
                "8",
                "",
                "",
            ),
            (
                "A-0034",
                33,
                RequestKind::Material,
                "G'isht, 5-qavat devorlari",
                "Кирпич, стены 5 этажа",
                "M-301",
                24_000.0,
                "dona",
                Priority::Normal,
                RequestStatus::Delivered,
                "9",
                "",
                "",
            ),
            (
                "A-0035",
                26,
                RequestKind::Labor,
                "Qo'shimcha 4 ta g'isht teruvchi",
                "Дополнительно 4 каменщика",
                "",
                4.0,
                "kishi",
                Priority::Normal,
                RequestStatus::Rejected,
                "9",
                "Byudjetda o'rin yo'q, muddat qayta ko'rildi",
                "Нет бюджета, срок пересмотрен",
            ),
            (
                "A-0036",
                19,
                RequestKind::Material,
                "Issiqlik izolyatsiyasi, fasad",
                "Теплоизоляция, фасад",
                "M-305",
                40.0,
                "m3",
                Priority::Normal,
                RequestStatus::InPurchase,
                "12",
                "",
                "",
            ),
            (
                "A-0037",
                14,
                RequestKind::Document,
                "Yangilangan KJ chizmalari",
                "Обновлённые чертежи КЖ",
                "",
                1.0,
                "komplekt",
                Priority::High,
                RequestStatus::Approved,
                "",
                "",
                "",
            ),
            (
                "A-0038",
                9,
                RequestKind::Material,
                "Kabel VVGng 3x2,5",
                "Кабель ВВГнг 3х2,5",
                "M-501",
                1_200.0,
                "m",
                Priority::Normal,
                RequestStatus::Approved,
                "13",
                "",
                "",
            ),
            (
                "A-0039",
                5,
                RequestKind::Material,
                "Beton B30, 6-qavat ustunlari",
                "Бетон B30, колонны 6 этажа",
                "M-104",
                60.0,
                "m3",
                Priority::Urgent,
                RequestStatus::New,
                "10",
                "",
                "",
            ),
            (
                "A-0040",
                2,
                RequestKind::Machine,
                "Beton nasosi, 2 smena",
                "Бетононасос, 2 смены",
                "",
                2.0,
                "smena",
                Priority::High,
                RequestStatus::New,
                "10",
                "",
                "",
            ),
        ];
        let requester = if ru {
            "Юсупов Б.Р."
        } else {
            "Yusupov B.R."
        };
        let mut req_ids: Vec<(String, i64)> = Vec::new();
        for (number, back, kind, uz, rux, code, qty, unit, priority, status, wbs, rj_uz, rj_ru) in
            reqs
        {
            let id = self.insert_request(&Request {
                id: 0,
                project_id: pid,
                number: number.into(),
                date: d(back),
                kind,
                title: if ru { rux } else { uz }.into(),
                material_id: by_code(code),
                qty,
                unit: unit.into(),
                requester: requester.into(),
                need_date: d(back - 12),
                priority,
                status,
                task_id: by_wbs(wbs),
                reject_reason: if ru { rj_ru } else { rj_uz }.into(),
                note: String::new(),
            });
            req_ids.push((number.to_string(), id));
        }

        // ---------- Xaridlar ----------
        // (raqam, ariza, kun oldin, yetkazib beruvchi, mavzu uz/ru, miqdor, birlik,
        //  narx, yetkazish kuni, holat, kelgan miqdor, bo'lim)
        let buys: [PurchaseDef; 8] = [
            (
                "X-0044",
                "A-0031",
                60,
                "Toshkent Beton",
                "Beton B25",
                "Бетон B25",
                80.0,
                "m3",
                720_000.0,
                56,
                PurchaseStatus::Closed,
                80.0,
                Section::Kj,
            ),
            (
                "X-0045",
                "A-0032",
                52,
                "Uzmetkombinat",
                "Armatura A500S d12",
                "Арматура А500С d12",
                6.0,
                "t",
                8_600_000.0,
                46,
                PurchaseStatus::Closed,
                6.0,
                Section::Kj,
            ),
            (
                "X-0046",
                "A-0033",
                40,
                "Kran-Servis",
                "Avtokran 25 t ijarasi",
                "Аренда автокрана 25 т",
                3.0,
                "smena",
                2_400_000.0,
                37,
                PurchaseStatus::Closed,
                3.0,
                Section::None,
            ),
            (
                "X-0047",
                "A-0034",
                32,
                "G'isht zavodi №3",
                "Qizil g'isht M150",
                "Кирпич красный М150",
                24_000.0,
                "dona",
                1_450.0,
                24,
                PurchaseStatus::Delivered,
                24_000.0,
                Section::Ar,
            ),
            (
                "X-0048",
                "A-0036",
                18,
                "IzolyatsiyaPro",
                "Issiqlik izolyatsiyasi 100 mm",
                "Теплоизоляция 100 мм",
                40.0,
                "m3",
                610_000.0,
                6,
                PurchaseStatus::Paid,
                24.0,
                Section::Ar,
            ),
            (
                "X-0049",
                "A-0038",
                8,
                "ElektroSnab",
                "Kabel VVGng 3x2,5",
                "Кабель ВВГнг 3х2,5",
                1_200.0,
                "m",
                20_500.0,
                -3,
                PurchaseStatus::Ordered,
                0.0,
                Section::Eom,
            ),
            (
                "X-0050",
                "",
                6,
                "Toshkent Beton",
                "Beton B30",
                "Бетон B30",
                60.0,
                "m3",
                770_000.0,
                -1,
                PurchaseStatus::Ordered,
                0.0,
                Section::Kj,
            ),
            (
                "X-0051",
                "",
                3,
                "SantexTrade",
                "PP quvur d32",
                "Труба ПП d32",
                800.0,
                "m",
                17_500.0,
                -8,
                PurchaseStatus::Draft,
                0.0,
                Section::Vk,
            ),
        ];
        for (
            number,
            req,
            back,
            supplier,
            uz,
            rux,
            qty,
            unit,
            price,
            deliver,
            status,
            got,
            section,
        ) in buys
        {
            self.insert_purchase(&Purchase {
                id: 0,
                project_id: pid,
                request_id: req_ids.iter().find(|(n, _)| n == req).map(|(_, id)| *id),
                number: number.into(),
                date: d(back),
                supplier: supplier.into(),
                title: if ru { rux } else { uz }.into(),
                qty,
                unit: unit.into(),
                price,
                currency: "UZS".into(),
                delivery_date: d(deliver),
                status,
                delivered_qty: got,
                section,
                // Xarid arizadan keladi, ariza esa aniq ishga bog'langan —
                // shuning uchun ishni ariza orqali topamiz.
                task_id: reqs.iter().find(|r| r.0 == req).and_then(|r| by_wbs(r.10)),
                contract_id: None,
                // Shoshilinch xaridlar — arizasiz kelganlari: ular odatda
                // rejadan tashqarida paydo bo'ladi.
                urgent: req.is_empty(),
                buyer: if ru { "Ким В.С." } else { "Kim V.S." }.into(),
                material_id: None,
                substitute_for: None,
                // Arizadan kelgan xaridlar texnik kelishuvdan o'tgan;
                // shoshilinch olinganlari esa yo'q — nazorat ekranida
                // aynan shu holat ko'rinishi kerak (TZ X.18).
                tech_ok: !req.is_empty(),
                tech_by: if req.is_empty() {
                    String::new()
                } else if ru {
                    "Собиров Р.Х.".into()
                } else {
                    "Sobirov R.X.".to_string()
                },
                // To'lov holati namunada har xil: to'liq to'langan,
                // qisman to'langan va muddati o'tgan qarz — to'lov
                // intizomi ekrani bo'sh ko'rinmasin (TZ X.23).
                paid: match status {
                    PurchaseStatus::Paid | PurchaseStatus::Delivered => qty * price,
                    PurchaseStatus::Ordered => qty * price * 0.3,
                    _ => 0.0,
                },
                pay_due: match status {
                    PurchaseStatus::Ordered => Some(d(back) + chrono::Duration::days(10)),
                    PurchaseStatus::Draft => None,
                    _ => Some(d(back) + chrono::Duration::days(20)),
                },
                note: String::new(),
            });
        }
    }

    /// Asboblar va ularni berish namunasi (TZ XI.32-34).
    pub fn seed_demo_tools(&self, pid: i64, ru: bool) {
        if !self.tools(pid).is_empty() {
            return;
        }
        let today = chrono::Local::now().date_naive();
        let d = |back: i64| today - chrono::Duration::days(back);
        let workers = self.workers(pid);
        if workers.is_empty() {
            return;
        }

        // (kod, nomi uz/ru, tur, inventar №, narx, holat, tekshiruv kuni)
        let tools: [ToolDef; 10] = [
            (
                "AS-101",
                "Perforator Bosch GBH 2-26",
                "Перфоратор Bosch GBH 2-26",
                ToolKind::Power,
                "INV-1042",
                4_800_000.0,
                ToolCondition::Good,
                Some(-120),
            ),
            (
                "AS-102",
                "Burchak silliqlagich 230 mm",
                "УШМ 230 мм",
                ToolKind::Power,
                "INV-1043",
                2_600_000.0,
                ToolCondition::Good,
                Some(-95),
            ),
            (
                "AS-103",
                "Payvand apparati 200 A",
                "Сварочный аппарат 200 А",
                ToolKind::Power,
                "INV-1051",
                6_400_000.0,
                ToolCondition::Worn,
                Some(18),
            ),
            (
                "AS-104",
                "Vibrator, chuqurlik",
                "Вибратор глубинный",
                ToolKind::Power,
                "INV-1063",
                3_900_000.0,
                ToolCondition::Repair,
                None,
            ),
            (
                "AS-201",
                "Lazerli nivelir",
                "Лазерный нивелир",
                ToolKind::Measure,
                "INV-2011",
                5_200_000.0,
                ToolCondition::Good,
                Some(-210),
            ),
            (
                "AS-202",
                "Teodolit",
                "Теодолит",
                ToolKind::Measure,
                "INV-2014",
                12_400_000.0,
                ToolCondition::Good,
                Some(-45),
            ),
            (
                "AS-203",
                "Ruletka 50 m",
                "Рулетка 50 м",
                ToolKind::Hand,
                "INV-2020",
                320_000.0,
                ToolCondition::Good,
                None,
            ),
            (
                "AS-301",
                "Beton aralashtirgich 130 l",
                "Бетономешалка 130 л",
                ToolKind::Power,
                "INV-3001",
                4_100_000.0,
                ToolCondition::Good,
                None,
            ),
            (
                "AS-401",
                "Iskala, seksiya",
                "Леса, секция",
                ToolKind::Scaffold,
                "INV-4010",
                1_800_000.0,
                ToolCondition::Good,
                None,
            ),
            (
                "AS-402",
                "Ko'chma zinapoya 6 m",
                "Лестница приставная 6 м",
                ToolKind::Scaffold,
                "INV-4022",
                950_000.0,
                ToolCondition::Worn,
                None,
            ),
        ];
        let mut ids = Vec::new();
        for (code, uz, rux, kind, inv, price, condition, check) in tools {
            ids.push(self.insert_tool(&Tool {
                id: 0,
                project_id: pid,
                code: code.into(),
                name: if ru { rux } else { uz }.into(),
                kind,
                inventory_no: inv.into(),
                price,
                condition,
                check_due: check.map(d),
                note: String::new(),
            }));
        }

        // Berish: bir qismi ishchilarda, bittasi muddati o'tgan holda.
        // (asbob indeksi, ishchi indeksi, berilgan kun, muddat kuni, qaytarilgan)
        let issues: [ToolIssueDef; 6] = [
            (0, 0, 42, Some(28), Some(30)),
            (0, 1, 12, Some(-2), None),
            (1, 2, 9, Some(5), None),
            (2, 4, 26, Some(12), None),
            (4, 3, 60, Some(46), Some(48)),
            (6, 5, 3, Some(-11), None),
        ];
        for (ti, wi, back, due, ret) in issues {
            let (Some(tool), Some(worker)) = (ids.get(ti), workers.get(wi)) else {
                continue;
            };
            self.insert_tool_issue(&ToolIssue {
                id: 0,
                project_id: pid,
                tool_id: *tool,
                worker_id: worker.id,
                issued: d(back),
                due: due.map(d),
                returned: ret.map(d),
                note: String::new(),
            });
        }
    }

    /// Laboratoriya va maydon sinovlari namunasi (TZ XIV.22, 24-25).
    pub fn seed_demo_lab(&self, pid: i64, ru: bool) {
        if !self.lab_tests(pid).is_empty() {
            return;
        }
        let today = chrono::Local::now().date_naive();
        let d = |back: i64| today - chrono::Duration::days(back);
        let tasks = self.tasks(pid).unwrap_or_default();
        let by_wbs = |w: &str| tasks.iter().find(|t| t.wbs == w).map(|t| t.id);

        // (tur, raqam, mavzu uz/ru, kun oldin, qiymat, talab, birlik, natija, VBS)
        let tests: [LabTestDef; 8] = [
            (
                LabTestKind::Weld,
                "SV-014",
                "Ferma tayanchi choklari",
                "Швы опирания фермы",
                74,
                Some(100.0),
                Some(95.0),
                "%",
                LabTestResult::Pass,
                "8",
            ),
            (
                LabTestKind::Weld,
                "SV-015",
                "Zakladnoy detallar choklari",
                "Швы закладных деталей",
                52,
                Some(88.0),
                Some(95.0),
                "%",
                LabTestResult::Fail,
                "8",
            ),
            (
                LabTestKind::Pressure,
                "OP-007",
                "Isitish tizimi, 1-qavat",
                "Система отопления, 1 этаж",
                38,
                Some(6.0),
                Some(6.0),
                "bar",
                LabTestResult::Pass,
                "11",
            ),
            (
                LabTestKind::Pressure,
                "OP-008",
                "Suv quvuri, ko'tarma",
                "Водопровод, стояк",
                24,
                Some(9.0),
                Some(10.0),
                "bar",
                LabTestResult::Fail,
                "11",
            ),
            (
                LabTestKind::Insulation,
                "IZ-003",
                "Kabel liniyasi, shchit",
                "Кабельная линия, щитовая",
                18,
                Some(52.0),
                Some(0.5),
                "MOm",
                LabTestResult::Pass,
                "13",
            ),
            (
                LabTestKind::Soil,
                "GR-002",
                "Zichlash koeffitsiyenti",
                "Коэффициент уплотнения",
                96,
                Some(0.97),
                Some(0.95),
                "",
                LabTestResult::Pass,
                "2",
            ),
            (
                LabTestKind::Commission,
                "PN-001",
                "Shamollatish, ishga tushirish",
                "Вентиляция, пусконаладка",
                6,
                None,
                None,
                "",
                LabTestResult::Waiting,
                "12",
            ),
            (
                LabTestKind::Weld,
                "SV-016",
                "Fasad karkasi choklari",
                "Швы фасадного каркаса",
                3,
                None,
                Some(95.0),
                "%",
                LabTestResult::Waiting,
                "12",
            ),
        ];
        for (kind, number, uz, rux, back, value, required, unit, result, wbs) in tests {
            self.insert_lab_test(&LabTest {
                id: 0,
                project_id: pid,
                task_id: by_wbs(wbs),
                kind,
                number: number.into(),
                subject: if ru { rux } else { uz }.into(),
                date: d(back),
                value,
                required,
                unit: unit.into(),
                result,
                lab: if ru {
                    "Стройлаборатория №4"
                } else {
                    "4-son qurilish laboratoriyasi"
                }
                .into(),
                // Salbiy natijadan keyin qayta sinov tayinlanadi.
                retest: (result == LabTestResult::Fail).then(|| d(back - 14)),
                note: String::new(),
            });
        }
    }

    /// Texnika bandligi va ta'miri namunasi (TZ XVI.10-12, 23-24).
    pub fn seed_demo_machine_plan(&self, pid: i64, ru: bool) {
        if !self.machine_bookings(pid).is_empty() {
            return;
        }
        let today = chrono::Local::now().date_naive();
        let d = |back: i64| today - chrono::Duration::days(back);
        let machines = self.machines(pid);
        if machines.is_empty() {
            return;
        }
        let tasks = self.tasks(pid).unwrap_or_default();
        let by_wbs = |w: &str| tasks.iter().find(|t| t.wbs == w).map(|t| t.id);

        // (texnika indeksi, VBS, boshlanish kuni, tugash kuni, smena)
        // Uchinchi va to'rtinchi bandlik ataylab kesishadi — to'qnashuv
        // ekranda ko'rinishi kerak.
        let plan: [(usize, &str, i64, i64, f64); 5] = [
            (0, "8", 30, 10, 2.0),
            (1, "9", 24, 6, 1.0),
            (0, "10", -2, -20, 2.0),
            (0, "12", -14, -30, 1.0),
            (2, "11", -6, -25, 1.0),
        ];
        for (mi, wbs, from, to, shifts) in plan {
            let Some(m) = machines.get(mi) else { continue };
            self.insert_machine_booking(&MachineBooking {
                id: 0,
                project_id: pid,
                machine_id: m.id,
                task_id: by_wbs(wbs),
                from: d(from),
                to: d(to),
                shifts,
                note: String::new(),
            });
        }

        // (texnika indeksi, tur, boshlangan kun, tugagan kun, sabab uz/ru, xarajat)
        let repairs: [MachineRepairDef; 5] = [
            (
                0,
                RepairKind::Service,
                96,
                Some(95),
                "Rejali TX-2",
                "Плановое ТО-2",
                3_200_000.0,
            ),
            (
                1,
                RepairKind::Fault,
                74,
                Some(70),
                "Gidravlika shlangi yorilgan",
                "Порыв гидравлического шланга",
                5_800_000.0,
            ),
            (
                1,
                RepairKind::Fault,
                41,
                Some(36),
                "Yurish qismi ta'miri",
                "Ремонт ходовой части",
                12_400_000.0,
            ),
            (
                2,
                RepairKind::Check,
                28,
                Some(28),
                "Texnik ko'rik",
                "Технический осмотр",
                900_000.0,
            ),
            (
                1,
                RepairKind::Fault,
                4,
                None,
                "Dvigatel ishlamayapti",
                "Не запускается двигатель",
                6_500_000.0,
            ),
        ];
        for (mi, kind, started, finished, uz, rux, cost) in repairs {
            let Some(m) = machines.get(mi) else { continue };
            self.insert_machine_repair(&MachineRepair {
                id: 0,
                project_id: pid,
                machine_id: m.id,
                kind,
                started: d(started),
                finished: finished.map(d),
                reason: if ru { rux } else { uz }.into(),
                cost,
                hours_at: 0.0,
                note: String::new(),
            });
        }
    }

    /// Xavfli zonalar va xavfsizlik inventari namunasi (TZ XV.15, 22-24).
    /// Kunlik ko'rik namunasi (TZ XVI.28).
    ///
    /// Bugungi kun ataylab to'liq to'ldirilmaydi: bitta texnika ko'rikdan
    /// o'tmagan holda qoladi — mexanik kabineti bo'sh ko'rinmasin va
    /// e'tiroz qanday ishlashi ko'rinib tursin.
    pub fn seed_demo_machine_checks(&self, pid: i64, ru: bool) {
        if !self.machine_checks(pid).is_empty() {
            return;
        }
        let today = chrono::Local::now().date_naive();
        let machines = self.machines(pid);
        let by = if ru { "Ким В.С." } else { "Kim V.S." };

        for (i, m) in machines.iter().enumerate() {
            // Oxirgi texnikani ataylab ko'riksiz qoldiramiz.
            if i + 1 == machines.len() && machines.len() > 1 {
                continue;
            }
            // Ikkinchisida nosozlik topilgan, lekin bartaraf etilgan.
            let fault = if i == 1 {
                if ru {
                    "Габаритный фонарь не горел — заменён"
                } else {
                    "Gabarit chirog'i yonmadi — almashtirildi"
                }
            } else {
                ""
            };
            self.insert_machine_check(&MachineCheck {
                id: 0,
                project_id: pid,
                machine_id: m.id,
                date: today,
                by: by.into(),
                items_ok: if fault.is_empty() { 6 } else { 5 },
                items_total: 6,
                fault: fault.into(),
                allowed: true,
                note: String::new(),
            });
        }
    }

    /// Namunaviy izohlar (umumiy «izoh va muhokama» mexanizmi).
    ///
    /// Fayl biriktirmalari namunaga qo'shilmaydi: yo'llar bu kompyuterda
    /// mavjud bo'lmaydi va ekranda «fayl topilmadi» bo'lib qizarardi —
    /// mijozga buzuq narsa ko'rsatishdan ko'ra bo'sh ro'yxat halolroq.
    pub fn seed_demo_notes(&self, pid: i64, ru: bool) {
        if !self.notes(pid).is_empty() {
            return;
        }
        let today = chrono::Local::now().date_naive();
        let at = |back: i64| format!("{} 10:20", today - chrono::Duration::days(back));

        let add = |target: NoteTarget,
                   target_id: i64,
                   author: &str,
                   back: i64,
                   text: &str,
                   parent: Option<i64>,
                   resolved: bool|
         -> i64 {
            self.insert_note(&Note {
                id: 0,
                project_id: pid,
                target,
                target_id,
                author: author.into(),
                at: at(back),
                text: text.into(),
                parent,
                resolved,
            })
        };

        // Sifat tekshiruvi bo'yicha muhokama: savol va unga javob.
        if let Some(q) = self
            .quality_checks(pid)
            .into_iter()
            .find(|q| !q.defect.trim().is_empty())
        {
            let root = add(
                NoteTarget::Quality,
                q.id,
                if ru {
                    "Собиров Р.Х."
                } else {
                    "Sobirov R.X."
                },
                6,
                if ru {
                    "Дефект устранён? Нужно фото после исправления."
                } else {
                    "Nuqson bartaraf etildimi? Tuzatishdan keyingi foto kerak."
                },
                None,
                false,
            );
            add(
                NoteTarget::Quality,
                q.id,
                if ru {
                    "Рахимов Ш.А."
                } else {
                    "Rahimov Sh.A."
                },
                4,
                if ru {
                    "Работы переделаны, фото приложу завтра."
                } else {
                    "Ish qayta bajarildi, fotoni ertaga biriktiraman."
                },
                Some(root),
                false,
            );
        }

        // Texnik nazorat izohi — hal qilingan holat.
        if let Some(i) = self.inspections(pid).into_iter().next() {
            let root = add(
                NoteTarget::Inspection,
                i.id,
                if ru {
                    "Юсупов Б.Р."
                } else {
                    "Yusupov B.R."
                },
                9,
                if ru {
                    "Отметки по осям 3-4 не совпадают с проектом."
                } else {
                    "3-4 o'qlar bo'yicha belgilar loyihaga mos kelmadi."
                },
                None,
                true,
            );
            add(
                NoteTarget::Inspection,
                i.id,
                if ru {
                    "Проектный институт"
                } else {
                    "Loyiha instituti"
                },
                7,
                if ru {
                    "Выпущено изменение, отметки уточнены."
                } else {
                    "O'zgartirish chiqarildi, belgilar aniqlashtirildi."
                },
                Some(root),
                true,
            );
        }

        // Xarid bo'yicha ochiq savol.
        if let Some(pu) = self.purchases(pid).into_iter().next() {
            add(
                NoteTarget::Purchase,
                pu.id,
                if ru {
                    "Каримов А.А."
                } else {
                    "Karimov A.A."
                },
                3,
                if ru {
                    "При приёмке приложите фото накладной и партии."
                } else {
                    "Qabulda hujjat va partiya fotosini biriktiring."
                },
                None,
                false,
            );
        }
    }

    pub fn seed_demo_zones(&self, pid: i64, ru: bool) {
        if !self.safety_zones(pid).is_empty() {
            return;
        }
        let today = chrono::Local::now().date_naive();
        let d = |back: i64| today - chrono::Duration::days(back);
        let boss = if ru {
            "Юсупов Б.Р."
        } else {
            "Yusupov B.R."
        };

        // (tur, nomi uz/ru, joy uz/ru, chora uz/ru, tekshiruv kuni, chora ko'rilgan)
        let zones: [ZoneDef; 9] = [
            (
                ZoneKind::Lifting,
                "Kran ish zonasi",
                "Зона работы крана",
                "A-D o'qlari",
                "Оси А-Д",
                "To'siq va signalchi",
                "Ограждение и сигнальщик",
                -14,
                true,
            ),
            (
                ZoneKind::Danger,
                "Chekka kontur, 6-qavat",
                "Край перекрытия, 6 этаж",
                "6-qavat",
                "6 этаж",
                "Muhofaza to'sig'i",
                "Защитное ограждение",
                -7,
                true,
            ),
            (
                ZoneKind::Excavation,
                "Kotlovan, shimoliy tomon",
                "Котлован, северная сторона",
                "Bosh reja",
                "Генплан",
                "To'siq va belgilar",
                "Ограждение и знаки",
                4,
                false,
            ),
            (
                ZoneKind::Electric,
                "Vaqtinchalik elektr shchiti",
                "Временный электрощит",
                "1-qavat",
                "1 этаж",
                "Qulf va ogohlantirish belgisi",
                "Замок и предупреждающий знак",
                -21,
                true,
            ),
            (
                ZoneKind::Fire,
                "Yong'in o'chirgichlar, 1-3 qavat",
                "Огнетушители, 1-3 этаж",
                "Zinapoyalar",
                "Лестничные клетки",
                "12 ta OP-5, tekshirilgan",
                "12 шт. ОП-5, проверены",
                -30,
                true,
            ),
            (
                ZoneKind::Fire,
                "Yong'in o'chirgichlar, 4-6 qavat",
                "Огнетушители, 4-6 этаж",
                "Zinapoyalar",
                "Лестничные клетки",
                "9 ta OP-5",
                "9 шт. ОП-5",
                12,
                true,
            ),
            (
                ZoneKind::Evacuation,
                "Evakuatsiya rejasi va belgilar",
                "План эвакуации и знаки",
                "Har qavatda",
                "На каждом этаже",
                "Rejalar osilgan, belgilar o'rnatilgan",
                "Планы вывешены, знаки установлены",
                -60,
                true,
            ),
            (
                ZoneKind::Evacuation,
                "Evakuatsiya yo'li, 5-qavat",
                "Путь эвакуации, 5 этаж",
                "5-qavat",
                "5 этаж",
                "Yo'l materialdan tozalanishi kerak",
                "Проход нужно освободить от материалов",
                2,
                false,
            ),
            (
                ZoneKind::Emergency,
                "Favqulodda vaziyat aloqasi",
                "Связь при ЧС",
                "Prorab vagonchasi",
                "Прорабская",
                "Telefonlar ro'yxati va aptechka",
                "Список телефонов и аптечка",
                -45,
                true,
            ),
        ];
        for (kind, uz, rux, pl_uz, pl_ru, m_uz, m_ru, check, ready) in zones {
            self.insert_safety_zone(&SafetyZone {
                id: 0,
                project_id: pid,
                kind,
                name: if ru { rux } else { uz }.into(),
                place: if ru { pl_ru } else { pl_uz }.into(),
                measure: if ru { m_ru } else { m_uz }.into(),
                responsible: boss.into(),
                check_due: Some(d(check)),
                checked_at: ready.then(|| d(check + 30)),
                ready,
                note: String::new(),
            });
        }
    }

    pub fn seed_demo_resources(&self, pid: i64, ru: bool) {
        if !self.workers(pid).is_empty() {
            return;
        }
        let today = chrono::Local::now().date_naive();
        let tasks = self.tasks(pid).unwrap_or_default();
        let by_wbs = |w: &str| tasks.iter().find(|t| t.wbs == w).map(|t| t.id);
        let materials = self.materials(pid);
        let by_code = |c: &str| materials.iter().find(|m| m.code == c).map(|m| m.id);

        // ---------- XIII. Ishchilar va tabel ----------
        let crew: [(&str, &str, f64); 6] = if ru {
            [
                ("Юсупов Б.Р.", "Прораб", 45_000.0),
                ("Каримов А.Т.", "Бетонщик", 32_000.0),
                ("Назаров Ш.И.", "Арматурщик", 32_000.0),
                ("Умаров Д.К.", "Каменщик", 30_000.0),
                ("Хасанов Ф.М.", "Электромонтажник", 35_000.0),
                ("Собиров Н.А.", "Разнорабочий", 22_000.0),
            ]
        } else {
            [
                ("Yusupov B.R.", "Prorab", 45_000.0),
                ("Karimov A.T.", "Betonchi", 32_000.0),
                ("Nazarov Sh.I.", "Armaturachi", 32_000.0),
                ("Umarov D.K.", "G'ishtchi", 30_000.0),
                ("Hasanov F.M.", "Elektromontajchi", 35_000.0),
                ("Sobirov N.A.", "Yordamchi ishchi", 22_000.0),
            ]
        };
        let org = if ru {
            "ООО «Навруз Курилиш»"
        } else {
            "«Navro'z Qurilish» MChJ"
        };
        // Ikki brigada: monolitchilar va pardozchilar (TZ XIII.8).
        let brigade = |name: &str, foreman: &str, task_id: Option<i64>| {
            self.insert_brigade(&Brigade {
                id: 0,
                project_id: pid,
                name: name.into(),
                foreman: foreman.into(),
                task_id,
                note: String::new(),
            })
        };
        let (b_monolit, b_finish) = if ru {
            (
                brigade("Бригада №1 — монолит", "Каримов А.Т.", by_wbs("7")),
                brigade("Бригада №2 — отделка", "Умаров Д.К.", by_wbs("9")),
            )
        } else {
            (
                brigade("1-brigada — monolit", "Karimov A.T.", by_wbs("7")),
                brigade("2-brigada — pardoz", "Umarov D.K.", by_wbs("9")),
            )
        };

        let mut ids = Vec::new();
        for (i, (name, position, rate)) in crew.into_iter().enumerate() {
            // Prorab brigadaga kirmaydi, qolganlari ikkiga bo'linadi.
            let brigade_id = match i {
                0 => None,
                1..=3 => Some(b_monolit),
                _ => Some(b_finish),
            };
            ids.push(self.insert_worker(&Worker {
                id: 0,
                project_id: pid,
                name: name.into(),
                position: position.into(),
                org: org.into(),
                hourly_rate: rate,
                active: true,
                brigade_id,
            }));
        }

        // Oxirgi ikki hafta: ish kunlari 8 soat, shanba 6, yakshanba dam.
        for back in 0..14 {
            let day = today - chrono::Duration::days(back);
            let wd = day.weekday().num_days_from_monday();
            if wd == 6 {
                continue;
            }
            let base = if wd == 5 { 6.0 } else { 8.0 };
            for (i, wid) in ids.iter().enumerate() {
                if *wid == 0 {
                    continue;
                }
                // Prorab har kuni, brigada esa navbat bilan biroz farqli ishlaydi.
                let hours = if i == 0 {
                    base
                } else if (back as usize + i).is_multiple_of(7) {
                    0.0
                } else if (back as usize + i).is_multiple_of(5) {
                    base + 2.0
                } else {
                    base
                };
                if hours > 0.0 {
                    self.set_timesheet(pid, *wid, day, hours);
                    // Brigada ishi tabelda ko'rinsin — tannarx shundan yig'iladi.
                    let task = match i {
                        0 => None,
                        1..=3 => by_wbs("7"),
                        _ => by_wbs("9"),
                    };
                    if task.is_some() {
                        self.set_timesheet_task(pid, *wid, day, task);
                    }
                    // Uchinchi ishchi kechqurun smenada ishlaydi (TZ XIII.11).
                    if i == 3 && back % 4 == 1 {
                        self.set_timesheet_shift(pid, *wid, day, Shift::Evening);
                    }
                }
            }
        }

        // Yo'qliklar va bo'sh turish (TZ XIII.16–22).
        let mark = |wid: Option<&i64>, days_ago: i64, kind: DayKind, hours: f64| {
            if let Some(wid) = wid {
                let day = today - chrono::Duration::days(days_ago);
                self.set_timesheet_kind(pid, *wid, day, kind);
                if hours > 0.0 {
                    self.set_timesheet(pid, *wid, day, hours);
                }
            }
        };
        // Beton kelmagani uchun brigada yarim kun bo'sh turdi.
        mark(ids.get(1), 6, DayKind::Downtime, 4.0);
        mark(ids.get(2), 6, DayKind::Downtime, 4.0);
        // Kasallik varaqasi va sababsiz yo'qlik.
        mark(ids.get(5), 4, DayKind::Sick, 0.0);
        mark(ids.get(5), 3, DayKind::Sick, 0.0);
        mark(ids.get(4), 2, DayKind::Absent, 0.0);

        // ---------- XIV. Sifat nazorati ----------
        let inspector = if ru {
            "ПТО: Саидова М.И."
        } else {
            "PTO: Saidova M.I."
        };
        let qc = |kind: QualityKind,
                  days_ago: i64,
                  subject: &str,
                  task_id: Option<i64>,
                  material_id: Option<i64>,
                  result: QualityResult,
                  defect: &str,
                  deadline_in: Option<i64>| {
            self.insert_quality(&QualityCheck {
                id: 0,
                project_id: pid,
                kind,
                date: today - chrono::Duration::days(days_ago),
                task_id,
                material_id,
                subject: subject.into(),
                inspector: inspector.into(),
                result,
                defect: defect.into(),
                deadline: deadline_in.map(|d| today + chrono::Duration::days(d)),
                checklist_id: None,
                fixed_at: None,
                note: String::new(),
            });
        };
        qc(
            QualityKind::Input,
            26,
            if ru {
                "Приемка кирпича М150"
            } else {
                "M150 g'ishtni qabul qilish"
            },
            None,
            by_code("M-201"),
            QualityResult::Pass,
            "",
            None,
        );
        // Sertifikati muddati o'tgan armatura — kirish nazoratidan o'tmadi.
        qc(
            QualityKind::Input,
            12,
            if ru {
                "Приемка арматуры А500С"
            } else {
                "A500S armaturani qabul qilish"
            },
            None,
            by_code("M-102"),
            QualityResult::Fail,
            if ru {
                "Сертификат просрочен, партия не допущена"
            } else {
                "Sertifikat muddati o'tgan, partiya qabul qilinmadi"
            },
            Some(-3),
        );
        qc(
            QualityKind::Operational,
            8,
            if ru {
                "Опалубка колонн 9 этажа"
            } else {
                "9-qavat ustunlari opalubkasi"
            },
            by_wbs("7"),
            None,
            QualityResult::Conditional,
            if ru {
                "Отклонение по вертикали 6 мм при допуске 5 мм"
            } else {
                "Vertikal bo'yicha 6 mm og'ish, ruxsat 5 mm"
            },
            // Muddati bugun tugaydi — kunlik xulosada ko'rinadi.
            Some(0),
        );
        qc(
            QualityKind::Acceptance,
            30,
            if ru {
                "Приемка фундаментной плиты"
            } else {
                "Poydevor plitasini qabul qilish"
            },
            by_wbs("3"),
            None,
            QualityResult::Pass,
            "",
            None,
        );
        qc(
            QualityKind::Operational,
            3,
            if ru {
                "Кладка наружных стен, оси А-В"
            } else {
                "Tashqi devor g'ishtligi, A-B o'qlari"
            },
            by_wbs("9"),
            None,
            QualityResult::Pass,
            "",
            None,
        );

        // Chek-list namunalari (TZ XIV.8): monolit ishlari va g'isht terish.
        let checklist =
            |name: &str, section: Section, kind: QualityKind, items: &[(&str, &str, &str)]| {
                let id = self.insert_checklist(&Checklist {
                    id: 0,
                    project_id: pid,
                    name: name.into(),
                    section,
                    kind,
                    note: String::new(),
                });
                for (n, (text, doc, clause)) in items.iter().enumerate() {
                    self.insert_checklist_item(&ChecklistItem {
                        id: 0,
                        checklist_id: id,
                        pos: n as i64 + 1,
                        text: (*text).into(),
                        norm_doc: (*doc).into(),
                        norm_clause: (*clause).into(),
                    });
                }
                id
            };
        if ru {
            checklist(
                "Монолитные работы — операционный контроль",
                Section::Kj,
                QualityKind::Operational,
                &[
                    (
                        "Соответствие опалубки проекту",
                        "ШНК 3.03.01-98",
                        "п. 2.108",
                    ),
                    ("Класс и защитный слой арматуры", "ШНК 2.03.01-96", "п. 5.5"),
                    (
                        "Отметки и геометрия конструкции",
                        "ШНК 3.01.03-97",
                        "п. 4.3",
                    ),
                    (
                        "Чистота опалубки перед бетонированием",
                        "ШНК 3.03.01-98",
                        "п. 2.110",
                    ),
                    (
                        "Наличие паспорта на бетонную смесь",
                        "ГОСТ 7473-2010",
                        "п. 6.2",
                    ),
                ],
            );
            checklist(
                "Кирпичная кладка — приёмочный контроль",
                Section::Ar,
                QualityKind::Acceptance,
                &[
                    ("Отклонение стены от вертикали", "ШНК 3.03.01-98", "табл. 9"),
                    ("Толщина и заполнение швов", "ШНК 3.03.01-98", "п. 7.29"),
                    ("Перевязка швов", "ШНК 3.03.01-98", "п. 7.10"),
                    ("Наличие акта скрытых работ", "ШНК 3.01.01-03", "п. 6.4"),
                ],
            );
        } else {
            checklist(
                "Monolit ishlari — operatsion nazorat",
                Section::Kj,
                QualityKind::Operational,
                &[
                    (
                        "Qolipning loyihaga mosligi",
                        "ShNQ 3.03.01-98",
                        "2.108-band",
                    ),
                    (
                        "Armatura sinfi va himoya qatlami",
                        "ShNQ 2.03.01-96",
                        "5.5-band",
                    ),
                    (
                        "Konstruksiya belgilari va geometriyasi",
                        "ShNQ 3.01.03-97",
                        "4.3-band",
                    ),
                    (
                        "Betonlashdan oldin qolip tozaligi",
                        "ShNQ 3.03.01-98",
                        "2.110-band",
                    ),
                    ("Beton aralashmasi pasporti", "GOST 7473-2010", "6.2-band"),
                ],
            );
            checklist(
                "G'isht terish — qabul nazorati",
                Section::Ar,
                QualityKind::Acceptance,
                &[
                    (
                        "Devorning vertikaldan og'ishi",
                        "ShNQ 3.03.01-98",
                        "9-jadval",
                    ),
                    (
                        "Chok qalinligi va to'ldirilishi",
                        "ShNQ 3.03.01-98",
                        "7.29-band",
                    ),
                    ("Choklarning bog'lanishi", "ShNQ 3.03.01-98", "7.10-band"),
                    (
                        "Yashirin ishlar dalolatnomasi",
                        "ShNQ 3.01.01-03",
                        "6.4-band",
                    ),
                ],
            );
        }

        // Namunadagi tekshiruvlarga chek-list biriktiramiz — nuqtalar paneli
        // bo'sh ko'rinmasin. Bittasida nomuvofiqlik ataylab qoldirilgan.
        let lists = self.checklists(pid);
        for check in self.quality_checks(pid) {
            let Some(list) = lists.iter().find(|c| c.kind == check.kind) else {
                continue;
            };
            if self.apply_checklist(check.id, list.id) == 0 {
                continue;
            }
            let mut x = check.clone();
            x.checklist_id = Some(list.id);
            self.update_quality(&x);

            // O'tgan tekshiruvda hamma nuqta mos; nuqsonlisida bittasi mos emas.
            let points = self.check_points(pid);
            for (n, p) in points.iter().filter(|p| p.check_id == check.id).enumerate() {
                let mut p = p.clone();
                p.result = if check.result == QualityResult::Pass {
                    PointResult::Pass
                } else if n == 1 {
                    PointResult::Fail
                } else {
                    PointResult::Pass
                };
                self.update_check_point(&p);
            }
        }

        // Ruxsatlar va SIZ (TZ XV.4–9). Ataylab uch xil holat qoldirilgan:
        // hammasi joyida, bittasining muddati o'tgan, bittasida SIZ yo'q.
        let permit = |worker_id: i64, kind: PermitKind, days_ago: i64, valid_days: i64| {
            self.insert_worker_permit(&WorkerPermit {
                id: 0,
                project_id: pid,
                worker_id,
                kind,
                number: String::new(),
                issued: today - chrono::Duration::days(days_ago),
                valid_until: today + chrono::Duration::days(valid_days),
                note: String::new(),
            });
        };
        let ppe = |worker_id: i64, item: PpeItem, days_ago: i64| {
            self.insert_ppe_issue(&PpeIssue {
                id: 0,
                project_id: pid,
                worker_id,
                item,
                issued: today - chrono::Duration::days(days_ago),
                months: item.months(),
                note: String::new(),
            });
        };
        for (i, wid) in ids.iter().enumerate() {
            if *wid == 0 {
                continue;
            }
            // Oxirgi ishchining kirish instruktaji muddati o'tgan.
            let induction_left = if i + 1 == ids.len() { -10 } else { 300 };
            permit(*wid, PermitKind::Induction, 60, induction_left);
            permit(*wid, PermitKind::Medical, 90, 250);
            // Monolitchilar balandlikda ishlaydi.
            if (1..=3).contains(&i) {
                // Ikkinchisining balandlik ruxsati tez orada tugaydi.
                permit(*wid, PermitKind::Height, 200, if i == 2 { 12 } else { 160 });
            }
            if i == 4 {
                permit(*wid, PermitKind::Electric, 120, 240);
            }

            // SIZ: to'rtinchi ishchiga qo'lqop berilmagan.
            for item in PpeItem::REQUIRED {
                if i == 3 && *item == PpeItem::Gloves {
                    continue;
                }
                ppe(*wid, *item, 20);
            }
            if (1..=3).contains(&i) {
                ppe(*wid, PpeItem::Harness, 20);
            }
        }

        // Naryad-dopusk (TZ XV.10–12): biri to'g'ri, biri kamchilikli.
        let permit_doc = |number: &str,
                          kind: PermitKind,
                          task_id: Option<i64>,
                          place: &str,
                          from: i64,
                          to: i64,
                          issuer: &str,
                          supervisor: &str,
                          workers: &[i64],
                          measures: &str,
                          status: PermitStatus| {
            let mut p = WorkPermit {
                id: 0,
                project_id: pid,
                number: number.into(),
                kind,
                task_id,
                place: place.into(),
                date_from: today - chrono::Duration::days(from),
                date_to: today + chrono::Duration::days(to),
                issuer: issuer.into(),
                supervisor: supervisor.into(),
                workers: String::new(),
                measures: measures.into(),
                status,
                note: String::new(),
            };
            p.set_workers(workers);
            self.insert_work_permit(&p);
        };
        let ok_workers: Vec<i64> = ids.iter().skip(1).take(2).copied().collect();
        let bad_workers: Vec<i64> = ids.iter().skip(3).take(2).copied().collect();
        if ru {
            permit_doc(
                "НД-001",
                PermitKind::Height,
                by_wbs("7"),
                "7-9 этаж, ось А-Г",
                2,
                5,
                "Юсупов Б.Р.",
                "Саидова М.И.",
                &ok_workers,
                "Страховочные привязи, ограждение зоны, инструктаж на рабочем месте",
                PermitStatus::Open,
            );
            permit_doc(
                "НД-002",
                PermitKind::HotWork,
                by_wbs("8"),
                "Кровля",
                1,
                3,
                "Юсупов Б.Р.",
                "",
                &bad_workers,
                "",
                PermitStatus::Open,
            );
        } else {
            permit_doc(
                "ND-001",
                PermitKind::Height,
                by_wbs("7"),
                "7-9 qavat, A-G o'qlari",
                2,
                5,
                "Yusupov B.R.",
                "Saidova M.I.",
                &ok_workers,
                "Saqlovchi arqonlar, zonani to'sish, ish joyida instruktaj",
                PermitStatus::Open,
            );
            permit_doc(
                "ND-002",
                PermitKind::HotWork,
                by_wbs("8"),
                "Tom yopish",
                1,
                3,
                "Yusupov B.R.",
                "",
                &bad_workers,
                "",
                PermitStatus::Open,
            );
        }

        // ---------- XV. Xavfsizlik ----------
        let safety_resp = if ru {
            "Инженер по ТБ: Эргашев К."
        } else {
            "TX muhandisi: Ergashev K."
        };
        let se = |days_ago: i64,
                  kind: SafetyKind,
                  severity: Severity,
                  place: &str,
                  description: &str,
                  measure: &str,
                  deadline_in: Option<i64>,
                  status: IssueStatus| {
            self.insert_safety(&SafetyEvent {
                id: 0,
                project_id: pid,
                date: today - chrono::Duration::days(days_ago),
                kind,
                severity,
                place: place.into(),
                description: description.into(),
                responsible: safety_resp.into(),
                measure: measure.into(),
                deadline: deadline_in.map(|d| today + chrono::Duration::days(d)),
                status,
                root_cause: RootCause::Unknown,
            });
        };
        se(
            21,
            SafetyKind::Training,
            Severity::Info,
            if ru {
                "Штаб строительства"
            } else {
                "Qurilish shtabi"
            },
            if ru {
                "Первичный инструктаж, 12 человек"
            } else {
                "Boshlang'ich instruktaj, 12 kishi"
            },
            if ru {
                "Журнал заполнен"
            } else {
                "Jurnal to'ldirildi"
            },
            None,
            IssueStatus::Fixed,
        );
        // Muddati o'tgan va yopilmagan buzilish.
        se(
            9,
            SafetyKind::Violation,
            Severity::Warning,
            if ru {
                "8 этаж, ось Б"
            } else {
                "8-qavat, B o'qi"
            },
            if ru {
                "Работа на высоте без страховочной привязи"
            } else {
                "Balandlikda strahovka kamarisiz ishlash"
            },
            if ru {
                "Выдать привязи, повторный инструктаж"
            } else {
                "Kamar berish, takroriy instruktaj"
            },
            Some(-2),
            IssueStatus::Open,
        );
        se(
            5,
            SafetyKind::NearMiss,
            Severity::Warning,
            if ru {
                "Зона крана"
            } else {
                "Kran zonasi"
            },
            if ru {
                "Падение доски с 7 этажа, пострадавших нет"
            } else {
                "7-qavatdan taxta tushdi, jabrlangan yo'q"
            },
            if ru {
                "Установить защитный козырек"
            } else {
                "Himoya kozirkasi o'rnatish"
            },
            Some(3),
            IssueStatus::InWork,
        );
        se(
            2,
            SafetyKind::Inspection,
            Severity::Info,
            if ru {
                "Объект целиком"
            } else {
                "Butun obyekt"
            },
            if ru {
                "Плановая проверка ТБ"
            } else {
                "Rejali TX tekshiruvi"
            },
            if ru {
                "Замечания устранены на месте"
            } else {
                "Kamchiliklar joyida bartaraf etildi"
            },
            None,
            IssueStatus::Fixed,
        );

        // ---------- XVI. Texnika ----------
        let owner = if ru {
            "ООО «СтройМеханизация»"
        } else {
            "«StroyMexanizatsiya» MChJ"
        };
        #[allow(clippy::too_many_arguments)]
        let mch = |name: &str,
                   kind: MachineKind,
                   reg_no: &str,
                   status: MachineStatus,
                   hour_rate: f64,
                   operator: &str,
                   inspection_in: Option<i64>,
                   fuel_norm: f64,
                   service_hours: f64,
                   service_done: f64,
                   rented: bool|
         -> i64 {
            self.insert_machine(&Machine {
                id: 0,
                project_id: pid,
                name: name.into(),
                kind,
                reg_no: reg_no.into(),
                owner: owner.into(),
                status,
                hour_rate,
                operator: operator.into(),
                inspection_until: inspection_in.map(|d| today + chrono::Duration::days(d)),
                fuel_norm,
                service_hours,
                service_done,
                rented,
                // Balans qiymati: ijaradagi texnikada u yo'q.
                price: if rented { 0.0 } else { hour_rate * 2_400.0 },
            })
        };
        let crane = mch(
            if ru {
                "Башенный кран КБ-403"
            } else {
                "KB-403 minorali kran"
            },
            MachineKind::Crane,
            "01 A 123 BC",
            MachineStatus::Working,
            180_000.0,
            if ru {
                "Тошматов А."
            } else {
                "Toshmatov A."
            },
            Some(120),
            // Minorali kran elektrda ishlaydi — yoqilg'i normasi yo'q.
            0.0,
            500.0,
            0.0,
            false,
        );
        // Texnik ko'rik muddati o'tgan — ishlatib bo'lmaydi.
        let excavator = mch(
            if ru {
                "Экскаватор Hyundai R220"
            } else {
                "Hyundai R220 ekskavator"
            },
            MachineKind::Excavator,
            "01 B 456 CD",
            MachineStatus::Idle,
            210_000.0,
            if ru {
                "Рахимов Ш."
            } else {
                "Rahimov Sh."
            },
            Some(-14),
            12.0,
            250.0,
            // TX oralig'i tugashiga oz qoldi — ogohlantirish ko'rinsin.
            60.0,
            false,
        );
        let pump = mch(
            if ru {
                "Автобетононасос 37 м"
            } else {
                "37 m avtobetonnasos"
            },
            MachineKind::Concrete,
            "01 C 789 DE",
            MachineStatus::Working,
            340_000.0,
            if ru { "Аминов Р." } else { "Aminov R." },
            Some(45),
            18.0,
            300.0,
            0.0,
            // Nasos ijaraga olingan.
            true,
        );
        // Lift ta'mirda — smenasi yo'q, faqat parkda turadi.
        let _lift = mch(
            if ru {
                "Строительный подъемник"
            } else {
                "Qurilish liftlari"
            },
            MachineKind::Lift,
            "01 D 012 EF",
            MachineStatus::Repair,
            90_000.0,
            String::new().as_str(),
            Some(200),
            0.0,
            0.0,
            0.0,
            false,
        );

        // Oxirgi 20 kunlik smenalar.
        let karkas = by_wbs("7");
        let devor = by_wbs("9");
        for back in 0..20 {
            let day = today - chrono::Duration::days(back);
            if day.weekday().num_days_from_monday() == 6 {
                continue;
            }
            // Har bir smena — yo'l varaqasi: raqami, haydovchisi, marshruti.
            #[allow(clippy::too_many_arguments)]
            let log = |seq: i64,
                       machine_id: i64,
                       hours: f64,
                       fuel: f64,
                       task_id: Option<i64>,
                       driver: &str,
                       route: &str,
                       odo: f64,
                       km: f64,
                       trips: i64,
                       cargo: f64| {
                if machine_id > 0 && hours > 0.0 {
                    self.insert_machine_log(&MachineLog {
                        id: 0,
                        project_id: pid,
                        machine_id,
                        date: day,
                        hours,
                        fuel,
                        task_id,
                        // Raqam kun va texnika bo'yicha noyob bo'lishi kerak:
                        // bir kunda ikki mashina bir raqam ostida yura olmaydi.
                        number: format!("YV-{:04}", (100 - back) * 10 + seq),
                        driver: driver.into(),
                        route: route.into(),
                        odo_start: odo,
                        odo_end: if km > 0.0 { odo + km } else { 0.0 },
                        trips,
                        cargo,
                        note: String::new(),
                    });
                }
            };
            let (op_crane, op_pump, op_exc) = if ru {
                ("Тошматов А.", "Аминов Р.", "Рахимов Ш.")
            } else {
                ("Toshmatov A.", "Aminov R.", "Rahimov Sh.")
            };
            let (r_site, r_concrete, r_soil) = if ru {
                ("Объект, стройплощадка", "РБУ — объект", "Объект — отвал")
            } else {
                (
                    "Obyekt, qurilish maydoni",
                    "BTZ — obyekt",
                    "Obyekt — to'kish joyi",
                )
            };
            // Kran maydonda turadi — spidometri yo'q.
            log(
                1, crane, 8.0, 0.0, karkas, op_crane, r_site, 0.0, 0.0, 0, 0.0,
            );
            // Nasos faqat betonlash kunlarida chiqadi.
            if back % 3 == 0 {
                // Yoqilg'i normasi 18 l/soat: 5 soatga 90 litr — normada.
                log(
                    2,
                    pump,
                    5.0,
                    90.0,
                    karkas,
                    op_pump,
                    r_concrete,
                    12_000.0 + (20 - back) as f64 * 40.0,
                    36.0,
                    3,
                    45.0,
                );
            }
            if back % 4 == 1 {
                // Ekskavatorda ortiqcha sarf ataylab qoldirilgan:
                // norma 12 l/soat, 6 soatga 72 litr, faktda 90.
                log(
                    3,
                    excavator,
                    6.0,
                    90.0,
                    devor,
                    op_exc,
                    r_soil,
                    8_400.0 + (20 - back) as f64 * 25.0,
                    22.0,
                    0,
                    0.0,
                );
            }
        }
    }

    /// Sotuv namunasi: ikkita blok, kvartiralar va turli holatdagi shartnomalar
    /// (TZ XIX-XX). Bir shartnomada to'lov ataylab kechiktirilgan — muddati
    /// o'tgan qarz qanday ko'rinishini ko'rsatish uchun.
    pub fn seed_demo_sales(&self, pid: i64, ru: bool) {
        if !self.blocks(pid).is_empty() {
            return;
        }
        let today = chrono::Local::now().date_naive();

        // Qavatdagi to'rt kvartira: maydon, yashash maydoni, xonalar.
        const PLAN: [(f64, f64, i64); 4] = [
            (42.5, 24.0, 1),
            (58.0, 33.0, 2),
            (74.5, 44.0, 3),
            (48.0, 28.0, 2),
        ];
        const FLOORS: i64 = 9;
        let base_price = 9_400_000.0;

        let mut units: Vec<(i64, i64)> = Vec::new(); // (unit_id, maydon indeksi)
        for (bi, name) in [
            (0, if ru { "1-й блок" } else { "1-blok" }),
            (1, if ru { "2-й блок" } else { "2-blok" }),
        ] {
            let bid = self.insert_block(&Block {
                id: 0,
                project_id: pid,
                name: name.into(),
                floors: FLOORS,
                first_floor: 1,
                note: String::new(),
            });
            if bid == 0 {
                continue;
            }
            for floor in 1..=FLOORS {
                for (i, (area, living, rooms)) in PLAN.iter().enumerate() {
                    // Birinchi qavat tijorat uchun, qolgani turar-joy.
                    let kind = if floor == 1 {
                        UnitKind::Commercial
                    } else {
                        UnitKind::Flat
                    };
                    // Yuqori qavat qimmatroq, birinchi qavat esa tijorat narxida.
                    let price = if floor == 1 {
                        base_price * 1.35
                    } else {
                        base_price + (floor - 2) as f64 * 60_000.0
                    };
                    let number = bi * 100 + (floor - 1) * PLAN.len() as i64 + i as i64 + 1;
                    let id = self.insert_unit(&Unit {
                        id: 0,
                        project_id: pid,
                        block_id: bid,
                        number: number.to_string(),
                        floor,
                        position: i as i64 + 1,
                        kind,
                        rooms: if kind == UnitKind::Commercial {
                            0
                        } else {
                            *rooms
                        },
                        area: *area,
                        area_living: if kind == UnitKind::Commercial {
                            0.0
                        } else {
                            *living
                        },
                        price_per_m2: price,
                        status: UnitStatus::Free,
                        layout: format!("{}{}", rooms, if ru { "К" } else { "X" }),
                        note: String::new(),
                    });
                    if id > 0 && bi == 0 {
                        units.push((id, floor));
                    }
                }
            }
        }

        // Shartnomalar: birinchi blokning bir qismi sotilgan.
        let clients: [(&str, &str); 8] = if ru {
            [
                ("Ахмедов Ж.Т.", "+998 90 123-45-67"),
                ("Каримова Н.С.", "+998 91 234-56-78"),
                ("Усмонов Ш.Б.", "+998 93 345-67-89"),
                ("Юлдашева Д.А.", "+998 94 456-78-90"),
                ("Рахматов О.К.", "+998 95 567-89-01"),
                ("Сафарова М.И.", "+998 97 678-90-12"),
                ("Тошматов А.Р.", "+998 98 789-01-23"),
                ("Эргашева З.Н.", "+998 99 890-12-34"),
            ]
        } else {
            [
                ("Ahmedov J.T.", "+998 90 123-45-67"),
                ("Karimova N.S.", "+998 91 234-56-78"),
                ("Usmonov Sh.B.", "+998 93 345-67-89"),
                ("Yuldasheva D.A.", "+998 94 456-78-90"),
                ("Rahmatov O.K.", "+998 95 567-89-01"),
                ("Safarova M.I.", "+998 97 678-90-12"),
                ("Toshmatov A.R.", "+998 98 789-01-23"),
                ("Ergasheva Z.N.", "+998 99 890-12-34"),
            ]
        };
        let manager = if ru {
            "Отдел продаж: Собиров Т."
        } else {
            "Sotuv bo'limi: Sobirov T."
        };

        // (kvartira indeksi, kunlar oldin, to'lov turi, holat, chegirma %, boshlang'ich %, oy, to'langan oy)
        type DemoDeal = (usize, i64, PayKind, DealStatus, f64, f64, i64, i64);
        let plan: [DemoDeal; 8] = [
            (
                4,
                150,
                PayKind::Cash,
                DealStatus::Completed,
                3.0,
                100.0,
                0,
                1,
            ),
            (
                5,
                130,
                PayKind::Credit,
                DealStatus::Completed,
                0.0,
                30.0,
                0,
                1,
            ),
            (
                8,
                110,
                PayKind::Installment,
                DealStatus::Signed,
                0.0,
                30.0,
                12,
                3,
            ),
            (9, 95, PayKind::Subsidy, DealStatus::Signed, 5.0, 40.0, 6, 2),
            (
                12,
                70,
                PayKind::Installment,
                DealStatus::Signed,
                0.0,
                20.0,
                18,
                1,
            ),
            (
                13,
                45,
                PayKind::Barter,
                DealStatus::Signed,
                0.0,
                100.0,
                0,
                1,
            ),
            (
                16,
                20,
                PayKind::Installment,
                DealStatus::Reserved,
                0.0,
                15.0,
                24,
                0,
            ),
            (17, 6, PayKind::Mixed, DealStatus::Reserved, 0.0, 25.0, 9, 0),
        ];

        let all = self.units(pid);
        for (n, (idx, days, pay_kind, status, disc, pre, months, paid_months)) in
            plan.iter().enumerate()
        {
            let Some((uid, _)) = units.get(*idx) else {
                continue;
            };
            let Some(u) = all.iter().find(|x| x.id == *uid) else {
                continue;
            };
            let price = u.price();
            let deal = Deal {
                id: 0,
                project_id: pid,
                unit_id: *uid,
                number: format!("S-{:03}", n + 1),
                date: today - chrono::Duration::days(*days),
                client: clients[n].0.into(),
                phone: clients[n].1.into(),
                client_doc: String::new(),
                pay_kind: *pay_kind,
                price,
                discount: price * disc / 100.0,
                prepayment: (price - price * disc / 100.0) * pre / 100.0,
                months: *months,
                status: *status,
                manager: manager.into(),
                note: String::new(),
            };
            let did = self.insert_deal(&deal);
            if did == 0 {
                continue;
            }
            let mut deal = deal;
            deal.id = did;

            // Grafik va unga tushgan to'lovlar.
            for (i, mut row) in crate::sales::build_schedule(&deal).into_iter().enumerate() {
                if (i as i64) < *paid_months {
                    row.paid = row.planned;
                    row.paid_date = Some(row.due);
                }
                self.insert_payment(&row);
            }

            // Kvartira holati shartnomaga ergashadi.
            if let Some(want) = crate::sales::status_for(Some(&deal)) {
                let mut copy = u.clone();
                copy.status = want;
                self.update_unit(&copy);
            }
        }
    }

    /// Ariza va xaridlar namunasi (TZ IX-X).
    ///
    /// Zanjirning uchta holati ko'rsatilgan: to'liq yopilgan (ariza -> xarid ->
    /// ombor kirimi), yo'ldagi xarid va hali tasdiqlanmagan ariza. Bittasining
    /// muddati ataylab o'tkazib yuborilgan.
    pub fn seed_demo_supply(&self, pid: i64, ru: bool) {
        if !self.requests(pid).is_empty() {
            return;
        }
        let today = chrono::Local::now().date_naive();
        let materials = self.materials(pid);
        let by_code = |c: &str| materials.iter().find(|m| m.code == c).map(|m| m.id);
        let tasks = self.tasks(pid).unwrap_or_default();
        let by_wbs = |w: &str| tasks.iter().find(|t| t.wbs == w).map(|t| t.id);

        let req = |number: &str,
                   days_ago: i64,
                   kind: RequestKind,
                   title: &str,
                   material_id: Option<i64>,
                   qty: f64,
                   unit: &str,
                   need_in: i64,
                   priority: Priority,
                   status: RequestStatus,
                   task_id: Option<i64>|
         -> i64 {
            self.insert_request(&Request {
                id: 0,
                project_id: pid,
                number: number.into(),
                date: today - chrono::Duration::days(days_ago),
                kind,
                title: title.into(),
                material_id,
                qty,
                unit: unit.into(),
                requester: if ru {
                    "Юсупов Б.Р.".into()
                } else {
                    "Yusupov B.R.".to_string()
                },
                need_date: today + chrono::Duration::days(need_in),
                priority,
                status,
                task_id,
                reject_reason: String::new(),
                note: String::new(),
            })
        };

        // 1. Yopilgan zanjir: g'isht so'raldi, xarid qilindi, omborga kirim bo'ldi.
        let z1 = req(
            "Z-001",
            32,
            RequestKind::Material,
            if ru {
                "Кирпич керамический М150"
            } else {
                "Keramik g'isht M150"
            },
            by_code("M-201"),
            96_000.0,
            if ru { "шт" } else { "dona" },
            -20,
            Priority::Normal,
            RequestStatus::Closed,
            by_wbs("9"),
        );
        // 2. Yo'lda: armatura buyurtma qilingan, hali kelmagan.
        let z2 = req(
            "Z-002",
            10,
            RequestKind::Material,
            if ru {
                "Арматура А500С d16"
            } else {
                "A500S armatura d16"
            },
            by_code("M-102"),
            24.0,
            if ru { "т" } else { "t" },
            6,
            Priority::High,
            RequestStatus::InPurchase,
            by_wbs("7"),
        );
        // 3. Muddati o'tgan, ammo yopilmagan — ogohlantirish uchun.
        let z3 = req(
            "Z-003",
            18,
            RequestKind::Material,
            if ru {
                "Труба ПП 110"
            } else {
                "PP 110 truba"
            },
            by_code("M-401"),
            300.0,
            if ru { "м" } else { "m" },
            -4,
            Priority::Urgent,
            RequestStatus::Approved,
            None,
        );
        // 4. Tasdiqlanmagan ariza — texnika.
        req(
            "Z-004",
            2,
            RequestKind::Machine,
            if ru {
                "Автобетононасос, 3 смены"
            } else {
                "Avtobetonnasos, 3 smena"
            },
            None,
            3.0,
            if ru { "смена" } else { "smena" },
            12,
            Priority::High,
            RequestStatus::New,
            by_wbs("7"),
        );

        let pur = |number: &str,
                   request_id: Option<i64>,
                   days_ago: i64,
                   supplier: &str,
                   title: &str,
                   qty: f64,
                   unit: &str,
                   price: f64,
                   delivery_in: i64,
                   status: PurchaseStatus,
                   delivered_qty: f64,
                   section: Section| {
            self.insert_purchase(&Purchase {
                id: 0,
                project_id: pid,
                request_id,
                number: number.into(),
                date: today - chrono::Duration::days(days_ago),
                supplier: supplier.into(),
                title: title.into(),
                qty,
                unit: unit.into(),
                price,
                currency: "UZS".into(),
                delivery_date: today + chrono::Duration::days(delivery_in),
                status,
                delivered_qty,
                section,
                task_id: None,
                contract_id: None,
                urgent: false,
                buyer: String::new(),
                material_id: None,
                substitute_for: None,
                paid: 0.0,
                pay_due: None,
                tech_ok: false,
                tech_by: String::new(),
                note: String::new(),
            });
        };

        let s1 = if ru {
            "ООО «СтройБаза»"
        } else {
            "«StroyBaza» MChJ"
        };
        let s2 = if ru {
            "ООО «МеталлСнаб»"
        } else {
            "«MetallSnab» MChJ"
        };

        // Kelishuv marshrutlari (TZ IX.8): birinchisi to'liq kelishilgan,
        // ikkinchisi loyiha rahbarini kutmoqda — bosqichlar tartibi ko'rinsin.
        let step = |request_id: i64,
                    step: i64,
                    role: Role,
                    approver: &str,
                    decision: ApprovalDecision,
                    days_ago: Option<i64>| {
            self.insert_approval(&Approval {
                id: 0,
                project_id: pid,
                request_id,
                step,
                role: role.code().into(),
                approver: approver.into(),
                decision,
                decided_at: days_ago.map(|d| today - chrono::Duration::days(d)),
                comment: String::new(),
            });
        };
        let foreman = if ru {
            "Юсупов Б.Р."
        } else {
            "Yusupov B.R."
        };
        let manager = if ru {
            "Саидова М.И."
        } else {
            "Saidova M.I."
        };
        let director = if ru {
            "Ахмедов Р.С."
        } else {
            "Ahmedov R.S."
        };
        step(
            z1,
            1,
            Role::Foreman,
            foreman,
            ApprovalDecision::Approved,
            Some(31),
        );
        step(
            z1,
            2,
            Role::ProjectManager,
            manager,
            ApprovalDecision::Approved,
            Some(30),
        );
        step(
            z2,
            1,
            Role::Foreman,
            foreman,
            ApprovalDecision::Approved,
            Some(9),
        );
        step(
            z1,
            3,
            Role::Director,
            director,
            ApprovalDecision::Approved,
            Some(30),
        );
        step(
            z2,
            2,
            Role::ProjectManager,
            "",
            ApprovalDecision::Pending,
            None,
        );
        step(z2, 3, Role::Director, "", ApprovalDecision::Pending, None);

        // Z-001 uchun xarid: hujjat raqami TTN-1150 — ombor kirimi bilan bir xil,
        // shuning uchun «kirim qilingan» deb ko'rsatiladi.
        pur(
            "TTN-1150",
            Some(z1),
            28,
            s1,
            if ru {
                "Кирпич керамический М150"
            } else {
                "Keramik g'isht M150"
            },
            96_000.0,
            if ru { "шт" } else { "dona" },
            1_400.0,
            -26,
            PurchaseStatus::Closed,
            96_000.0,
            Section::Ar,
        );
        // Z-002: buyurtma berilgan, yo'lda.
        pur(
            "X-002",
            Some(z2),
            8,
            s2,
            if ru {
                "Арматура А500С d16"
            } else {
                "A500S armatura d16"
            },
            24.0,
            if ru { "т" } else { "t" },
            10_200_000.0,
            // Bugun kutilmoqda — kunlik xulosada chiqadi.
            0,
            PurchaseStatus::Paid,
            0.0,
            Section::Kj,
        );
        // Z-003: qisman qoplangan xarid, muddati o'tgan.
        pur(
            "X-003",
            Some(z3),
            12,
            s1,
            if ru {
                "Труба ПП 110"
            } else {
                "PP 110 truba"
            },
            180.0,
            if ru { "м" } else { "m" },
            41_000.0,
            -2,
            PurchaseStatus::Ordered,
            // Qisman yetkazilgan: 180 dan 120 tasi kelgan (TZ X.30).
            120.0,
            Section::Vk,
        );

        // Yetkazib beruvchilar kartochkasi (TZ X.7): tarix xaridlardan
        // hisoblanadi, bu yerda faqat aloqa ma'lumoti turadi.
        let sup = |name: &str, inn: &str, contact: &str, phone: &str| {
            self.insert_supplier(&Supplier {
                id: 0,
                project_id: pid,
                name: name.into(),
                inn: inn.into(),
                contact: contact.into(),
                phone: phone.into(),
                blocked: false,
                note: String::new(),
            });
        };
        if ru {
            sup(s1, "302145879", "Рахимов Ж.", "+998 90 123-45-67");
            sup(s2, "304871256", "Эргашев Т.", "+998 91 234-56-78");
            sup(
                "ООО «БетонПро»",
                "301447790",
                "Юлдашев К.",
                "+998 93 345-67-89",
            );
        } else {
            sup(s1, "302145879", "Rahimov J.", "+998 90 123-45-67");
            sup(s2, "304871256", "Ergashev T.", "+998 91 234-56-78");
            sup(
                "«BetonPro» MChJ",
                "301447790",
                "Yuldashev K.",
                "+998 93 345-67-89",
            );
        }

        // Uchta tijorat taklifi bitta arizaga (TZ X.9–12): eng arzoni uzoq
        // yetkazadi — tanlov faqat narxga qarab qilinmasligi ko'rinsin.
        let quote = |supplier: &str,
                     title: &str,
                     qty: f64,
                     unit: &str,
                     price: f64,
                     delivery_days: i64,
                     days_ago: i64,
                     chosen: bool| {
            self.insert_quote(&Quote {
                id: 0,
                project_id: pid,
                request_id: Some(z2),
                supplier: supplier.into(),
                title: title.into(),
                qty,
                unit: unit.into(),
                price,
                currency: "UZS".into(),
                delivery_days,
                valid_until: Some(today + chrono::Duration::days(10)),
                chosen,
                date: today - chrono::Duration::days(days_ago),
                note: String::new(),
            });
        };
        let rebar_title = if ru {
            "Арматура А500С d16"
        } else {
            "A500S armatura d16"
        };
        let unit = if ru { "т" } else { "t" };
        quote(s2, rebar_title, 24.0, unit, 10_200_000.0, 5, 10, true);
        quote(s1, rebar_title, 24.0, unit, 9_850_000.0, 21, 10, false);
        if ru {
            quote(
                "ООО «БетонПро»",
                rebar_title,
                24.0,
                unit,
                10_900_000.0,
                3,
                9,
                false,
            );
        } else {
            quote(
                "«BetonPro» MChJ",
                rebar_title,
                24.0,
                unit,
                10_900_000.0,
                3,
                9,
                false,
            );
        }

        // Bo'limlar bo'yicha xarid byudjeti (TZ X.34–35).
        let budget = |section: Section, planned: f64| {
            self.insert_purchase_budget(&PurchaseBudget {
                id: 0,
                project_id: pid,
                section,
                planned,
                note: String::new(),
            });
        };
        // Byudjet obyekt hajmiga mos: KJ bo'yicha xaridlar 200 mln dan
        // oshadi, shuning uchun reja undan sezilarli katta bo'lishi kerak.
        budget(Section::Kj, 900_000_000.0);
        budget(Section::Ar, 250_000_000.0);
        // EOM ataylab qoldirilgan: byudjeti tuzilmagan bo'limda xarid
        // bo'lishi — ekranda ko'rinishi kerak bo'lgan holat.
        // Suv va kanalizatsiya byudjeti ataylab kam — oshib ketgani ko'rinsin.
        budget(Section::Vk, 5_000_000.0);
    }

    /// Material katalogi va ombor harakatlari namunasi (TZ XI-XII).
    ///
    /// Ataylab uch xil muammoli holat qoldirilgan: bir materialning qoldig'i
    /// minimal zaxiradan past, bittasining sertifikat muddati o'tgan, yana
    /// bittasida sertifikat umuman kiritilmagan — ekran bo'sh ko'rinmasin va
    /// ogohlantirishlar qanday ishlashi ko'rinib tursin.
    pub fn seed_demo_stock(&self, pid: i64, ru: bool) {
        // Ombor tarixi bor bo'lsa — bu haqiqiy ma'lumot, tegmaymiz.
        if !self.stock_moves(pid).is_empty() {
            return;
        }
        let existing = self.materials(pid);
        let today = chrono::Local::now().date_naive();
        let tasks = self.tasks(pid).unwrap_or_default();
        let by_wbs = |w: &str| tasks.iter().find(|t| t.wbs == w).map(|t| t.id);

        let mat = |code: &str,
                   name: &str,
                   unit: &str,
                   section: Section,
                   spec: &str,
                   cert_no: &str,
                   cert_days: Option<i64>,
                   min_stock: f64,
                   price: f64|
         -> i64 {
            // Bir xil kod ikki marta qo'shilmasin.
            if let Some(m) = existing.iter().find(|m| m.code == code) {
                return m.id;
            }
            self.insert_material(&Material {
                id: 0,
                project_id: pid,
                code: code.into(),
                name: name.into(),
                unit: unit.into(),
                section,
                spec: spec.into(),
                cert_no: cert_no.into(),
                cert_until: cert_days.map(|d| today + chrono::Duration::days(d)),
                min_stock,
                price,
                estimate_code: String::new(),
                spec_ref: String::new(),
                special: String::new(),
                banned: false,
                ban_reason: String::new(),
                note: String::new(),
            })
        };

        let concrete = mat(
            "M-101",
            if ru {
                "Бетон товарный"
            } else {
                "Tayyor beton"
            },
            if ru { "м3" } else { "m3" },
            Section::Kj,
            "B25 W6 F150",
            "SS-2411/24",
            Some(180),
            120.0,
            720_000.0,
        );
        // Sertifikat muddati o'tgan — armatura ishlatilishi to'xtatilishi kerak.
        let rebar = mat(
            "M-102",
            if ru {
                "Арматура А500С"
            } else {
                "A500S armatura"
            },
            if ru { "т" } else { "t" },
            Section::Kj,
            "d12-d32, GOST 34028-2016",
            "SS-1907/23",
            Some(-25),
            8.0,
            9_800_000.0,
        );
        // Qoldiq minimal zaxiradan past — buyurtma kerak.
        let brick = mat(
            "M-201",
            if ru {
                "Кирпич керамический"
            } else {
                "Keramik g'isht"
            },
            if ru { "шт" } else { "dona" },
            Section::Ar,
            if ru {
                "М150, 250х120х65"
            } else {
                "M150, 250x120x65"
            },
            "SS-3302/25",
            Some(400),
            40_000.0,
            1_450.0,
        );
        // Sertifikat kiritilmagan.
        let cable = mat(
            "M-301",
            if ru {
                "Кабель ВВГнг 3x4"
            } else {
                "VVGng 3x4 kabel"
            },
            if ru { "м" } else { "m" },
            Section::Eom,
            "0.66 kV",
            "",
            None,
            500.0,
            28_000.0,
        );
        let pipe = mat(
            "M-401",
            if ru {
                "Труба канализационная ПП 110"
            } else {
                "PP 110 kanalizatsiya trubasi"
            },
            if ru { "м" } else { "m" },
            Section::Vk,
            if ru {
                "SN4, раструбная"
            } else {
                "SN4, rastrubli"
            },
            "SS-2210/24",
            Some(95),
            120.0,
            41_000.0,
        );

        let mv = |material_id: i64,
                  days_ago: i64,
                  kind: MoveKind,
                  qty: f64,
                  price: f64,
                  document: &str,
                  counterparty: &str,
                  task_id: Option<i64>| {
            self.insert_stock_move(&StockMove {
                id: 0,
                project_id: pid,
                material_id,
                date: today - chrono::Duration::days(days_ago),
                kind,
                qty,
                price,
                document: document.into(),
                counterparty: counterparty.into(),
                task_id,
                note: String::new(),
                warehouse_id: None,
                batch_id: None,
            });
        };
        // Ombor va partiyasi ko'rsatilgan harakat.
        let mvw = |material_id: i64,
                   days_ago: i64,
                   kind: MoveKind,
                   qty: f64,
                   price: f64,
                   document: &str,
                   counterparty: &str,
                   warehouse_id: Option<i64>,
                   batch_id: Option<i64>,
                   task_id: Option<i64>| {
            self.insert_stock_move(&StockMove {
                id: 0,
                project_id: pid,
                material_id,
                date: today - chrono::Duration::days(days_ago),
                kind,
                qty,
                price,
                document: document.into(),
                counterparty: counterparty.into(),
                task_id,
                note: String::new(),
                warehouse_id,
                batch_id,
            });
        };

        let karkas = by_wbs("7");
        let plita = by_wbs("3");
        let devor = by_wbs("9");
        let supplier = if ru {
            "ООО «СтройБаза»"
        } else {
            "«StroyBaza» MChJ"
        };
        let supplier2 = if ru {
            "ООО «МеталлСнаб»"
        } else {
            "«MetallSnab» MChJ"
        };

        // Omborlar (TZ XI.3): obyektdagi ombor va ochiq maydon.
        let main_wh = self.insert_warehouse(&Warehouse {
            id: 0,
            project_id: pid,
            name: if ru {
                "Склад объекта"
            } else {
                "Obyekt ombori"
            }
            .into(),
            kind: WarehouseKind::Object,
            responsible: if ru {
                "Кладовщик: Азимов Р."
            } else {
                "Omborchi: Azimov R."
            }
            .into(),
            note: String::new(),
        });
        let open_wh = self.insert_warehouse(&Warehouse {
            id: 0,
            project_id: pid,
            name: if ru {
                "Открытая площадка"
            } else {
                "Ochiq maydon"
            }
            .into(),
            kind: WarehouseKind::Open,
            responsible: String::new(),
            note: String::new(),
        });

        // Partiyalar (TZ XI.9): armatura ikki partiyada kelgan, birinchisining
        // sertifikati muddati o'tgan; betonning yaroqlilik muddati qisqa —
        // shunda FEFO navbati ko'rinadi.
        let batch = |material_id: i64,
                     number: &str,
                     days_ago: i64,
                     supplier: &str,
                     cert: &str,
                     cert_days: Option<i64>,
                     expires_in: Option<i64>|
         -> i64 {
            self.insert_batch(&Batch {
                id: 0,
                project_id: pid,
                material_id,
                number: number.into(),
                received: today - chrono::Duration::days(days_ago),
                supplier: supplier.into(),
                cert_no: cert.into(),
                cert_until: cert_days.map(|d| today + chrono::Duration::days(d)),
                expires: expires_in.map(|d| today + chrono::Duration::days(d)),
                note: String::new(),
            })
        };
        let rebar_a = batch(rebar, "P-001", 45, supplier2, "SS-1907/23", Some(-25), None);
        let rebar_b = batch(rebar, "P-002", 12, supplier2, "SS-2604/25", Some(240), None);
        let concrete_a = batch(
            concrete,
            "P-010",
            5,
            supplier,
            "SS-2411/24",
            Some(180),
            Some(2),
        );

        mv(
            concrete,
            40,
            MoveKind::In,
            900.0,
            700_000.0,
            "TTN-1120",
            supplier,
            None,
        );
        mv(
            concrete,
            22,
            MoveKind::In,
            600.0,
            745_000.0,
            "TTN-1188",
            supplier,
            None,
        );
        mv(
            concrete,
            30,
            MoveKind::Out,
            640.0,
            0.0,
            "M-29/03",
            "",
            plita,
        );
        mv(
            concrete,
            5,
            MoveKind::Out,
            720.0,
            0.0,
            "M-29/08",
            "",
            karkas,
        );
        mv(
            concrete,
            4,
            MoveKind::WriteOff,
            12.0,
            0.0,
            "AKT-07",
            "",
            karkas,
        );

        mv(rebar, 28, MoveKind::Out, 41.0, 0.0, "M-29/04", "", plita);

        mv(
            brick,
            26,
            MoveKind::In,
            96_000.0,
            1_400.0,
            "TTN-1150",
            supplier,
            None,
        );
        mv(brick, 8, MoveKind::Out, 68_000.0, 0.0, "M-29/06", "", devor);
        mv(
            brick,
            2,
            MoveKind::WriteOff,
            2_400.0,
            0.0,
            "AKT-09",
            "",
            devor,
        );

        mv(
            cable,
            18,
            MoveKind::In,
            1_800.0,
            27_500.0,
            "TTN-1174",
            supplier,
            None,
        );
        mv(cable, 6, MoveKind::Out, 640.0, 0.0, "M-29/07", "", None);

        mv(
            pipe,
            20,
            MoveKind::In,
            740.0,
            40_000.0,
            "TTN-1179",
            supplier,
            None,
        );
        mv(pipe, 7, MoveKind::Out, 310.0, 0.0, "M-29/05", "", None);

        // Ombor va partiya ko'rsatilgan harakatlar: armatura ikki partiyada,
        // beton — muddati yaqin partiyada (FEFO shu partiyani birinchi beradi).
        mvw(
            rebar,
            45,
            MoveKind::In,
            62.0,
            9_600_000.0,
            "TTN-0914",
            supplier2,
            Some(main_wh),
            Some(rebar_a),
            None,
        );
        mvw(
            rebar,
            12,
            MoveKind::In,
            18.0,
            10_100_000.0,
            "TTN-1201",
            supplier2,
            Some(main_wh),
            Some(rebar_b),
            None,
        );
        mvw(
            rebar,
            3,
            MoveKind::Out,
            24.0,
            0.0,
            "M-29/09",
            "",
            Some(main_wh),
            Some(rebar_a),
            // Chiqim ishga bog'lanadi — sarf hisobi shundan chiqadi.
            karkas,
        );
        mvw(
            concrete,
            5,
            MoveKind::In,
            60.0,
            745_000.0,
            "TTN-1190",
            supplier,
            Some(open_wh),
            Some(concrete_a),
            None,
        );
        // Ishdan ortgan material omborga qaytdi (TZ XI.21).
        mvw(
            brick,
            2,
            MoveKind::Return,
            1_200.0,
            0.0,
            "V-01",
            "",
            Some(main_wh),
            None,
            None,
        );

        // Rezerv (TZ XI.17): beton keyingi bosqich uchun band qilingan.
        self.insert_reservation(&Reservation {
            id: 0,
            project_id: pid,
            material_id: concrete,
            task_id: by_wbs("7"),
            qty: 80.0,
            date: today - chrono::Duration::days(2),
            until: Some(today + chrono::Duration::days(14)),
            note: if ru {
                "Под бетонирование 9 этажа"
            } else {
                "9-qavat betonlash uchun"
            }
            .into(),
        });

        // Materialning loyiha va smeta bilan bog'lanishi (TZ XII.5–7) hamda
        // analoglar. Bittasi ataylab tasdiqlanmagan holda qoldirilgan.
        let mut catalog = self.materials(pid);
        for m in catalog.iter_mut() {
            let (code, spec_ref, special) = match m.code.as_str() {
                "M-101" => (
                    "E-06-01-001",
                    "KJ-12, 4-varaq",
                    if ru {
                        "Морозостойкость F150"
                    } else {
                        "Sovuqqa chidamlilik F150"
                    },
                ),
                "M-102" => ("E-06-01-034", "KJ-08, 2-varaq", ""),
                "M-201" => ("E-08-02-001", "AR-04, 7-varaq", ""),
                "M-401" => ("E-18-03-012", "VK-02, 3-varaq", ""),
                _ => ("", "", ""),
            };
            m.estimate_code = code.into();
            m.spec_ref = spec_ref.into();
            m.special = special.into();
            self.update_material(m);
        }

        // Analog haqiqiy almashtiruvchi bo'lishi kerak: shuning uchun katalogga
        // shu maqsadda ikkita pozitsiya qo'shiladi.
        let alt_mat = |code: &str, name: &str, unit: &str, section: Section, price: f64| -> i64 {
            if let Some(m) = catalog.iter().find(|m| m.code == code) {
                return m.id;
            }
            self.insert_material(&Material {
                id: 0,
                project_id: pid,
                code: code.into(),
                name: name.into(),
                unit: unit.into(),
                section,
                spec: String::new(),
                cert_no: String::new(),
                cert_until: None,
                min_stock: 0.0,
                price,
                estimate_code: String::new(),
                spec_ref: String::new(),
                special: String::new(),
                banned: false,
                ban_reason: String::new(),
                note: String::new(),
            })
        };
        let by_code = |c: &str| catalog.iter().find(|m| m.code == c).map(|m| m.id);

        // Beton — boshqa zavoddan, narxi biroz qimmat: tasdiqlangan analog.
        let beton_alt = alt_mat(
            "M-103",
            if ru {
                "Бетон B25 W6 (другой завод)"
            } else {
                "B25 W6 beton (boshqa zavod)"
            },
            if ru { "м3" } else { "m3" },
            Section::Kj,
            745_000.0,
        );
        if let Some(concrete) = by_code("M-101") {
            self.insert_material_alt(&MaterialAlt {
                id: 0,
                project_id: pid,
                material_id: concrete,
                alt_id: beton_alt,
                approved_by: if ru {
                    "ГИП: Ахмедов Р.С."
                } else {
                    "BLM: Ahmedov R.S."
                }
                .into(),
                approved_at: Some(today - chrono::Duration::days(20)),
                note: String::new(),
            });
        }

        // PP truba o'rniga PE — arzonroq, lekin hali tasdiqlanmagan.
        let pipe_alt = alt_mat(
            "M-402",
            if ru {
                "Труба ПЭ 110 канализационная"
            } else {
                "PE 110 kanalizatsiya trubasi"
            },
            if ru { "м" } else { "m" },
            Section::Vk,
            36_000.0,
        );
        if let Some(pipe) = by_code("M-401") {
            self.insert_material_alt(&MaterialAlt {
                id: 0,
                project_id: pid,
                material_id: pipe,
                alt_id: pipe_alt,
                approved_by: String::new(),
                approved_at: None,
                note: String::new(),
            });
        }

        // Sarf normalari (TZ XI.15): armaturada ortiqcha sarf ataylab
        // qoldirilgan — «Normativ / fakt» ko'rinishi shuni ko'rsatadi.
        let norm = |task: Option<i64>, material_id: i64, per_unit: f64, tolerance: f64| {
            if let Some(task_id) = task {
                self.insert_material_norm(&MaterialNorm {
                    id: 0,
                    project_id: pid,
                    task_id,
                    material_id,
                    per_unit,
                    tolerance,
                    note: String::new(),
                });
            }
        };
        // Armaturada karkas bo'yicha ortiqcha sarf ataylab qoldirilgan —
        // qolgan uchtasi ruxsat chegarasida.
        norm(karkas, rebar, 0.0235, 3.0);
        norm(karkas, concrete, 0.75, 2.0);
        norm(plita, rebar, 0.0225, 3.0);
        norm(devor, brick, 18.0, 4.0);
    }

    /// PPR kartalari, jurnal yozuvlari va ijro hujjatlari namunasi.
    /// Bu yerda ham bir nechta nomuvofiqlik ataylab qoldirilgan.
    fn seed_demo_execution(&self, pid: i64, ru: bool) {
        let tasks = self.tasks(pid).unwrap_or_default();
        let by_wbs = |w: &str| tasks.iter().find(|t| t.wbs == w).cloned();
        let today = chrono::Local::now().date_naive();

        let author = if ru {
            "ПТО: Саидова М.И."
        } else {
            "PTO: Saidova M.I."
        };
        let card = |kind: PprKind,
                    number: &str,
                    name: &str,
                    section: Section,
                    task_id: Option<i64>,
                    workers: i64,
                    machines: i64,
                    approved: bool| {
            self.insert_ppr(&PprDoc {
                id: 0,
                project_id: pid,
                kind,
                number: number.into(),
                name: name.into(),
                section,
                task_id,
                workers,
                machines,
                path: String::new(),
                approved,
                author: author.into(),
                approved_at: approved.then(|| today - chrono::Duration::days(60)),
                note: String::new(),
            });
        };

        // Tasdiqlangan karta — to'g'ri holat.
        card(
            PprKind::TechCard,
            "TK-03",
            if ru {
                "Устройство фундаментной плиты"
            } else {
                "Poydevor plitasini qurish"
            },
            Section::Kj,
            by_wbs("3").map(|t| t.id),
            24,
            3,
            true,
        );
        // Ish 65 % bajarilgan, karta esa tasdiqlanmagan — kritik nomuvofiqlik.
        card(
            PprKind::TechCard,
            "TK-07",
            if ru {
                "Монолитный каркас, этажи 7-9"
            } else {
                "Monolit karkas, 7-9 qavat"
            },
            Section::Kj,
            by_wbs("7").map(|t| t.id),
            32,
            4,
            false,
        );
        card(
            PprKind::QualityCard,
            "KK-02",
            if ru {
                "Контроль бетонных работ"
            } else {
                "Beton ishlarini nazorat qilish"
            },
            Section::Kj,
            by_wbs("3").map(|t| t.id),
            0,
            0,
            true,
        );
        card(
            PprKind::TechCard,
            "TK-09",
            if ru {
                "Кладка наружных стен"
            } else {
                "Tashqi devorlar g'ishtligi"
            },
            Section::Ar,
            by_wbs("9").map(|t| t.id),
            18,
            2,
            true,
        );
        card(
            PprKind::TechCard,
            "TK-14",
            if ru {
                "Электромонтажные работы"
            } else {
                "Elektromontaj ishlari"
            },
            Section::Eom,
            by_wbs("14").map(|t| t.id),
            14,
            1,
            true,
        );
        // Ishga bog'lanmagan karta.
        card(
            PprKind::SafetyCard,
            "XK-01",
            if ru {
                "Работы на высоте"
            } else {
                "Balandlikdagi ishlar"
            },
            Section::None,
            None,
            0,
            0,
            true,
        );

        // Jurnal: oxirgi uch kun.
        let entry = |days_ago: i64,
                     task_id: Option<i64>,
                     volume: f64,
                     unit: &str,
                     text: &str,
                     workers: i64,
                     machines: i64| {
            self.insert_journal(&JournalEntry {
                id: 0,
                project_id: pid,
                date: today - chrono::Duration::days(days_ago),
                author: if ru {
                    "Юсупов Б.Р.".into()
                } else {
                    "Yusupov B.R.".to_string()
                },
                weather: if ru {
                    "Ясно".into()
                } else {
                    "Ochiq".to_string()
                },
                temperature: 24.0,
                workers,
                machines,
                task_id,
                volume,
                unit: unit.into(),
                text: text.into(),
                remarks: String::new(),
                photos: String::new(),
                gps: String::new(),
            });
        };
        // 7-ish hozir bajarilmoqda — jurnal yozuvlari shunga tegishli.
        let karkas = by_wbs("7").map(|t| t.id);
        entry(
            2,
            karkas,
            420.0,
            "m3",
            if ru {
                "Бетонирование колонн 8 этажа"
            } else {
                "8-qavat ustunlarini betonlash"
            },
            28,
            3,
        );
        entry(
            1,
            karkas,
            380.0,
            "m3",
            if ru {
                "Бетонирование перекрытия 8 этажа"
            } else {
                "8-qavat oralig'ini betonlash"
            },
            31,
            4,
        );
        entry(
            0,
            karkas,
            365.0,
            "m3",
            if ru {
                "Армирование колонн 9 этажа"
            } else {
                "9-qavat ustunlarini armaturalash"
            },
            26,
            2,
        );

        // Ijro hujjatlari: 2 va 3-ishlar hujjatlangan, 1-ish esa yo'q.
        let doc = |kind: ExecDocKind,
                   number: &str,
                   name: &str,
                   task_id: Option<i64>,
                   status: ExecDocStatus| {
            self.insert_exec_doc(&ExecDoc {
                id: 0,
                project_id: pid,
                kind,
                number: number.into(),
                name: name.into(),
                date: today - chrono::Duration::days(30),
                task_id,
                status,
                responsible: if ru {
                    "Рахимов Ш.А.".into()
                } else {
                    "Rahimov Sh.A.".to_string()
                },
                version: 1,
                replaces: None,
                note: String::new(),
            });
        };
        doc(
            ExecDocKind::Hidden,
            "AOSR-014",
            if ru {
                "Акт на устройство котлована"
            } else {
                "Kotlovan qurilishi dalolatnomasi"
            },
            by_wbs("2").map(|t| t.id),
            ExecDocStatus::Signed,
        );
        doc(
            ExecDocKind::Hidden,
            "AOSR-021",
            if ru {
                "Акт на армирование плиты"
            } else {
                "Plita armaturasi dalolatnomasi"
            },
            by_wbs("3").map(|t| t.id),
            ExecDocStatus::OnReview,
        );

        // Rad etilgan hujjat va uning ikkinchi versiyasi (TZ IV.19).
        // Eskisi arxivda qoladi — nima sababdan qayta ishlangani ko'rinishi
        // kerak, shuning uchun o'chirilmaydi.
        let rejected = self.insert_exec_doc(&ExecDoc {
            id: 0,
            project_id: pid,
            kind: ExecDocKind::Scheme,
            number: "IS-007".into(),
            name: if ru {
                "Исполнительная схема плиты 3 этажа".into()
            } else {
                "3-qavat plitasi ijro sxemasi".to_string()
            },
            date: today - chrono::Duration::days(24),
            task_id: by_wbs("3").map(|t| t.id),
            status: ExecDocStatus::Rejected,
            responsible: if ru {
                "Рахимов Ш.А.".into()
            } else {
                "Rahimov Sh.A.".to_string()
            },
            version: 1,
            replaces: None,
            note: if ru {
                "Отметки не совпадают с проектом".into()
            } else {
                "Belgilar loyihaga mos kelmadi".to_string()
            },
        });
        self.insert_exec_doc(&ExecDoc {
            id: 0,
            project_id: pid,
            kind: ExecDocKind::Scheme,
            number: "IS-007/2".into(),
            name: if ru {
                "Исполнительная схема плиты 3 этажа".into()
            } else {
                "3-qavat plitasi ijro sxemasi".to_string()
            },
            date: today - chrono::Duration::days(12),
            task_id: by_wbs("3").map(|t| t.id),
            status: ExecDocStatus::OnReview,
            responsible: if ru {
                "Рахимов Ш.А.".into()
            } else {
                "Rahimov Sh.A.".to_string()
            },
            version: 2,
            replaces: Some(rejected),
            note: String::new(),
        });
    }

    /// Smeta namunasi. Pozitsiyalar nomi GPR ishlari nomiga mos qilib olinadi —
    /// shunda hajm va birlik bo'yicha solishtirish ishlaydi (TZ III.5-III.6).
    /// Smetaning ikkinchi varianti (TZ III.21).
    ///
    /// Variantlarni solishtirish funksiyasi bitta smeta bilan ishlamaydi:
    /// namunada ikkinchi variant bo'lishi kerak. U qayta ko'rib chiqilgan
    /// narxlar bilan — shuning uchun farq bo'lim kesimida ko'rinadi.
    fn seed_demo_estimate_alt(&self, pid: i64, ru: bool) {
        if self.estimates(pid).len() > 1 {
            return;
        }
        let Some(first) = self.estimates(pid).into_iter().next() else {
            return;
        };
        let items = self.estimate_items(first.id);
        if items.is_empty() {
            return;
        }

        let eid = self.insert_estimate(&Estimate {
            id: 0,
            project_id: pid,
            name: if ru {
                "Смета № 2 — после пересмотра цен".into()
            } else {
                "2-son smeta — narxlar qayta ko'rilgandan keyin".into()
            },
            currency: "UZS".into(),
            declared_total: 0.0,
            overhead_pct: 12.0,
            profit_pct: 7.0,
            vat_pct: 12.0,
            added_at: String::new(),
        });
        if eid == 0 {
            return;
        }

        // Ikkinchi variant birinchisidan tuziladi: shunda solishtirish
        // haqiqiy bo'ladi — bir xil ishlar, boshqa narx va hajm.
        for (i, it) in items.iter().enumerate() {
            // Har uchinchi pozitsiya arzonlashgan, qolganlari biroz qimmatlashgan.
            let k = if i % 3 == 0 { 0.88 } else { 1.06 };
            // Dublikat pozitsiya ikkinchi variantda olib tashlangan.
            if i == 5 {
                continue;
            }
            let price = (it.price * k / 1000.0).round() * 1000.0;
            self.insert_estimate_item(&EstimateItem {
                id: 0,
                estimate_id: eid,
                pos: (i + 1) as i64,
                section: it.section,
                code: it.code.clone(),
                name: it.name.clone(),
                unit: it.unit.clone(),
                qty: it.qty,
                price,
                cost: it.qty * price,
                task_id: it.task_id,
                note: String::new(),
            });
        }
    }

    fn seed_demo_estimate(&self, pid: i64, ru: bool) {
        let tasks = self.tasks(pid).unwrap_or_default();
        let by_wbs = |w: &str| tasks.iter().find(|t| t.wbs == w).cloned();

        let eid = self.insert_estimate(&Estimate {
            id: 0,
            project_id: pid,
            name: if ru {
                "Смета № 1 — общестроительные работы".into()
            } else {
                "1-son smeta — umumiy qurilish ishlari".into()
            },
            currency: "UZS".into(),
            declared_total: 0.0,
            // O'zbekistonda odatdagi darajalar: ustama 14%, foyda 8%, QQS 12%.
            overhead_pct: 14.0,
            profit_pct: 8.0,
            vat_pct: 12.0,
            added_at: String::new(),
        });
        if eid == 0 {
            return;
        }

        let mut pos = 0;
        let mut total = 0.0;
        #[allow(clippy::too_many_arguments)]
        let mut add = |section: Section,
                       code: &str,
                       name: String,
                       unit: &str,
                       qty: f64,
                       price: f64,
                       cost: f64,
                       task_id: Option<i64>| {
            pos += 1;
            total += cost;
            self.insert_estimate_item(&EstimateItem {
                id: 0,
                estimate_id: eid,
                pos,
                section,
                code: code.into(),
                name,
                unit: unit.into(),
                qty,
                price,
                cost,
                task_id,
                note: String::new(),
            });
        };

        // 1. To'g'ri pozitsiya: hajm ham, arifmetika ham loyihaga mos.
        if let Some(t) = by_wbs("3") {
            add(
                Section::Kj,
                "E6-1-1",
                t.name.clone(),
                "m3",
                t.volume,
                1_250_000.0,
                t.volume * 1_250_000.0,
                Some(t.id),
            );
        }
        // 2. Hajm loyihadagidan 18 % ga oshirilgan.
        if let Some(t) = by_wbs("5") {
            let qty = t.volume * 1.18;
            add(
                Section::Kj,
                "E6-1-22",
                t.name.clone(),
                "m3",
                qty,
                1_480_000.0,
                qty * 1_480_000.0,
                Some(t.id),
            );
        }
        // 3. Birlik loyihadagiga mos emas: m2 o'rniga m3.
        if let Some(t) = by_wbs("9") {
            add(
                Section::Ar,
                "E8-2-1",
                t.name.clone(),
                "m3",
                950.0,
                2_100_000.0,
                950.0 * 2_100_000.0,
                Some(t.id),
            );
        }
        // 4. Arifmetik xato: miqdor x narx summaga teng emas.
        if let Some(t) = by_wbs("17") {
            add(
                Section::Ar,
                "E10-1-4",
                t.name.clone(),
                "m2",
                1450.0,
                1_850_000.0,
                2_900_000_000.0,
                Some(t.id),
            );
        }
        // 5-6. Dublikat: bir xil nom, bir xil miqdor.
        let dup = if ru {
            "Устройство стяжки пола, 50 мм".to_string()
        } else {
            "Pol styashkasini qurish, 50 mm".to_string()
        };
        add(
            Section::Ar,
            "E11-1-9",
            dup.clone(),
            "m2",
            4200.0,
            185_000.0,
            4200.0 * 185_000.0,
            None,
        );
        add(
            Section::Ar,
            "E11-1-9",
            dup,
            "m2",
            4200.0,
            185_000.0,
            4200.0 * 185_000.0,
            None,
        );
        // 7. Noma'lum o'lchov birligi.
        add(
            Section::Ar,
            "E12-3-2",
            if ru {
                "Гидроизоляция рулонная".into()
            } else {
                "Rulonli gidroizolyatsiya".to_string()
            },
            if ru { "рулон" } else { "rulon" },
            860.0,
            420_000.0,
            860.0 * 420_000.0,
            None,
        );
        // 8. Nol miqdor.
        add(
            Section::Km,
            "E9-1-3",
            if ru {
                "Монтаж закладных деталей".into()
            } else {
                "Zakladnoy detallarni montaj qilish".to_string()
            },
            "t",
            0.0,
            9_800_000.0,
            0.0,
            None,
        );
        // 9-10. Bir xil rasenka kodi, narx 42 % ga farq qiladi.
        add(
            Section::Eom,
            "E21-1-5",
            if ru {
                "Прокладка кабеля ВВГнг 3х2,5".into()
            } else {
                "VVGng 3x2,5 kabelini yotqizish".to_string()
            },
            "m",
            9200.0,
            38_000.0,
            9200.0 * 38_000.0,
            None,
        );
        add(
            Section::Eom,
            "E21-1-5",
            if ru {
                "Прокладка кабеля ВВГнг 3х2,5 (2 этап)".into()
            } else {
                "VVGng 3x2,5 kabelini yotqizish (2-bosqich)".to_string()
            },
            "m",
            9300.0,
            54_000.0,
            9300.0 * 54_000.0,
            None,
        );

        // Hujjatdagi yakun pozitsiyalar yig'indisiga teng emas — TZ III.4.
        let declared = total + 145_000_000.0;
        let _ = self.conn().execute(
            "UPDATE estimate SET declared_total=?2 WHERE id=?1",
            params![eid, declared],
        );
    }
}

/// Namunaviy tekshiruv: tur, raqam, reja kuni (orqaga), o'tkazilgan kuni,
/// joy (uz), joy (ru), natija, bartaraf muddati, bartaraf etilgan sana, VBS.
type InspectionDef = (
    InspectionKind,
    &'static str,
    i64,
    Option<i64>,
    &'static str,
    &'static str,
    InspectionResult,
    Option<i64>,
    Option<i64>,
    &'static str,
);

/// Namunaviy beton sinovi: namuna, marka, konstruksiya (uz), konstruksiya (ru),
/// quyilgan kun (orqaga), yosh, talab (MPa), natija (MPa).
type ConcreteDef = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    i64,
    i64,
    f64,
    Option<f64>,
);

/// Namunaviy shartnoma o'zgarishi: shartnoma, raqam, tur, kun (orqaga),
/// tavsif (uz), tavsif (ru), summa, kun, holat.
type ChangeDef = (
    i64,
    &'static str,
    ChangeKind,
    i64,
    &'static str,
    &'static str,
    f64,
    i64,
    ChangeStatus,
);

/// Namunaviy to'lov bosqichi: shartnoma, raqam, asos (uz), asos (ru),
/// muddat (orqaga), summa, to'langan, to'langan kun.
type StageDef = (
    i64,
    &'static str,
    &'static str,
    &'static str,
    i64,
    f64,
    f64,
    Option<i64>,
);

/// Namunaviy qabul hujjati: VBS, raqam, kun (orqaga), hajm, birlik, summa, holat.
type AcceptDef = (
    &'static str,
    &'static str,
    i64,
    f64,
    &'static str,
    f64,
    AcceptState,
);

/// Namunaviy nomuvofiqlik tavsifi: modul, bo'lim, kod, varaq, joy, element,
/// sarlavha (uz/ru), tavsif (uz/ru), og'irlik, norma hujjati va bandi,
/// tavsiya (uz/ru), mas'ul, muddat (kun, orqaga), holat, avtomatikmi.
type IssueDef = (
    IssueModule,
    Section,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Severity,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Option<i64>,
    IssueStatus,
    bool,
);

/// Namunaviy ijro hujjati: tur, raqam, nomi (uz/ru), VBS, kun (orqaga), holat.
type ExecDocDef = (
    ExecDocKind,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    i64,
    ExecDocStatus,
);

/// Namunaviy sifat tekshiruvi: tur, kun (orqaga), mavzu (uz/ru), natija,
/// nuqson (uz/ru), bartaraf muddati, bartaraf etilgan kun.
type QualityDef = (
    QualityKind,
    i64,
    &'static str,
    &'static str,
    QualityResult,
    &'static str,
    &'static str,
    Option<i64>,
    Option<i64>,
);

/// Namunaviy xavfsizlik hodisasi: tur, og'irlik, kun (orqaga), joy (uz/ru),
/// tavsif (uz/ru), chora (uz/ru), holat.
type SafetyDef = (
    SafetyKind,
    Severity,
    i64,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    IssueStatus,
    RootCause,
);

/// Namunaviy ishchi: ism (uz/ru), lavozim (uz/ru), soatlik stavka, brigada.
type WorkerDef = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    f64,
    usize,
);

/// Namunaviy material: kod, nomi (uz/ru), birlik, bo'lim, minimal zaxira,
/// narx, sertifikat raqami.
type MaterialDef = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Section,
    f64,
    f64,
    &'static str,
);

/// Namunaviy ariza: raqam, kun (orqaga), tur, mavzu (uz/ru), material kodi,
/// miqdor, birlik, muhimlik, holat, VBS, rad sababi (uz/ru).
type RequestDef = (
    &'static str,
    i64,
    RequestKind,
    &'static str,
    &'static str,
    &'static str,
    f64,
    &'static str,
    Priority,
    RequestStatus,
    &'static str,
    &'static str,
    &'static str,
);

/// Namunaviy xarid: raqam, ariza raqami, kun (orqaga), yetkazib beruvchi,
/// mavzu (uz/ru), miqdor, birlik, narx, yetkazish kuni, holat, kelgan
/// miqdor, bo'lim.
type PurchaseDef = (
    &'static str,
    &'static str,
    i64,
    &'static str,
    &'static str,
    &'static str,
    f64,
    &'static str,
    f64,
    i64,
    PurchaseStatus,
    f64,
    Section,
);

/// Namunaviy asbob berish: asbob indeksi, ishchi indeksi, berilgan kun
/// (orqaga), qaytarish muddati, qaytarilgan kun.
type ToolIssueDef = (usize, usize, i64, Option<i64>, Option<i64>);

/// Namunaviy asbob: kod, nomi (uz/ru), tur, inventar raqami, narx, holat,
/// tekshiruv sanasi (kun, orqaga).
type ToolDef = (
    &'static str,
    &'static str,
    &'static str,
    ToolKind,
    &'static str,
    f64,
    ToolCondition,
    Option<i64>,
);

/// Namunaviy sinov: tur, raqam, mavzu (uz/ru), kun (orqaga), qiymat,
/// talab, birlik, natija, VBS.
type LabTestDef = (
    LabTestKind,
    &'static str,
    &'static str,
    &'static str,
    i64,
    Option<f64>,
    Option<f64>,
    &'static str,
    LabTestResult,
    &'static str,
);

/// Namunaviy ta'mir: texnika indeksi, tur, boshlangan kun (orqaga),
/// tugagan kun, sabab (uz/ru), xarajat.
type MachineRepairDef = (
    usize,
    RepairKind,
    i64,
    Option<i64>,
    &'static str,
    &'static str,
    f64,
);

/// Namunaviy xavfsizlik zonasi: tur, nomi (uz/ru), joy (uz/ru),
/// chora (uz/ru), tekshiruv kuni (orqaga), chora ko'rilganmi.
type ZoneDef = (
    ZoneKind,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    i64,
    bool,
);

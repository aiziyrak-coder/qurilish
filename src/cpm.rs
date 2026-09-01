//! Расчет сетевого графика: CPM (Critical Path Method).
//!
//! Прямой проход даёт ранние даты (ES/EF), обратный — поздние (LS/LF).
//! Разница даёт общий резерв; работы с нулевым резервом образуют критический путь.
//! Даты хранятся как смещение в днях от начала проекта, чтобы расчет был целочисленным.

use crate::model::{Link, LinkType, Task};
use chrono::NaiveDate;
use std::collections::HashMap;

/// Результат расчета по одной работе.
#[derive(Debug, Clone, Copy)]
pub struct Calc {
    /// Ранний старт, дней от начала проекта.
    pub es: i64,
    /// Раннее окончание (включительно).
    pub ef: i64,
    /// Поздний старт.
    pub ls: i64,
    /// Позднее окончание.
    pub lf: i64,
    /// Общий резерв времени, дней.
    pub slack: i64,
    /// Работа лежит на критическом пути.
    pub critical: bool,
}

/// Результат расчета всего графика.
#[derive(Debug, Clone, Default)]
pub struct Schedule {
    pub calc: HashMap<i64, Calc>,
    /// Длительность проекта, дней.
    pub project_days: i64,
    /// Работы, участвующие в цикле зависимостей (расчет для них недостоверен).
    pub cycles: Vec<i64>,
}

impl Schedule {
    pub fn get(&self, task_id: i64) -> Option<Calc> {
        self.calc.get(&task_id).copied()
    }

    pub fn is_critical(&self, task_id: i64) -> bool {
        self.calc.get(&task_id).map(|c| c.critical).unwrap_or(false)
    }
}

/// Топологическая сортировка (алгоритм Кана). Возвращает порядок и список работ в циклах.
fn topo_order(tasks: &[Task], links: &[Link]) -> (Vec<i64>, Vec<i64>) {
    let ids: Vec<i64> = tasks.iter().map(|t| t.id).collect();
    let mut indeg: HashMap<i64, usize> = ids.iter().map(|&i| (i, 0)).collect();
    let mut out: HashMap<i64, Vec<i64>> = ids.iter().map(|&i| (i, Vec::new())).collect();

    for l in links {
        if !indeg.contains_key(&l.pred) || !indeg.contains_key(&l.succ) {
            continue; // связь на удалённую работу
        }
        out.entry(l.pred).or_default().push(l.succ);
        *indeg.entry(l.succ).or_default() += 1;
    }

    let mut queue: Vec<i64> = ids.iter().copied().filter(|i| indeg[i] == 0).collect();
    let mut order = Vec::with_capacity(ids.len());

    while let Some(id) = queue.pop() {
        order.push(id);
        for succ in out.get(&id).cloned().unwrap_or_default() {
            let d = indeg.entry(succ).or_default();
            *d -= 1;
            if *d == 0 {
                queue.push(succ);
            }
        }
    }

    let cycles: Vec<i64> = ids.into_iter().filter(|i| indeg[i] > 0).collect();
    (order, cycles)
}

/// Полный расчет графика. `origin` — дата начала проекта (нулевой день).
pub fn compute(tasks: &[Task], links: &[Link], origin: NaiveDate) -> Schedule {
    if tasks.is_empty() {
        return Schedule::default();
    }

    let by_id: HashMap<i64, &Task> = tasks.iter().map(|t| (t.id, t)).collect();
    let (order, cycles) = topo_order(tasks, links);

    // Входящие связи для прямого прохода, исходящие — для обратного.
    let mut incoming: HashMap<i64, Vec<&Link>> = HashMap::new();
    let mut outgoing: HashMap<i64, Vec<&Link>> = HashMap::new();
    for l in links {
        if by_id.contains_key(&l.pred) && by_id.contains_key(&l.succ) {
            incoming.entry(l.succ).or_default().push(l);
            outgoing.entry(l.pred).or_default().push(l);
        }
    }

    let dur = |id: i64| -> i64 { by_id.get(&id).map(|t| t.duration.max(1)).unwrap_or(1) };

    // ---- Прямой проход: ранние даты ----
    let mut es: HashMap<i64, i64> = HashMap::new();
    for &id in &order {
        let t = by_id[&id];
        // Закреплённая работа не может начаться раньше своей плановой даты.
        let floor = if t.pinned {
            (t.plan_start - origin).num_days()
        } else {
            0
        };
        let mut start = floor;

        for l in incoming.get(&id).cloned().unwrap_or_default() {
            let p_es = *es.get(&l.pred).unwrap_or(&0);
            let p_ef = p_es + dur(l.pred) - 1;
            let candidate = match l.kind {
                LinkType::Fs => p_ef + 1 + l.lag,
                LinkType::Ss => p_es + l.lag,
                LinkType::Ff => p_ef + l.lag - (dur(id) - 1),
                LinkType::Sf => p_es + l.lag - (dur(id) - 1),
            };
            start = start.max(candidate);
        }
        es.insert(id, start.max(0));
    }
    // Работы в циклах не получили значения — ставим их плановую дату.
    for t in tasks {
        es.entry(t.id)
            .or_insert_with(|| ((t.plan_start - origin).num_days()).max(0));
    }

    let project_end = tasks
        .iter()
        .map(|t| es[&t.id] + dur(t.id) - 1)
        .max()
        .unwrap_or(0);

    // ---- Обратный проход: поздние даты ----
    let mut lf: HashMap<i64, i64> = HashMap::new();
    for &id in order.iter().rev() {
        let succs = outgoing.get(&id).cloned().unwrap_or_default();
        let mut finish = if succs.is_empty() {
            project_end
        } else {
            i64::MAX
        };
        for l in succs {
            let s_lf = *lf.get(&l.succ).unwrap_or(&project_end);
            let s_ls = s_lf - dur(l.succ) + 1;
            let candidate = match l.kind {
                LinkType::Fs => s_ls - 1 - l.lag,
                LinkType::Ss => s_ls - l.lag + dur(id) - 1,
                LinkType::Ff => s_lf - l.lag,
                LinkType::Sf => s_lf - l.lag + dur(id) - 1,
            };
            finish = finish.min(candidate);
        }
        if finish == i64::MAX {
            finish = project_end;
        }
        lf.insert(id, finish);
    }
    for t in tasks {
        lf.entry(t.id).or_insert(project_end);
    }

    let mut calc = HashMap::with_capacity(tasks.len());
    for t in tasks {
        let d = dur(t.id);
        let es_v = es[&t.id];
        let lf_v = lf[&t.id];
        let slack = lf_v - (es_v + d - 1);
        calc.insert(
            t.id,
            Calc {
                es: es_v,
                ef: es_v + d - 1,
                ls: lf_v - d + 1,
                lf: lf_v,
                slack,
                critical: slack <= 0 && !cycles.contains(&t.id),
            },
        );
    }

    Schedule {
        calc,
        project_days: project_end + 1,
        cycles,
    }
}

/// Сводка План/Факт по проекту на заданную дату (ТЗ I.2).
#[derive(Debug, Clone, Default)]
pub struct Progress {
    /// Плановый процент выполнения проекта на сегодня.
    pub plan_pct: f64,
    /// Фактический процент выполнения.
    pub fact_pct: f64,
    /// Просроченные работы: не завершены, а плановое окончание уже прошло.
    pub overdue: Vec<i64>,
    /// Работы, идущие сегодня.
    pub in_progress: Vec<i64>,
    /// Прогнозная дата окончания строительства.
    pub forecast_end: Option<NaiveDate>,
    /// Отставание от плана в днях (отрицательное — опережение).
    pub delay_days: i64,
}

/// Вес работы в проекте — по длительности. Так план/факт не зависит от числа работ.
fn weight(t: &Task) -> f64 {
    t.duration.max(1) as f64
}

/// Расчет план/факт и прогноза окончания.
pub fn progress(tasks: &[Task], sched: &Schedule, origin: NaiveDate, today: NaiveDate) -> Progress {
    if tasks.is_empty() {
        return Progress::default();
    }
    let today_off = (today - origin).num_days();
    let total_w: f64 = tasks.iter().map(weight).sum();

    let mut plan_done = 0.0;
    let mut fact_done = 0.0;
    let mut overdue = Vec::new();
    let mut in_progress = Vec::new();

    for t in tasks {
        let c = match sched.get(t.id) {
            Some(c) => c,
            None => continue,
        };
        let w = weight(t);
        let d = t.duration.max(1) as f64;

        // Плановая доля работы, которая должна быть выполнена на сегодня.
        let elapsed = (today_off - c.es + 1).clamp(0, t.duration.max(1)) as f64;
        plan_done += w * (elapsed / d);
        fact_done += w * (t.progress / 100.0);

        let done = t.progress >= 99.999 || t.fact_end.is_some();
        if !done && today_off > c.ef {
            overdue.push(t.id);
        }
        if !done && today_off >= c.es && today_off <= c.ef {
            in_progress.push(t.id);
        }
    }

    let plan_pct = (plan_done / total_w * 100.0).clamp(0.0, 100.0);
    let fact_pct = (fact_done / total_w * 100.0).clamp(0.0, 100.0);

    // Прогноз: если фактический темп ниже планового, конец сдвигается пропорционально.
    let plan_end_off = sched.project_days - 1;
    let forecast_off = if fact_pct > 0.5 && plan_pct > 0.5 {
        let rate = fact_pct / plan_pct;
        if rate >= 1.0 {
            plan_end_off
        } else {
            let remaining = (plan_end_off - today_off).max(0) as f64;
            today_off + (remaining / rate.max(0.05)).round() as i64
        }
    } else {
        plan_end_off
    };

    Progress {
        plan_pct,
        fact_pct,
        overdue,
        in_progress,
        forecast_end: Some(origin + chrono::Duration::days(forecast_off)),
        delay_days: forecast_off - plan_end_off,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Section;

    fn task(id: i64, dur: i64) -> Task {
        Task {
            id,
            project_id: 1,
            wbs: String::new(),
            name: format!("T{id}"),
            section: Section::None,
            responsible: String::new(),
            duration: dur,
            plan_start: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            fact_start: None,
            fact_end: None,
            progress: 0.0,
            pinned: false,
            volume: 0.0,
            unit: String::new(),
        }
    }

    fn fs(id: i64, pred: i64, succ: i64) -> Link {
        Link {
            id,
            pred,
            succ,
            kind: LinkType::Fs,
            lag: 0,
        }
    }

    /// Классическая цепочка A(3) → B(5) → D(2), параллельно A → C(2) → D.
    /// Критический путь идёт через B, у C появляется резерв 3 дня.
    #[test]
    fn critical_path_and_slack() {
        let origin = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let tasks = vec![task(1, 3), task(2, 5), task(3, 2), task(4, 2)];
        let links = vec![fs(1, 1, 2), fs(2, 1, 3), fs(3, 2, 4), fs(4, 3, 4)];
        let s = compute(&tasks, &links, origin);

        assert_eq!(s.get(1).unwrap().es, 0);
        assert_eq!(s.get(2).unwrap().es, 3);
        assert_eq!(s.get(4).unwrap().es, 8);
        assert_eq!(s.project_days, 10);

        assert!(s.is_critical(1));
        assert!(s.is_critical(2));
        assert!(s.is_critical(4));
        assert!(!s.is_critical(3));
        assert_eq!(s.get(3).unwrap().slack, 3);
    }

    #[test]
    fn lag_shifts_successor() {
        let origin = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let tasks = vec![task(1, 3), task(2, 2)];
        let mut l = fs(1, 1, 2);
        l.lag = 4;
        let s = compute(&tasks, &[l], origin);
        assert_eq!(s.get(2).unwrap().es, 7);
    }

    #[test]
    fn start_to_start_runs_parallel() {
        let origin = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let tasks = vec![task(1, 10), task(2, 4)];
        let l = Link {
            id: 1,
            pred: 1,
            succ: 2,
            kind: LinkType::Ss,
            lag: 2,
        };
        let s = compute(&tasks, &[l], origin);
        assert_eq!(s.get(2).unwrap().es, 2);
    }

    /// Цикл не должен приводить к зависанию или панике.
    #[test]
    fn cycle_is_detected_not_fatal() {
        let origin = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let tasks = vec![task(1, 2), task(2, 2)];
        let links = vec![fs(1, 1, 2), fs(2, 2, 1)];
        let s = compute(&tasks, &links, origin);
        assert_eq!(s.cycles.len(), 2);
        assert!(!s.is_critical(1));
    }

    #[test]
    fn pinned_task_does_not_move_earlier() {
        let origin = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let mut t = task(1, 3);
        t.pinned = true;
        t.plan_start = NaiveDate::from_ymd_opt(2026, 1, 11).unwrap();
        let s = compute(&[t], &[], origin);
        assert_eq!(s.get(1).unwrap().es, 10);
    }

    #[test]
    fn overdue_task_is_reported() {
        let origin = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let tasks = vec![task(1, 5)];
        let s = compute(&tasks, &[], origin);
        let today = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
        let p = progress(&tasks, &s, origin, today);
        assert_eq!(p.overdue, vec![1]);
        assert!(p.fact_pct < 0.001);
    }
}

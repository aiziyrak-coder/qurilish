//! Ikki tilli interfeys / Двуязычный интерфейс.
//!
//! Til global holatda saqlanadi: interfeys bir oqimda chiziladi, shuning uchun
//! har bir funksiyaga `Lang` uzatishdan ko'ra global qiymat qulayroq.

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Uz,
    Ru,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::Uz, Lang::Ru];

    /// Tilning o'z nomi — ro'yxatda har doim o'z tilida ko'rsatiladi.
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::Uz => "O'zbekcha",
            Lang::Ru => "Русский",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::Uz => "uz",
            Lang::Ru => "ru",
        }
    }

    pub fn parse(s: &str) -> Lang {
        match s {
            "ru" => Lang::Ru,
            _ => Lang::Uz,
        }
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set_lang(l: Lang) {
    CURRENT.store(if l == Lang::Ru { 1 } else { 0 }, Ordering::Relaxed);
}

pub fn lang() -> Lang {
    if CURRENT.load(Ordering::Relaxed) == 1 {
        Lang::Ru
    } else {
        Lang::Uz
    }
}

/// Kalit bo'yicha tarjima. Kalit topilmasa «?» qaytadi — yetishmayotgan
/// satr interfeysda darrov ko'zga tashlanadi.
pub fn t(key: &str) -> &'static str {
    match lookup(key) {
        Some((uz, ru)) => match lang() {
            Lang::Uz => uz,
            Lang::Ru => ru,
        },
        // Kalit topilmadi — uni interfeysda ko'rsatib, e'tiborni tortamiz.
        None => "?",
    }
}

/// Ikkala tildagi matn. Yangi satr qo'shganda shu ro'yxatga qo'shiladi.
fn lookup(key: &str) -> Option<(&'static str, &'static str)> {
    let pair = match key {
        // ---------- Umumiy / Общее ----------
        "app_subtitle" => ("Qurilish intellektual platformasi", "Строительная интеллектуальная платформа"),
        "object" => ("Obyekt:", "Объект:"),
        "no_object" => ("— tanlanmagan —", "— не выбран —"),
        "no_object_selected" => ("Obyekt tanlanmagan", "Объект не выбран"),
        "no_objects_yet" => ("Hozircha obyektlar yo'q", "Объектов пока нет"),
        "create_first_object" => (
            "Chap paneldagi tugma orqali birinchi obyektni yarating.",
            "Создайте первый объект кнопкой в левой панели.",
        ),
        "new_object" => ("+ Yangi obyekt", "+ Новый объект"),
        "new_object_name" => ("Yangi obyekt", "Новый объект"),
        "local_db" => ("Lokal baza:", "Локальная база:"),
        "cancel" => ("Bekor qilish", "Отмена"),
        "delete" => ("O'chirish", "Удалить"),
        "create" => ("Yaratish", "Создать"),
        "days_short" => ("kun", "дн."),
        "days" => ("kun", "дней"),
        "pcs_short" => ("dona", "шт."),
        "yes" => ("ha", "да"),
        "no" => ("yo'q", "нет"),
        "dash" => ("—", "—"),

        // ---------- Navigatsiya / Навигация ----------
        "nav_ai" => ("AI TEKSHIRUV", "AI ПРОВЕРКА"),
        "nav_exec" => ("IJRO VA NAZORAT", "ИСПОЛНЕНИЕ И КОНТРОЛЬ"),
        "nav_system" => ("TIZIM", "СИСТЕМА"),
        "screen_dashboard" => ("Umumiy ko'rinish", "Обзор"),
        "screen_passport" => ("Obyekt pasporti", "Паспорт объекта"),
        "screen_gantt" => ("GPR — ishlar grafigi", "ГПР — график работ"),
        "screen_ppr" => ("PPR", "ППР"),
        "screen_ai_check" => ("AI loyiha tekshiruvi", "AI проверка проекта"),
        "screen_estimate" => ("AI smeta tekshiruvi", "AI проверка смет"),
        "screen_journal" => ("Ishlar jurnali", "Журнал работ"),
        "screen_quality" => ("Sifat", "Качество"),
        "screen_settings" => ("Sozlamalar", "Настройки"),

        // ---------- Obyekt statusi / Статус объекта ----------
        "status_design" => ("Loyihalash", "Проектирование"),
        "status_tender" => ("Tender", "Тендер"),
        "status_in_progress" => ("Qurilish", "Строительство"),
        "status_suspended" => ("To'xtatilgan", "Приостановлен"),
        "status_completed" => ("Yakunlangan", "Завершен"),

        // ---------- Ishtirokchilar / Участники ----------
        "role_client" => ("Buyurtmachi", "Заказчик"),
        "role_contractor" => ("Bosh pudratchi", "Подрядчик"),
        "role_subcontractor" => ("Subpudratchi", "Субподрядчик"),
        "role_designer" => ("Loyihachi", "Проектировщик"),
        "role_tech_supervision" => ("Texnik nazorat", "Технадзор"),
        "role_author_supervision" => ("Mualliflik nazorati", "Авторский надзор"),

        // ---------- Bog'lanish turlari / Типы связей ----------
        "link_fs" => ("FS — tugash → boshlanish", "FS — окончание к началу"),
        "link_ss" => ("SS — boshlanish → boshlanish", "SS — начало к началу"),
        "link_ff" => ("FF — tugash → tugash", "FF — окончание к окончанию"),
        "link_sf" => ("SF — boshlanish → tugash", "SF — начало к окончанию"),

        // ---------- Pasport / Паспорт ----------
        "passport_title" => ("Obyekt pasporti", "Паспорт объекта"),
        "card_object" => ("Obyekt kartochkasi", "Карточка объекта"),
        "field_name" => ("Nomi", "Наименование"),
        "field_code" => ("Obyekt shifri", "Шифр объекта"),
        "field_address" => ("Manzil", "Адрес"),
        "field_status" => ("Obyekt holati", "Статус объекта"),
        "card_dates" => ("Muddatlar", "Сроки"),
        "field_start" => ("Qurilish boshlanishi", "Начало строительства"),
        "field_planned_end" => ("Rejadagi tugash", "Плановое окончание"),
        "field_duration" => ("Rejadagi davomiyligi", "Плановая длительность"),
        "field_cpm_end" => ("GPR bo'yicha tugash (CPM)", "Окончание по ГПР (CPM)"),
        "later_than_contract" => (
            "shartnoma muddatidan kech, kun:",
            "позже договорного срока на, дн.:",
        ),
        "card_finance" => ("Moliyalashtirish", "Финансирование"),
        "field_contract_sum" => ("Shartnoma summasi", "Сумма договора"),
        "field_currency" => ("Valyuta", "Валюта"),
        "field_funding" => ("Moliyalashtirish manbai", "Источник финансирования"),
        "card_parties" => ("Loyiha ishtirokchilari", "Участники проекта"),
        "parties_empty" => ("Ishtirokchilar kiritilmagan", "Участники не заданы"),
        "col_role" => ("Rol", "Роль"),
        "col_org" => ("Tashkilot", "Организация"),
        "col_person" => ("Mas'ul shaxs", "Ответственное лицо"),
        "col_email" => ("E-mail", "E-mail"),
        "add_party" => ("+ Ishtirokchi qo'shish", "+ Добавить участника"),
        "delete_party" => ("Ishtirokchini o'chirish", "Удалить участника"),
        "card_notes" => ("Izohlar", "Примечания"),
        "delete_object" => ("Obyektni o'chirish", "Удалить объект"),
        "object_deleted" => ("Obyekt o'chirildi", "Объект удален"),
        "save_failed" => ("Saqlab bo'lmadi", "Не удалось сохранить"),

        // ---------- Ko'rsatkichlar / Показатели ----------
        "kpi_progress" => ("Bajarilgan", "Выполнение"),
        "kpi_plan_today" => ("Bugungi reja", "План на сегодня"),
        "kpi_fact" => ("Fakt", "Факт"),
        "kpi_plan_is" => ("reja", "план"),
        "kpi_by_durations" => ("ishlar davomiyligi bo'yicha", "по длительностям работ"),
        "kpi_deviation" => ("chetlanish", "отклонение"),
        "kpi_pp" => ("f.p.", "п.п."),
        "kpi_overdue" => ("Muddati o'tgan", "Просрочено"),
        "kpi_overdue_tasks" => ("Muddati o'tgan ishlar", "Просрочено работ"),
        "kpi_overdue_hint" => (
            "muddat tugagan, ish yopilmagan",
            "срок истек, работа не закрыта",
        ),
        "kpi_today" => ("Bugun bajarilmoqda", "Идут сегодня"),
        "kpi_in_progress" => ("ish bajarilmoqda", "работ в исполнении"),
        "kpi_critical" => ("Kritik yo'l", "Критический путь"),
        "kpi_tasks_count" => ("ish", "работ"),
        "kpi_cpm_length" => ("davomiyligi", "длительность"),
        "kpi_gpr_end" => ("GPR bo'yicha tugash", "Окончание по ГПР"),
        "kpi_contract" => ("shartnoma:", "договор:"),
        "kpi_forecast" => ("Tugash prognozi", "Прогноз окончания"),
        "kpi_delay" => ("kechikish", "отставание"),
        "kpi_on_track" => ("grafik bo'yicha", "в графике"),

        // ---------- Umumiy ko'rinish / Обзор ----------
        "block_analytics" => ("Tahlil xulosasi", "Сводка анализа"),
        "block_analytics_open" => ("Batafsil tahlil", "Подробный анализ"),
        "block_sales" => ("Sotuv", "Продажи"),
        "block_sales_open" => ("Shaxmatkani ochish", "Открыть шахматку"),
        "block_overdue" => ("Muddati o'tgan ishlar", "Просроченные работы"),
        "no_overdue" => ("Muddati o'tgan ishlar yo'q.", "Просроченных работ нет."),
        "col_task" => ("Ish", "Работа"),
        "col_section_short" => ("Bo'lim", "Разд."),
        "col_responsible" => ("Mas'ul", "Ответственный"),
        "col_deadline" => ("Muddat", "Срок"),
        "col_overdue_by" => ("Kechikish", "Просрочка"),
        "open_in_gantt" => ("GPR da ochish", "Открыть в ГПР"),
        "block_sections" => ("Loyiha bo'limlari", "Разделы проекта"),
        "sections_empty" => (
            "Ishlarda bo'lim ko'rsatilmagan.",
            "Разделы у работ не указаны.",
        ),
        "tasks_short" => ("ish", "раб."),
        "block_today" => ("Bugun bajarilayotgan ishlar", "Работы в исполнении сегодня"),
        "no_active_today" => ("Bugun faol ishlar yo'q.", "Сегодня активных работ нет."),
        "until" => ("gacha", "до"),
        "critical_path_lower" => ("kritik yo'l", "критический путь"),

        // ---------- GPR / ГПР ----------
        "add_task" => ("+ Ish", "+ Работа"),
        "link_tasks" => ("Bog'lash", "Связать"),
        "link_hint" => (
            "Bosing, so'ng diagrammada yoki jadvalda keyingi ishni tanlang",
            "Нажмите, затем выберите работу-преемника на диаграмме или в таблице",
        ),
        "scale" => ("Masshtab:", "Масштаб:"),
        "scale_day" => ("Kun", "День"),
        "scale_week" => ("Hafta", "Неделя"),
        "scale_month" => ("Oy", "Месяц"),
        "only_critical" => ("Faqat kritik yo'l", "Только критический путь"),
        "all_sections" => ("Barcha bo'limlar", "Все разделы"),
        "search_tasks" => ("Ishlar bo'yicha qidiruv…", "Поиск по работам…"),
        "fit_btn" => ("Butun grafik", "Весь график"),
        "fit_hint" => (
            "Masshtabni butun grafik oynaga sig'adigan qilib tanlaydi",
            "Подбирает масштаб так, чтобы весь график поместился в окно",
        ),
        "today_btn" => ("Bugun", "Сегодня"),
        "today_marker" => ("bugun", "сегодня"),
        "cycle_warning" => (
            "Diqqat: bog'lanishlarda sikl aniqlandi, ishlar soni:",
            "Обнаружен цикл зависимостей, работ:",
        ),
        "cycle_warning_tail" => (
            "Ular uchun hisob ishonchsiz — bog'lanishlarni tekshiring.",
            "Расчет для них недостоверен — проверьте связи.",
        ),
        "col_num" => ("№", "№"),
        "col_task_name" => ("Ish nomi", "Наименование работы"),
        "col_days" => ("Kun", "Дн."),
        "col_done_pct" => ("Baj.%", "Вып.%"),
        "col_slack" => ("Zax.", "Рез."),
        "crit_short" => ("kritik", "крит."),
        "task_section" => ("Bo'lim", "Раздел"),
        "task_responsible" => ("Mas'ul", "Ответственный"),
        "task_duration" => ("Davomiyligi, kun", "Длительность, дн."),
        "task_done" => ("Bajarildi, %", "Выполнено, %"),
        "task_volume" => ("Hajmi", "Объем"),
        "task_unit" => ("o'lch. bir.", "ед. изм."),
        "task_pin" => ("Sanani mahkamlash", "Закрепить дату"),
        "task_pin_hint" => (
            "CPM bu ishning boshlanishini rejadagi sanadan oldinga surmaydi",
            "CPM не будет двигать начало этой работы раньше плановой даты",
        ),
        "task_plan_start" => ("Rejadagi boshlanish", "Плановое начало"),
        "task_new_name" => ("Yangi ish", "Новая работа"),
        "insp_task" => ("Ish", "Работа"),
        "insp_wbs" => ("ISS", "ВБС"),
        "insp_calc" => ("Grafik hisobi", "Расчет графика"),
        "insp_es" => ("Erta boshlanish (ES)", "Раннее начало (ES)"),
        "insp_ef" => ("Erta tugash (EF)", "Раннее окончание (EF)"),
        "insp_ls" => ("Kech boshlanish (LS)", "Позднее начало (LS)"),
        "insp_lf" => ("Kech tugash (LF)", "Позднее окончание (LF)"),
        "insp_slack" => ("Umumiy zaxira", "Общий резерв"),
        "insp_critical" => ("Kritik yo'l", "Критический путь"),
        "insp_overdue" => ("Ish muddati o'tgan", "Работа просрочена"),
        "insp_links" => ("Bog'lanishlar", "Связи"),
        "insp_preds" => ("Oldingi ishlar", "Предшественники"),
        "insp_succs" => ("Keyingi ishlar", "Преемники"),
        "hover_plan" => ("Reja", "План"),
        "hover_done" => ("Bajarildi", "Выполнено"),
        "hover_slack" => ("Zaxira", "Резерв"),
        "hover_critical" => (
            "Kritik yo'l — kechikish obyekt topshirilishini suradi",
            "Критический путь — задержка сдвинет сдачу объекта",
        ),
        "hover_overdue" => ("Muddati o'tgan", "Просрочена"),

        // ---------- Dialoglar / Диалоги ----------
        "delete_task_title" => ("Ishni o'chirilsinmi?", "Удалить работу?"),
        "delete_task_hint" => (
            "Bu ish bilan bog'liq bog'lanishlar ham o'chiriladi.",
            "Связи с этой работой также будут удалены.",
        ),
        "link_dialog_title" => ("Ishlar bog'lanishi", "Связь работ"),
        "link_pred" => ("Oldingi ish:", "Предшественник:"),
        "link_succ" => ("Keyingi ish:", "Преемник:"),
        "link_type" => ("Bog'lanish turi", "Тип связи"),
        "link_lag" => ("Lag, kun:", "Лаг, дней:"),

        // ---------- Toastlar / Уведомления ----------
        "task_deleted" => ("Ish o'chirildi", "Работа удалена"),
        "link_added" => (
            "Bog'lanish qo'shildi, grafik qayta hisoblandi",
            "Связь добавлена, график пересчитан",
        ),
        "link_cycle" => (
            "Diqqat: bu bog'lanish sikl hosil qiladi",
            "Внимание: связь образует цикл зависимостей",
        ),
        "link_self" => (
            "Ishni o'zi bilan bog'lab bo'lmaydi",
            "Нельзя связать работу саму с собой",
        ),
        "err_read_objects" => ("Obyektlarni o'qishda xato", "Ошибка чтения объектов"),
        "err_save_task" => ("Ishni saqlab bo'lmadi", "Не удалось сохранить работу"),
        "err_add_task" => ("Ish qo'shib bo'lmadi", "Не удалось добавить работу"),
        "err_del_task" => ("Ishni o'chirib bo'lmadi", "Не удалось удалить работу"),
        "err_add_link" => ("Bog'lanish yaratib bo'lmadi", "Не удалось создать связь"),
        "err_create_object" => ("Obyekt yaratib bo'lmadi", "Не удалось создать объект"),
        "err_db" => (
            "Ma'lumotlar bazasini ochib bo'lmadi:",
            "Не удалось открыть базу данных:",
        ),

        // ---------- Sozlamalar / Настройки ----------
        "settings_title" => ("Sozlamalar", "Настройки"),
        "set_group_ui" => ("Interfeys", "Интерфейс"),
        "set_language" => ("Interfeys tili", "Язык интерфейса"),
        "set_language_hint" => (
            "Standart til — o'zbekcha. O'zgarish darhol qo'llanadi.",
            "Язык по умолчанию — узбекский. Изменение применяется сразу.",
        ),
        "set_theme" => ("Mavzu", "Тема оформления"),
        "theme_light" => ("Yorug'", "Светлая"),
        "theme_dark" => ("Qorong'i", "Темная"),
        "set_scale" => ("Interfeys masshtabi", "Масштаб интерфейса"),
        "set_scale_hint" => (
            "Katta monitorlar yoki kichik shrift uchun",
            "Для крупных мониторов или мелкого шрифта",
        ),
        "set_group_gantt" => ("Ishlar grafigi", "График работ"),
        "set_default_scale" => ("Standart masshtab", "Масштаб по умолчанию"),
        "set_show_weekends" => (
            "Dam olish kunlarini ajratib ko'rsatish",
            "Выделять выходные дни",
        ),
        "set_show_weekends_hint" => (
            "Kunlik masshtabda shanba va yakshanba fonda ajratiladi",
            "В дневном масштабе суббота и воскресенье выделяются фоном",
        ),
        "set_group_defaults" => ("Standart qiymatlar", "Значения по умолчанию"),
        "set_currency" => ("Valyuta", "Валюта"),
        "set_currency_hint" => (
            "Yangi obyektlar uchun ishlatiladi",
            "Используется для новых объектов",
        ),
        "set_task_duration" => (
            "Yangi ish davomiyligi, kun",
            "Длительность новой работы, дн.",
        ),
        "set_group_data" => ("Ma'lumotlar", "Данные"),
        "set_db_path" => ("Baza fayli", "Файл базы данных"),
        "set_open_folder" => ("Papkani ochish", "Открыть папку"),
        "set_objects_count" => ("Obyektlar soni", "Объектов в базе"),
        // ---------- Rollar ----------
        "ur_admin" => ("Administrator", "Администратор"),
        "ur_admin_hint" => (
            "Hamma bo'limni ko'radi va o'zgartiradi.",
            "Видит и изменяет все разделы.",
        ),
        "ur_director" => ("Direktor", "Директор"),
        "ur_director_hint" => (
            "Hammasini ko'radi; pasport, moliya, smeta, xaridlar va shartnomalar bo'yicha qaror qabul qiladi.",
            "Видит все; принимает решения по паспорту, финансам, смете, закупкам и договорам.",
        ),
        "ur_pm" => ("Loyiha rahbari", "Руководитель проекта"),
        "ur_pm_hint" => (
            "Muddat va reja: GPR, PPR, arizalar, ijro hujjatlari, pasport.",
            "Сроки и план: график, ППР, заявки, исполнительная документация, паспорт.",
        ),
        "ur_foreman" => ("Prorab", "Прораб"),
        "ur_foreman_hint" => (
            "Kunlik ijro: jurnal, tabel, texnika smenalari, bajarilish foizi, arizalar, ombor, xavfsizlik.",
            "Ежедневное исполнение: журнал, табель, смены техники, процент выполнения, заявки, склад, ТБ.",
        ),
        "ur_brigadier" => ("Brigadir", "Бригадир"),
        "ur_brigadier_hint" => (
            "Prorabdan tor: faqat kunlik yozuv va o'z brigadasining tabeli.",
            "Уже прораба: только ежедневная запись и табель своей бригады.",
        ),
        "ur_supervisor" => ("Texnik nazorat", "Технадзор"),
        "ur_supervisor_hint" => (
            "Nazorat: ijro hujjatlari, sifat, xavfsizlik, loyiha tekshiruvi, PPR.",
            "Контроль: исполнительная документация, качество, ТБ, проверка проекта, ППР.",
        ),
        "ur_designer" => ("Mualliflik nazorati", "Авторский надзор"),
        "ur_designer_hint" => (
            "Loyiha yechimlari: elementlar, nomuvofiqliklar, PPR. Ijroga tegmaydi.",
            "Проектные решения: элементы, несоответствия, ППР. Исполнения не касается.",
        ),
        "ur_estimator" => ("Smetachi", "Сметчик"),
        "ur_estimator_hint" => (
            "Smeta, qiymat va material narxlari; xaridlar bo'yicha kelishuv.",
            "Смета, стоимость и цены материалов; согласование по закупкам.",
        ),
        "ur_supply" => ("Ta'minotchi", "Снабженец"),
        "ur_supply_hint" => (
            "Arizalar, xaridlar, material katalogi va ombor kirimi.",
            "Заявки, закупки, каталог материалов и приход на склад.",
        ),
        "ur_storekeeper" => ("Omborchi", "Кладовщик"),
        "ur_storekeeper_hint" => (
            "Ombor harakatlari va material katalogi.",
            "Движения склада и каталог материалов.",
        ),
        "ur_mechanic" => ("Mexanik", "Механик"),
        "ur_mechanic_hint" => (
            "Texnika parki, smenalar, motosoat va texnik ko'rik.",
            "Парк техники, смены, моточасы и техосмотр.",
        ),
        "ur_quality" => ("Sifat muhandisi", "Инженер по качеству"),
        "ur_quality_hint" => (
            "Kirish, operatsion va qabul nazorati; ijro hujjatlari.",
            "Входной, операционный и приемочный контроль; исполнительная документация.",
        ),
        "ur_safety" => ("Mehnat muhofazasi", "Охрана труда"),
        "ur_safety_hint" => (
            "Hodisalar, instruktajlar, chora-tadbirlar.",
            "События, инструктажи, мероприятия.",
        ),
        "ur_hr" => ("Kadrlar / tabelchi", "Кадры / табельщик"),
        "ur_hr_hint" => (
            "Ishchilar ro'yxati va tabel.",
            "Список рабочих и табель.",
        ),
        "ur_accountant" => ("Buxgalter", "Бухгалтер"),
        "ur_accountant_hint" => (
            "Pul harakatini ko'radi, lekin o'zgartirmaydi: yozuv boshqa modullardan keladi.",
            "Видит движение денег, но не изменяет: записи приходят из других модулей.",
        ),
        "ur_sales" => ("Sotuv menejeri", "Менеджер продаж"),
        "ur_sales_hint" => (
            "Kvartiralar, shartnomalar va to'lovlar.",
            "Квартиры, договоры и платежи.",
        ),
        "ur_client" => ("Buyurtmachi", "Заказчик"),
        "ur_client_hint" => (
            "Faqat ko'rish: hech narsani o'zgartirmaydi.",
            "Только просмотр: ничего не изменяет.",
        ),
        "role_nobody" => ("Rol tanlanmagan", "Роль не выбрана"),
        "role_no_users" => (
            "Foydalanuvchilar sozlamalarda qo'shiladi.",
            "Пользователи добавляются в настройках.",
        ),
        "role_switch_hint" => (
            "Rol ish taqsimoti uchun; bu parol bilan himoya emas.",
            "Роль — для разделения работы; это не защита паролем.",
        ),
        "role_readonly" => (
            "Bu ekran shu rolda faqat ko'rish uchun",
            "Этот экран в этой роли только для просмотра",
        ),
        "role_current" => ("Joriy rol", "Текущая роль"),
        // ---------- Til modeli ----------
        "set_group_llm" => ("Til modeli (ixtiyoriy)", "Языковая модель (необязательно)"),
        "set_llm_warning" => (
            "Yoqilsa, savol va unga biriktirilgan obyekt ma'lumoti tashqi xizmatga jo'natiladi. Ma'lumot kompyuterdan chiqadi va uni qaytarib bo'lmaydi. O'chiq turganda ilova hech qayerga ulanmaydi.",
            "При включении вопрос и приложенные к нему данные объекта отправляются во внешний сервис. Данные покидают компьютер, и вернуть их нельзя. В выключенном состоянии программа никуда не подключается.",
        ),
        "set_llm_not_built" => (
            "Bu yig'ilishda tarmoq qismi yo'q: ilova `--no-default-features` bilan yig'ilgan, shuning uchun ulanish umuman mumkin emas.",
            "В этой сборке нет сетевой части: программа собрана с `--no-default-features`, поэтому подключение невозможно в принципе.",
        ),
        "set_llm_enabled" => ("Yoqilgan", "Включено"),
        "set_llm_endpoint" => ("Xizmat manzili", "Адрес сервиса"),
        "set_llm_model" => ("Model nomi", "Название модели"),
        "set_llm_key" => ("API kaliti", "API-ключ"),
        "set_llm_ready" => (
            "Yoqilgan: yordamchidagi «AI suhbat» tabi ishlaydi.",
            "Включено: вкладка «AI-диалог» в помощнике работает.",
        ),
        "set_llm_off" => (
            "O'chiq: yordamchi faqat o'z bazasidan javob beradi.",
            "Выключено: помощник отвечает только по своей базе.",
        ),
        "cp_ask_model" => ("Modeldan so'rash", "Спросить модель"),
        "cp_ask_model_hint" => (
            "Savol va yuqoridagi sonlar tashqi xizmatga jo'natiladi.",
            "Вопрос и приведенные выше числа будут отправлены во внешний сервис.",
        ),
        "cp_llm_answer" => ("Model javobi", "Ответ модели"),
        "cp_llm_note" => (
            "Bu javob tashqi model tomonidan yozilgan. Sonlar yuqoridagi hisobdan olingan — javobni shu bilan solishtiring.",
            "Этот ответ написан внешней моделью. Числа взяты из расчета выше — сверяйте ответ с ним.",
        ),
        "cp_llm_failed" => ("Model javob bermadi", "Модель не ответила"),
        "set_llm_preset" => ("Tayyor nomlar", "Готовые названия"),
        "set_llm_timeout" => ("Kutish muddati", "Время ожидания"),
        "set_llm_timeout_hint" => ("soniya", "секунд"),
        "set_llm_test" => ("Ulanishni sinash", "Проверить подключение"),
        "set_llm_test_hint" => (
            "Bitta qisqa savol jo'natiladi: kalit va model to'g'riligi shu yerda ma'lum bo'ladi.",
            "Отправится один короткий вопрос: сразу станет видно, верны ли ключ и модель.",
        ),
        "set_llm_test_question" => (
            "Ulanish ishlayaptimi? Qisqa javob bering.",
            "Подключение работает? Ответьте коротко.",
        ),
        "cp_tab_chat" => ("AI suhbat", "AI-диалог"),
        "cp_chat_off" => ("AI suhbat o'chiq", "AI-диалог выключен"),
        "cp_chat_off_hint" => (
            "Sozlamalarda til modelini yoqing va API kalitini kiriting. Yoqilmaguncha ilova hech qayerga ulanmaydi.",
            "Включите языковую модель в настройках и введите API-ключ. Пока не включено, программа никуда не подключается.",
        ),
        "cp_chat_open_settings" => ("Sozlamalarni ochish", "Открыть настройки"),
        "cp_chat_empty" => (
            "Savol yozing — javobga obyekt bo'yicha tayyor sonlar biriktiriladi.",
            "Напишите вопрос — к ответу будут приложены готовые числа по объекту.",
        ),
        "cp_chat_placeholder" => (
            "Masalan: qaysi ishlar kechikkan va nima qilish kerak?",
            "Например: какие работы отстают и что делать?",
        ),
        "cp_chat_waiting" => ("Model javob yozyapti…", "Модель пишет ответ…"),
        "cp_chat_you" => ("Siz", "Вы"),
        "cp_chat_clear" => ("Suhbatni tozalash", "Очистить диалог"),
        "cp_chat_retry" => ("Qaytadan urinish", "Повторить"),
        "cp_chat_tokens" => ("token sarflandi", "токенов израсходовано"),
        "cp_chat_note" => (
            "model son hisoblamaydi — sonlar dastur bazasidan biriktiriladi",
            "модель не считает числа — они приложены из базы программы",
        ),
        "llm_err_not_configured" => (
            "Til modeli yoqilmagan yoki sozlama to'liq emas.",
            "Языковая модель не включена или настройка неполная.",
        ),
        "llm_err_auth" => (
            "API kaliti qabul qilinmadi. Kalitni sozlamada tekshiring.",
            "API-ключ не принят. Проверьте ключ в настройках.",
        ),
        "llm_err_rate" => (
            "So'rovlar chegarasi yoki hisobdagi mablag' tugadi. Birozdan keyin urinib ko'ring.",
            "Исчерпан лимит запросов или средства на счёте. Попробуйте чуть позже.",
        ),
        "llm_err_service" => (
            "Xizmat tomonida xato. Bu vaqtinchalik — qaytadan urinib ko'ring.",
            "Ошибка на стороне сервиса. Это временно — попробуйте ещё раз.",
        ),
        "llm_err_request" => (
            "So'rov qabul qilinmadi. Model nomi to'g'ri ekanini tekshiring.",
            "Запрос не принят. Проверьте, верно ли указано название модели.",
        ),
        "llm_err_transport" => (
            "Xizmatga ulanib bo'lmadi: internet yoki manzilni tekshiring.",
            "Не удалось подключиться к сервису: проверьте интернет или адрес.",
        ),
        "llm_err_reply" => (
            "Javob tushunarsiz ko'rinishda keldi.",
            "Ответ пришёл в непонятном виде.",
        ),

        "set_group_roles" => ("Foydalanuvchilar va rollar", "Пользователи и роли"),
        "set_roles_note" => (
            "Rol ish taqsimoti uchun: kim nimani to'ldirishini belgilaydi. Baza fayli ochiq, shuning uchun bu parol bilan himoya emas — haqiqiy kirish nazorati server qismi bilan keladi.",
            "Роль нужна для разделения работы: кто что заполняет. Файл базы открыт, поэтому это не защита паролем — реальный контроль доступа появится вместе с серверной частью.",
        ),
        "add_user" => ("+ Foydalanuvchi", "+ Пользователь"),
        "user_new_name" => ("Yangi foydalanuvchi", "Новый пользователь"),
        "col_user" => ("Ism", "Имя"),

        "set_package" => ("Loyiha paketi", "Пакет объекта"),
        "set_package_export" => ("Paketga chiqarish…", "Выгрузить пакет…"),
        "set_package_export_hint" => (
            "Kunlik ijro (jurnal, tabel, smenalar, sifat, xavfsizlik) matn fayliga yoziladi va boshqa kompyuterga olib borilishi mumkin. Grafik va smeta chiqmaydi — ular ofisda yuritiladi.",
            "Ежедневное исполнение (журнал, табель, смены, качество, ТБ) записывается в текстовый файл и переносится на другой компьютер. График и смета не выгружаются — они ведутся в офисе.",
        ),
        "set_package_import" => ("Paketdan olish…", "Загрузить пакет…"),
        "set_package_import_hint" => (
            "Paketdagi yozuvlar qo'shiladi. Mavjud yozuvlar qayta yozilmaydi — ziddiyatni odam hal qiladi.",
            "Записи из пакета добавляются. Существующие не перезаписываются — конфликт решает человек.",
        ),
        "set_package_saved" => ("Paket saqlandi, qatorlar:", "Пакет сохранен, строк:"),
        "set_package_failed" => ("Paket bilan ishlab bo'lmadi", "С пакетом поработать не удалось"),
        "set_package_bad" => (
            "Bu QURAi paketi emasga o'xshaydi.",
            "Похоже, это не пакет QURAi.",
        ),
        "set_backup" => ("Zaxira nusxa", "Резервная копия"),
        "set_backup_btn" => ("Nusxa saqlash…", "Сохранить копию…"),
        "set_backup_hint" => (
            "Bazaning izchil nusxasi bitta faylga yoziladi. Ma'lumot yo'qolsa, shu fayldan tiklanadi.",
            "Целостная копия базы записывается в один файл. При потере данных восстановление идет из него.",
        ),
        "set_backup_done" => ("Zaxira nusxa saqlandi:", "Резервная копия сохранена:"),
        "set_backup_failed" => ("Zaxira nusxa saqlanmadi", "Резервная копия не сохранена"),
        "set_create_demo" => ("Namoyish obyektini yaratish", "Создать демо-объект"),
        "set_demo_created" => ("Namoyish obyekti yaratildi", "Демо-объект создан"),
        "set_clear_demo" => ("Namunani o'chirish", "Удалить демо-данные"),
        "set_clear_demo_hint" => (
            "Namunaviy obyektlar va ularning barcha yozuvlari o'chiriladi. Bu amalni qaytarib bo'lmaydi — avval zaxira nusxa oling.",
            "Демонстрационные объекты и все их записи будут удалены. Действие необратимо — сначала сделайте резервную копию.",
        ),
        "set_demo_cleared" => ("Namunaviy obyekt o'chirildi:", "Удалено демо-объектов:"),
        "err_delete" => ("O'chirib bo'lmadi", "Не удалось удалить"),
        "set_group_about" => ("Dastur haqi", "О программе"),
        "set_version" => ("Versiya", "Версия"),
        "set_stack" => ("Texnologiya", "Технология"),
        "set_stack_value" => (
            "Rust + egui — native desktop, webview yo'q",
            "Rust + egui — нативный desktop, без webview",
        ),
        "set_module" => ("Amalga oshirilgan modul", "Реализованный модуль"),
        "set_module_value" => (
            "I. Qurilish loyihasini boshqarish (pasport, GPR, CPM)",
            "I. Управление строительным проектом (паспорт, ГПР, CPM)",
        ),
        "set_saved" => ("Sozlama saqlandi", "Настройка сохранена"),

        // ---------- II-III. Muhimlik va holat / Важность и статус ----------
        "sev_critical" => ("Kritik", "Критическая"),
        "sev_major" => ("Jiddiy", "Существенная"),
        "sev_warning" => ("Ogohlantirish", "Предупреждение"),
        "sev_info" => ("Ma'lumot", "Информация"),
        "sev_ok" => ("Normada", "В норме"),
        "ist_open" => ("Ochiq", "Открыто"),
        "ist_in_work" => ("Ishlanmoqda", "В работе"),
        "ist_fixed" => ("Tuzatilgan", "Исправлено"),
        "ist_rejected" => ("Rad etilgan", "Отклонено"),
        "imod_project" => ("Loyiha", "Проект"),
        "imod_estimate" => ("Smeta", "Смета"),
        "imod_quality" => ("Sifat", "Качество"),
        "imod_safety" => ("Xavfsizlik", "Безопасность"),

        // ---------- II. Loyiha elementlari / Элементы проекта ----------
        "ek_room" => ("Xona", "Помещение"),
        "ek_wall" => ("Devor", "Стена"),
        "ek_door" => ("Eshik", "Дверь"),
        "ek_window" => ("Deraza", "Окно"),
        "ek_column" => ("Ustun", "Колонна"),
        "ek_beam" => ("Rigel", "Балка"),
        "ek_slab" => ("Plita", "Плита"),
        "ek_opening" => ("Teshik", "Отверстие"),
        "ek_pipe" => ("Quvur", "Труба"),
        "ek_duct" => ("Havo yo'li", "Воздуховод"),
        "ek_cable" => ("Kabel", "Кабель"),
        "ek_device" => ("Qurilma", "Прибор"),
        "ek_other" => ("Boshqa", "Прочее"),
        "rel_contains" => ("tarkibida", "содержит"),
        "rel_serves" => ("xizmat qiladi", "обслуживает"),
        "rel_crosses" => ("kesib o'tadi", "пересекает"),
        "rel_supported_by" => ("tayanadi", "опирается на"),
        "rel_powered_by" => ("quvvat oladi", "запитан от"),
        "rel_related" => ("bog'liq", "связан с"),

        // ---------- IV. Ijro hujjatlari / Исполнительная документация ----------
        "edk_hidden" => ("Yashirin ishlar dalolatnomasi", "Акт скрытых работ"),
        "edk_acceptance" => ("Qabul dalolatnomasi", "Акт приемки"),
        "edk_test" => ("Sinov bayonnomasi", "Протокол испытаний"),
        "edk_passport" => ("Pasport / sertifikat", "Паспорт / сертификат"),
        "edk_scheme" => ("Ijro sxemasi", "Исполнительная схема"),
        "edk_other" => ("Boshqa", "Прочее"),
        "eds_draft" => ("Qoralama", "Черновик"),
        "eds_review" => ("Ko'rib chiqishda", "На рассмотрении"),
        "eds_signed" => ("Imzolangan", "Подписан"),
        "eds_rejected" => ("Rad etilgan", "Отклонен"),

        // ---------- IX-X. Arizalar va xaridlar / Заявки и закупки ----------
        "rk_material" => ("Material", "Материал"),
        "rk_machine" => ("Texnika", "Техника"),
        "rk_labor" => ("Ishchi kuchi", "Рабочая сила"),
        "rk_document" => ("Hujjat", "Документ"),
        "rk_other" => ("Boshqa", "Прочее"),
        "rs_new" => ("Yangi", "Новая"),
        "rs_approved" => ("Tasdiqlangan", "Утверждена"),
        "rs_purchase" => ("Xaridda", "В закупке"),
        "rs_delivered" => ("Yetkazilgan", "Доставлено"),
        "rs_closed" => ("Yopilgan", "Закрыта"),
        "rs_rejected" => ("Rad etilgan", "Отклонена"),
        "pr_low" => ("Past", "Низкий"),
        "pr_normal" => ("O'rta", "Обычный"),
        "pr_high" => ("Yuqori", "Высокий"),
        "pr_urgent" => ("Shoshilinch", "Срочный"),
        "ps_draft" => ("Qoralama", "Черновик"),
        "ps_ordered" => ("Buyurtma berilgan", "Заказано"),
        "ps_paid" => ("To'langan", "Оплачено"),
        "ps_delivered" => ("Yetkazilgan", "Доставлено"),
        "ps_closed" => ("Yopilgan", "Закрыто"),

        // ---------- XI-XVI. Ombor, sifat, xavfsizlik, texnika ----------
        "mk_in" => ("Kirim", "Приход"),
        "mk_out" => ("Chiqim", "Расход"),
        "mk_writeoff" => ("Hisobdan chiqarish", "Списание"),
        "qk_input" => ("Kirish nazorati", "Входной контроль"),
        "qk_operational" => ("Operatsion nazorat", "Операционный контроль"),
        "qk_acceptance" => ("Qabul nazorati", "Приемочный контроль"),
        "qr_pass" => ("Mos", "Соответствует"),
        "qr_conditional" => ("Shartli mos", "Условно годно"),
        "qr_fail" => ("Mos emas", "Не соответствует"),
        "sk_violation" => ("Buzilish", "Нарушение"),
        "sk_near_miss" => ("Xavfli holat", "Опасная ситуация"),
        "sk_incident" => ("Baxtsiz hodisa", "Несчастный случай"),
        "sk_inspection" => ("Tekshiruv", "Проверка"),
        "sk_training" => ("Instruktaj", "Инструктаж"),
        "mck_crane" => ("Kran", "Кран"),
        "mck_excavator" => ("Ekskavator", "Экскаватор"),
        "mck_loader" => ("Pogruzchik", "Погрузчик"),
        "mck_truck" => ("Yuk mashinasi", "Самосвал"),
        "mck_concrete" => ("Beton texnikasi", "Бетонная техника"),
        "mck_lift" => ("Podyomnik", "Подъемник"),
        "mck_other" => ("Boshqa", "Прочее"),
        "ms_working" => ("Ishlamoqda", "Работает"),
        "ms_idle" => ("Bo'sh", "Простой"),
        "ms_repair" => ("Ta'mirda", "В ремонте"),
        "ms_off" => ("Obyektda emas", "Не на объекте"),

        // ---------- Tekshiruv qoidalari / Правила проверки ----------
        "rule_est_arith" => ("Arifmetika: miqdor x narx", "Арифметика: количество x цена"),
        "rule_est_total" => ("Yakuniy summa", "Итоговая сумма"),
        "rule_est_dup" => ("Takrorlangan pozitsiyalar", "Дублирующиеся позиции"),
        "rule_est_unit" => ("O'lchov birliklari", "Единицы измерения"),
        "rule_est_volume" => ("Hajmlarning mosligi", "Соответствие объемов"),
        "rule_est_price" => ("Narx anomaliyalari", "Аномалии цен"),
        "rule_est_missing" => ("Smetada yo'q ishlar", "Отсутствующие в смете работы"),
        "rule_est_zero" => ("Nol va manfiy qiymatlar", "Нулевые и отрицательные значения"),
        "rule_prj_dup_mark" => ("Takrorlangan markalar", "Дублирующиеся марки"),
        "rule_prj_no_sheet" => ("Varaq ko'rsatilmagan", "Не указан лист"),
        "rule_prj_zero_size" => ("O'lcham ko'rsatilmagan", "Не указан размер"),
        "rule_prj_ar_kj" => ("AR - KJ: teshiklar", "АР - КЖ: проемы"),
        "rule_prj_ar_vk" => ("AR - VK: xona ta'minoti", "АР - ВК: обеспечение помещений"),
        "rule_prj_ar_ov" => ("AR - OV: xona ta'minoti", "АР - ОВ: обеспечение помещений"),
        "rule_prj_ar_eom" => ("AR - EOM: xona ta'minoti", "АР - ЭОМ: обеспечение помещений"),
        "rule_prj_cross" => ("Tarmoq konstruksiyani kesib o'tishi", "Пересечение сетей с конструкциями"),
        "rule_prj_slope" => ("Kanalizatsiya ukloni", "Уклон канализации"),
        "rule_prj_power" => ("OV/VK/PB/SS - EOM: elektr ta'minoti", "ОВ/ВК/ПБ/СС - ЭОМ: электропитание"),
        "rule_prj_km_kj" => ("KM - KJ: tayanch va ankerlar", "КМ - КЖ: опирание и анкеры"),
        // II.9-10. Kuchsiz tok va yong'in xavfsizligi qoidalari
        "rule_prj_pb_room" => ("PB: xonada yong'in qurilmasi", "ПБ: пожарное устройство в помещении"),
        "rule_prj_pb_exit" => ("PB: evakuatsiya chiqishi", "ПБ: эвакуационный выход"),
        "rule_prj_pb_water" => ("PB: o'chirish tizimiga suv", "ПБ: вода для системы тушения"),
        "rule_prj_ss_cable" => ("SS: qurilma kabelga ulanmagan", "СС: устройство не подключено кабелем"),
        "rule_prj_ss_place" => ("SS: o'rnatish joyi ko'rsatilmagan", "СС: не указано место установки"),
        "chk_pb_room_title" => ("Xona yong'in nazoratisiz", "Помещение без пожарной защиты"),
        "chk_pb_room_desc" => (
            "yong'in signalizatsiyasi yoki o'chirish qurilmasi ko'zda tutilmagan",
            "не предусмотрено устройство сигнализации или тушения"
        ),
        "chk_pb_room_fix" => (
            "PB bo'limiga xonaga xizmat qiladigan qurilma qo'shing.",
            "Добавьте в раздел ПБ устройство, обслуживающее помещение."
        ),
        "chk_pb_exit_title" => ("Xonadan chiqish yo'q", "Из помещения нет выхода"),
        "chk_pb_exit_desc" => (
            "eshik ko'rsatilmagan — evakuatsiya yo'li aniqlanmagan",
            "дверь не указана — путь эвакуации не определён"
        ),
        "chk_pb_exit_fix" => (
            "AR bo'limida xonaga eshik qo'shing yoki evakuatsiya yo'lini ko'rsating.",
            "Добавьте дверь в разделе АР или укажите путь эвакуации."
        ),
        "chk_pb_water_title" => ("O'chirish tizimiga suv ta'minoti yo'q", "Нет водоснабжения системы тушения"),
        "chk_pb_water_desc" => (
            "VK bo'limi bilan bog'lanmagan — suv qayerdan kelishi noma'lum",
            "не связано с разделом ВК — источник воды неизвестен"
        ),
        "chk_pb_water_fix" => (
            "Qurilmani VK bo'limidagi quvur yoki nasosga bog'lang.",
            "Свяжите устройство с трубой или насосом раздела ВК."
        ),
        "chk_ss_cable_title" => ("Kuchsiz tok qurilmasi ulanmagan", "Устройство слаботочки не подключено"),
        "chk_ss_cable_desc" => (
            "hech qanday kabelga bog'lanmagan",
            "не связано ни с одним кабелем"
        ),
        "chk_ss_cable_fix" => (
            "SS bo'limida qurilmani kabelga bog'lang.",
            "Свяжите устройство с кабелем в разделе СС."
        ),
        "chk_ss_place_title" => ("O'rnatish joyi ko'rsatilmagan", "Не указано место установки"),
        "chk_ss_place_desc" => (
            "xona ko'rsatilmagan va hech bir xonaga kiritilmagan",
            "помещение не указано и устройство не включено ни в одно помещение"
        ),
        "chk_ss_place_fix" => (
            "Qurilma kartochkasida xonani to'ldiring yoki xonaga bog'lang.",
            "Заполните помещение в карточке устройства или свяжите с помещением."
        ),
        "rule_prj_orphan" => ("Bog'lanmagan elementlar", "Несвязанные элементы"),

        // ---------- Tekshiruv matnlari / Тексты проверок ----------
        "norm_missing" => (
            "Me'yoriy asos reyestrga kiritilmagan - muhandis tekshiruvi talab qilinadi.",
            "Нормативное основание не внесено в реестр - требуется проверка инженером.",
        ),
        "chk_threshold_unconfirmed" => (
            "Chegara qiymati reyestrda tasdiqlanmagan - vaqtinchalik qiymat ishlatildi.",
            "Пороговое значение не подтверждено реестром - использовано временное значение.",
        ),
        "chk_pos" => ("Pozitsiya", "Позиция"),
        "chk_price" => ("Narx", "Цена"),
        "chk_computed" => ("Hisoblangan", "Расчет"),
        "chk_in_estimate" => ("Smetada", "В смете"),
        "chk_in_project" => ("Loyihada", "В проекте"),
        "chk_diff" => ("Farq", "Разница"),
        "chk_deviation" => ("Chetlanish", "Отклонение"),
        "chk_conclusion" => ("Xulosa", "Вывод"),
        "chk_needs_check" => (
            "asos hujjat bilan tasdiqlanishi kerak",
            "требуется подтверждение основания документом",
        ),
        "chk_declared" => ("Hujjatda ko'rsatilgan", "Заявлено в документе"),
        "chk_items_sum" => ("Pozitsiyalar yig'indisi", "Сумма позиций"),
        "chk_estimate_total" => ("Smeta yakuni", "Итог сметы"),
        "chk_times" => ("marta", "раз(а)"),
        "chk_arith_title" => ("Arifmetik xato", "Арифметическая ошибка"),
        "chk_arith_fix" => (
            "Pozitsiyani qayta hisoblang yoki miqdor va narxni tekshiring.",
            "Пересчитайте позицию либо проверьте количество и цену.",
        ),
        "chk_total_title" => ("Yakuniy summa mos emas", "Итоговая сумма не сходится"),
        "chk_total_fix" => (
            "Smeta yakunini pozitsiyalar yig'indisiga keltiring.",
            "Приведите итог сметы к сумме позиций.",
        ),
        "chk_dup_title" => ("Takrorlanish ehtimoli", "Возможное дублирование"),
        "chk_dup_positions" => ("pozitsiyalarda", "в позициях"),
        "chk_dup_same_qty" => (
            "Miqdorlar bir xil - ikki marta hisoblangan bo'lishi mumkin.",
            "Количества совпадают - возможен двойной счет.",
        ),
        "chk_dup_diff_qty" => (
            "Miqdorlar turlicha - bo'linish asoslanganini tekshiring.",
            "Количества различаются - проверьте обоснованность разбивки.",
        ),
        "chk_dup_fix" => (
            "Pozitsiyalarni solishtiring va takrorini olib tashlang.",
            "Сверьте позиции и удалите дубль.",
        ),
        "chk_unit_unknown" => ("Noma'lum o'lchov birligi", "Неизвестная единица измерения"),
        "chk_unit_unknown_desc" => (
            "birlik ma'lumotnomada topilmadi, hajmlarni solishtirib bo'lmaydi",
            "единица не найдена в справочнике, сверка объемов невозможна",
        ),
        "chk_unit_title" => ("O'lchov birligi mos emas", "Единица измерения не совпадает"),
        "chk_unit_fix" => (
            "Smeta va loyihadagi birlikni bir xillashtiring.",
            "Приведите единицу измерения в смете и проекте к одной.",
        ),
        "chk_vol_over" => ("Hajm oshirilgan bo'lishi mumkin", "Возможное завышение объема"),
        "chk_vol_under" => ("Hajm kamaytirilgan bo'lishi mumkin", "Возможное занижение объема"),
        "chk_vol_fix" => (
            "Hajmni loyiha hujjati bo'yicha qayta hisoblang.",
            "Пересчитайте объем по проектной документации.",
        ),
        "chk_price_title" => ("Narx bo'yicha chetlanish", "Отклонение по цене"),
        "chk_price_range" => ("narx oralig'i", "разброс цен"),
        "chk_price_fix" => (
            "Bir xil ish uchun narxni bir xillashtiring yoki farqni asoslang.",
            "Приведите цену к единой или обоснуйте разницу.",
        ),
        "chk_zero_title" => ("Nol yoki manfiy qiymat", "Нулевое или отрицательное значение"),
        "chk_zero_fix" => (
            "Pozitsiyani to'ldiring yoki smetadan olib tashlang.",
            "Заполните позицию либо исключите ее из сметы.",
        ),
        "chk_missing_title" => ("Ish tushib qolgan bo'lishi mumkin", "Возможно отсутствующая работа"),
        "chk_missing_desc" => (
            "grafikda bor, lekin smetada topilmadi",
            "есть в графике, но не найдена в смете",
        ),
        "chk_missing_fix" => (
            "Ishni smetaga kiriting yoki grafikdan chiqaring.",
            "Внесите работу в смету либо исключите из графика.",
        ),
        "chk_dupmark_title" => ("Marka takrorlanishi mumkin", "Возможное дублирование марки"),
        "chk_dupmark_fix" => (
            "Markalarni tekshiring: bir bo'limda marka noyob bo'lishi kerak.",
            "Проверьте марки: в пределах раздела марка должна быть уникальной.",
        ),
        "chk_nosheet_title" => ("Varaq ko'rsatilmagan", "Не указан лист"),
        "chk_nosheet_desc" => (
            "element qaysi chizmadan olinganini kuzatib bo'lmaydi",
            "невозможно отследить, с какого чертежа взят элемент",
        ),
        "chk_nosheet_fix" => (
            "Element uchun varaq raqamini ko'rsating.",
            "Укажите номер листа для элемента.",
        ),
        "chk_zerosize_title" => ("O'lcham ko'rsatilmagan", "Не указан размер"),
        "chk_zerosize_desc" => (
            "diametr yoki kesim nolga teng - montaj va smeta uchun yetarli emas",
            "диаметр или сечение равны нулю - недостаточно для монтажа и сметы",
        ),
        "chk_zerosize_fix" => ("Elementning o'lchamini kiriting.", "Внесите размер элемента."),
        "chk_arkj_title" => ("Teshik ko'zda tutilmagan", "Проем не предусмотрен"),
        "chk_arkj_desc" => (
            "uchun konstruksiyada teshik topilmadi (AR - KJ)",
            "для него не найден проем в конструкции (АР - КЖ)",
        ),
        "chk_arkj_fix" => (
            "KJ bo'limiga teshikni kiriting yoki elementni AR da aniqlashtiring.",
            "Внесите проем в раздел КЖ либо уточните элемент в АР.",
        ),
        "chk_room_service_title" => (
            "Xona muhandislik ta'minotisiz",
            "Помещение без инженерного обеспечения",
        ),
        "chk_arvk_desc" => (
            "suv va kanalizatsiya ko'zda tutilmagan",
            "не предусмотрено водоснабжение и канализация",
        ),
        "chk_arov_desc" => (
            "isitish yoki ventilyatsiya ko'zda tutilmagan",
            "не предусмотрено отопление или вентиляция",
        ),
        "chk_areom_desc" => ("elektr ta'minoti ko'zda tutilmagan", "не предусмотрено электроснабжение"),
        "chk_room_service_fix" => (
            "Tegishli bo'limda xona uchun tarmoqni loyihalang yoki xonani turkumdan chiqaring.",
            "Запроектируйте сеть для помещения в соответствующем разделе либо исключите помещение.",
        ),
        "chk_cross_title" => ("Tarmoq konstruksiyani kesib o'tadi", "Сеть пересекает конструкцию"),
        "chk_cross_desc" => ("kesib o'tadi:", "пересекает:"),
        "chk_cross_fix" => (
            "Teshik yoki gilza ko'zda tuting hamda KJ bilan kelishing.",
            "Предусмотрите проем или гильзу и согласуйте с КЖ.",
        ),
        "chk_slope_title" => ("Kanalizatsiya ukloni yetarli emas", "Недостаточный уклон канализации"),
        "chk_slope_actual" => ("Loyihadagi uklon", "Уклон по проекту"),
        "chk_slope_min" => ("Minimal uklon", "Минимальный уклон"),
        "chk_slope_fix" => (
            "Uklonni oshiring yoki quvur diametrini qayta tanlang.",
            "Увеличьте уклон либо пересмотрите диаметр трубы.",
        ),
        "chk_power_title" => (
            "Elektr ta'minoti ko'zda tutilmagan",
            "Электропитание не предусмотрено",
        ),
        "chk_power_desc" => (
            "qurilma uchun EOM bo'limida quvvat manbai bilan bog'lanish topilmadi",
            "для оборудования не найдена связь с источником питания в разделе ЭОМ",
        ),
        "chk_power_fix" => (
            "EOM bo'limida qurilmani quvvat manbaiga ulang; yong'in tizimlari uchun zaxira ta'minot ham talab qilinadi.",
            "Подключите оборудование к источнику питания в разделе ЭОМ; для противопожарных систем требуется и резервное питание.",
        ),
        "chk_kmkj_title" => ("Tayanch aniqlanmagan", "Опирание не определено"),
        "chk_kmkj_desc" => (
            "metall konstruksiya uchun KJ bo'limida tayanch yoki zakladnoy bilan bog'lanish topilmadi",
            "для металлоконструкции не найдена связь с опорой или закладной деталью в разделе КЖ",
        ),
        "chk_kmkj_fix" => (
            "Tayanch tugunini KJ bilan kelishing: anker boltlar zakladnoy detallar bilan mos kelishi kerak.",
            "Согласуйте узел опирания с КЖ: анкерные болты должны совпадать с закладными деталями.",
        ),
        "chk_orphan_title" => ("Bog'lanmagan element", "Несвязанный элемент"),
        "chk_orphan_desc" => (
            "element grafda hech narsa bilan bog'lanmagan - cross-check ishlamaydi",
            "элемент не связан ни с чем в графе - кросс-проверка невозможна",
        ),
        "chk_orphan_fix" => (
            "Elementni xona yoki konstruksiya bilan bog'lang.",
            "Свяжите элемент с помещением или конструкцией.",
        ),

        // ---------- II-III. Tekshiruv ekranlari / Экраны проверки ----------
        "tab_issues" => ("Nomuvofiqliklar", "Несоответствия"),
        "tab_elements" => ("Loyiha elementlari", "Элементы проекта"),
        "tab_relations" => ("Bog'lanishlar", "Связи"),
        "tab_norms" => ("Normativ reyestri", "Реестр нормативов"),
        "tab_estimate_items" => ("Smeta pozitsiyalari", "Позиции сметы"),
        "run_project_check" => ("Loyihani tekshirish", "Проверить проект"),
        "run_estimate_check" => ("Smetani tekshirish", "Проверить смету"),
        "run_check_hint" => (
            "Qoidalar qayta ishga tushadi. Holati o'zgartirilgan nomuvofiqliklar saqlanib qoladi.",
            "Правила будут запущены заново. Несоответствия с измененным статусом сохранятся.",
        ),
        "issues_found" => ("ta nomuvofiqlik topildi", "несоответствий найдено"),
        "no_issues" => ("Nomuvofiqlik topilmadi", "Несоответствий не найдено"),
        "issues_empty" => (
            "Ro'yxat bo'sh. Tekshiruvni ishga tushiring yoki filtrlarni bo'shating.",
            "Список пуст. Запустите проверку или сбросьте фильтры.",
        ),
        "search_issues" => ("Qidirish...", "Поиск..."),
        "all_severities" => ("Barcha darajalar", "Все уровни"),
        "all_statuses" => ("Barcha holatlar", "Все статусы"),
        "select_issue" => ("Ro'yxatdan nomuvofiqlikni tanlang", "Выберите несоответствие из списка"),
        "issue_norm" => ("Me'yoriy asos", "Нормативное основание"),
        "issue_recommendation" => ("Tavsiya", "Рекомендация"),
        "issue_responsible" => ("Mas'ul", "Ответственный"),
        "issue_status" => ("Holat", "Статус"),
        "issue_status_hint" => (
            "«Ochiq»dan boshqa holatga o'tkazilgan yozuv qayta tekshiruvda tiklanmaydi.",
            "Запись, переведенная из «Открыто», не восстанавливается при повторной проверке.",
        ),
        "issue_auto" => ("avtomatik topilgan", "найдено автоматически"),
        "issue_manual" => ("qo'lda kiritilgan", "внесено вручную"),
        "kpi_issues_total" => ("Jami", "Всего"),
        "kpi_issues_hint" => ("nomuvofiqliklar", "несоответствий"),
        "kpi_issues_critical" => ("Kritik", "Критических"),
        "kpi_issues_critical_hint" => ("darhol hal qilinsin", "требуют немедленного решения"),
        "kpi_issues_major" => ("Jiddiy", "Существенных"),
        "kpi_issues_major_hint" => ("ishlar boshlanishidan oldin", "до начала работ"),
        "kpi_issues_open" => ("Ochiq", "Открытых"),
        "kpi_issues_open_hint" => ("hali yopilmagan", "еще не закрыты"),
        "kpi_elements" => ("Elementlar", "Элементов"),
        "report_by_section" => ("Bo'limlar kesimida", "В разрезе разделов"),
        "report_total" => ("Jami", "Всего"),
        "report_empty" => ("Tekshiruv hali o'tkazilmagan", "Проверка еще не проводилась"),
        "kpi_links_graph" => ("graf bog'lanishi", "связей графа"),

        // ---------- Jadval ustunlari / Колонки таблиц ----------
        "col_code" => ("Kod", "Код"),
        "col_severity" => ("Muhimlik", "Важность"),
        "col_title" => ("Nomuvofiqlik", "Несоответствие"),
        "col_element" => ("Element", "Элемент"),
        "col_location" => ("Joylashuv", "Расположение"),
        "col_status" => ("Holat", "Статус"),
        "col_sheet" => ("Varaq", "Лист"),
        "col_created" => ("Qo'shilgan", "Добавлено"),
        "col_mark" => ("Marka", "Марка"),
        "col_kind" => ("Turi", "Тип"),
        "col_room" => ("Xona", "Помещение"),
        "col_axis" => ("O'q", "Ось"),
        "col_level" => ("Qavat", "Этаж"),
        "col_size" => ("O'lcham", "Размер"),
        "col_value" => ("Parametr", "Параметр"),
        "col_unit" => ("Birlik", "Ед. изм."),
        "col_note" => ("Izoh", "Примечание"),
        "col_name" => ("Nomi", "Наименование"),
        "col_qty" => ("Miqdor", "Кол-во"),
        "col_price" => ("Narx", "Цена"),
        "col_cost" => ("Summa", "Сумма"),
        "col_computed" => ("Hisoblangan", "Расчет"),
        "col_diff" => ("Farq", "Разница"),
        "col_pos" => ("№", "№"),

        // ---------- Elementlar va bog'lanishlar / Элементы и связи ----------
        "import_ifc" => ("IFC dan o'qish", "Импорт из IFC"),
        "import_ifc_hint" => (
            "IFC (ochiq BIM formati) faylidan elementlar va ular orasidagi bog'lanishlar o'qiladi. DWG va RVT yopiq formatlar — ular hujjat sifatida biriktiriladi.",
            "Из файла IFC (открытый формат BIM) читаются элементы и связи между ними. DWG и RVT — закрытые форматы, они прикрепляются как документы.",
        ),
        "ifc_failed" => ("IFC fayli o'qilmadi", "Файл IFC не прочитан"),
        "ifc_empty" => (
            "Faylda o'qiladigan yozuv topilmadi — bu IFC fayli emasga o'xshaydi.",
            "В файле не найдено записей — похоже, это не файл IFC.",
        ),
        "ifc_no_elements" => (
            "Faylda tanish element topilmadi.",
            "В файле не найдено распознаваемых элементов.",
        ),
        "ifc_added" => ("Qo'shildi:", "Добавлено:"),
        "ifc_links" => ("bog'lanish:", "связей:"),
        "ifc_existing" => ("mavjud edi:", "уже было:"),
        "add_element" => ("+ Element", "+ Элемент"),
        "delete_element" => ("Elementni o'chirish", "Удалить элемент"),
        "select_element" => ("Ro'yxatdan elementni tanlang", "Выберите элемент из списка"),
        "element_card" => ("Element", "Элемент"),
        "elements_hint" => (
            "Element - bilimlar grafining tuguni: xona, quvur, teshik, ustun. Qoidalar shular ustida ishlaydi.",
            "Элемент - узел графа знаний: помещение, труба, проем, колонна. Правила работают по ним.",
        ),
        "no_elements" => (
            "Loyiha elementlari kiritilmagan - tekshiradigan narsa yo'q",
            "Элементы проекта не внесены - проверять нечего",
        ),
        "value_name_hint" => ("uklon, quvvat...", "уклон, мощность..."),
        "add_relation" => ("Yangi bog'lanish", "Новая связь"),
        "relations_hint" => (
            "Bog'lanishlar bo'limlararo tekshiruv uchun kerak: xonani nima ta'minlaydi, quvur nimani kesib o'tadi.",
            "Связи нужны для межразделной проверки: что обслуживает помещение, что пересекает труба.",
        ),
        "relations_empty" => ("Bog'lanishlar yo'q", "Связей нет"),
        "relations_need_elements" => (
            "Bog'lanish uchun kamida ikkita element kerak",
            "Для связи нужно минимум два элемента",
        ),
        "relation_self" => ("Element o'zi bilan bog'lanmaydi", "Элемент нельзя связать с самим собой"),
        "impact_title" => ("O'zgarish ta'siri", "Влияние изменения"),
        "impact_hint" => (
            "Element o'zgarsa, qaysi bo'limlar ta'sirlanadi (TZ II.18)",
            "Какие разделы затронет изменение элемента (ТЗ II.18)",
        ),
        "impact_empty" => ("Bog'liq elementlar yo'q", "Связанных элементов нет"),

        // ---------- Normativ reyestri / Реестр нормативов ----------
        "norm_registry_hint" => (
            "TZ II.17: dastur normativni o'ylab topmaydi. Har bir qoidaning me'yoriy asosi shu yerda qo'lda kiritiladi; kiritilmagan bo'lsa, xatoda «muhandis tekshiruvi talab qilinadi» deb yoziladi.",
            "ТЗ II.17: программа не выдумывает нормативы. Нормативное основание каждого правила вносится здесь вручную; если не внесено, в замечании пишется «требуется проверка инженером».",
        ),
        "norm_filled" => ("asos kiritilgan", "основание внесено"),
        "norm_empty" => ("asos kiritilmagan", "основание не внесено"),
        "norm_doc" => ("Hujjat", "Документ"),
        "norm_doc_hint" => ("ShNQ, GOST, KMK...", "ШНК, ГОСТ, СНиП..."),
        "norm_edition" => ("Tahriri", "Редакция"),
        "norm_clause" => ("Band", "Пункт"),
        "norm_clause_hint" => ("masalan 6.4.3", "например 6.4.3"),
        "norm_text" => ("Talab matni", "Текст требования"),
        "norm_param" => ("Chegara qiymati", "Пороговое значение"),
        "norm_param_set" => ("tasdiqlangan", "подтверждено"),
        "norm_param_hint" => (
            "Belgilanmasa, qoida vaqtinchalik qiymatdan foydalanadi va buni xatoda ochiq yozadi.",
            "Если не отмечено, правило использует временное значение и прямо пишет об этом в замечании.",
        ),
        "norm_source" => ("Manba", "Источник"),
        "norm_save" => ("Saqlash", "Сохранить"),
        "norm_saved" => ("Normativ saqlandi", "Норматив сохранен"),

        // ---------- Smeta ekrani / Экран сметы ----------
        "estimate_label" => ("Smeta:", "Смета:"),
        "estimate_name" => ("Smeta nomi", "Название сметы"),
        "estimate_new_name" => ("Yangi smeta", "Новая смета"),
        "add_estimate" => ("+ Smeta", "+ Смета"),
        "delete_estimate" => ("Smetani o'chirish", "Удалить смету"),
        "no_estimates" => ("Smeta kiritilmagan", "Сметы не внесены"),
        "add_item" => ("+ Pozitsiya", "+ Позиция"),
        "items_hint" => (
            "«Summa» hujjatdagi qiymat, «Hisoblangan» - miqdor x narx. Farq bo'lsa, tekshiruv buni kritik xato sifatida chiqaradi.",
            "«Сумма» - значение из документа, «Расчет» - количество x цена. При расхождении проверка выдаст критическую ошибку.",
        ),
        "estimate_declared" => ("Hujjatdagi yakun", "Итог по документу"),
        "estimate_computed" => ("Pozitsiyalar yig'indisi", "Сумма позиций"),
        "estimate_items_count" => ("ta pozitsiya", "позиций"),

        // ---------- Obyektni o'chirish / Удаление объекта ----------
        "delete_object_title" => ("Obyektni o'chirish", "Удаление объекта"),
        "delete_object_hint" => (
            "Obyekt bilan birga uning barcha ishlari, bog'lanishlari, elementlari,\nsmetalari va tekshiruv natijalari o'chiriladi. Buni qaytarib bo'lmaydi.",
            "Вместе с объектом будут удалены все его работы, связи, элементы,\nсметы и результаты проверок. Отменить это нельзя.",
        ),
        "seed_modules_done" => (
            "Namoyish ma'lumoti yaratildi: elementlar va smeta",
            "Созданы демонстрационные данные: элементы и смета",
        ),

        // ---------- Import / Импорт ----------
        "import_estimate" => ("Smetani import qilish", "Импорт сметы"),
        "import_hint" => (
            "XLSX, XLS, ODS yoki CSV. Ustunlar nomi bo'yicha topiladi: nomi, birligi, miqdori, narxi, summasi.",
            "XLSX, XLS, ODS или CSV. Колонки определяются по названию: наименование, единица, количество, цена, стоимость.",
        ),
        "import_done" => ("pozitsiya import qilindi", "позиций импортировано"),
        "import_skipped" => ("qator o'qilmadi", "строк не прочитано"),
        "import_failed" => ("Import bajarilmadi", "Импорт не выполнен"),
        "import_no_header" => (
            "Faylda smeta jadvali topilmadi: nomi, miqdori va narxi ustunlari aniqlanmadi.",
            "В файле не найдена таблица сметы: не определены колонки наименования, количества и цены.",
        ),
        "import_no_items" => (
            "Jadval topildi, lekin birorta pozitsiya o'qilmadi.",
            "Таблица найдена, но ни одна позиция не прочитана.",
        ),
        "import_no_sheets" => ("Faylda o'qiladigan varaq yo'q", "В файле нет читаемых листов"),
        "import_bad_format" => (
            "Format qo'llab-quvvatlanmaydi. XLSX, XLS, ODS yoki CSV tanlang.",
            "Формат не поддерживается. Выберите XLSX, XLS, ODS или CSV.",
        ),
        "import_file_filter" => ("Smeta fayllari", "Файлы смет"),

        // ---------- Hujjatlar va foto / Документы и фото ----------
        "card_documents" => ("Hujjatlar va foto", "Документы и фото"),
        "add_document" => ("+ Fayl qo'shish", "+ Добавить файл"),
        "documents_hint" => (
            "Fayl ko'chirilmaydi — bazada uning yo'li saqlanadi.",
            "Файл не копируется — в базе сохраняется путь к нему.",
        ),
        "documents_empty" => ("Hujjat qo'shilmagan", "Документы не добавлены"),
        "photos" => ("Foto", "Фото"),
        "open" => ("Ochish", "Открыть"),
        "open_file" => ("Faylni ochish", "Открыть файл"),
        "file_missing" => ("Fayl topilmadi", "Файл не найден"),
        "remove_from_list" => ("Ro'yxatdan olib tashlash", "Убрать из списка"),
        "col_format" => ("Format", "Формат"),
        "col_added" => ("Qo'shilgan", "Добавлено"),

        // ---------- I.3. PPR ----------
        "screen_exec_docs" => ("Ijro hujjatlari", "Исполнительная документация"),
        "ppr_kind_ppr" => ("PPR", "ППР"),
        "ppr_kind_tech" => ("Texnologik karta", "Технологическая карта"),
        "ppr_kind_quality" => ("Sifat nazorati kartasi", "Карта контроля качества"),
        "ppr_kind_safety" => ("Xavfsizlik kartasi", "Карта безопасности"),
        "imod_ppr" => ("PPR", "ППР"),
        "tab_ppr_cards" => ("Kartalar", "Карты"),
        "add_ppr" => ("+ Karta", "+ Карта"),
        "run_ppr_check" => ("PPR ni tekshirish", "Проверить ППР"),
        "ppr_new_name" => ("Yangi texnologik karta", "Новая технологическая карта"),
        "ppr_empty" => ("PPR va texnologik kartalar kiritilmagan", "ППР и технологические карты не внесены"),
        "ppr_hint" => (
            "Har bir kartani GPR ishiga bog'lang va talab qilinadigan odam va texnikani ko'rsating — tekshiruv shu ma'lumot bo'yicha ishlaydi.",
            "Свяжите каждую карту с работой ГПР и укажите требуемых людей и технику — проверка работает по этим данным.",
        ),
        "avail_workers" => ("Mavjud ishchi:", "Есть рабочих:"),
        "avail_machines" => ("Mavjud texnika:", "Есть техники:"),
        "avail_hint" => (
            "Nol bo'lsa, yetarlilik tekshirilmaydi",
            "При нуле достаточность не проверяется",
        ),
        "approved_short" => ("tasdiqlangan", "утверждена"),
        "attach" => ("Fayl", "Файл"),
        "no_task" => ("— ishga bog'lanmagan —", "— не привязано к работе —"),
        "no_tasks" => ("Grafikda ish yo'q", "В графике нет работ"),
        "col_number" => ("Raqam", "Номер"),
        "col_workers" => ("Ishchi", "Рабочих"),
        "col_machines" => ("Texnika", "Техники"),
        "col_approved" => ("Tasdiq", "Утв."),
        "col_file" => ("Fayl", "Файл"),
        "col_date" => ("Sana", "Дата"),
        "col_author" => ("Muallif", "Автор"),
        "col_weather" => ("Ob-havo", "Погода"),
        "col_volume" => ("Hajm", "Объем"),

        // ---------- PPR tekshiruvi qoidalari ----------
        "rule_ppr_missing" => ("Texnologik karta yo'q", "Отсутствует технологическая карта"),
        "rule_ppr_not_approved" => ("Karta tasdiqlanmagan", "Карта не утверждена"),
        "rule_ppr_orphan" => ("Karta ishga bog'lanmagan", "Карта не привязана к работе"),
        "rule_ppr_resource" => ("Resurs yetarliligi", "Достаточность ресурсов"),
        "chk_ppr_missing_title" => ("Texnologik karta yo'q", "Нет технологической карты"),
        "chk_ppr_missing_critical" => (
            "ish kritik yo'lda, lekin PPR yoki texnologik karta topilmadi",
            "работа на критическом пути, но ППР или технологическая карта не найдена",
        ),
        "chk_ppr_missing_started" => (
            "ish boshlangan, lekin PPR yoki texnologik karta topilmadi",
            "работа начата, но ППР или технологическая карта не найдена",
        ),
        "chk_ppr_missing_fix" => (
            "Ish uchun texnologik kartani tayyorlang va tasdiqlang.",
            "Разработайте и утвердите технологическую карту на работу.",
        ),
        "chk_ppr_unapproved_title" => (
            "Ish tasdiqlanmagan karta bo'yicha bajarilmoqda",
            "Работа ведется по неутвержденной карте",
        ),
        "chk_ppr_unapproved_desc" => ("tasdiqlanmagan", "не утверждена"),
        "chk_ppr_unapproved_fix" => (
            "Ishni to'xtating yoki kartani tasdiqlating.",
            "Приостановите работу либо утвердите карту.",
        ),
        "chk_ppr_orphan_title" => ("Karta ishga bog'lanmagan", "Карта не привязана к работе"),
        "chk_ppr_orphan_desc" => (
            "GPR dagi ish ko'rsatilmagan yoki bog'langan ish o'chirilgan",
            "работа в ГПР не указана либо связанная работа удалена",
        ),
        "chk_ppr_orphan_fix" => (
            "Kartani tegishli ishga bog'lang.",
            "Привяжите карту к соответствующей работе.",
        ),
        "chk_ppr_workers_title" => ("Ishchilar yetishmaydi", "Не хватает рабочих"),
        "chk_ppr_machines_title" => ("Texnika yetishmaydi", "Не хватает техники"),
        "chk_ppr_workers" => ("ishchi", "рабочих"),
        "chk_ppr_machines" => ("birlik texnika", "ед. техники"),
        "chk_ppr_need" => ("Eng yuqori talab", "Пиковая потребность"),
        "chk_ppr_have" => ("Mavjud", "Имеется"),
        "chk_ppr_peak_day" => ("Grafikning kuni", "День графика"),
        "chk_ppr_resource_fix" => (
            "Ishlarni vaqt bo'yicha ajrating yoki qo'shimcha resurs jalb qiling.",
            "Разнесите работы во времени либо привлеките дополнительный ресурс.",
        ),

        // ---------- V. Ishlar jurnali ----------
        "add_journal_entry" => ("+ Yozuv", "+ Запись"),
        "apply_to_gantt" => ("GPR ni fakt bo'yicha yangilash", "Обновить ГПР по факту"),
        "apply_to_gantt_hint" => (
            "Jurnaldagi hajmlar yig'indisi ishning bajarilish foiziga aylanadi.",
            "Сумма объемов из журнала превращается в процент выполнения работы.",
        ),
        "journal_entries" => ("ta yozuv", "записей"),
        "journal_empty" => ("Jurnal bo'sh", "Журнал пуст"),
        "journal_hint" => (
            "Har bir yozuvni GPR ishiga bog'lang va bajarilgan hajmni ko'rsating.",
            "Свяжите каждую запись с работой ГПР и укажите выполненный объем.",
        ),
        "journal_text_hint" => (
            "Bugun bajarilgan ishlar...",
            "Выполненные за день работы...",
        ),
        "journal_remarks_hint" => ("E'tirozlar va muammolar", "Замечания и проблемы"),
        "journal_applied" => ("ta ish yangilandi", "работ обновлено"),
        "journal_no_volume" => (
            "ta ishda hajm ko'rsatilmagan",
            "работ без указанного объема",
        ),

        // ---------- III.17-26. Smeta tuzilishi va bog'lanishlari ----------
        "tab_structure" => ("Tuzilish", "Структура"),
        "nav_estimate_short" => ("Smeta", "Смета"),
        "estimate_empty" => (
            "Smeta kiritilmagan — «Pozitsiyalar» bo'limidan boshlang",
            "Смета не внесена — начните с раздела «Позиции»",
        ),

        "est_coefficients" => ("Koeffitsiyentlar va yakuniy summa", "Коэффициенты и итог"),
        "est_coefficients_hint" => (
            "Ustama to'g'ridan-to'g'ri xarajatga, foyda ustama bilan birga olingan summaga, QQS eng oxirida qo'yiladi.",
            "Накладные — на прямые затраты, прибыль — на сумму с накладными, НДС — в самом конце.",
        ),
        "est_overhead" => ("Ustama xarajatlar", "Накладные расходы"),
        "est_overhead_hint" => ("to'g'ridan-to'g'ri xarajatdan", "от прямых затрат"),
        "est_profit" => ("Smeta foydasi", "Сметная прибыль"),
        "est_profit_hint" => ("ustama bilan birga summadan", "от суммы с накладными"),
        "est_vat" => ("QQS", "НДС"),
        "est_vat_hint" => ("yakuniy summadan", "от итоговой суммы"),
        "est_direct" => ("To'g'ridan-to'g'ri xarajat", "Прямые затраты"),
        "est_before_vat" => ("QQS gacha", "Итого без НДС"),
        "est_total" => ("Yakuniy summa", "Всего по смете"),
        "est_declared" => ("Hujjatda ko'rsatilgan", "Указано в документе"),

        "est_no_overhead" => (
            "Ustama xarajat foizi ko'rsatilmagan — smeta to'liq emas",
            "Не указан процент накладных расходов — смета неполная",
        ),
        "est_no_profit" => (
            "Smeta foydasi ko'rsatilmagan",
            "Не указана сметная прибыль",
        ),
        "est_no_vat" => ("QQS ko'rsatilmagan", "Не указан НДС"),
        "est_suspicious" => ("Koeffitsiyent juda yuqori —", "Коэффициент слишком высокий —"),
        "est_mismatch" => (
            "Hisoblangan summa hujjatdagidan farq qiladi:",
            "Расчётная сумма отличается от указанной в документе:",
        ),

        "est_link" => ("GPR bilan bog'lanish", "Связь с ГПР"),
        "est_link_hint" => (
            "Pozitsiya ishga bog'lanmasa uni kim bajarishi noma'lum; ish pozitsiyasiz qolsa uning qiymati noma'lum. Nom bo'yicha taxminiy moslik hisoblanmaydi.",
            "Если позиция не связана с работой — неизвестно, кто её выполнит; если работа без позиции — неизвестна её стоимость. Приблизительное совпадение по названию не считается.",
        ),
        "est_linked" => ("pozitsiya ishga bog'langan", "позиций связано с работами"),
        "est_free_items" => ("Bog'lanmagan pozitsiya:", "Несвязанных позиций:"),
        "est_free_tasks" => ("Smetada pozitsiyasi yo'q ish:", "Работ без позиции в смете:"),

        "est_compare" => ("Variantlarni solishtirish", "Сравнение вариантов"),
        "est_compare_with" => ("Solishtirish:", "Сравнить с:"),
        "est_current" => ("Joriy", "Текущая"),
        "est_other" => ("Boshqa variant", "Другой вариант"),
        "est_budget" => ("Byudjet bilan solishtirish", "Сверка с бюджетом"),
        "est_budget_hint" => (
            "Bo'lim bo'yicha smeta va xarid byudjeti. Byudjet belgilanmagan bo'lim ham chiqadi — nol byudjet ham javob.",
            "Смета и бюджет закупок по разделам. Разделы без бюджета тоже показаны — нулевой бюджет тоже ответ.",
        ),

        // ---------- IV. Ijro hujjatlari ----------
        "add_exec_doc" => ("+ Hujjat", "+ Документ"),
        "exec_docs_hint" => (
            "Hujjatni GPR ishiga bog'lang — tizim qaysi ish hujjatsiz qolganini o'zi ko'rsatadi.",
            "Свяжите документ с работой ГПР — система сама покажет, какая работа осталась без документа.",
        ),
        "exec_docs_empty" => ("Ijro hujjatlari kiritilmagan", "Исполнительная документация не внесена"),
        "docs_required" => ("Rasmiylashtirilmagan", "Не оформлено"),
        "docs_required_hint" => (
            "Bo'lim bo'yicha talab qilinadigan, lekin hali rasmiylashtirilmagan hujjatlar",
            "Требуемые по разделу, но еще не оформленные документы",
        ),
        "docs_required_none" => (
            "Talab qilinadigan hujjatlar rasmiylashtirilgan",
            "Все требуемые документы оформлены",
        ),
        "docs_create_hint" => (
            "Shu ish uchun hujjat yaratish",
            "Создать документ для этой работы",
        ),
        "kpi_docs_total" => ("Hujjatlar", "Документов"),
        "kpi_docs_hint" => ("obyekt bo'yicha", "по объекту"),
        "kpi_docs_signed" => ("Imzolangan", "Подписано"),
        "kpi_docs_signed_hint" => ("yopilgan", "закрыто"),
        "kpi_docs_review" => ("Ko'rib chiqishda", "На рассмотрении"),
        "kpi_docs_review_hint" => ("javob kutilmoqda", "ожидают ответа"),
        "kpi_docs_missing" => ("Rasmiylashtirilmagan", "Не оформлено"),
        "kpi_docs_missing_hint" => ("talab qilinadi", "требуется по разделу"),

        // ---------- Navigatsiya guruhlari / Группы навигации ----------
        "nav_object" => ("OBYEKT", "ОБЪЕКТ"),
        "nav_cabinets" => ("KABINETLAR", "КАБИНЕТЫ"),
        "nav_supply" => ("TA'MINOT", "СНАБЖЕНИЕ"),
        "nav_resources" => ("RESURSLAR", "РЕСУРСЫ"),
        "nav_analytics" => ("ANALITIKA", "АНАЛИТИКА"),

        // ---------- Modul ekranlari / Экраны модулей ----------
        "screen_safety" => ("Xavfsizlik", "Безопасность"),
        "screen_foreman" => ("Prorab ilovasi", "Приложение прораба"),
        "screen_tech_supervision" => ("Texnik nazorat", "Технадзор"),
        "screen_client" => ("Buyurtmachi kabineti", "Кабинет заказчика"),
        "screen_requests" => ("Arizalar", "Заявки"),
        "screen_purchases" => ("Xaridlar", "Закупки"),
        "screen_warehouse" => ("Ombor", "Склад"),
        "screen_materials" => ("Materiallar", "Материалы"),
        "screen_timesheet" => ("Tabel", "Табель"),
        "screen_machines" => ("Mashinalar", "Машины"),
        "screen_analytics" => ("AI analitika", "AI аналитика"),
        "screen_copilot" => ("AI Copilot", "AI Copilot"),

        // ---------- Modul holati / Состояние модуля ----------

        // ---------- TZ tavsiflari / Описания по ТЗ ----------

        // ---------- Boshqaruv paneli / Панель управления ----------
        "sc_title" => ("Bajarilish egri chizig'i", "Кривая выполнения"),
        "sc_plan" => ("Reja", "План"),
        "sc_fact" => ("Fakt", "Факт"),
        "sc_forecast" => ("Prognoz", "Прогноз"),
        "tl_elapsed" => ("kun o'tdi", "дней прошло"),
        "tl_left" => ("kun qoldi", "дней осталось"),
        "kpi_max_delay" => ("eng kattasi", "наибольшее"),
        "no_overdue_short" => ("hammasi muddatida", "все в срок"),
        "attention_title" => ("Diqqat talab qiladi", "Требует внимания"),
        "attention_empty" => (
            "Hammasi joyida — ochiq muammo yo'q",
            "Все в порядке — открытых проблем нет",
        ),
        "attention_hint" => (
            "Qatorga bosilsa, tegishli bo'lim ochiladi",
            "Клик по строке открывает соответствующий раздел",
        ),
        "att_overdue_tasks" => ("ish muddati o'tgan", "работ просрочено"),
        "att_gpr_late" => (
            "GPR shartnoma muddatidan kechikadi",
            "ГПР позже срока договора",
        ),
        "att_ppr_unapproved" => (
            "ish tasdiqlanmagan karta bo'yicha bajarilmoqda",
            "работ ведется по неутвержденной карте",
        ),
        "att_project_issues" => (
            "jiddiy nomuvofiqlik — loyiha tekshiruvida",
            "серьезных несоответствий — в проверке проекта",
        ),
        "att_estimate_issues" => (
            "jiddiy nomuvofiqlik — smeta tekshiruvida",
            "серьезных несоответствий — в проверке смет",
        ),
        "att_ppr_issues" => (
            "nomuvofiqlik — PPR tekshiruvida",
            "несоответствий — в проверке ППР",
        ),
        "att_missing_docs" => (
            "tugallangan ish hujjatlanmagan",
            "завершенных работ не оформлено",
        ),
        "att_cycles" => (
            "bog'lanish sikli hisobni buzmoqda",
            "циклов связей ломают расчет",
        ),
        "upcoming_title" => ("Yaqin 14 kun", "Ближайшие 14 дней"),
        "upcoming_empty" => (
            "Yaqin ikki haftada boshlanish yoki topshirish yo'q",
            "В ближайшие две недели нет стартов и сдач",
        ),
        "ev_start" => ("boshlanadi", "начинается"),
        "ev_end" => ("topshirilishi kerak", "срок сдачи"),
        "more_prefix" => ("yana", "еще"),
        "sections_plan_hint" => (
            "chiziqcha — bugungi kunga rejadagi daraja",
            "штрих — плановый уровень на сегодня",
        ),

        // ---------- Pasport 2.0 / Паспорт 2.0 ----------
        "field_object_type" => ("Obyekt turi", "Тип объекта"),
        "object_type_hint" => (
            "turar-joy, savdo, sanoat, ijtimoiy...",
            "жилой, торговый, промышленный, социальный...",
        ),
        "field_floors" => ("Qavatlar soni", "Этажность"),
        "field_area" => ("Umumiy maydon, m²", "Общая площадь, м²"),
        "field_paid" => ("To'langan", "Оплачено"),
        "finance_remaining" => ("Qoldiq", "Остаток"),
        "passport_summary" => ("Obyekt holati", "Состояние объекта"),
        "sum_tasks" => ("Grafikdagi ishlar", "Работ в графике"),
        "sum_parties" => ("Ishtirokchilar", "Участников"),
        "sum_docs" => ("Hujjat va foto", "Документов и фото"),
        "sum_issues" => ("Ochiq nomuvofiqliklar", "Открытых несоответствий"),
        "passport_completeness" => ("Pasport to'ldirilishi", "Заполненность паспорта"),
        "pc_address" => ("Manzil kiritilgan", "Указан адрес"),
        "pc_type" => ("Obyekt turi kiritilgan", "Указан тип объекта"),
        "pc_dates" => ("Muddatlar belgilangan", "Заданы сроки"),
        "pc_sum" => ("Shartnoma summasi kiritilgan", "Указана сумма договора"),
        "pc_parties" => ("Barcha 6 rol to'ldirilgan", "Заполнены все 6 ролей"),
        "pc_docs" => ("Hujjat biriktirilgan", "Прикреплены документы"),
        "pc_hint" => (
            "To'liq pasport — hisobotlar va kabinetlar uchun asos",
            "Полный паспорт — основа отчетов и кабинетов",
        ),
        "missing_roles" => ("Qo'shilmagan:", "Не добавлены:"),
        "stage_hint" => (
            "Bosqichga bosib holatni o'zgartiring",
            "Нажмите на этап, чтобы изменить статус",
        ),
        "suspend_hint" => (
            "Qurilishni to'xtatilgan deb belgilash yoki qaytarish",
            "Отметить стройку приостановленной или вернуть в работу",
        ),
        "delete_object_note" => (
            "Obyekt barcha ma'lumoti bilan o'chiriladi — tasdiqlash so'raladi",
            "Объект удаляется со всеми данными — будет запрошено подтверждение",
        ),

        // ---------- GPR 2.0 / ГПР 2.0 ----------
        "col_start_short" => ("Boshl.", "Начало"),
        "col_end_short" => ("Tugash", "Оконч."),
        "filter_overdue" => ("Faqat muddati o'tgan", "Только просроченные"),
        "linking_cursor" => ("keyingi ishni tanlang", "выберите следующую работу"),
        "task_fact_start" => ("Fakt boshl.", "Факт начало"),
        "task_fact_end" => ("Fakt tugash", "Факт оконч."),
        "insp_add_pred" => ("+ Oldingi ish", "+ Предшественник"),
        "insp_move_up" => ("Ro'yxatda yuqoriga", "Выше в списке"),
        "insp_move_down" => ("Ro'yxatda pastga", "Ниже в списке"),
        "gantt_analysis" => ("Tahlil", "Анализ"),
        "an_title" => ("Grafik tahlili", "Анализ графика"),
        "an_hint" => (
            "Strelkalar — tanlov; Ctrl+strelka — ishni surish; Alt+strelka — tartib; Delete — o'chirish",
            "Стрелки — выбор; Ctrl+стрелка — сдвиг работы; Alt+стрелка — порядок; Delete — удалить",
        ),
        "an_overdue" => ("Qaysi ishlar muddati o'tgan", "Какие работы просрочены"),
        "an_none_overdue" => (
            "Muddati o'tgan ish yo'q — grafik ushlanmoqda",
            "Просроченных работ нет — график держится",
        ),
        "an_blame" => ("Mas'ullar kesimida", "В разрезе ответственных"),
        "an_todo" => ("Nima qilish kerak", "Что нужно сделать"),
        "an_delay_fc" => (
            "Joriy sur'atda obyekt kechikadi:",
            "При текущем темпе объект опоздает на:",
        ),
        "an_pace" => (
            "Vaqtida topshirish uchun sur'atni oshirish kerak:",
            "Чтобы сдать в срок, темп нужно поднять в:",
        ),
        "an_pace_ok" => (
            "Sur'at yetarli — reja ushlanmoqda",
            "Темп достаточный — план выдерживается",
        ),
        "an_crit_next" => (
            "Kritik yo'ldagi navbatdagi ishlar — ular kechiksa, topshirish suriladi:",
            "Ближайшие работы критического пути — их задержка сдвинет сдачу:",
        ),

        // ---------- PPR 2.0 / ППР 2.0 ----------
        "tab_ppr_resources" => ("Resurslar", "Ресурсы"),
        "tab_ppr_coverage" => ("Ishlar qoplanishi", "Покрытие работ"),
        "col_ppr_author" => ("Ishlab chiqqan", "Разработал"),
        "col_critical_short" => ("Kritik yo'l", "Крит. путь"),
        "col_ppr_card" => ("PPR / texnologik karta", "ППР / технологическая карта"),
        "not_approved_short" => ("tasdiqlanmagan", "не утв."),
        "avail_not_set" => ("mavjud resurs kiritilmagan", "ресурс не задан"),

        // KPI
        "kpi_ppr_cards" => ("Kartalar", "Карт"),
        "kpi_ppr_approved" => ("tasdiqlangan", "утверждено"),
        "kpi_ppr_coverage" => ("Qoplanish", "Покрытие"),
        "kpi_ppr_crit_cov" => ("Kritik yo'l qoplangan", "Крит. путь покрыт"),
        "kpi_ppr_crit_tasks" => ("kritik ish", "крит. работ"),
        "kpi_ppr_peak_workers" => ("Ishchi cho'qqisi", "Пик рабочих"),
        "kpi_ppr_peak_machines" => ("Texnika cho'qqisi", "Пик техники"),

        // Resurslar
        "res_workers_title" => (
            "Ishchilarga kunlik talab",
            "Суточная потребность в рабочих",
        ),
        "res_machines_title" => (
            "Texnikaga kunlik talab",
            "Суточная потребность в технике",
        ),
        "res_no_data" => (
            "Kartalarda resurs ko'rsatilmagan",
            "В картах не указаны ресурсы",
        ),
        "res_no_data_hint" => (
            "«Kartalar» bo'limida har bir kartaga kerakli ishchi va texnika sonini kiriting — gistogramma shundan quriladi.",
            "В разделе «Карты» укажите нужное число рабочих и техники — гистограмма строится по этим данным.",
        ),
        "res_set_capacity" => (
            "Mavjud resursni yuqorida ko'rsating — shundan keyin yetishmovchilik hisoblanadi",
            "Укажите наличный ресурс выше — после этого будет рассчитан дефицит",
        ),
        "res_enough" => (
            "Butun grafik davomida resurs yetarli",
            "Ресурса хватает на всем протяжении графика",
        ),
        "res_peak" => ("cho'qqi:", "пик:"),

        // Qoplanish
        "coverage_hint" => (
            "Har bir GPR ishi uchun PPR yoki texnologik karta bo'lishi kerak. Kartasi yo'q ish uchun uni shu yerdan yaratish mumkin.",
            "Для каждой работы ГПР должна быть ППР или технологическая карта. Для работы без карты ее можно создать здесь.",
        ),
        "cov_no_card" => ("karta yo'q", "карты нет"),
        "cov_create" => ("Yaratish", "Создать"),
        "cov_create_hint" => (
            "Shu ish uchun texnologik karta yaratish",
            "Создать технологическую карту для этой работы",
        ),

        // Yangi qoidalar
        "rule_ppr_seq" => ("Texnologik ketma-ketlik", "Технологическая последовательность"),
        "rule_ppr_risk" => ("Risk: kartasiz kritik ish", "Риск: критическая работа без карты"),
        "chk_ppr_seq_title" => (
            "Texnologik ketma-ketlik buzilgan",
            "Нарушена технологическая последовательность",
        ),
        "chk_ppr_seq_desc" => (
            "boshlangan, lekin undan oldin tugashi kerak bo'lgan ish tugallanmagan:",
            "начата, хотя предшествующая работа не завершена:",
        ),
        "chk_ppr_seq_fix" => (
            "Ishni to'xtating yoki oldingi ishni tugating; ketma-ketlik o'zgargan bo'lsa, GPR dagi bog'lanishni qayta ko'ring.",
            "Приостановите работу либо завершите предшествующую; если последовательность изменилась, пересмотрите связь в ГПР.",
        ),
        "chk_ppr_risk_title" => (
            "Kritik ish kartasiz boshlanmoqda",
            "Критическая работа начинается без карты",
        ),
        "chk_ppr_risk_starts" => ("boshlanishiga", "до начала"),
        "chk_ppr_risk_desc" => (
            "kritik yo'lda, tasdiqlangan texnologik karta yo'q",
            "на критическом пути, утвержденной технологической карты нет",
        ),
        "chk_ppr_risk_fix" => (
            "Kartani ish boshlanishigacha tayyorlab tasdiqlating — kechikish butun obyektni suradi.",
            "Подготовьте и утвердите карту до начала работы — задержка сдвинет весь объект.",
        ),
        "chk_ppr_over_days" => ("kun ortiqcha yuklangan", "дней с перегрузкой"),

        "unpin_all" => ("Mahkamlashni bekor qilish:", "Снять закрепление:"),
        "unpin_hint" => (
            "Mahkamlangan ish CPM bo'yicha siljimaydi. Bekor qilinsa, grafik hisobga qaytadi.",
            "Закрепленная работа не двигается по CPM. После снятия график вернется к расчету.",
        ),
        "unpinned_msg" => ("ta ish bo'shatildi", "работ откреплено"),

        // ---------- II. ACTION va graf ----------
        "tab_action" => ("Bajarish rejasi", "План устранения"),
        "tab_graph" => ("Bilimlar grafi", "Граф знаний"),
        "act_open" => ("Bajarilishi kerak", "К устранению"),
        "act_open_hint" => ("ochiq va ishlanmoqda", "открыто и в работе"),
        "act_critical" => ("Kritik", "Критических"),
        "act_critical_hint" => ("birinchi navbatda", "в первую очередь"),
        "act_with_deadline" => ("Muddat qo'yilgan", "Со сроком"),
        "act_with_deadline_hint" => ("qolganiga muddat kerak", "остальным нужен срок"),
        "act_overdue" => ("Muddati o'tgan", "Просрочено"),
        "act_overdue_hint" => ("bartaraf etilmagan", "не устранено"),
        "act_empty" => (
            "Bajarilishi kerak bo'lgan nomuvofiqlik yo'q",
            "Несоответствий к устранению нет",
        ),
        "act_no_owner" => ("Mas'ul belgilanmagan", "Ответственный не назначен"),
        "act_items" => ("ta nomuvofiqlik", "несоответствий"),
        "act_critical_of_them" => ("tasi kritik", "из них критических"),
        "act_deadline" => ("muddat", "срок"),
        "act_late" => ("kechikdi", "просрочено"),
        "graph_hint" => (
            "Elementlar bo'limlar bo'yicha ustunlarga joylashtirilgan; chiziqlar — ular orasidagi bog'lanishlar. Tugunni bosing — uning aloqalari ajratiladi.",
            "Элементы разложены по колонкам разделов; линии — связи между ними. Нажмите на узел — его связи выделятся.",
        ),

        // ---------- III. AI COST CONTROL ----------
        "tab_cost_control" => ("Qiymat nazorati", "Контроль стоимости"),
        "cc_no_items" => ("Smetada pozitsiya yo'q", "В смете нет позиций"),
        "cc_total" => ("Smeta qiymati", "Стоимость сметы"),
        "cc_attention" => ("Tekshirishga arziydi", "Требует проверки"),
        "cc_of_total" => ("smetadan", "от сметы"),
        "cc_duplicates" => ("Takrorlanish", "Дублирование"),
        "cc_mismatch" => ("Loyihaga mos emas", "Не соответствует проекту"),
        "cc_mismatch_hint" => ("pozitsiya", "позиций"),
        "cc_saving" => ("Mumkin bo'lgan tejam", "Возможная экономия"),
        "cc_saving_hint" => ("narxni tenglashtirishdan", "от выравнивания цен"),
        "cc_breakdown" => ("Qanday hisoblangan", "Как посчитано"),
        "cc_items_sum" => ("Pozitsiyalar yig'indisi", "Сумма позиций"),
        "cc_items_sum_hint" => (
            "barcha pozitsiyalardagi «summa» ustuni",
            "колонка «сумма» по всем позициям",
        ),
        "cc_declared_diff" => (
            "Hujjatdagi yakun bilan farq",
            "Расхождение с итогом документа",
        ),
        "cc_declared_diff_hint" => (
            "pozitsiyalar yig'indisi minus hujjatdagi yakun",
            "сумма позиций минус итог по документу",
        ),
        "cc_arithmetic" => ("Arifmetik farqlar", "Арифметические расхождения"),
        "cc_arithmetic_hint" => (
            "summa minus miqdor x narx, barcha pozitsiyalar bo'yicha",
            "сумма минус количество x цена, по всем позициям",
        ),
        "cc_volume_excess" => (
            "Loyihadan oshgan hajm qiymati",
            "Стоимость превышения объема над проектом",
        ),
        "cc_volume_excess_hint" => (
            "(smetadagi miqdor − loyihadagi hajm) x narx",
            "(количество в смете − объем по проекту) x цена",
        ),
        "cc_duplicate_cost" => (
            "Takrorlangan pozitsiyalar qiymati",
            "Стоимость дублирующихся позиций",
        ),
        "cc_duplicate_cost_hint" => (
            "bir xil nomdagi pozitsiyalarning birinchisidan keyingilari",
            "позиции с одинаковым названием, кроме первой",
        ),
        "cc_price_saving" => (
            "Narxni eng pastiga keltirsa",
            "Если привести цену к минимальной",
        ),
        "cc_price_saving_hint" => (
            "bir xil rasenka kodidagi pozitsiyalar bo'yicha",
            "по позициям с одинаковым шифром расценки",
        ),
        "cc_disclaimer" => (
            "Bu sonlar xato emas, tekshirishga arziydigan summani ko'rsatadi. Har bir holat asoslangan bo'lishi mumkin — «Nomuvofiqliklar» bo'limida sababi bilan ko'ring.",
            "Эти суммы не являются ошибкой — они показывают, что стоит проверить. Каждый случай может быть обоснован; причина указана в разделе «Несоответствия».",
        ),
        "cc_by_section" => (
            "Bo'limlar kesimidagi qiymat",
            "Стоимость в разрезе разделов",
        ),

        "kpi_docs_overdue" => ("Kechiktirilgan", "Просрочено оформление"),
        "kpi_docs_overdue_hint" => (
            "ish tugagan, hujjati yo'q",
            "работа завершена, документа нет",
        ),
        "docs_late" => ("kechikdi", "просрочено"),
        "docs_pending" => ("kutilmoqda", "ожидается"),
        "docs_unsigned" => ("imzolanmagan", "не подписан"),

        // ---------- V. Jurnal 2.0 ----------
        "jr_last" => ("Oxirgi yozuv", "Последняя запись"),
        "jr_never" => ("jurnal bo'sh", "журнал пуст"),
        "jr_today" => ("bugun to'ldirilgan", "заполнен сегодня"),
        "jr_ago" => ("oldin", "назад"),
        "jr_covered" => ("Oxirgi 30 kun", "Последние 30 дней"),
        "jr_covered_hint" => ("kun to'ldirilgan", "дней заполнено"),
        "jr_avg_workers" => ("O'rtacha ishchi", "Рабочих в среднем"),
        "jr_avg_workers_hint" => ("oxirgi 30 kun", "за последние 30 дней"),
        "jr_photos" => ("Foto", "Фото"),
        "jr_photos_hint" => ("jurnalga biriktirilgan", "прикреплено к журналу"),
        "jr_add_photo" => ("+ Foto", "+ Фото"),
        "jr_no_photos" => ("foto biriktirilmagan", "фото не прикреплены"),

        // ---------- Umumiy qidiruv / Общий поиск ----------
        "search_title" => ("Qidiruv", "Поиск"),
        "search_button" => ("Qidiruv  Ctrl+K", "Поиск  Ctrl+K"),
        "search_hint" => (
            "Ish, element, smeta pozitsiyasi, nomuvofiqlik, karta yoki bo'lim nomi...",
            "Работа, элемент, позиция сметы, несоответствие, карта или название раздела...",
        ),
        "search_short" => ("Qidiruv", "Поиск"),
        "search_keys" => (
            "Ctrl+K yoki / — ochish, Esc — yopish",
            "Ctrl+K или / — открыть, Esc — закрыть",
        ),
        "search_empty_query" => (
            "Qidirish uchun matn kiriting",
            "Введите текст для поиска",
        ),
        "search_nothing" => ("Hech narsa topilmadi", "Ничего не найдено"),
        "search_kind_issue" => ("Nomuvofiqlik", "Несоответствие"),
        "search_kind_element" => ("Element", "Элемент"),
        "search_kind_estimate" => ("Smeta", "Смета"),
        "search_kind_screen" => ("Bo'lim", "Раздел"),
        "search_module" => ("modul", "модуль"),
        "screen_gantt_short" => ("GPR", "ГПР"),

        // ---------- IX. Arizalar ----------
        "add_request" => ("+ Ariza", "+ Заявка"),
        "request_from_stock" => ("Zaxira bo'yicha ariza", "Заявка по запасу"),
        "request_from_stock_hint" => (
            "Qoldig'i minimal zaxiradan past bo'lgan har bir material uchun ariza ochiladi. Ochiq ariza bor bo'lsa takrorlanmaydi.",
            "Для каждого материала с остатком ниже минимума создается заявка. При наличии открытой заявки дубль не создается.",
        ),
        "requests_hint" => (
            "Ehtiyoj -> ariza -> tasdiqlash. Qoplanish bog'langan xaridlardan hisoblanadi.",
            "Потребность -> заявка -> согласование. Покрытие считается по связанным закупкам.",
        ),
        "requests_empty" => (
            "Ariza yozilmagan — «+ Ariza» yoki «Zaxira bo'yicha ariza» dan boshlang",
            "Заявок нет — начните с «+ Заявка» или «Заявка по запасу»",
        ),
        "requests_created" => ("Yangi ariza yaratildi:", "Создано заявок:"),
        "kpi_requests" => ("Arizalar", "Заявок"),
        "kpi_requests_hint" => ("jami", "всего"),
        "kpi_req_new" => ("Tasdiqlanmagan", "Не согласовано"),
        "kpi_req_new_hint" => ("qaror kutmoqda", "ожидают решения"),
        "kpi_req_late" => ("Muddati o'tgan", "Просрочено"),
        "kpi_req_late_hint" => ("kerak bo'lgan sana o'tdi", "срок потребности прошел"),
        "kpi_req_uncovered" => ("Qoplanmagan", "Не покрыто"),
        "kpi_req_uncovered_hint" => ("xarid yetarli emas", "закупки недостаточно"),
        "col_item" => ("Nomi", "Наименование"),
        "col_requester" => ("So'ragan", "Заявитель"),
        "col_need_date" => ("Kerak bo'lgan sana", "Срок потребности"),
        "col_priority" => ("Muhimlik", "Приоритет"),
        "col_coverage" => ("Qoplanish", "Покрытие"),
        "no_purchase" => ("xarid yo'q", "нет закупки"),
        "delivered_short" => ("keldi", "получено"),
        "late_delivery_short" => ("kech keladi", "поздняя поставка"),

        // ---------- X. Xaridlar ----------
        "add_purchase" => ("+ Xarid", "+ Закупка"),
        "purchase_from_requests" => ("Arizalar bo'yicha xarid", "Закупка по заявкам"),
        "purchase_from_requests_hint" => (
            "Tasdiqlangan, ammo xaridi ochilmagan har bir ariza uchun xarid yaratiladi: miqdor, birlik va muddat arizadan olinadi.",
            "Для каждой согласованной заявки без закупки создается закупка: количество, единица и срок берутся из заявки.",
        ),
        "post_to_stock" => ("Omborga kirim qilish", "Оприходовать на склад"),
        "post_to_stock_hint" => (
            "Yetkazilgan, ammo omborda kirimi yo'q xaridlar uchun kirim harakati yoziladi. Hujjat raqami xarid raqami bo'ladi, shuning uchun takrorlanmaydi.",
            "По доставленным закупкам без прихода создается движение прихода. Номер документа — номер закупки, поэтому дубля не будет.",
        ),
        "purchases_hint" => (
            "Ariza -> xarid -> yetkazish. Yetkazilgan xarid omborga kirim bo'lishi kerak.",
            "Заявка -> закупка -> поставка. Доставленная закупка должна попасть на склад.",
        ),
        "purchases_empty" => (
            "Xarid yozilmagan — «+ Xarid» yoki «Arizalar bo'yicha xarid» dan boshlang",
            "Закупок нет — начните с «+ Закупка» или «Закупка по заявкам»",
        ),
        "purchases_created" => ("Yangi xarid yaratildi:", "Создано закупок:"),
        "posted_to_stock" => ("Omborga kirim qilindi:", "Оприходовано на склад:"),
        "kpi_purchase_total" => ("Xaridlar summasi", "Сумма закупок"),
        "kpi_purchase_total_hint" => ("jami", "всего"),
        "kpi_purchase_open" => ("Yetkazilmagan", "Не поставлено"),
        "kpi_purchase_open_hint" => ("yo'ldagi summa", "сумма в пути"),
        "kpi_purchase_late" => ("Muddati o'tgan", "Просрочено"),
        "kpi_purchase_late_hint" => ("yetkazish sanasi o'tdi", "срок поставки прошел"),
        "kpi_purchase_unposted" => ("Kirim qilinmagan", "Не оприходовано"),
        "kpi_purchase_unposted_hint" => ("keldi, omborda yo'q", "получено, но не на складе"),
        "col_supplier" => ("Yetkazib beruvchi", "Поставщик"),
        "col_request" => ("Ariza", "Заявка"),
        "col_delivery" => ("Yetkazish", "Поставка"),
        "col_stock_post" => ("Ombor", "Склад"),
        "stock_pending" => ("kirim kutmoqda", "ждет прихода"),
        "stock_posted" => ("kirim qilingan", "оприходовано"),
        "purchase_over_request" => (
            "Miqdori arizadagidan ko'p bo'lgan xaridlar:",
            "Закупки с количеством больше заявки:",
        ),

        // ---------- XI-XII. Ombor va materiallar ----------
        "add_material" => ("+ Material", "+ Материал"),
        "material_new_name" => ("Yangi material", "Новый материал"),
        "load_demo_stock" => ("Namuna katalogni yuklash", "Загрузить пример каталога"),
        "materials_hint" => (
            "Katalog loyiha, smeta, xarid va omborni bog'laydi — kod va birlik bir xil bo'lsin.",
            "Каталог связывает проект, смету, закупки и склад — код и единица должны совпадать.",
        ),
        "materials_empty" => (
            "Material katalogi bo'sh — «+ Material» bilan boshlang",
            "Каталог материалов пуст — начните с «+ Материал»",
        ),
        "col_spec" => ("Texnik tavsif", "Характеристики"),
        "col_spec_hint" => ("marka, sinf, GOST", "марка, класс, ГОСТ"),
        "col_cert" => ("Sertifikat", "Сертификат"),
        "col_cert_until" => ("Amal qiladi", "Действует до"),
        "col_min_stock" => ("Min. zaxira", "Мин. запас"),
        "col_balance" => ("Qoldiq", "Остаток"),
        "col_material" => ("Material", "Материал"),
        "col_unit_price" => ("Birlik narxi", "Цена за единицу"),
        "col_stock_value" => ("Qoldiq qiymati", "Стоимость остатка"),
        "col_in_out" => ("Kirim / chiqim", "Приход / расход"),
        "col_last_move" => ("Oxirgi harakat", "Последнее движение"),
        "col_document" => ("Hujjat", "Документ"),
        "col_counterparty" => ("Kontragent", "Контрагент"),
        "col_sum" => ("Summa", "Сумма"),
        "cert_expired" => ("muddati o'tgan", "просрочен"),
        "below_min_short" => ("kam", "мало"),

        "kpi_materials" => ("Materiallar", "Материалов"),
        "kpi_materials_hint" => ("katalogda", "в каталоге"),
        "kpi_stock_value" => ("Ombor qiymati", "Стоимость склада"),
        "kpi_stock_value_hint" => ("joriy qoldiq bo'yicha", "по текущему остатку"),
        "kpi_below_min" => ("Zaxiradan kam", "Ниже минимума"),
        "kpi_below_min_hint" => ("buyurtma kerak", "нужен заказ"),
        "kpi_cert_expired" => ("Sertifikat muddati o'tgan", "Сертификат просрочен"),
        "kpi_cert_soon" => ("tasi yaqinda tugaydi", "истекают скоро"),
        "kpi_no_cert" => ("Sertifikatsiz", "Без сертификата"),
        "kpi_no_cert_hint" => ("hujjat kiritilmagan", "документ не внесен"),
        "kpi_negative" => ("Manfiy qoldiq", "Отрицательный остаток"),
        "kpi_negative_hint" => ("hujjatlarda xato", "ошибка в документах"),
        "kpi_moves_30" => ("Harakatlar", "Движений"),
        "kpi_moves_30_hint" => ("oxirgi 30 kun", "за 30 дней"),

        "wh_no_materials" => (
            "Avval material katalogini to'ldiring",
            "Сначала заполните каталог материалов",
        ),
        "wh_no_materials_hint" => (
            "Ombor harakatlari katalogdagi materialga bog'lanadi — «XII Materiallar» bo'limiga o'ting.",
            "Движения склада привязываются к материалу из каталога — перейдите в раздел «XII Материалы».",
        ),
        "wh_add_in" => ("+ Kirim", "+ Приход"),
        "wh_add_out" => ("+ Chiqim", "+ Расход"),
        "wh_add_writeoff" => ("+ Hisobdan chiqarish", "+ Списание"),
        "wh_hint" => (
            "Qoldiq harakatlardan hisoblanadi — alohida kiritilmaydi.",
            "Остаток рассчитывается из движений — отдельно не вводится.",
        ),
        "wh_tab_balance" => ("Qoldiqlar", "Остатки"),
        "wh_tab_moves" => ("Harakatlar", "Движения"),
        "wh_moves_empty" => ("Harakat yozilmagan", "Движения не внесены"),
        "wh_negative_warning" => (
            "Manfiy qoldiq bor: chiqim kirimdan ko'p yozilgan. Hujjatlarni tekshiring.",
            "Есть отрицательный остаток: расход больше прихода. Проверьте документы.",
        ),

        // ---------- XVII.30-31, 37. Pul oqimi, kassa uzilishi, kunlik xulosa ----------
        "an_tab_findings" => ("Topilmalar", "Находки"),
        "an_tab_cash" => ("Pul oqimi", "Денежный поток"),
        "an_tab_briefing" => ("Kunlik xulosa", "Сводка дня"),

        "cash_hint" => (
            "O'tgan oylarda fakt, kelasi oylarda reja. Qoldiq — bank hisobidagi pul emas, davr boshidan yig'ilgan oqim.",
            "За прошедшие месяцы — факт, за будущие — план. Остаток не деньги на счёте, а накопленный поток с начала периода.",
        ),
        "cash_gap_title" => ("Kassa uzilishi kutilmoqda", "Ожидается кассовый разрыв"),
        "cash_gap_amount" => ("yetishmaydi:", "не хватает:"),
        "cash_gap_months" => ("oydan keyin", "мес. вперёд"),

        // ---------- Excel eksporti va hujjatlar ----------
        "export" => ("Excel", "Excel"),
        "export_hint" => (
            "Joriy ekran jadvalini .xlsx ga saqlash (Ctrl+E). Sonlar son bo'lib chiqadi — Excel da darrov yig'indi olish mumkin.",
            "Сохранить таблицу текущего экрана в .xlsx (Ctrl+E). Числа остаются числами — в Excel сразу можно посчитать итог.",
        ),
        "export_none" => (
            "Bu ekranda eksport qilinadigan jadval yo'q",
            "На этом экране нет таблицы для выгрузки",
        ),
        "export_done" => ("Saqlandi, qator:", "Сохранено, строк:"),
        "export_failed" => ("Saqlab bo'lmadi", "Не удалось сохранить"),
        "cp_q_cash" => ("Pul oqimi qanday?", "Как с денежным потоком?"),
        "cp_l_this_month_in" => ("Shu oy kirim", "Поступления за месяц"),
        "cp_l_this_month_out" => ("Shu oy chiqim", "Расход за месяц"),
        "col_expense" => ("Jami chiqim", "Итого расход"),
        "cash_gap_hint" => (
            "To'lov grafigi va buyurtma qilingan xaridlar bo'yicha hisoblangan. Xaridni surish yoki to'lovni tezlashtirish kerak.",
            "Рассчитано по графику платежей и размещённым заказам. Нужно сдвинуть закупки или ускорить поступления.",
        ),
        "cash_gap_none" => (
            "Kelgusi oylarda kassa uzilishi ko'rinmayapti",
            "В ближайшие месяцы кассовый разрыв не просматривается",
        ),
        "col_month" => ("Oy", "Месяц"),
        "col_basis" => ("Asos", "Основа"),
        "col_plan" => ("reja", "план"),
        "col_income" => ("Kirim", "Поступления"),
        "col_purchases" => ("Xaridlar", "Закупки"),
        "col_payroll" => ("Ish haqi", "Зарплата"),
        "col_net" => ("Farq", "Сальдо"),
        "col_cumulative" => ("Yig'ilgan oqim", "Накопленный поток"),

        // Kunlik xulosa (TZ XVII.37)
        "br_title" => ("Bugungi kun", "Сводка на сегодня"),
        "br_hint" => (
            "Faqat bugun e'tibor talab qiladigan narsalar. Qatorga bosilsa tegishli bo'lim ochiladi.",
            "Только то, что требует внимания сегодня. По строке открывается нужный раздел.",
        ),
        "br_empty" => ("Bugunga shoshilinch narsa yo'q", "На сегодня срочного нет"),
        "br_empty_hint" => (
            "Muddatlar, yetkazishlar va hodisalar bo'yicha bugungi kun toza",
            "По срокам, поставкам и происшествиям сегодня чисто",
        ),
        "br_tasks_due" => ("Bugun tugashi kerak bo'lgan ishlar:", "Работ должно завершиться сегодня:"),
        "br_deliveries" => ("Bugun kutilayotgan yetkazish:", "Ожидается поставок сегодня:"),
        "br_payments_due" => ("Bugungi to'lov muddati:", "Срок платежа сегодня:"),
        "br_safety" => ("Oxirgi ikki kunda xavfsizlik hodisalari:", "Происшествий по безопасности за два дня:"),
        "br_defects" => ("Bugun bartaraf etilishi kerak bo'lgan nuqsonlar:", "Дефектов к устранению сегодня:"),
        "br_absent" => ("Bugun ishga chiqmaganlar:", "Отсутствующих сегодня:"),

        // ---------- XVII. Analitika ----------
        "analytics_hint" => (
            "Barcha modullardan yig'ilgan faktlar. Dastur hukm chiqarmaydi — fakt, hisob va tavsiya ajratilgan.",
            "Факты, собранные из всех модулей. Программа не выносит вердикт — факт, расчет и рекомендация разделены.",
        ),
        "col_object" => ("Obyekt", "Объект"),
        "an_export" => ("Hisobotni saqlash", "Сохранить отчет"),
        "an_export_hint" => (
            "Ko'rsatkichlar va topilmalar matn faylida saqlanadi — pochta yoki yig'ilish uchun.",
            "Показатели и находки сохраняются в текстовый файл — для письма или совещания.",
        ),
        "an_export_done" => ("Hisobot saqlandi:", "Отчет сохранен:"),
        "an_export_failed" => ("Hisobot saqlanmadi", "Отчет не сохранен"),
        "an_report_title" => ("obyekt bo'yicha tahlil hisoboti", "аналитический отчет по объекту"),
        "an_report_metrics" => ("Ko'rsatkichlar", "Показатели"),
        "an_findings_count" => ("ta topilma", "находок"),
        "an_report_findings" => ("Topilmalar", "Находки"),
        "an_report_note" => (
            "Hisobot dastur bazasidagi ma'lumotdan hisoblangan. Dastur hukm chiqarmaydi: fakt, hisob va tavsiya ajratilgan.",
            "Отчет рассчитан по данным базы программы. Программа не выносит вердикт: факт, расчет и рекомендация разделены.",
        ),
        "an_health" => ("Obyekt sog'lomligi", "Здоровье объекта"),
        "an_health_hint" => (
            "100 dan topilmalar og'irligi ayriladi: kritik −15, jiddiy −8, ogohlantirish −4.",
            "Из 100 вычитается вес находок: критично −15, серьезно −8, предупреждение −4.",
        ),
        "an_all" => ("Hammasi", "Все"),
        "an_open" => ("O'tish", "Перейти"),
        "an_evidence" => ("Hisob", "Расчет"),
        "an_action" => ("Tavsiya", "Рекомендация"),
        "an_nothing" => ("E'tibor talab qiladigan holat topilmadi", "Ситуаций, требующих внимания, не найдено"),
        "an_nothing_hint" => (
            "Bu tekshiruv modullardagi ma'lumotga tayanadi — ma'lumot to'ldirilsa, tahlil ham to'liqroq bo'ladi.",
            "Проверка опирается на данные модулей — чем полнее данные, тем полнее анализ.",
        ),

        // Yo'nalishlar
        "area_schedule" => ("Muddat", "Сроки"),
        "area_cost" => ("Pul", "Деньги"),
        "area_docs" => ("Hujjatlar", "Документы"),
        "area_supply" => ("Ta'minot", "Снабжение"),
        "area_quality" => ("Sifat", "Качество"),
        "area_safety" => ("Xavfsizlik", "Безопасность"),
        "area_resources" => ("Resurslar", "Ресурсы"),
        "area_sales" => ("Sotuv", "Продажи"),

        // Umumiy o'lchov birliklari
        "an_tasks" => ("Ishlar", "Работ"),
        "an_issues" => ("Nomuvofiqliklar", "Несоответствий"),
        "an_requests" => ("Arizalar", "Заявок"),
        "an_purchases" => ("Xaridlar", "Закупок"),
        "an_materials" => ("Materiallar", "Материалов"),
        "an_defects" => ("Nuqsonlar", "Дефектов"),
        "an_events" => ("Hodisalar", "Событий"),
        "an_machines" => ("Texnika", "Техники"),
        "an_workers" => ("Ishchilar", "Рабочих"),
        "an_deals" => ("Shartnomalar", "Договоров"),
        "an_positions" => ("pozitsiya", "позиций"),
        "an_amount" => ("Summa", "Сумма"),
        "an_plan_fact" => ("Reja / fakt", "План / факт"),
        "an_delay" => ("kechikish", "отставание"),
        "an_estimate" => ("Smeta:", "Смета:"),
        "an_contract" => ("shartnoma:", "договор:"),
        "an_earned" => ("Bajarildi:", "Выполнено:"),
        "an_paid" => ("to'landi:", "оплачено:"),
        "an_sold" => ("sotilgan", "продано"),
        "an_built" => ("qurilgan", "построено"),

        // Ko'rsatkichlar
        "an_m_progress" => ("Bajarilish", "Выполнение"),
        "an_plan" => ("reja", "план"),
        "an_m_delay" => ("Kechikish", "Отставание"),
        "an_forecast" => ("prognoz", "прогноз"),
        "an_m_estimate" => ("Smeta summasi", "Сумма сметы"),
        "an_m_issues" => ("Ochiq nomuvofiqlik", "Открытых несоответствий"),
        "an_m_issues_hint" => ("AI tekshiruvidan", "из AI-проверки"),
        "an_m_stock" => ("Ombor qiymati", "Стоимость склада"),
        "an_below_min" => ("zaxiradan kam", "ниже минимума"),
        "an_m_quality" => ("Sifat", "Качество"),
        "an_checks" => ("tekshiruv", "проверок"),
        "an_m_safety" => ("Ochiq xavfsizlik", "Открытых по ТБ"),
        "an_m_safety_hint" => ("chora kutmoqda", "ожидают мер"),
        "an_m_resources" => ("Ish soati, 7 kun", "Часов, 7 дней"),
        "an_machine_hours" => ("motosoat", "моточасов"),
        "an_m_sales" => ("Sotilgan", "Продано"),
        "an_received" => ("tushum", "поступило"),

        // Muddat qoidalari
        "an_s1_fact" => (
            "Bog'lanishlarda halqa bor — grafik hisobi ishonchsiz.",
            "В связях есть цикл — расчет графика недостоверен.",
        ),
        "an_s1_action" => (
            "GPR da halqadagi bog'lanishlardan birini olib tashlang.",
            "Удалите одну из связей цикла в графике.",
        ),
        "an_s2_fact" => (
            "Obyekt rejadan orqada bo'lishi mumkin.",
            "Объект, возможно, отстает от плана.",
        ),
        "an_s2_action" => (
            "Kritik yo'ldagi ishlarni ko'rib chiqing: resursni kuchaytirish yoki muddatni qayta kelishish.",
            "Пересмотрите работы критического пути: усилить ресурс или пересогласовать срок.",
        ),
        "an_s3_fact" => (
            "Muddati o'tgan, ammo yopilmagan ishlar bor.",
            "Есть работы с прошедшим сроком, но не закрытые.",
        ),
        "an_s3_action" => (
            "Jurnal orqali haqiqiy hajmni kiriting yoki muddatni yangilang.",
            "Внесите фактический объем через журнал или обновите срок.",
        ),
        "an_s4_fact" => (
            "Kritik yo'ldagi ish kechikkan — umumiy muddat siljishi mumkin.",
            "Работа на критическом пути просрочена — общий срок может сдвинуться.",
        ),
        "an_s4_action" => (
            "Shu ishlarni birinchi navbatda hal qiling; qolganlarida zaxira bor.",
            "Решайте эти работы в первую очередь; у остальных есть резерв.",
        ),

        // Pul qoidalari
        "an_c1_fact" => (
            "Smetadagi hajm loyihadagidan oshgan bo'lishi mumkin.",
            "Объем в смете, возможно, превышает проектный.",
        ),
        "an_c1_action" => (
            "«AI smeta tekshiruvi» da farq qilgan pozitsiyalarni ko'ring.",
            "Посмотрите расходящиеся позиции в «AI-проверке сметы».",
        ),
        "an_c2_fact" => (
            "Smetada takrorlangan pozitsiyalar bor.",
            "В смете есть повторяющиеся позиции.",
        ),
        "an_c2_action" => (
            "Takrorlarni birlashtiring yoki farqni izohlang.",
            "Объедините дубли или объясните различие.",
        ),
        "an_c3_fact" => (
            "Bir xil ish uchun narxlar farq qiladi.",
            "Цены на одинаковые работы различаются.",
        ),
        "an_c3_action" => (
            "Eng past narxga keltirilsa tejash imkoni bor.",
            "Приведение к минимальной цене дает экономию.",
        ),
        "an_c4_fact" => (
            "Smeta summasi shartnoma summasidan oshgan.",
            "Сумма сметы превышает сумму договора.",
        ),
        "an_c4_action" => (
            "Qo'shimcha kelishuv kerak bo'lishi mumkin — buyurtmachi bilan muhokama qiling.",
            "Может потребоваться допсоглашение — обсудите с заказчиком.",
        ),
        "an_c5_fact" => (
            "Bajarilgan ish moliyalashtirishdan oldinda — kassa uzilishi ehtimoli.",
            "Выполнение опережает финансирование — возможен кассовый разрыв.",
        ),
        "an_c5_action" => (
            "Bajarilgan ish dalolatnomalarini taqdim eting va to'lovni tezlashtiring.",
            "Предъявите акты выполненных работ и ускорьте оплату.",
        ),

        // Hujjat qoidalari
        "an_d1_fact" => (
            "Tugallangan ishlarda imzolangan ijro hujjati yo'q.",
            "У завершенных работ нет подписанной исполнительной документации.",
        ),
        "an_d1_action" => (
            "«Ijro hujjatlari» da dalolatnomalarni rasmiylashtiring.",
            "Оформите акты в разделе «Исполнительная документация».",
        ),
        "an_d2_fact" => (
            "AI tekshiruvining yopilmagan kritik nomuvofiqliklari bor.",
            "Есть незакрытые критические несоответствия AI-проверки.",
        ),
        "an_d2_action" => (
            "Har birini muhandis tekshiruvidan o'tkazing va holatini yangilang.",
            "Проверьте каждое инженером и обновите статус.",
        ),

        // Ta'minot qoidalari
        "an_p1_fact" => (
            "Kerak bo'lgan sanasi o'tgan arizalar bor.",
            "Есть заявки с прошедшим сроком потребности.",
        ),
        "an_p1_action" => (
            "Xaridni tezlashtiring yoki ehtiyoj sanasini qayta kelishing.",
            "Ускорьте закупку или пересогласуйте срок потребности.",
        ),
        "an_p2_fact" => (
            "Yetkazilgan xaridlar omborga kirim qilinmagan.",
            "Доставленные закупки не оприходованы на склад.",
        ),
        "an_p2_action" => (
            "«Xaridlar» da «Omborga kirim qilish» tugmasini bosing.",
            "Нажмите «Оприходовать на склад» в разделе «Закупки».",
        ),
        "an_p3_fact" => (
            "Ombor qoldig'i manfiy — hujjatlarda xato bo'lishi mumkin.",
            "Остаток на складе отрицательный — возможна ошибка в документах.",
        ),
        "an_p3_action" => (
            "Kirim va chiqim hujjatlarini solishtiring.",
            "Сверьте приходные и расходные документы.",
        ),
        "an_p4_fact" => (
            "Zaxiradan kam material bor va u bo'yicha ochiq ariza yo'q.",
            "Есть материалы ниже минимума без открытой заявки.",
        ),
        "an_p4_action" => (
            "«Arizalar» da «Zaxira bo'yicha ariza» tugmasi shuni bir bosishda ochadi.",
            "Кнопка «Заявка по запасу» в разделе «Заявки» создаст их одним нажатием.",
        ),
        "an_p5_fact" => (
            "Sertifikat muddati o'tgan material qoldig'i bor.",
            "Есть остаток материала с просроченным сертификатом.",
        ),
        "an_p5_action" => (
            "Sertifikatni yangilang yoki materialni ishlatishni to'xtating.",
            "Обновите сертификат или прекратите применение материала.",
        ),

        // Sifat qoidalari
        "an_q1_fact" => (
            "Bartaraf etish muddati o'tgan nuqsonlar bor.",
            "Есть дефекты с прошедшим сроком устранения.",
        ),
        "an_q1_action" => (
            "Nuqsonni bartaraf eting va qayta nazoratdan o'tkazing.",
            "Устраните дефект и проведите повторный контроль.",
        ),
        "an_q2_fact" => (
            "Talabga mos kelmagan nazorat yozuvlari ulushi yuqori.",
            "Высока доля записей контроля «не соответствует».",
        ),
        "an_q2_action" => (
            "Takrorlanayotgan sababni aniqlang: material, texnologiya yoki malaka.",
            "Определите повторяющуюся причину: материал, технология или квалификация.",
        ),
        "an_q3_fact" => (
            "Tugallangan ishlarda qabul nazorati yozilmagan.",
            "У завершенных работ нет записи приемочного контроля.",
        ),
        "an_q3_action" => (
            "Qabul nazoratini o'tkazing — ijro hujjati uchun asos bo'ladi.",
            "Проведите приемочный контроль — это основание для исполнительной документации.",
        ),

        // Xavfsizlik qoidalari
        "an_x1_fact" => (
            "Muddati o'tgan, chora ko'rilmagan xavfsizlik yozuvlari bor.",
            "Есть записи по ТБ с прошедшим сроком и без принятых мер.",
        ),
        "an_x1_action" => (
            "Chorani bajaring va yozuvni yoping.",
            "Выполните меру и закройте запись.",
        ),
        "an_x2_fact" => (
            "Obyektda baxtsiz hodisa qayd etilgan.",
            "На объекте зафиксирован несчастный случай.",
        ),
        "an_x2_action" => (
            "Tekshiruv materiallarini to'ldiring va oldini olish choralarini yozing.",
            "Заполните материалы расследования и внесите предупредительные меры.",
        ),
        "an_x3_fact" => (
            "Ishchilar ro'yxatda, instruktaj esa yozilmagan.",
            "Рабочие в списке есть, а инструктаж не внесен.",
        ),
        "an_x3_action" => (
            "Instruktajni o'tkazing va «Xavfsizlik» da qayd eting.",
            "Проведите инструктаж и зафиксируйте в разделе «Безопасность».",
        ),

        // Resurs qoidalari
        "an_r1_fact" => (
            "Texnik ko'rigi tugagan texnikada smena yozilgan.",
            "По технике с истекшим техосмотром внесены смены.",
        ),
        "an_r1_action" => (
            "Texnik ko'rikni yangilang yoki texnikani ishdan chiqaring.",
            "Обновите техосмотр или выведите технику из работы.",
        ),
        "an_r2_fact" => (
            "Texnik ko'rik muddati tugagan texnika bor.",
            "Есть техника с истекшим сроком техосмотра.",
        ),
        "an_r2_action" => (
            "Ko'rikdan o'tkazing — aks holda ishga qo'yib bo'lmaydi.",
            "Пройдите техосмотр — иначе технику нельзя допускать к работе.",
        ),
        "an_r3_fact" => (
            "«Ishlamoqda» deb belgilangan texnikada oxirgi 14 kunda smena yo'q.",
            "У техники со статусом «работает» нет смен за последние 14 дней.",
        ),
        "an_r3_action" => (
            "Holatni aniqlashtiring yoki smenalarni kiriting — ijara bekorga to'lanayotgan bo'lishi mumkin.",
            "Уточните статус или внесите смены — возможно, аренда оплачивается впустую.",
        ),
        "an_r4_fact" => (
            "Ishchilar bor, oxirgi haftada tabel to'ldirilmagan.",
            "Рабочие есть, но табель за последнюю неделю не заполнен.",
        ),
        "an_r4_action" => (
            "Tabelni to'ldiring — ish haqi va bandlik hisobi shunga tayanadi.",
            "Заполните табель — на нем строится расчет зарплаты и занятости.",
        ),

        // Sotuv qoidalari
        "an_v1_fact" => (
            "Muddati o'tgan to'lovlar bor.",
            "Есть просроченные платежи.",
        ),
        "an_v1_action" => (
            "Mijozlar bilan bog'laning; grafik qayta ko'rib chiqilishi mumkin.",
            "Свяжитесь с клиентами; график может быть пересмотрен.",
        ),
        "an_v2_fact" => (
            "Sotuv qurilishdan orqada qolmoqda.",
            "Продажи отстают от хода строительства.",
        ),
        "an_v2_action" => (
            "Narx yoki to'lov shartlarini ko'rib chiqing.",
            "Пересмотрите цену или условия оплаты.",
        ),
        "an_v3_fact" => (
            "30 kundan ortiq band qilingan, ammo imzolanmagan shartnomalar bor.",
            "Есть брони старше 30 дней без подписанного договора.",
        ),
        "an_v3_action" => (
            "Bandlikni tasdiqlang yoki bo'shating — kvartira sotuvdan chiqib turibdi.",
            "Подтвердите бронь или освободите — квартира выведена из продажи.",
        ),
        "an_v4_fact" => (
            "Shartnoma bor, to'lov grafigi tuzilmagan.",
            "Договор есть, график платежей не составлен.",
        ),
        "an_v4_action" => (
            "Shartnoma kartochkasida «Grafikni qayta qurish» tugmasini bosing.",
            "Нажмите «Пересобрать график» в карточке договора.",
        ),

        // ---------- VI. Prorab ish o'rni ----------
        "foreman_today" => ("Bugungi kun", "Сегодняшний день"),
        "foreman_hint" => (
            "Kunni shu yerda yoping: bajarilish, jurnal, tabel va smenalar.",
            "Закройте день здесь: выполнение, журнал, табель и смены.",
        ),
        "foreman_tasks" => ("Bugun ketayotgan ishlar", "Работы, идущие сегодня"),
        "foreman_no_tasks" => (
            "Bugunga rejalashtirilgan ish yo'q.",
            "На сегодня работ не запланировано.",
        ),
        "foreman_journal" => ("Kunlik jurnal", "Журнал за день"),
        "foreman_no_journal" => (
            "Bugungi kun uchun jurnal yozuvi yo'q.",
            "Записи журнала за сегодня нет.",
        ),
        "foreman_start_journal" => ("Bugungi yozuvni ochish", "Открыть запись за сегодня"),
        "foreman_journal_saved" => (
            "O'zgarishlar darhol saqlanadi",
            "Изменения сохраняются сразу",
        ),
        "foreman_crew" => ("Brigada va soatlar", "Бригада и часы"),
        "foreman_no_workers" => (
            "Ishchilar ro'yxati bo'sh — «Tabel» bo'limida qo'shing.",
            "Список рабочих пуст — добавьте в разделе «Табель».",
        ),
        "foreman_fill_shift" => ("Butun brigadaga smena", "Смена всей бригаде"),
        "foreman_fill_shift_hint" => (
            "Faol ishchilarning hammasiga bugunga 8 soat qo'yadi; keyin alohida tuzatish mumkin.",
            "Проставит 8 часов за сегодня всем активным рабочим; потом можно поправить.",
        ),
        "foreman_machines" => ("Texnika smenalari", "Смены техники"),
        "foreman_no_machines" => (
            "Texnika kiritilmagan — «Mashinalar» bo'limida qo'shing.",
            "Техника не внесена — добавьте в разделе «Техника».",
        ),
        "foreman_add_shift" => ("Smena ochish", "Открыть смену"),
        "foreman_attention" => ("Diqqat talab qiladi", "Требует внимания"),
        "foreman_all_clear" => (
            "Ochiq xavfsizlik yozuvi va zaxira muammosi yo'q.",
            "Открытых записей по ТБ и проблем с запасом нет.",
        ),
        "open_journal" => ("Jurnalni ochish", "Открыть журнал"),
        "inspection_expired" => ("texnik ko'rik o'tgan", "техосмотр просрочен"),
        "col_wbs" => ("№", "№"),
        "col_progress" => ("Bajarilish", "Выполнение"),
        "kpi_running_today" => ("Bugungi ishlar", "Работ сегодня"),
        "kpi_running_today_hint" => ("rejaga ko'ra", "по графику"),
        "kpi_crew_today" => ("Brigadada", "В бригаде"),
        "kpi_crew_today_hint" => ("tabelga kiritilgan", "внесено в табель"),
        "kpi_machines_today" => ("Texnika smenasi", "Смен техники"),
        "kpi_machines_today_hint" => ("bugun", "сегодня"),
        "kpi_journal_today" => ("Jurnal to'ldirildi", "Журнал заполнен"),
        "kpi_journal_today_hint" => ("bugungi yozuv", "запись за сегодня"),
        "kpi_open_safety" => ("Ochiq xavfsizlik", "Открытых по ТБ"),
        "kpi_open_safety_hint" => ("chora kutmoqda", "ожидают мер"),

        // ---------- Navigatsiya belgilari ----------
        "readiness_storage" => ("baza tayyor, ekran qolgan", "база готова, экран впереди"),
        "readiness_planned" => ("rejalashtirilgan", "запланировано"),
        "legend_storage" => ("baza tayyor, ekran qolgan", "база готова, экран впереди"),
        "legend_planned" => ("rejalashtirilgan", "запланировано"),

        // ---------- XVIII. Yordamchi ----------
        "cp_title" => ("Ma'lumot bo'yicha yordamchi", "Помощник по данным"),
        "cp_disclaimer" => (
            "Bu yerda til modeli ishlatilmaydi: javob faqat shu bazadagi ma'lumotdan hisoblanadi va tekshirilishi mumkin.",
            "Языковая модель здесь не используется: ответ считается только по данным этой базы и может быть проверен.",
        ),
        "cp_placeholder" => (
            "Savolni yozing: «ombor holati», «nima kechikkan», «pul qanday»…",
            "Напишите вопрос: «состояние склада», «что просрочено», «что с деньгами»…",
        ),
        "cp_ask" => ("So'rash", "Спросить"),
        "cp_clear" => ("Tozalash", "Очистить"),
        "cp_check" => ("Tekshirish", "Проверить"),
        "cp_suggestions" => ("Tayyor savollar:", "Готовые вопросы:"),
        "cp_start" => (
            "Savol tanlang yoki o'zingiz yozing.",
            "Выберите вопрос или напишите свой.",
        ),
        "cp_unknown" => (
            "Bu savolga javob bera olmayman.",
            "На этот вопрос ответить не могу.",
        ),
        "cp_unknown_hint" => (
            "Javobni o'ylab topmayman — quyidagi tayyor savollardan birini tanlang.",
            "Я не придумываю ответ — выберите один из готовых вопросов ниже.",
        ),
        "cp_note" => (
            "Javob shu obyektning bazasidagi ma'lumotdan hisoblangan. «Tekshirish» tugmasi manba ekranini ochadi.",
            "Ответ рассчитан по данным базы этого объекта. Кнопка «Проверить» откроет исходный экран.",
        ),

        // Savollar
        "cp_q_overview" => ("Obyekt qanday ketyapti?", "Как идет объект?"),
        "cp_q_attention" => ("Nimaga e'tibor berish kerak?", "На что обратить внимание?"),
        "cp_q_delays" => ("Nima kechikkan?", "Что просрочено?"),
        "cp_q_critical" => ("Kritik yo'lda nima bor?", "Что на критическом пути?"),
        "cp_q_money" => ("Pul holati qanday?", "Что с деньгами?"),
        "cp_q_docs" => ("Hujjatlar tayyormi?", "Готовы ли документы?"),
        "cp_q_supply" => ("Ta'minot qanday?", "Как со снабжением?"),
        "cp_q_stock" => ("Omborda nima bor?", "Что на складе?"),
        "cp_q_crew" => ("Kim ishlayapti?", "Кто работает?"),
        "cp_q_machines" => ("Texnika qanday ishlayapti?", "Как работает техника?"),
        "cp_q_quality" => ("Sifat qanday?", "Как с качеством?"),
        "cp_q_safety" => ("Xavfsizlikda muammo bormi?", "Есть ли проблемы по ТБ?"),
        "cp_q_sales" => ("Sotuv qanday ketyapti?", "Как идут продажи?"),

        // Javob qatorlari
        "cp_l_fact" => ("Bajarilgan", "Выполнено"),
        "cp_l_plan" => ("Rejaga ko'ra", "По плану"),
        "cp_l_delay" => ("Kechikish", "Отставание"),
        "cp_l_forecast" => ("Tugash prognozi", "Прогноз окончания"),
        "cp_l_health" => ("Sog'lomlik indeksi", "Индекс здоровья"),
        "cp_l_none" => ("Natija", "Результат"),
        "cp_l_count" => ("Jami", "Всего"),
        "cp_l_signed" => ("Imzolangan hujjatlar", "Подписанных документов"),
        "cp_l_waiting" => ("Imzo kutmoqda", "Ждут подписи"),
        "cp_l_open_requests" => ("Ochiq arizalar", "Открытых заявок"),
        "cp_l_today_people" => ("Bugun tabelda", "Сегодня в табеле"),
        "cp_l_today_hours" => ("Bugungi soat", "Часов сегодня"),
        "cp_l_no_timesheet" => ("Diqqat", "Внимание"),
        "cp_no_overdue" => ("Muddati o'tgan ish yo'q.", "Просроченных работ нет."),
        "cp_no_critical" => ("Kritik yo'l aniqlanmadi.", "Критический путь не определен."),
        "cp_no_sales" => ("Bu obyekt sotuvda emas.", "Этот объект не в продаже."),

        // ---------- VII. Texnik nazorat kabineti ----------
        "sv_title" => ("Ko'rib chiqish navbati", "Очередь на рассмотрение"),
        "sv_hint" => (
            "Qaror shu ilovada qayd etiladi; masofadan imzolash server qismini talab qiladi.",
            "Решение фиксируется в этом приложении; удаленная подпись требует серверной части.",
        ),
        "sv_kpi_docs" => ("Imzo kutmoqda", "Ждут подписи"),
        "sv_kpi_docs_hint" => ("ijro hujjatlari", "исполнительная документация"),
        "sv_kpi_rejected" => ("Rad etilgan", "Отклонено"),
        "sv_kpi_rejected_hint" => ("qayta ishlash kerak", "требуется доработка"),
        "sv_kpi_quality" => ("Sifat bo'yicha", "По качеству"),
        "sv_kpi_quality_hint" => ("mos emas yoki shartli", "не соответствует или условно"),
        "sv_kpi_issues" => ("Kritik nomuvofiqlik", "Критических несоответствий"),
        "sv_kpi_issues_hint" => ("yopilmagan", "не закрыто"),
        "sv_kpi_safety" => ("Xavfsizlik muddati", "Просрочка по ТБ"),
        "sv_kpi_safety_hint" => ("chora ko'rilmagan", "меры не приняты"),
        "sv_docs" => ("Ijro hujjatlari", "Исполнительная документация"),
        "sv_docs_empty" => ("Imzo kutayotgan hujjat yo'q.", "Документов, ждущих подписи, нет."),
        "sv_sign" => ("Imzolash", "Подписать"),
        "sv_reject" => ("Rad etish", "Отклонить"),
        "sv_open_docs" => ("Barcha hujjatlar", "Все документы"),
        "sv_quality" => ("Sifat nazorati", "Контроль качества"),
        "sv_quality_empty" => ("Barcha yozuvlar talabga mos.", "Все записи соответствуют требованиям."),
        "sv_open_quality" => ("Sifat bo'limi", "Раздел качества"),
        "sv_issues" => ("Nomuvofiqliklar", "Несоответствия"),
        "sv_issues_empty" => ("Ochiq jiddiy nomuvofiqlik yo'q.", "Открытых серьезных несоответствий нет."),
        "sv_open_issues" => ("AI tekshiruvi", "AI-проверка"),
        "sv_safety" => ("Mehnat xavfsizligi", "Охрана труда"),
        "sv_safety_empty" => ("Ochiq yozuv yo'q.", "Открытых записей нет."),
        "sv_open_safety" => ("Xavfsizlik bo'limi", "Раздел безопасности"),
        "sv_more" => ("ta yana", "еще"),

        // ---------- VIII. Buyurtmachi kabineti ----------
        "cl_hint" => (
            "Faqat ko'rish uchun: bu yerdan ma'lumot o'zgartirilmaydi.",
            "Только для просмотра: данные отсюда не изменяются.",
        ),
        "cl_progress" => ("Bajarilish", "Выполнение"),
        "cl_deadline" => ("Shartnoma muddati", "Срок по договору"),
        "cl_days_left" => ("kun qoldi", "дней осталось"),
        "cl_days_over" => ("kun o'tdi", "дней просрочено"),
        "cl_forecast" => ("Tugash prognozi", "Прогноз окончания"),
        "cl_on_time" => ("muddatida", "в срок"),
        "cl_contract" => ("Shartnoma summasi", "Сумма договора"),
        "cl_contract_hint" => ("qurilish bo'yicha", "по строительству"),
        "cl_paid" => ("To'langan", "Оплачено"),
        "cl_of_contract" => ("shartnomadan", "от договора"),
        "cl_sections" => ("Bo'limlar bo'yicha bajarilish", "Выполнение по разделам"),
        "cl_no_sections" => ("Ishlar kiritilmagan.", "Работы не внесены."),
        "cl_finance" => ("Moliya", "Финансы"),
        "cl_estimate" => ("Smeta summasi", "Сумма сметы"),
        "cl_earned" => ("Bajarilgan ish qiymati", "Стоимость выполненных работ"),
        "cl_unpaid" => ("Bajarilgan, to'lanmagan", "Выполнено, не оплачено"),
        "cl_finance_note" => (
            "Bajarilgan ish qiymati shartnoma summasi va bajarilish foizidan hisoblanadi.",
            "Стоимость выполненных работ считается из суммы договора и процента выполнения.",
        ),
        "cl_recent" => ("Oxirgi ish kunlari", "Последние рабочие дни"),
        "cl_no_recent" => ("Jurnal yozuvlari yo'q.", "Записей журнала нет."),
        "cl_sales" => ("Sotuv holati", "Состояние продаж"),
        "cl_sold" => ("Sotilgan birliklar", "Продано единиц"),

        // ---------- IX.8-10, 31-32. Kelishuv marshruti ----------
        "ad_pending" => ("Kutilmoqda", "Ожидает"),
        "ad_approved" => ("Kelishildi", "Согласовано"),
        "ad_rejected" => ("Rad etildi", "Отклонено"),

        "col_route" => ("Kelishuv", "Согласование"),
        "route_none" => ("marshrut yo'q", "маршрута нет"),
        "route_waiting" => ("kutilmoqda", "ожидает"),
        "route_approved" => ("kelishildi", "согласовано"),
        "route_rejected" => ("rad etdi:", "отклонил:"),
        "route_open_hint" => (
            "Kelishuv marshrutini ochish",
            "Открыть маршрут согласования",
        ),

        "route_title" => ("Kelishuv marshruti", "Маршрут согласования"),
        "route_amount" => ("Ariza summasi", "Сумма заявки"),
        "route_by_limit" => ("Marshrut:", "Маршрут:"),
        "route_limits_hint" => (
            "Marshrut summaga qarab ochiladi: 10 mln gacha — prorab, 100 mln gacha — loyiha rahbari ham, undan yuqorisi — direktor ham.",
            "Маршрут зависит от суммы: до 10 млн — прораб, до 100 млн — ещё руководитель проекта, выше — ещё директор.",
        ),
        "route_over_budget" => (
            "Bo'lim byudjetidan oshadi:",
            "Превышает бюджет раздела на:",
        ),
        "route_budget_left" => (
            "Byudjetda qoladi:",
            "Останется в бюджете:",
        ),
        "route_no_budget" => (
            "Bu bo'lim uchun byudjet belgilanmagan",
            "Бюджет по этому разделу не задан",
        ),
        "route_not_built" => (
            "Marshrut hali ochilmagan",
            "Маршрут ещё не открыт",
        ),
        "route_build" => ("Marshrutni ochish", "Открыть маршрут"),
        "route_build_hint" => (
            "Ariza summasiga mos bosqichlar yaratiladi.",
            "Создаются этапы, соответствующие сумме заявки.",
        ),
        "route_stale" => (
            "Marshrut ariza summasiga mos emas — qayta oching",
            "Маршрут не соответствует сумме заявки — пересоздайте",
        ),
        "route_rebuild" => ("Marshrutni qayta ochish", "Пересоздать маршрут"),
        "route_rebuild_hint" => (
            "Barcha bosqichlar va qarorlar o'chiriladi.",
            "Все этапы и решения будут удалены.",
        ),
        "route_approve" => ("Kelishish", "Согласовать"),
        "route_reject" => ("Rad etish", "Отклонить"),
        "route_wrong_role" => (
            "Bu bosqichni kelishadi:",
            "Этот этап согласует:",
        ),
        "route_unknown_user" => ("Foydalanuvchi tanlanmagan", "Пользователь не выбран"),

        "reject_reason" => ("Rad etish sababi", "Причина отклонения"),
        "reject_reason_hint" => (
            "Nima uchun rad etildi va nima qilish kerak",
            "Почему отклонено и что нужно сделать",
        ),
        "reject_reason_missing" => (
            "Sabab yozilmagan — arizani bergan odam nima qilishini bilmaydi",
            "Причина не указана — заявитель не поймёт, что делать",
        ),

        // ---------- X.7-15, 30, 34-35. KP, yetkazib beruvchilar, byudjet ----------
        "pu_tab_orders" => ("Buyurtmalar", "Заказы"),
        "pu_tab_quotes" => ("Tijorat takliflari", "Коммерческие предложения"),
        "pu_tab_suppliers" => ("Yetkazib beruvchilar", "Поставщики"),
        "pu_tab_budget" => ("Byudjet", "Бюджет"),

        "col_section" => ("Bo'lim", "Раздел"),
        "col_delivered" => ("Kelgan", "Поступило"),
        "purchase_partial" => ("Qisman yetkazilgan, qoldi:", "Поставлено частично, осталось:"),
        "price_anomaly" => (
            "Narx katalogdagidan farq qiladi:",
            "Цена отличается от каталожной:",
        ),

        // Tijorat takliflari (TZ X.9-12, 15)
        "add_quote" => ("+ Taklif", "+ Предложение"),
        "quotes_hint" => (
            "Bir arizaga bir nechta taklif kiritiladi va solishtiriladi. Eng arzoni va eng tezi belgilanadi, tanlov sizniki.",
            "На одну заявку вносится несколько предложений и сравнивается. Отмечаются самое дешёвое и самое быстрое, выбор за вами.",
        ),
        "quotes_empty" => (
            "Tijorat taklifi kiritilmagan — «+ Taklif» bilan boshlang",
            "Коммерческие предложения не внесены — начните с «+ Предложение»",
        ),
        "col_over_best" => ("Eng arzondan", "От лучшей"),
        "col_delivery_days" => ("Muddat, kun", "Срок, дней"),
        "col_valid_until" => ("Kuchda", "Действует до"),
        "col_verdict" => ("Xulosa", "Вывод"),
        "quote_cheapest" => ("eng arzon", "самое дешёвое"),
        "quote_fastest" => ("eng tez", "самое быстрое"),
        "quote_best" => ("arzon va tez", "дешевле и быстрее"),
        "quote_expired" => ("muddati o'tgan", "срок истёк"),
        "quote_choose" => ("Tanlash", "Выбрать"),
        "quote_choose_hint" => (
            "Bir arizada faqat bitta taklif tanlangan bo'ladi.",
            "По одной заявке выбирается только одно предложение.",
        ),
        "quote_chosen" => ("tanlandi", "выбрано"),
        "quote_to_purchase" => ("Xarid ochish", "Создать заказ"),
        "quote_purchase_created" => (
            "Taklif bo'yicha xarid ochildi",
            "По предложению создан заказ",
        ),

        // Yetkazib beruvchilar (TZ X.7-8, 16, 40)
        "add_supplier" => ("+ Yetkazib beruvchi", "+ Поставщик"),
        "supplier_new_name" => ("Yangi yetkazib beruvchi", "Новый поставщик"),
        "suppliers_from_purchases" => (
            "Xaridlardan to'ldirish",
            "Заполнить из заказов",
        ),
        "suppliers_hint" => (
            "Tarix alohida saqlanmaydi — u xaridlardan hisoblanadi, shuning uchun kartochka va haqiqiy buyurtmalar hech qachon zid bo'lmaydi.",
            "История не хранится отдельно — она считается из заказов, поэтому карточка и реальные заказы никогда не расходятся.",
        ),
        "suppliers_empty" => (
            "Yetkazib beruvchi kartochkasi yo'q — «Xaridlardan to'ldirish» tugmasini bosing",
            "Карточек поставщиков нет — нажмите «Заполнить из заказов»",
        ),
        "col_inn" => ("STIR", "ИНН"),
        "col_contact" => ("Aloqa", "Контакт"),
        "col_phone" => ("Telefon", "Телефон"),
        "col_orders" => ("Buyurtma", "Заказов"),
        "orders_open_hint" => (
            "Jami buyurtma · shundan to'liq yetkazilmagani",
            "Всего заказов · из них не поставленных полностью",
        ),
        "col_on_time" => ("Muddatida", "В срок"),
        "col_avg_delay" => ("O'rtacha kechikish", "Средняя задержка"),
        "col_last_order" => ("Oxirgi buyurtma", "Последний заказ"),
        "col_blocked" => ("Taqiq", "Блок"),
        "supplier_blocked" => ("ishlamaymiz", "не работаем"),

        // Byudjet (TZ X.34-35)
        "add_budget" => ("+ Bo'lim byudjeti", "+ Бюджет раздела"),
        "budget_hint" => (
            "Reja bo'lim bo'yicha qo'lda qo'yiladi, sarflangani xaridlardan hisoblanadi. Oshib ketgani qizil bo'ladi.",
            "План по разделу задаётся вручную, расход считается из заказов. Превышение выделяется красным.",
        ),
        "budget_empty" => (
            "Byudjet ham, xarid ham yo'q",
            "Нет ни бюджета, ни заказов",
        ),
        "budget_all_sections" => (
            "Barcha bo'limlar uchun byudjet allaqachon bor",
            "Бюджет уже задан для всех разделов",
        ),
        "col_ordered" => ("Buyurtma qilingan", "Заказано"),
        "col_left" => ("Qoldi", "Остаток"),
        "kpi_budget_planned" => ("Reja", "План"),
        "kpi_budget_planned_hint" => ("bo'limlar bo'yicha", "по разделам"),
        "kpi_budget_ordered" => ("Buyurtma qilingan", "Заказано"),
        "kpi_budget_ordered_hint" => ("barcha xaridlar", "все заказы"),
        "kpi_budget_left" => ("Qoldi", "Остаток"),
        "kpi_budget_left_hint" => ("rejadan", "от плана"),
        "kpi_budget_over" => ("Oshib ketgan", "Превышено"),
        "kpi_budget_over_hint" => ("bo'lim", "разделов"),

        // ---------- XVIII.12-30, 44. Takliflar va bajarish ----------
        "cp_tab_ask" => ("Savol-javob", "Вопрос-ответ"),
        "cp_tab_actions" => ("Takliflar", "Предложения"),
        "cp_actions_hint" => (
            "Bazadagi holatdan chiqqan takliflar. Har biri qoralama: nima qilinishi va qaysi sondan chiqqani yozilgan. Bajarish uchun tasdiqlash kerak.",
            "Предложения, вытекающие из состояния базы. Каждое — черновик: указано, что будет сделано и из каких цифр это следует. Для выполнения нужно подтверждение.",
        ),
        "cp_actions_empty" => (
            "Hozircha taklif yo'q",
            "Предложений пока нет",
        ),
        "cp_actions_empty_hint" => (
            "Ta'minot, hujjatlar, sifat va xavfsizlik bo'yicha ochiq masala topilmadi",
            "По снабжению, документам, качеству и безопасности открытых вопросов не найдено",
        ),
        "cp_run" => ("Bajarish", "Выполнить"),
        "cp_open" => ("Ochish", "Открыть"),
        "cp_confirm_q" => (
            "Yozuv yaratiladi. Bajarilsinmi?",
            "Будет создана запись. Выполнить?",
        ),
        "cp_confirm_yes" => ("Ha, bajarilsin", "Да, выполнить"),

        // Taklif matnlari
        "ac_create_request" => ("Ariza ochish:", "Открыть заявку:"),
        "ac_create_purchase" => ("Xarid ochish, ariza:", "Открыть заказ, заявка:"),
        "ac_post_stock" => ("Omborga kirim qilish:", "Оприходовать на склад:"),
        "ac_create_doc" => ("Ijro hujjati ochish:", "Открыть исполнительный документ:"),
        "ac_set_deadline" => ("Bartaraf etish muddatini qo'yish:", "Задать срок устранения:"),
        "ac_close_permit" => ("Naryadni yopish:", "Закрыть наряд:"),
        "ac_build_route" => ("Kelishuv marshrutini ochish:", "Открыть маршрут согласования:"),

        "ac_free" => ("erkin qoldiq", "свободный остаток"),
        "ac_min" => ("minimal zaxira", "минимальный запас"),
        "ac_needed" => ("kerak:", "нужно:"),
        "ac_arrived" => ("kelgan:", "поступило:"),
        "ac_done" => ("ish tugallangan, bo'lim", "работа завершена, раздел"),
        "ac_permit_expired" => ("muddati tugagan:", "срок истёк:"),
        "ac_from_copilot" => (
            "Yordamchi taklifi bo'yicha yaratildi",
            "Создано по предложению помощника",
        ),

        // Bajarish natijalari
        "ac_done_request" => ("Ariza ochildi:", "Заявка открыта:"),
        "ac_done_purchase" => ("Xarid ochildi:", "Заказ открыт:"),
        "ac_done_stock" => ("Omborga kirim qilindi:", "Оприходовано на склад:"),
        "ac_done_doc" => ("Hujjat ochildi:", "Документ открыт:"),
        "ac_done_deadline" => ("Muddat qo'yildi:", "Срок задан:"),
        "ac_done_permit" => ("Naryad yopildi:", "Наряд закрыт:"),
        "ac_done_route" => ("Marshrut ochildi, bosqich:", "Маршрут открыт, этапов:"),

        // Xatolar
        "ac_no_material" => ("Material topilmadi", "Материал не найден"),
        "ac_no_request" => ("Ariza topilmadi", "Заявка не найдена"),
        "ac_no_purchase" => ("Xarid topilmadi", "Заказ не найден"),
        "ac_no_task" => ("Ish topilmadi", "Работа не найдена"),
        "ac_no_check" => ("Tekshiruv topilmadi", "Проверка не найдена"),
        "ac_no_inspection" => ("Tekshiruv topilmadi", "Проверка не найдена"),
        "ac_no_permit" => ("Naryad topilmadi", "Наряд не найден"),
        "ac_check_passed" => (
            "Tekshiruv o'tgan — muddat kerak emas",
            "Проверка пройдена — срок не нужен",
        ),

        // ---------- XI. Omborlar, partiyalar, rezerv, inventarizatsiya ----------
        // Ombor turlari (TZ XI.3)
        "wk_central" => ("Markaziy", "Центральный"),
        "wk_object" => ("Obyekt ombori", "Склад объекта"),
        "wk_temp" => ("Vaqtinchalik", "Временный"),
        "wk_open" => ("Ochiq maydon", "Открытая площадка"),
        "wk_fuel" => ("YoMM", "ГСМ"),
        "wk_tool" => ("Asboblar", "Инструмент"),
        "wk_equip" => ("Jihozlar", "Оборудование"),
        "wk_wear" => ("Ish kiyimi va SIZ", "Спецодежда и СИЗ"),
        "wk_returns" => ("Qaytgan material", "Возвратные материалы"),

        "mk_return" => ("Qaytarish", "Возврат"),
        "wh_all" => ("Barcha omborlar", "Все склады"),
        "add_warehouse" => ("+ Ombor", "+ Склад"),
        "add_warehouse_hint" => (
            "Obyektda bir nechta ombor bo'lishi mumkin: markaziy, vaqtinchalik, YoMM, asbob va boshqalar. Har birining qoldig'i alohida yuritiladi.",
            "На объекте может быть несколько складов: центральный, временный, ГСМ, инструмент и другие. Остаток каждого ведется отдельно.",
        ),
        "warehouse_new_name" => ("Ombor", "Склад"),
        "wh_add_return" => ("+ Qaytarish", "+ Возврат"),
        "wh_add_transfer" => ("+ Ko'chirish", "+ Перемещение"),
        "writeoff_blind" => (
            "Sababi yozilmagan hisobdan chiqarish bor — «Izoh» ustuniga sababni yozing",
            "Есть списания без причины — укажите причину в графе «Примечание»",
        ),
        "stock_idle_hint" => (
            "90 kundan beri harakat yo'q — material uzoq turibdi. Boshqa obyektga berish yoki sotishni ko'rib chiqing.",
            "Более 90 дней нет движения — материал залежался. Рассмотрите передачу на другой объект или продажу.",
        ),
        "wh_transfer_hint" => (
            "Ikki yozuv yaratiladi: bir ombordan chiqim, ikkinchisiga kirim. Ikkalasi bir hujjat raqami ostida turadi.",
            "Создаются две записи: расход с одного склада и приход на другой. Обе под одним номером документа.",
        ),
        "wh_transfer_need_two" => (
            "Ko'chirish uchun kamida ikkita ombor kerak",
            "Для перемещения нужно минимум два склада",
        ),
        "wh_tab_batches" => ("Partiyalar", "Партии"),
        "wh_tab_reserve" => ("Rezerv", "Резерв"),
        "wh_tab_inventory" => ("Inventarizatsiya", "Инвентаризация"),

        "col_warehouse" => ("Ombor", "Склад"),
        "col_batch" => ("Partiya", "Партия"),
        "col_reserved" => ("Rezervda", "В резерве"),
        "col_available" => ("Erkin qoldiq", "Свободный остаток"),
        "col_received" => ("Kelgan sana", "Дата прихода"),
        "col_expires" => ("Yaroqlilik", "Годен до"),
        "col_fefo" => ("Navbat", "Очередь"),
        "col_until" => ("Muddat", "Срок"),
        "col_book" => ("Hisob bo'yicha", "По учету"),

        "kpi_reserved" => ("Rezervda", "В резерве"),
        "kpi_reserved_hint" => ("ta material band", "материалов забронировано"),

        // Partiyalar (TZ XI.9-10, XI.28)
        "add_batch" => ("+ Partiya", "+ Партия"),
        "batches_hint" => (
            "Sertifikat va yaroqlilik muddati partiyaga bog'lanadi. Navbat FEFO bo'yicha: muddati birinchi tugaydigan partiya birinchi ishlatiladi.",
            "Сертификат и срок годности привязаны к партии. Очередь по FEFO: первой используется партия, у которой раньше истекает срок.",
        ),
        "batches_empty" => (
            "Partiya yozilmagan — «+ Partiya» bilan boshlang",
            "Партии не внесены — начните с «+ Партия»",
        ),
        "batch_next" => ("keyingi ishlatiladi", "используется следующей"),
        "batch_expired" => ("muddati o'tgan", "срок истек"),
        "cert_stale" => ("sertifikat muddati o'tgan", "сертификат просрочен"),
        "cert_stale_hint" => (
            "Sertifikat amal qilish muddati tugagan — bunday partiyani ishga bermaslik kerak.",
            "Срок действия сертификата истек — такую партию нельзя выдавать в работу.",
        ),

        // Rezerv (TZ XI.17)
        "add_reservation" => ("+ Rezerv", "+ Резерв"),
        "reserve_hint" => (
            "Rezerv qoldiqni kamaytirmaydi, erkin qoldiqni kamaytiradi: material aniq ishga band qilinadi.",
            "Резерв не уменьшает остаток, он уменьшает свободный остаток: материал бронируется под конкретную работу.",
        ),
        "reserve_empty" => (
            "Rezerv yo'q — material aniq ishga band qilinmagan",
            "Резервов нет — материал не забронирован под работу",
        ),
        "reserve_expired" => ("muddati o'tgan", "срок истек"),
        "reserve_over" => ("yetishmaydi:", "не хватает:"),

        // Inventarizatsiya (TZ XI.24-25)
        "start_inventory" => ("Inventarizatsiya boshlash", "Начать инвентаризацию"),
        "start_inventory_hint" => (
            "Hozirgi qoldiq «hisob bo'yicha» ustuniga yozib qo'yiladi; siz faqat haqiqiy miqdorni kiritasiz.",
            "Текущий остаток записывается в графу «по учету»; вы вносите только фактическое количество.",
        ),
        "inventory_hint" => (
            "Yopilgandan keyin farqlar tuzatuvchi harakatga aylanadi va o'zgartirilmaydi.",
            "После закрытия расхождения превращаются в корректирующие движения и не изменяются.",
        ),
        "inventory_empty" => (
            "Inventarizatsiya o'tkazilmagan",
            "Инвентаризация не проводилась",
        ),
        "inventory_started" => ("Inventarizatsiya boshlandi", "Инвентаризация начата"),
        "inventory_closed" => ("Inventarizatsiya yopildi, farqlar:", "Инвентаризация закрыта, расхождений:"),
        "close_inventory" => ("Yopish", "Закрыть"),
        "close_inventory_hint" => (
            "Farqlar bo'yicha tuzatuvchi harakat yoziladi: kamomad — hisobdan chiqarish, ortiqcha — kirim.",
            "По расхождениям записываются корректирующие движения: недостача — списание, излишек — приход.",
        ),
        "inv_open" => ("ochiq", "открыта"),
        "inv_closed" => ("yopilgan", "закрыта"),
        "inv_closed_note" => (
            "Yopilgan — o'zgartirib bo'lmaydi",
            "Закрыта — изменить нельзя",
        ),
        "inv_no_diff" => ("Farq yo'q", "Расхождений нет"),
        "inv_diff_count" => ("Farqlar", "Расхождений"),
        "inv_short" => ("Kamomad", "Недостача"),
        "inv_over" => ("Ortiqcha", "Излишек"),
        "inv_document" => ("Inventarizatsiya", "Инвентаризация"),
        "inv_adjust_note" => (
            "Inventarizatsiya bo'yicha tuzatish",
            "Корректировка по инвентаризации",
        ),

        // ---------- XI.14-15, XII.21-22. Sarf normalari ----------
        "mat_tab_catalog" => ("Katalog", "Каталог"),
        "mat_tab_norms" => ("Sarf normalari", "Нормы расхода"),
        "mat_tab_usage" => ("Normativ / fakt", "Норма / факт"),

        "add_norm" => ("+ Norma", "+ Норма"),
        "norms_hint" => (
            "Ishning bir birligiga qancha material ketishi. Normativ sarf shundan hisoblanadi: norma × bajarilgan hajm.",
            "Сколько материала уходит на единицу работы. Нормативный расход считается отсюда: норма × выполненный объем.",
        ),
        "norms_empty" => (
            "Norma kiritilmagan — «Norma» bilan boshlang",
            "Нормы не заданы — начните с «Норма»",
        ),
        "norm_needs_task" => (
            "Norma uchun avval GPRda ish va katalogda material bo'lishi kerak",
            "Для нормы сначала нужна работа в ГПР и материал в каталоге",
        ),
        "col_per_unit" => ("Bir birlikka", "На единицу"),
        "col_tolerance" => ("Ruxsat", "Допуск"),
        "col_done_volume" => ("Bajarilgan", "Выполнено"),
        "col_norm" => ("Normativ", "Норматив"),
        "col_over_cost" => ("Ortiqcha summa", "Сумма перерасхода"),

        "usage_empty" => (
            "Taqqoslash uchun sarf normasi kerak — «Sarf normalari» ko'rinishiga o'ting",
            "Для сравнения нужна норма расхода — перейдите в «Нормы расхода»",
        ),
        "usage_hint" => (
            "Normativ sarf bajarilgan hajmga qarab hisoblanadi. Ruxsat etilgan foizdan oshgani qizil bo'ladi.",
            "Нормативный расход считается по выполненному объему. Превышение сверх допуска выделено красным.",
        ),
        "kpi_norm_lines" => ("Normalar", "Норм"),
        "kpi_norm_lines_hint" => ("ish × material", "работа × материал"),
        "kpi_overuse" => ("Ortiqcha sarf", "Перерасход"),
        "kpi_overuse_hint" => ("ruxsatdan oshgan", "сверх допуска"),
        "kpi_overuse_cost" => ("Ortiqcha summa", "Сумма перерасхода"),
        "kpi_overuse_cost_hint" => ("ortiqcha sarf qiymati", "стоимость перерасхода"),

        // ---------- Rasmiy hujjatlar: KS-2, KS-3, M-29, AOSR ----------
        "doc_number" => ("Hujjat raqami", "Номер документа"),
        "doc_date" => ("Sana", "Дата"),
        "doc_object" => ("Obyekt", "Объект"),
        "doc_address" => ("Manzil", "Адрес"),
        "doc_client" => ("Buyurtmachi", "Заказчик"),
        "doc_contractor" => ("Pudratchi", "Подрядчик"),
        "doc_period" => ("Hisobot davri", "Отчётный период"),
        "doc_signatures" => ("Imzolar", "Подписи"),
        "doc_sign_line" => ("imzo _______________", "подпись _______________"),
        "doc_made_by" => (
            "Hujjat QURAi da bazadagi ma'lumotdan tuzildi. Imzolash va tasdiqlash — qog'ozda.",
            "Документ сформирован в QURAi по данным базы. Подписание и утверждение — на бумаге.",
        ),
        "doc_pos" => ("№", "№"),
        "doc_save" => ("Hujjatni saqlash", "Сохранить документ"),

        // ---------- Amallar tarixi ----------
        "au_insert" => ("qo'shildi", "добавлено"),
        "au_update" => ("o'zgartirildi", "изменено"),
        "au_delete" => ("o'chirildi", "удалено"),
        "set_group_audit" => ("Amallar tarixi", "История действий"),
        "set_audit_hint" => (
            "Kim nima o'zgartirgani yozib boriladi. Baza fayli ochiq bo'lgani uchun bu himoya emas — bu tiklash va tushuntirish vositasi: son qayerdan kelganini keyin ham aytib beradi.",
            "Записывается, кто что изменил. Файл базы открыт, поэтому это не защита — это средство восстановления и объяснения: откуда взялась цифра, можно будет узнать позже.",
        ),
        "set_audit_count" => ("Yozuvlar", "Записей"),
        "set_audit_empty" => (
            "Hali hech narsa o'zgartirilmagan",
            "Пока ничего не изменялось",
        ),
        "col_when" => ("Qachon", "Когда"),
        "col_who" => ("Kim", "Кто"),

        "cl_contracts" => ("Shartnomalar", "Договоры"),
        "cl_no_contracts" => ("Shartnoma kiritilmagan", "Договоры не внесены"),
        "cl_no_change" => ("o'zgarishsiz", "без изменений"),
        "cl_pending" => ("kelishuvda:", "на согласовании:"),
        "cl_days" => ("kun", "дн."),
        "cl_payments" => ("To'lov intizomi", "Платёжная дисциплина"),
        "cl_pay_progress" => ("Jadval bajarilishi", "Выполнение графика"),
        "cl_pay_planned" => ("Jadval bo'yicha", "По графику"),
        "cl_pay_paid" => ("To'langan", "Оплачено"),
        "cl_pay_debt" => ("Muddati o'tgan qarz", "Просроченный долг"),
        "cl_pay_soon" => ("30 kun ichida to'lash", "К оплате за 30 дней"),
        "cl_no_debt" => ("qarz yo'q", "долгов нет"),
        "cl_days_late" => ("kun kechikish", "дн. просрочки"),
        "cl_acceptance" => ("Qabulingizni kutmoqda", "Ожидают вашей приёмки"),
        "cl_acceptance_hint" => (
            "Qaror «Shartnomalar» ekranida qabul qilinadi.",
            "Решение принимается на экране «Договоры».",
        ),
        "cl_week" => ("Haftalik hisobot", "Недельный отчёт"),
        "cl_w_progress" => ("Bajarilish", "Выполнение"),
        "cl_w_tasks" => ("Tugallandi / boshlandi", "Завершено / начато"),
        "cl_w_journal" => ("Jurnal yozilgan kun", "Дней с записью в журнале"),
        "cl_w_inspections" => ("Tekshiruv / salbiy", "Проверок / отрицательных"),
        "cl_w_issues" => ("Yangi nomuvofiqlik", "Новых замечаний"),
        "cl_w_docs" => ("Imzolangan hujjat", "Подписано документов"),
        "cl_w_paid" => ("Haftada to'langan", "Оплачено за неделю"),
        "cl_w_waiting" => ("Sizni kutmoqda: o'zgarish · qabul", "Ждут вас: изменения · приёмка"),

        "pf_move_title" => ("Obyektlar orasida ko'chirish", "Перемещение между объектами"),
        "pf_move_hint" => (
            "Bir obyektda ortiqcha, boshqasida yetishmayapti. Ko'chirish sotib olishdan arzon — lekin qaror sizniki.",
            "На одном объекте излишек, на другом нехватка. Перемещение дешевле закупки — но решение за вами.",
        ),
        "pf_move_saving" => ("tejaladi", "экономии"),
        "pf_move_from" => ("Qayerdan", "Откуда"),
        "pf_move_to" => ("Qayerga", "Куда"),
        "pf_move_value" => ("Qiymati", "Стоимость"),

        // ---------- III.33. Smeta zanjiri ----------
        "tab_chain" => ("Zanjir", "Цепочка"),
        "es_chain_hint" => (
            "Pul smetadan chiqib, ariza va xarid orqali omborga, u yerdan ishga o'tadi. Har bosqich bir qatorda — uzilish darhol ko'rinadi.",
            "Деньги идут из сметы через заявку и закупку на склад, оттуда в работу. Все этапы в одной строке — разрыв виден сразу.",
        ),
        "es_chain_empty" => (
            "Zanjir uchun ma'lumot yetarli emas",
            "Недостаточно данных для цепочки",
        ),
        "es_chain_planned" => ("Smeta bo'yicha", "По смете"),
        "es_chain_planned_hint" => ("ishlarga bog'langan pozitsiyalar", "позиции, привязанные к работам"),
        "es_chain_purchased" => ("Xarid qilingan", "Закуплено"),
        "es_chain_purchased_hint" => ("ishlarga bog'langan xaridlar", "закупки по работам"),
        "es_chain_actual" => ("Haqiqiy tannarx", "Фактическая себестоимость"),
        "es_chain_earned" => ("bajarilganiga to'g'ri keladi", "приходится на выполненное"),
        "es_chain_diff" => ("Farq", "Разница"),
        "es_chain_diff_hint" => ("bajarilgan reja minus fakt", "освоенный план минус факт"),
        "es_chain_gaps" => ("Zanjir uzilishi", "Разрывов в цепочке"),
        "es_chain_gaps_hint" => ("bosqich hujjatsiz o'tgan", "этап прошёл без документа"),
        "es_chain_c_plan" => ("Reja", "План"),
        "es_chain_c_request" => ("Ariza", "Заявка"),
        "es_chain_c_purchase" => ("Xarid", "Закупка"),
        "es_chain_c_issued" => ("Ishga berilgan", "Выдано в работу"),
        "es_chain_c_actual" => ("Fakt", "Факт"),
        "es_chain_c_diff" => ("Farq", "Разница"),
        "es_chain_gap" => ("uzilish bor", "есть разрыв"),

        // ---------- VIII. Kabinet: ogohlantirish, xarid, izoh ----------
        "cl_alerts" => ("Shartnoma bo'yicha ogohlantirishlar", "Предупреждения по договорам"),
        "cl_a_deviation" => ("qiymat dastlabkidan chetga chiqdi:", "стоимость отклонилась от первоначальной на"),
        "cl_a_pending" => ("kelishuvda turibdi,", "на согласовании уже"),
        "cl_a_overdue" => ("to'lov kechikdi,", "платёж просрочен на"),
        "cl_a_contract_overdue" => ("shartnoma muddati o'tdi,", "срок договора истёк"),
        "cl_a_gap" => ("to'lov jadvali summani qoplamaydi:", "график платежей не покрывает сумму:"),
        "cl_a_accept_pending" => ("qabul hujjati javobsiz,", "акт приёмки без ответа уже"),

        "cl_purchases" => ("Yirik xaridlar", "Крупные закупки"),
        "cl_quotes" => ("taklif solishtirilgan", "предложения сравнивались"),
        "cl_no_quotes" => ("taklif solishtirilmagan", "предложения не сравнивались"),

        "cl_remarks" => ("Mening izohlarim", "Мои замечания"),
        "cl_remarks_hint" => (
            "Bu kabinetdagi yagona yozish huquqi: ko'rgan narsangizni shu yerda qayd eting.",
            "Единственное право записи в кабинете: зафиксируйте здесь то, что увидели.",
        ),
        "cl_remarks_empty" => ("Hozircha izoh yo'q", "Замечаний пока нет"),
        "cl_remark_add" => ("Qo'shish", "Добавить"),
        "cl_remark_added" => ("Izoh qo'shildi", "Замечание добавлено"),

        // ---------- XV. Zonalar, inventar va sabab tahlili ----------
        "sf_tab_zones" => ("Zonalar va inventar", "Зоны и инвентарь"),
        "sf_tab_analysis" => ("Tahlil", "Анализ"),

        "zk_danger" => ("Xavfli zona", "Опасная зона"),
        "zk_lifting" => ("Yuk ko'tarish zonasi", "Зона подъёма грузов"),
        "zk_electric" => ("Elektr xavfi", "Электроопасность"),
        "zk_excavation" => ("Yer ishlari", "Земляные работы"),
        "zk_fire" => ("Yong'in inventari", "Пожарный инвентарь"),
        "zk_evacuation" => ("Evakuatsiya", "Эвакуация"),
        "zk_emergency" => ("Favqulodda vaziyat", "Чрезвычайная ситуация"),

        "rc_unknown" => ("Aniqlanmagan", "Не установлена"),
        "rc_no_training" => ("Instruktaj yetarli emas", "Недостаточный инструктаж"),
        "rc_no_ppe" => ("SIZ ishlatilmagan", "Не применялись СИЗ"),
        "rc_bad_equip" => ("Nosoz jihoz", "Неисправное оборудование"),
        "rc_no_barrier" => ("To'siq yoki belgi yo'q", "Нет ограждения или знака"),
        "rc_rush" => ("Shoshilinch ish", "Спешка"),
        "rc_weather" => ("Ob-havo sharoiti", "Погодные условия"),
        "rc_organisation" => ("Ish tashkil etilishi", "Организация работ"),

        "sf_zones_hint" => (
            "Xavfli zona ham, yong'in o'chirgich ham bir ro'yxatda: hammasida savol bitta — joyida turibdimi va muddati o'tmaganmi.",
            "Опасная зона и огнетушитель в одном списке: вопрос ко всем один — на месте ли и не истёк ли срок.",
        ),
        "sf_zones_empty" => ("Zona va inventar ro'yxati bo'sh", "Список зон и инвентаря пуст"),
        "sf_zones_total" => ("Yozuvlar", "Записей"),
        "sf_zones_total_hint" => ("zona va inventar", "зон и инвентаря"),
        "sf_zones_attention" => ("E'tibor kerak", "Требуют внимания"),
        "sf_zones_attention_hint" => ("chora yo'q yoki muddati o'tgan", "нет меры либо истёк срок"),
        "sf_zones_overdue" => ("Muddati o'tgan", "Просрочено"),
        "sf_zones_overdue_hint" => ("tekshiruv", "проверок"),
        "sf_zone_check" => ("Tekshiruv", "Проверка"),
        "sf_zone_ready" => ("Chora", "Мера"),
        "sf_zone_not_ready" => ("chora ko'rilmagan", "мера не принята"),
        "sf_zone_check_due" => ("tekshiruv muddati o'tgan", "просрочена проверка"),
        "sf_zone_ok" => ("tartibda", "в порядке"),

        "sf_analysis_hint" => (
            "Chora simptomga emas, sababga qaratilishi kerak: bitta hodisa tasodif, bir xil sababdagi uchtasi esa tizim nuqsoni.",
            "Мера должна быть направлена на причину, а не на симптом: одно событие — случайность, три по одной причине — сбой системы.",
        ),
        "sf_risks_title" => ("Nimalarga e'tibor berish kerak", "На что обратить внимание"),
        "sf_risks_none" => ("Ochiq xavf topilmadi", "Открытых рисков не найдено"),
        "sf_risk_not_ready" => ("chora ko'rilmagan", "мера не принята"),
        "sf_risk_overdue" => ("tekshiruv muddati o'tgan,", "проверка просрочена на"),
        "sf_risk_days" => ("kun", "дн."),
        "sf_risk_repeated" => ("marta takrorlangan", "раз повторилась"),
        "sf_risk_blocked" => ("ishchini ishga qo'yib bo'lmaydi", "рабочих нельзя допускать к работе"),
        "sf_risk_permits" => ("naryad kamchilik bilan ochiq", "нарядов открыто с замечаниями"),

        "sf_causes_title" => ("Ildiz sabablar", "Коренные причины"),
        "sf_causes_hint" => (
            "Faqat haqiqiy hodisalar: instruktaj va tekshiruv sabab talab qilmaydi.",
            "Только реальные события: инструктаж и проверка причины не требуют.",
        ),
        "sf_cause" => ("Sabab", "Причина"),
        "sf_cause_count" => ("Hodisa", "Событий"),
        "sf_cause_serious" => ("Jiddiy", "Серьёзных"),
        "sf_cause_pct" => ("Ulush", "Доля"),
        "sf_rating_title" => ("Mas'ullar bo'yicha", "По ответственным"),
        "sf_r_events" => ("Hodisa", "Событий"),
        "sf_r_violations" => ("Buzilish", "Нарушений"),
        "sf_r_incidents" => ("Baxtsiz hodisa", "Несчастных случаев"),
        "sf_r_open" => ("Ochiq", "Открыто"),
        "sf_r_overdue" => ("Muddati o'tgan", "Просрочено"),

        // ---------- XVI. Texnika bandligi va ta'miri ----------
        "mch_tab_plan" => ("Bandlik rejasi", "План занятости"),
        "mch_tab_repairs" => ("Ta'mir", "Ремонт"),

        "rk_planned" => ("Rejali ta'mir", "Плановый ремонт"),
        "rk_fault" => ("Nosozlik", "Неисправность"),
        "rk_service_to" => ("Texnik xizmat", "Техобслуживание"),
        "rk_check" => ("Texnik ko'rik", "Техосмотр"),

        "mch_plan_hint" => (
            "Jurnal faktni yozadi, bu esa rejani. Reja bo'lmasa to'qnashuv faqat maydonda ma'lum bo'ladi.",
            "Журнал фиксирует факт, а это — план. Без плана конфликт выяснится только на площадке.",
        ),
        "mch_add_booking" => ("+ Bandlik", "+ Бронь"),
        "mch_plan_empty" => ("Bandlik rejasi bo'sh", "План занятости пуст"),
        "mch_machine" => ("Texnika", "Техника"),
        "mch_from" => ("Boshlanish", "Начало"),
        "mch_to" => ("Tugash", "Окончание"),
        "mch_days" => ("Kun", "Дней"),
        "mch_shifts" => ("Smena/kun", "Смен/день"),
        "mch_conflict" => ("bir vaqtda ikki ishga band,", "занята на две работы одновременно,"),
        "mch_conflict_days" => ("kun kesishadi", "дн. пересечения"),
        "mch_b_conflict" => ("to'qnashuv", "конфликт"),
        "mch_b_past" => ("o'tgan", "прошло"),
        "mch_b_now" => ("hozir ishda", "сейчас в работе"),
        "mch_b_future" => ("oldinda", "впереди"),
        "mch_no_machines" => (
            "Texnika ro'yxati bo'sh: avval texnikani qo'shing.",
            "Список техники пуст: сначала добавьте технику.",
        ),

        "mch_repairs_hint" => (
            "Ta'mir qiymati balans qiymatining 40 % idan oshsa — almashtirish haqida o'ylash kerak.",
            "Если стоимость ремонта превысила 40 % балансовой стоимости — стоит подумать о замене.",
        ),
        "mch_add_repair" => ("+ Ta'mir", "+ Ремонт"),
        "mch_repairs_empty" => ("Ta'mir yozuvi yo'q", "Записей о ремонте нет"),
        "mch_r_in_repair" => ("Ta'mirda", "В ремонте"),
        "mch_r_in_repair_hint" => ("hozir ishlamayapti", "сейчас не работает"),
        "mch_r_cost" => ("Ta'mir xarajati", "Затраты на ремонт"),
        "mch_r_cost_hint" => ("butun davr uchun", "за весь период"),
        "mch_r_downtime" => ("Bo'sh turgan kun", "Дней простоя"),
        "mch_r_downtime_hint" => ("ta'mir tufayli", "из-за ремонта"),
        "mch_r_replace" => ("Almashtirish savoli", "Вопрос замены"),
        "mch_r_replace_hint" => ("ta'mir qiymati chegaradan oshgan", "стоимость ремонта выше предела"),
        "mch_r_replace_title" => (
            "Almashtirish haqida o'ylash kerak",
            "Стоит подумать о замене",
        ),
        "mch_r_of_price" => ("balans qiymatidan", "от балансовой стоимости"),
        "mch_r_started" => ("Boshlandi", "Начат"),
        "mch_r_finished" => ("Tugadi", "Завершён"),
        "mch_r_reason" => ("Sabab", "Причина"),
        "mch_r_amount" => ("Xarajat", "Затраты"),
        "mch_r_hours" => ("Motosoat", "Моточасы"),
        "mch_r_days" => ("kun", "дн."),

        // ---------- XIV. Sinovlar, xavflar va reyting ----------
        "ql_tab_tests" => ("Sinovlar", "Испытания"),
        "ql_tab_risks" => ("Xavflar", "Риски"),

        "lt_weld" => ("Payvand", "Сварка"),
        "lt_pressure" => ("Bosim sinovi", "Опрессовка"),
        "lt_insulation" => ("Izolyatsiya", "Изоляция"),
        "lt_commission" => ("Ishga tushirish", "Пусконаладка"),
        "lt_soil" => ("Grunt", "Грунт"),
        "lt_other" => ("Boshqa", "Прочее"),

        "ltr_waiting" => ("Kutilmoqda", "Ожидается"),
        "ltr_pass" => ("O'tdi", "Прошло"),
        "ltr_fail" => ("O'tmadi", "Не прошло"),

        "ql_add_test" => ("+ Sinov", "+ Испытание"),
        "ql_tests_hint" => (
            "Bu yerda «o'tdi / o'tmadi» muhim. Raqamli qiymat bo'lsa saqlanadi, lekin hukm laboratoriyaniki.",
            "Здесь важно «прошло / не прошло». Числовое значение сохраняется, но заключение — за лабораторией.",
        ),
        "ql_tests_empty" => ("Sinov yozuvi yo'q", "Записей об испытаниях нет"),
        "ql_tests_total" => ("Sinovlar", "Испытаний"),
        "ql_tests_total_hint" => ("jami", "всего"),
        "ql_tests_pending" => ("Natija kutilmoqda", "Ждут результата"),
        "ql_tests_pending_hint" => ("laboratoriyadan", "из лаборатории"),
        "ql_tests_failed" => ("O'tmadi", "Не прошло"),
        "ql_tests_failed_hint" => ("qayta sinov kerak", "нужно повторное испытание"),
        "ql_test_subject" => ("Nima sinaldi", "Что испытывалось"),
        "ql_test_value" => ("Qiymat", "Значение"),
        "ql_test_required" => ("Talab", "Требуется"),
        "ql_test_lab" => ("Laboratoriya", "Лаборатория"),

        "ql_risks_hint" => (
            "Bu bashorat emas — e'tibor ro'yxati: har bir sabab bugungi ma'lumotdan olingan va tekshirib ko'rish mumkin.",
            "Это не предсказание, а список внимания: каждая причина взята из сегодняшних данных и её можно проверить.",
        ),
        "ql_banned_title" => ("Taqiqlangan material ishlatilgan", "Использован запрещённый материал"),
        "ql_banned_hint" => (
            "Taqiq o'z-o'zidan chiqarishni to'xtatmaydi — shuning uchun uni ko'rsatamiz.",
            "Запрет сам по себе не останавливает выдачу — поэтому мы её показываем.",
        ),
        "ql_banned_moves" => ("harakat", "движений"),
        "ql_risks_title" => ("Nuqson ehtimoli yuqori ishlar", "Работы с высоким риском брака"),
        "ql_risks_none" => ("Xavfli ish topilmadi", "Рисковых работ не найдено"),
        "ql_r_past" => ("marta nuqson bo'lgan", "раз были дефекты"),
        "ql_r_delayed" => ("kechikmoqda", "отстаёт на"),
        "ql_r_days" => ("kun", "дн."),
        "ql_r_over" => ("normadan ortiq sarf", "перерасход материала"),
        "ql_r_no_ppr" => ("tasdiqlangan karta yo'q", "нет утверждённой техкарты"),
        "ql_r_no_inspection" => ("tekshiruv o'tkazilmagan", "проверка не проводилась"),

        "ql_rating_title" => ("Mas'ullar bo'yicha sifat", "Качество по ответственным"),
        "ql_rating_hint" => (
            "Ball sifat modulidagi umumiy ball bilan bir xil qoidada hisoblanadi.",
            "Балл считается по тому же правилу, что и общий балл модуля качества.",
        ),
        "ql_rating_name" => ("Mas'ul", "Ответственный"),
        "ql_rating_checks" => ("Tekshiruv", "Проверок"),
        "ql_rating_failed" => ("Salbiy", "Отрицательных"),
        "ql_rating_open" => ("Ochiq nuqson", "Открытых дефектов"),
        "ql_rating_overdue" => ("Muddati o'tgan", "Просрочено"),
        "ql_rating_score" => ("Ball", "Балл"),

        // ---------- XIII. Tabel davri, anomaliyalar va xodim ehtiyoji ----------
        "ts_tab_periods" => ("Davrlar", "Периоды"),
        "ts_tab_staff" => ("Xodimlar", "Персонал"),

        "ts_periods_hint" => (
            "Yopilgan oy tasodifan o'zgarmaydi. Tuzatish kerak bo'lsa davr qaytadan ochiladi va bu iz qoldiradi.",
            "Закрытый месяц не изменится случайно. Если нужна правка — период открывается заново, и это оставляет след.",
        ),
        "ts_periods_empty" => ("Tabelda yozuv yo'q", "В табеле нет записей"),
        "ts_period_month" => ("Oy", "Месяц"),
        "ts_period_days" => ("Kun", "Дней"),
        "ts_period_hours" => ("Soat", "Часов"),
        "ts_period_wage" => ("Ish haqi", "Зарплата"),
        "ts_period_open" => ("ochiq", "открыт"),
        "ts_period_closed" => ("yopilgan", "закрыт"),
        "ts_period_close" => ("Oyni yopish", "Закрыть месяц"),
        "ts_period_reopen" => ("Qayta ochish", "Открыть заново"),
        "ts_period_closed_msg" => ("Oy yopildi", "Месяц закрыт"),
        "ts_period_reopened" => ("Davr qayta ochildi", "Период открыт заново"),
        "ts_period_reopened_by" => ("Qayta ochdi:", "Открыл заново:"),

        "ts_staff_hint" => (
            "Anomaliya ayblov emas: har bir belgi tekshirishga sabab. Ko'pincha ular haqiqiy — avariya kuni yoki topshirish oldidan.",
            "Аномалия — не обвинение: каждый признак повод проверить. Часто они реальны — аварийный день или сдача объекта.",
        ),
        "ts_staff_have" => ("Faol ishchi", "Активных рабочих"),
        "ts_staff_have_hint" => ("ro'yxatda", "в списке"),
        "ts_staff_need" => ("Kerak bo'ladi", "Потребуется"),
        "ts_staff_hours" => ("soat", "часов"),
        "ts_staff_gap" => ("Farq", "Разница"),
        "ts_staff_gap_hint" => ("kelgusi 30 kun uchun", "на ближайшие 30 дней"),
        "ts_staff_anomalies" => ("Anomaliya", "Аномалий"),
        "ts_staff_anomalies_hint" => ("tekshirishga arziydi", "стоит проверить"),
        "ts_staff_clean" => ("Tabelda g'ayrioddiy holat topilmadi", "В табеле аномалий не найдено"),
        "ts_staff_list" => ("Nimalarga e'tibor berish kerak", "На что обратить внимание"),
        "ts_a_hours" => ("soat", "ч"),
        "ts_a_too_many" => ("kunlik chegaradan ko'p", "больше дневного предела"),
        "ts_a_weekend" => ("dam olish kunida ish", "работа в выходной"),
        "ts_a_no_rest" => ("kun ketma-ket, dam olishsiz", "дней подряд без выходного"),
        "ts_a_days" => ("kun davomida", "дней подряд по"),
        "ts_a_identical" => ("soat — bir xil yozilgan", "часов — записано одинаково"),

        // ---------- IX. Ariza turlari va tekshiruvi ----------
        "rk_transport" => ("Transport", "Транспорт"),
        "rk_repair" => ("Ta'mir", "Ремонт"),
        "rk_service" => ("Xizmat", "Услуга"),
        "rk_money" => ("Pul mablag'i", "Денежные средства"),

        "col_check" => ("Tekshiruv", "Проверка"),
        "rq_check_ok" => (
            "Tekshiruvda savol topilmadi.",
            "Проверка не выявила вопросов.",
        ),
        "rq_i_duplicate" => ("Shu material bo'yicha ochiq ariza bor:", "По этому материалу уже есть открытая заявка:"),
        "rq_i_not_in_estimate" => (
            "Material smetada uchramaydi — qo'shimcha ish bo'lishi mumkin.",
            "Материал не найден в смете — возможно, это дополнительные работы.",
        ),
        "rq_i_over_budget" => ("Bo'lim byudjeti oshadi:", "Бюджет раздела будет превышен на:"),
        "rq_i_cheaper" => ("Arzonroq analog bor:", "Есть более дешёвый аналог:"),
        "rq_i_no_task" => (
            "Ish ko'rsatilmagan: kechikish kimga ta'sir qilishi ko'rinmaydi.",
            "Не указана работа: не видно, на что повлияет задержка.",
        ),
        "rq_i_banned" => ("Material taqiqlangan", "Материал запрещён"),

        // ---------- XI.32-34. Asboblar va kamomad ----------
        "wh_tab_tools" => ("Asboblar", "Инструмент"),
        "wh_tab_shortage" => ("Kamomad", "Недостачи"),

        "tk_power" => ("Elektr asbob", "Электроинструмент"),
        "tk_hand" => ("Qo'l asbobi", "Ручной инструмент"),
        "tk_measure" => ("O'lchov asbobi", "Измерительный"),
        "tk_scaffold" => ("Iskala va inventar", "Леса и инвентарь"),
        "tk_other" => ("Boshqa", "Прочее"),

        "tc_good" => ("Yaroqli", "Исправен"),
        "tc_worn" => ("Eskirgan", "Изношен"),
        "tc_repair" => ("Ta'mirda", "В ремонте"),
        "tc_written" => ("Hisobdan chiqarilgan", "Списан"),

        "wh_tools_hint" => (
            "Asbob sarflanmaydi — qaytariladi. Shuning uchun bu yerda qoldiq emas, kimdaligi ko'rsatiladi.",
            "Инструмент не расходуется — он возвращается. Поэтому здесь не остаток, а у кого он находится.",
        ),
        "wh_add_tool" => ("+ Asbob", "+ Инструмент"),
        "wh_tools_empty" => ("Asboblar ro'yxati bo'sh", "Список инструмента пуст"),
        "wh_tools_total" => ("Asboblar", "Инструментов"),
        "wh_tools_total_hint" => ("ro'yxatda", "в списке"),
        "wh_tools_issued" => ("Ishchilarda", "У рабочих"),
        "wh_tools_value" => ("qiymatida", "на сумму"),
        "wh_tools_overdue" => ("Qaytarilmagan", "Не возвращено"),
        "wh_tools_overdue_hint" => ("muddati o'tgan", "срок вышел"),
        "wh_tools_repair" => ("Ishlamaydi", "Не в строю"),
        "wh_tools_repair_hint" => ("ta'mirda yoki hisobdan chiqarilgan", "в ремонте или списан"),
        "wh_tools_check" => ("Tekshiruv muddati", "Просрочена проверка"),
        "wh_tools_check_hint" => ("o'tgan asboblar", "инструментов"),
        "wh_tool_inv" => ("Inventar №", "Инвентарный №"),
        "wh_tool_condition" => ("Holati", "Состояние"),
        "wh_tool_check" => ("Tekshiruv", "Проверка"),
        "wh_tool_holder" => ("Kimda", "У кого"),
        "wh_tool_in_store" => ("omborda", "на складе"),
        "wh_tool_days" => ("kun", "дн."),
        "wh_tool_give" => ("berish", "выдать"),
        "wh_tool_return" => ("qaytarish", "вернуть"),
        "wh_tool_new" => ("Yangi asbob", "Новый инструмент"),
        "wh_tool_no_workers" => (
            "Faol ishchi yo'q: avval ishchilar ro'yxatini to'ldiring.",
            "Нет активных рабочих: сначала заполните список рабочих.",
        ),

        "wh_short_hint" => (
            "Bitta farq xato bo'lishi mumkin; takrorlangan farq esa tizimli sabab — o'lchov, saqlash yoki hisob tartibida.",
            "Одно расхождение может быть ошибкой; повторяющееся — системная причина: замер, хранение или порядок учёта.",
        ),
        "wh_short_none" => ("Kamomad topilmadi", "Недостач нет"),
        "wh_short_none_hint" => (
            "Yopilgan inventarizatsiyalarda farq chiqmagan.",
            "В закрытых инвентаризациях расхождений не было.",
        ),
        "wh_short_total" => ("Kamomad qiymati", "Стоимость недостач"),
        "wh_short_total_hint" => ("yopilgan inventarizatsiyalar bo'yicha", "по закрытым инвентаризациям"),
        "wh_short_repeated" => ("Takrorlangan", "Повторяющихся"),
        "wh_short_repeated_hint" => ("bir necha marta farq chiqqan", "расхождение не в первый раз"),
        "wh_short_times" => ("Marta", "Раз"),
        "wh_short_qty" => ("Kamomad", "Недостача"),
        "wh_short_surplus" => ("Ortiqcha", "Излишек"),
        "wh_short_cost" => ("Qiymati", "Стоимость"),
        "wh_short_note" => ("Xulosa", "Вывод"),
        "wh_short_systematic" => ("tizimli — sababni izlash kerak", "системно — нужно искать причину"),
        "wh_short_single" => ("bir marta — xato bo'lishi mumkin", "единично — возможна ошибка"),

        // ---------- XVII. Prognoz, ssenariy va yo'qotishlar ----------
        "an_tab_forecast" => ("Prognoz", "Прогноз"),
        "an_tab_scenario" => ("Ssenariy", "Сценарий"),
        "an_tab_losses" => ("Yo'qotishlar", "Потери"),

        "an_fc_hint" => (
            "Prognoz bitta taxminga tayanadi: bugungi tannarx darajasi oxirigacha saqlanadi. Boshqa taxmin kiritilmagan.",
            "Прогноз опирается на одно допущение: сегодняшний уровень себестоимости сохранится до конца. Других допущений нет.",
        ),
        "an_fc_too_early" => ("Prognoz uchun hali erta", "Для прогноза ещё рано"),
        "an_fc_too_early_hint" => (
            "Bajarilish 5 % dan kam bo'lganda yakuniy tannarxni hisoblash ishonchsiz.",
            "При выполнении менее 5 % расчёт итоговой себестоимости недостоверен.",
        ),
        "an_fc_contract" => ("Amaldagi summa", "Текущая сумма"),
        "an_fc_contract_hint" => ("shartnoma va kelishilgan o'zgarishlar", "договор и согласованные изменения"),
        "an_fc_cost" => ("Yakuniy tannarx", "Итоговая себестоимость"),
        "an_fc_cost_now" => ("bugungi kunga", "на сегодня"),
        "an_fc_profit" => ("Foyda prognozi", "Прогноз прибыли"),
        "an_fc_margin" => ("summadan", "от суммы"),
        "an_fc_receivable" => ("Debitorlik", "Дебиторская задолженность"),
        "an_fc_overdue" => ("muddati o'tgan", "просрочено"),
        "an_fc_revenue" => ("90 kunda tushum", "Поступления за 90 дней"),
        "an_fc_revenue_hint" => ("to'lov jadvali bo'yicha", "по графику платежей"),
        "an_fc_how" => ("Hisob qanday chiqdi", "Как получен расчёт"),
        "an_fc_how_progress" => ("Bajarilish", "Выполнение"),
        "an_fc_how_cost" => ("Bugungi tannarx", "Себестоимость на сегодня"),
        "an_fc_how_earned" => ("Bajarilgan ish qiymati", "Стоимость выполненных работ"),
        "an_fc_how_rule" => (
            "Yakuniy tannarx = bugungi tannarx ÷ bajarilish ulushi.",
            "Итоговая себестоимость = сегодняшняя ÷ доля выполнения.",
        ),

        "an_sc_hint" => (
            "Bu bashorat emas, arifmetika: berilgan taxminlar bugungi sonlarga qo'llanadi.",
            "Это не предсказание, а арифметика: заданные допущения применяются к сегодняшним числам.",
        ),
        "an_sc_delay" => ("Muddat suriladi, kun", "Срок сдвигается, дней"),
        "an_sc_price" => ("Material narxi", "Цена материалов"),
        "an_sc_wage" => ("Ish haqi", "Зарплата"),
        "an_sc_now" => ("Hozir", "Сейчас"),
        "an_sc_after" => ("Ssenariy bilan", "По сценарию"),
        "an_sc_finish" => ("Tugash sanasi", "Дата окончания"),
        "an_sc_cost" => ("Yakuniy tannarx", "Итоговая себестоимость"),
        "an_sc_profit" => ("Foyda", "Прибыль"),
        "an_sc_over" => (
            "Bu ssenariyda obyekt shartnoma muddatidan chiqib ketadi.",
            "При этом сценарии объект выходит за договорный срок.",
        ),
        "an_sc_note" => (
            "Material va ish haqi ulushlari bugungi tannarx tarkibidan olingan.",
            "Доли материалов и зарплаты взяты из состава сегодняшней себестоимости.",
        ),

        "an_op_hint" => (
            "Har biri pulda o'lchanadi: aniq summa bo'lsa, unga qarab qaror qabul qilinadi.",
            "Каждый пункт измерен в деньгах: по конкретной сумме проще принять решение.",
        ),
        "an_op_none" => ("Yashirin yo'qotish topilmadi", "Скрытых потерь не найдено"),
        "an_op_losses" => ("Yo'qotilgan", "Потеряно"),
        "an_op_losses_hint" => ("ortiqcha sarf va tanlanmagan tejash", "перерасход и упущенная экономия"),
        "an_op_frozen" => ("Muzlatilgan", "Заморожено"),
        "an_op_frozen_hint" => ("omborda va bo'sh texnikada", "на складе и в простаивающей технике"),

        "op_idle_stock" => ("Ortiqcha zaxira", "Избыточный запас"),
        "op_idle_stock_hint" => (
            "Qoldiq minimal zaxiradan uch baravar ko'p — pul omborda turibdi.",
            "Остаток втрое больше минимального запаса — деньги лежат на складе.",
        ),
        "op_idle_machines" => ("birlik texnika bo'sh turibdi", "ед. техники простаивает"),
        "op_idle_machines_hint" => (
            "Oxirgi 30 kunda ish kuni yo'q, lekin xarajati bor.",
            "За последние 30 дней нет рабочих дней, но расходы есть.",
        ),
        "op_over_usage" => ("Normadan ortiq sarf", "Перерасход сверх нормы"),
        "op_over_usage_hint" => (
            "Material normadan ko'p ketgan: isrof yoki hisobda xato.",
            "Материала израсходовано больше нормы: перерасход либо ошибка учёта.",
        ),
        "op_missed_quote" => ("Arzonroq taklif tanlanmagan", "Выбрано не самое дешёвое предложение"),
        "op_missed_quote_hint" => (
            "Xarid narxi eng past taklifdan yuqori — farq tejalmagan.",
            "Цена закупки выше минимального предложения — разница не сэкономлена.",
        ),

        "an_pr_title" => ("Unumdorlik", "Производительность"),
        "an_pr_hint" => (
            "Bir birlik ish qancha soat va qancha pulga tushgani. Eng qimmatlari yuqorida.",
            "Сколько часов и денег уходит на единицу работы. Самые дорогие — сверху.",
        ),
        "an_pr_done" => ("Bajarilgan", "Выполнено"),
        "an_pr_hours" => ("Soat", "Часов"),
        "an_pr_per_unit" => ("Soat/birlik", "Часов/ед."),
        "an_pr_cost_unit" => ("Birlik tannarxi", "Себестоимость ед."),
        "an_pr_vs_avg" => ("O'rtachadan", "От среднего"),

        // ---------- X. Xarid rejasi va risklar ----------
        "pu_tab_plan" => ("Xarid rejasi", "План закупок"),
        "pu_tab_risks" => ("Risklar", "Риски"),

        "pu_plan_hint" => (
            "Ro'yxat qo'lda tuzilmaydi: qoldiq, yo'ldagi buyurtma va yaqin ishlarning normativ ehtiyoji solishtiriladi.",
            "Список не составляется вручную: сравниваются остаток, заказанное в пути и нормативная потребность ближайших работ.",
        ),
        "pu_plan_empty" => ("Sotib olish kerak bo'lgan narsa yo'q", "Закупать нечего"),
        "pu_plan_empty_hint" => (
            "Qoldiq va yo'ldagi buyurtma yaqin ishlar uchun yetarli.",
            "Остатка и заказанного в пути хватает на ближайшие работы.",
        ),
        "pu_plan_kpi_items" => ("Pozitsiya", "Позиций"),
        "pu_plan_kpi_items_hint" => ("sotib olish kerak", "к закупке"),
        "pu_plan_kpi_sum" => ("Taxminiy summa", "Ориентировочно"),
        "pu_plan_kpi_sum_hint" => ("katalog narxi bo'yicha", "по цене каталога"),
        "pu_plan_kpi_tight" => ("Muddat siqilgan", "Сроки поджимают"),
        "pu_plan_kpi_tight_hint" => ("ikki haftadan kam qoldi", "осталось менее двух недель"),
        "pu_plan_available" => ("Erkin qoldiq", "Свободный остаток"),
        "pu_plan_ordered" => ("Yo'lda", "В пути"),
        "pu_plan_needed" => ("Ishlar uchun", "На работы"),
        "pu_plan_to_buy" => ("Sotib olish", "К закупке"),
        "pu_plan_cost" => ("Summa", "Сумма"),
        "pu_plan_need_by" => ("Qachongacha", "К сроку"),
        "pu_plan_has_request" => ("ariza bor", "заявка есть"),
        "pu_plan_no_request" => ("ariza yo'q", "заявки нет"),
        "pu_plan_make" => ("Ariza ochish", "Создать заявку"),
        "pu_plan_make_off" => (
            "Ariza allaqachon bor yoki bu rolda ariza ochib bo'lmaydi.",
            "Заявка уже есть либо эта роль не может создавать заявки.",
        ),
        "pu_plan_from_plan" => ("Xarid rejasidan", "Из плана закупок"),
        "pu_plan_made" => ("Ariza ochildi:", "Создана заявка:"),

        "pu_risks_hint" => (
            "Bu ayblov emas: har bir belgi — tekshirib ko'rish uchun sabab. Xulosa odamniki.",
            "Это не обвинение: каждый признак — повод проверить. Вывод остаётся за человеком.",
        ),
        "pu_risks_title" => ("Diqqat qaratish kerak", "На что обратить внимание"),
        "pu_risks_none" => ("Shubhali belgi topilmadi", "Подозрительных признаков нет"),
        "pu_risk_share" => (
            "bitta yetkazib beruvchining ulushi juda katta",
            "слишком большая доля одного поставщика",
        ),
        "pu_risk_no_quotes" => (
            "taklif solishtirilmagan",
            "предложения не сравнивались",
        ),
        "pu_risk_high_price" => (
            "narx katalogdan yuqori",
            "цена выше каталожной",
        ),
        "pu_risk_urgent" => (
            "shoshilinch xaridlar ulushi katta",
            "велика доля срочных закупок",
        ),

        "pu_buyers_title" => ("Xaridchilar", "Закупщики"),
        "pu_buyers_hint" => (
            "Kim nechta xarid rasmiylashtirgan va qanday sifatda.",
            "Кто сколько закупок оформил и с каким качеством.",
        ),
        "pu_buyers_none" => (
            "Xaridlarda mas'ul ko'rsatilmagan",
            "В закупках не указан ответственный",
        ),
        "pu_buyer" => ("Xaridchi", "Закупщик"),
        "pu_buyer_count" => ("Xarid", "Закупок"),
        "pu_buyer_amount" => ("Summa", "Сумма"),
        "pu_buyer_on_time" => ("Muddatida", "В срок"),
        "pu_buyer_quotes" => ("Taklif bilan", "С предложениями"),
        "pu_buyer_urgent" => ("Shoshilinch", "Срочных"),

        "col_urgent" => ("Shoshilinch", "Срочно"),
        "col_buyer" => ("Xaridchi", "Закупщик"),

        // ---------- Bildirishnomalar markazi ----------
        "screen_notices" => ("Bildirishnomalar", "Уведомления"),
        "nt_hint" => (
            "Barcha modullardan e'tibor talab qiladigan yozuvlar. Ro'yxat saqlanmaydi — muammo hal bo'lsa o'zi yo'qoladi.",
            "Записи из всех модулей, требующие внимания. Список не сохраняется — когда проблема решена, он исчезает сам.",
        ),
        "nt_all_clear" => ("E'tibor talab qiladigan narsa yo'q", "Ничего не требует внимания"),
        "nt_all_clear_hint" => (
            "Muddatlar, tekshiruvlar, to'lovlar va qoldiqlar bo'yicha ochiq savol yo'q.",
            "По срокам, проверкам, платежам и остаткам открытых вопросов нет.",
        ),
        "nt_kpi_total" => ("Jami", "Всего"),
        "nt_kpi_total_hint" => ("bildirishnoma", "уведомлений"),
        "nt_kpi_critical_hint" => ("darhol qarash kerak", "требует немедленного внимания"),
        "nt_kpi_major_hint" => ("bugun hal qilinsin", "решить сегодня"),
        "nt_kpi_warning_hint" => ("kuzatib borish", "держать на контроле"),
        "nt_days" => ("kun", "дн."),
        "nt_worst" => ("eng kattasi", "наибольшее"),
        "nt_worst_result" => ("eng past natija", "наименьший результат"),
        "nt_past_deadline" => ("muddati o'tgan", "с просроченным сроком"),

        "nt_src_schedule" => ("GPR", "ГПР"),
        "nt_src_inspection" => ("Texnik nazorat", "Технадзор"),
        "nt_src_quality" => ("Sifat", "Качество"),
        "nt_src_documents" => ("Hujjatlar", "Документы"),
        "nt_src_supply" => ("Ta'minot", "Снабжение"),
        "nt_src_stock" => ("Ombor", "Склад"),
        "nt_src_safety" => ("Xavfsizlik", "Безопасность"),
        "nt_src_machines" => ("Texnika", "Техника"),
        "nt_src_money" => ("Moliya", "Финансы"),
        "nt_src_client" => ("Buyurtmachi", "Заказчик"),

        "nt_tasks_overdue" => ("ish muddati o'tgan", "работ просрочено"),
        "nt_tasks_not_started" => ("ish boshlanmagan", "работ не начато"),
        "nt_inspections_overdue" => ("tekshiruv o'tkazilmagan", "проверок не проведено"),
        "nt_inspections_today" => ("tekshiruv bugunga", "проверок на сегодня"),
        "nt_inspections_today_hint" => ("chaqiruv bo'yicha", "по вызову"),
        "nt_inspection_defects" => ("tekshiruv nuqsoni ochiq", "дефектов по проверкам открыто"),
        "nt_concrete_due" => ("beton namunasi natijasi yo'q", "образцов бетона без результата"),
        "nt_concrete_due_hint" => ("sinov sanasi keldi", "дата испытания наступила"),
        "nt_concrete_failed" => ("beton talabga yetmadi", "образцов бетона не прошло"),
        "nt_geodesy_out" => ("geodezik nuqta dopuskdan chiqdi", "геодезических точек вне допуска"),
        "nt_geodesy_hint" => ("chetlanish dopuskdan katta", "отклонение больше допуска"),
        "nt_critical_issues" => ("kritik nomuvofiqlik ochiq", "критичных несоответствий открыто"),
        "nt_critical_hint" => ("loyiha yoki smeta tekshiruvidan", "из проверки проекта или сметы"),
        "nt_defects_overdue" => ("sifat nuqsoni muddati o'tgan", "дефектов качества просрочено"),
        "nt_docs_waiting" => ("hujjat imzo kutmoqda", "документов ждут подписи"),
        "nt_docs_waiting_hint" => ("texnik nazorat qaroriga", "решения технадзора"),
        "nt_docs_rejected" => ("hujjat rad etilgan", "документов отклонено"),
        "nt_docs_rejected_hint" => ("qayta ishlash kerak", "нужна доработка"),
        "nt_requests_new" => ("ariza ko'rib chiqilmagan", "заявок не рассмотрено"),
        "nt_requests_new_hint" => ("ta'minot qaroriga", "решения снабжения"),
        "nt_purchases_late" => ("yetkazish kechikdi", "поставок просрочено"),
        "nt_stock_low" => ("material minimal qoldiqdan past", "материалов ниже минимального остатка"),
        "nt_permits_expired" => ("ishchi ruxsatnomasi muddati o'tgan", "рабочих с просроченным допуском"),
        "nt_permits_expired_hint" => ("ishga qo'yib bo'lmaydi", "нельзя допускать к работе"),
        "nt_permits_expiring" => ("ruxsatnoma tez orada tugaydi", "допусков скоро истекают"),
        "nt_permits_expiring_hint" => ("oldindan yangilash kerak", "нужно продлить заранее"),
        "nt_ppe_missing" => ("ishchida majburiy SIZ yo'q", "рабочих без обязательных СИЗ"),
        "nt_ppe_missing_hint" => ("berilishi kerak", "необходимо выдать"),
        "nt_service_due" => ("texnika TX muddati keldi", "единиц техники требуют ТО"),
        "nt_payment_debt" => ("Muddati o'tgan qarz:", "Просроченный долг:"),
        "nt_payment_soon" => ("30 kun ichida to'lash:", "К оплате за 30 дней:"),
        "nt_payment_soon_hint" => ("to'lov jadvali bo'yicha", "по графику платежей"),
        "nt_changes_pending" => ("shartnoma o'zgarishi kelishuvda", "изменений договора на согласовании"),
        "nt_changes_pending_hint" => ("buyurtmachi qaroriga", "решения заказчика"),
        "nt_accept_pending" => ("qabul hujjati qaror kutmoqda", "актов приёмки ждут решения"),
        "nt_accept_pending_hint" => ("topshirilgan ishlar", "предъявленные работы"),
        "nt_accept_rejected" => ("qabul hujjati rad etilgan", "актов приёмки отклонено"),
        "nt_accept_rejected_hint" => ("sabab hujjatda yozilgan", "причина указана в акте"),

        // ---------- VIII. Shartnomalar va to'lovlar ----------
        "screen_contracts" => ("Shartnomalar", "Договоры"),

        "ck_general" => ("Bosh pudrat", "Генподряд"),
        "ck_sub" => ("Subpudrat", "Субподряд"),
        "ck_supply" => ("Yetkazib berish", "Поставка"),
        "ck_design" => ("Loyihalash", "Проектирование"),
        "ck_service" => ("Xizmat", "Услуги"),

        "cs_draft" => ("Loyiha", "Проект"),
        "cs_active" => ("Amalda", "Действует"),
        "cs_suspended" => ("To'xtatilgan", "Приостановлен"),
        "cs_closed" => ("Yopilgan", "Закрыт"),

        "cch_extra" => ("Qo'shimcha ish", "Доп. работы"),
        "cch_reduce" => ("Chiqarish", "Исключение"),
        "cch_price" => ("Narx", "Цена"),
        "cch_term" => ("Muddat", "Срок"),

        "chs_draft" => ("Qoralama", "Черновик"),
        "chs_sent" => ("Kelishuvda", "На согласовании"),
        "chs_approved" => ("Kelishildi", "Согласовано"),
        "chs_rejected" => ("Rad etildi", "Отклонено"),

        "as_submitted" => ("Topshirildi", "Предъявлено"),
        "as_accepted" => ("Qabul qilindi", "Принято"),
        "as_rejected" => ("Rad etildi", "Не принято"),

        "imod_client" => ("Buyurtmachi", "Заказчик"),
        "imod_tech" => ("Texnik nazorat", "Технадзор"),

        // IV. Ijro hujjatlari — imzolashdan oldingi tekshiruv
        "ed_tab_docs" => ("Hujjatlar", "Документы"),
        "ed_tab_review" => ("Imzo tekshiruvi", "Проверка перед подписью"),
        "ed_review_hint" => (
            "Har bir e'tiroz boshqa moduldagi yozuvga tayanadi: GPR bajarilishi, texnik nazorat natijasi, laboratoriya sinovi.",
            "Каждое замечание опирается на запись в другом модуле: выполнение ГПР, результат технадзора, лабораторное испытание."
        ),
        "ed_review_blocked" => ("Imzolash to'silgan", "Подпись заблокирована"),
        "ed_review_blocked_hint" => ("jiddiy kamchilik bor", "есть серьёзное замечание"),
        "ed_review_warn" => ("Ogohlantirish", "Предупреждения"),
        "ed_review_warn_hint" => ("imzolash mumkin, lekin e'tibor bering", "подписать можно, но обратите внимание"),
        "ed_review_hidden" => ("Yashirin ish to'sig'i", "Блокировка по скрытым работам"),
        "ed_review_hidden_hint" => ("dalolatnoma imzolanmagan", "акт не подписан"),
        "ed_review_docs" => ("Hujjatlar bo'yicha e'tirozlar", "Замечания по документам"),
        "ed_review_clean" => ("Imzoga qo'yilgan hujjatlarda kamchilik yo'q.", "В документах на подпись замечаний нет."),
        "ed_hidden_clean" => ("Yashirin ishlar bo'yicha to'siq yo'q.", "Блокировок по скрытым работам нет."),
        "ed_hidden_violated" => ("Buzilgan", "Нарушено"),
        "ed_hidden_waiting" => ("Kutilyapti", "Ожидает"),
        "ed_hidden_missing" => ("dalolatnoma yo'q", "акта нет"),
        "ed_hidden_unsigned" => ("imzolanmagan", "не подписан"),
        "ed_new_version" => ("Yangi versiya", "Новая версия"),
        "dp_unfinished" => ("Ish hali tugallanmagan", "Работа ещё не завершена"),
        "dp_no_task" => ("Hujjat GPR ishiga bog'lanmagan", "Документ не привязан к работе ГПР"),
        "dp_no_inspection" => ("Texnik nazorat tekshiruvi yo'q", "Нет проверки техназдора"),
        "dp_inspection_failed" => ("Tekshiruv natijasi salbiy", "Отрицательный результат проверки"),
        "dp_no_lab" => ("Laboratoriya sinovi yo'q", "Нет лабораторного испытания"),
        "dp_lab_failed" => ("Laboratoriya sinovi salbiy", "Лабораторное испытание не пройдено"),
        "dp_concrete_weak" => ("Beton namunasi talabga yetmadi", "Проба бетона не достигла требования"),
        "dp_dated_before" => ("Hujjat sanasi ish tugashidan oldin, kun", "Дата документа раньше окончания работ, дн."),
        "dp_duplicate" => ("Shu ish uchun imzolangan hujjat bor", "По этой работе уже есть подписанный документ"),
        "dp_incomplete" => ("Raqam yoki mas'ul ko'rsatilmagan", "Не указан номер или ответственный"),
        // XII. Materiallar — loyihaga moslik va komplekt
        "mat_tab_fit" => ("Loyihaga moslik", "Соответствие проекту"),
        "mat_tab_kits" => ("Komplekt", "Комплект"),
        "mat_fit_hint" => (
            "Kartochkadagi yozuvlar tekshiriladi: spetsifikatsiya havolasi, smeta rasenkasi, texnik tavsif va sertifikat.",
            "Проверяются записи карточки: ссылка на спецификацию, расценка сметы, техописание и сертификат."
        ),
        "mat_fit_critical" => ("Jiddiy", "Серьёзных"),
        "mat_fit_critical_hint" => ("ishlatilgan materialda kamchilik", "замечание по применённому материалу"),
        "mat_fit_used" => ("Ishlatilgan", "Применённых"),
        "mat_fit_used_hint" => ("e'tirozi bor va obyektda ishlatilgan", "с замечанием и применён на объекте"),
        "mat_fit_total" => ("Jami e'tiroz", "Всего замечаний"),
        "mat_fit_total_hint" => ("kartochkalar bo'yicha", "по карточкам"),
        "mat_fit_ok" => ("Barcha kartochkalar to'liq to'ldirilgan.", "Все карточки заполнены полностью."),
        "mat_fit_in_use" => ("obyektda ishlatilgan", "применён на объекте"),
        "fp_no_spec_ref" => ("Loyiha spetsifikatsiyasiga havola yo'q", "Нет ссылки на спецификацию проекта"),
        "fp_no_estimate_code" => ("Smeta rasenkasi ko'rsatilmagan", "Не указана расценка сметы"),
        "fp_no_spec" => ("Texnik tavsif bo'sh: marka va GOST yo'q", "Пустое техописание: нет марки и ГОСТ"),
        "fp_special" => ("Maxsus talab tavsifda aks etmagan", "Особое требование не отражено в описании"),
        "fp_no_cert" => ("Sertifikat raqami yo'q", "Нет номера сертификата"),
        "fp_cert_expired" => ("Sertifikat muddati o'tgan, kun", "Срок сертификата истёк, дн."),
        "fp_cert_expiring" => ("Sertifikat muddati tugayapti, kun", "Срок сертификата истекает, дн."),
        "fp_ban_no_reason" => ("Taqiq sababi yozilmagan", "Не указана причина запрета"),
        "fp_no_section" => ("Loyiha bo'limi ko'rsatilmagan", "Не указан раздел проекта"),
        "mat_kit_hint" => (
            "Bitta material yetishmasa ham ish boshlanmaydi — tayyorlik komplekt darajasida o'lchanadi.",
            "Работа не начнётся, если не хватает даже одного материала — готовность считается по комплекту."
        ),
        "mat_kit_incomplete" => ("To'liq emas", "Неполных"),
        "mat_kit_incomplete_hint" => ("komplekt yig'ilmagan", "комплект не собран"),
        "mat_kit_total" => ("Komplektlar", "Комплектов"),
        "mat_kit_total_hint" => ("yaqin ishlar bo'yicha", "по ближайшим работам"),
        "mat_kit_makers" => ("Yetkazib beruvchilar", "Поставщики"),
        "mat_kit_makers_hint" => ("solishtiriladigan materiallar", "материалов для сравнения"),
        "mat_kit_ready" => ("Tayyorlik", "Готовность"),
        "mat_kit_count" => ("Turlari", "Позиций"),
        "mat_kit_missing" => ("Yetishmaydi", "Не хватает"),
        "mat_kit_worst" => ("Eng katta kamomad", "Наибольшая нехватка"),
        "mat_kit_empty" => ("Yaqin ishlar uchun sarf normasi kiritilmagan.", "Для ближайших работ нормы расхода не заданы."),
        "mat_maker_hint" => (
            "Narx yolg'iz o'zi yetarli emas: muddat va oldingi xaridlar soni ham ustunda turadi.",
            "Одной цены мало: в колонках также срок и число прежних закупок."
        ),
        "mat_maker_empty" => ("Bir material bo'yicha ikkitadan ortiq taklif yo'q.", "Нет материалов с двумя и более предложениями."),
        "mat_maker_spread" => ("tarqoqlik", "разброс"),
        "mat_maker_best" => ("eng arzon", "дешевле всех"),
        "mat_maker_deals" => ("xarid", "закупок"),
        // VI. Prorab ilovasi — kunni yakunlash va texnika buzilishi
        "fm_day_close" => ("Kunni yakunlash", "Закрытие дня"),
        "fm_day_ready" => (
            "Kun yozuvlari to'liq — hisobotni yuborish mumkin.",
            "Записи за день полные — отчёт можно отправлять."
        ),
        "fm_day_blocked" => (
            "Kunni yopishdan oldin quyidagilar to'ldirilishi kerak:",
            "Перед закрытием дня нужно заполнить следующее:"
        ),
        "fm_day_almost" => (
            "Kun deyarli yopiq, lekin bularga e'tibor bering:",
            "День почти закрыт, но обратите внимание:"
        ),
        "di_no_journal" => ("Kunlik jurnalga yozuv kiritilmagan", "В общий журнал работ запись не внесена"),
        "di_no_weather" => ("Ob-havo yozilmagan", "Не записана погода"),
        "di_no_photo" => ("Foto biriktirilmagan", "Не приложено фото"),
        "di_no_timesheet" => ("Tabel to'ldirilmagan", "Табель не заполнен"),
        "di_crew_mismatch" => ("Jurnal va tabeldagi ishchi soni farq qiladi", "Число рабочих в журнале и табеле расходится"),
        "di_no_volume" => ("Bugungi hajm kiritilmagan", "Не введён объём за сегодня"),
        "di_open_issues" => ("Yopilmagan xavfsizlik holatlari", "Незакрытые случаи по охране труда"),
        "cw_quality" => ("Sifat: nuqson / qabul nazorati", "Качество: дефекты / точки контроля"),
        "cw_docs" => ("Imzolanmagan ijro hujjati", "Неподписанных исполнительных документов"),
        "cw_no_consumption" => ("Ishga material chiqim qilinmagan", "На работу материал не списан"),
        "cw_no_labour" => ("Ishga soat yozilmagan", "На работу не записаны часы"),
        "cw_no_volume" => ("Jurnalda hajm yozuvi yo'q", "В журнале нет записи объёма"),
        "fm_machine_broke" => ("Buzildi", "Сломалась"),
        "fm_machine_broke_hint" => (
            "Ta'mir yozuvi ochiladi va texnika holati «ta'mirda» ga o'tadi.",
            "Откроется запись о ремонте, статус техники станет «в ремонте»."
        ),
        "fm_machine_broke_reason" => ("Obyektda buzildi", "Сломалась на объекте"),
        "fm_machine_fixed" => ("Ta'mirdan chiqdi", "Из ремонта"),
        "fm_machine_fixed_hint" => (
            "Ochiq ta'mir yopiladi va texnika ishga qaytadi.",
            "Открытый ремонт закрывается, техника возвращается в работу."
        ),
        // V. Kunlik jurnal — kun tahlili va ertangi reja
        "jr_tab_entries" => ("Yozuvlar", "Записи"),
        "jr_tab_day" => ("Kun tahlili", "Разбор дня"),
        "jr_tab_tomorrow" => ("Ertangi reja", "План на завтра"),
        "jr_day_hint" => (
            "Kun ichida solishtirish muhim: oy oxirida jami bo'yicha qaraganda ortiqcha sarf o'rtachada yo'qoladi.",
            "Сравнивать важно внутри дня: в итогах за месяц перерасход растворяется в среднем."
        ),
        "jr_day_material" => ("Bugungi hajm va material", "Объём и материал за сегодня"),
        "jr_day_no_material" => (
            "Bugungi yozuvlar bo'yicha sarf normasi topilmadi.",
            "По сегодняшним записям нормы расхода не найдены."
        ),
        "jr_day_volume" => ("Hajm", "Объём"),
        "jr_day_norm" => ("Norma bo'yicha", "По норме"),
        "jr_day_issued" => ("Berilgan", "Выдано"),
        "jr_day_diff" => ("Farq", "Разница"),
        "jr_doubts" => ("Yozuvlardagi ziddiyat", "Противоречия в записях"),
        "jr_doubts_hint" => (
            "Bu ayblov emas — savol: dastur faqat ichki ziddiyatni ko'rsatadi, oxirgi so'z odamniki.",
            "Это не обвинение, а вопрос: программа показывает лишь внутреннее противоречие, последнее слово за человеком."
        ),
        "jr_doubts_none" => ("Yozuvlarda ziddiyat topilmadi.", "Противоречий в записях не найдено."),
        "jd_over_plan" => ("Hajm ishda qolganidan katta", "Объём больше остатка по работе"),
        "jd_no_timesheet" => ("Jurnalda ishchi bor, tabelda yo'q", "В журнале рабочие есть, в табеле нет"),
        "jd_repeated" => ("Bir xil hajm ketma-ket kunlarda", "Одинаковый объём подряд"),
        "jd_future" => ("Yozuv kelajak sanaga kiritilgan", "Запись внесена будущей датой"),
        "jd_rate" => ("Bir ishchiga hajm o'rtachadan keskin katta", "Объём на рабочего резко выше среднего"),
        "jr_tomorrow_hint" => (
            "Ertaga nima ketadi, material yetadimi va kim bor — bugun ko'rinib tursin.",
            "Что идёт завтра, хватает ли материала и кто есть — видно уже сегодня."
        ),
        "jr_tm_tasks" => ("Ertangi ishlar", "Работ завтра"),
        "jr_tm_tasks_hint" => ("grafik bo'yicha", "по графику"),
        "jr_tm_starts" => ("Yangi boshlanadi", "Начинается"),
        "jr_tm_starts_hint" => ("bugun ketmayotgan ishlar", "работ, не идущих сегодня"),
        "jr_tm_blocked" => ("To'siqli", "С препятствием"),
        "jr_tm_blocked_hint" => ("material yoki odam yetishmaydi", "не хватает материала или людей"),
        "jr_tm_empty" => ("Ertaga grafik bo'yicha ish yo'q.", "Завтра работ по графику нет."),
        "jr_tm_state" => ("Holat", "Состояние"),
        "jr_tm_left" => ("Qolgan hajm", "Остаток объёма"),
        "jr_tm_crew" => ("Bugungi brigada", "Бригада сегодня"),
        "jr_tm_new" => ("boshlanadi", "начинается"),
        "jr_tm_going" => ("davom etadi", "продолжается"),
        // VII. Chek-list va yakuniy qabul
        "in_tab_final" => ("Yakuniy qabul", "Итоговая приёмка"),
        "in_final_hint" => (
            "Tayyorlik yetti shart bo'yicha o'lchanadi va har biri o'z modulidagi yozuvdan olinadi.",
            "Готовность считается по семи условиям, каждое берётся из записей своего модуля."
        ),
        "in_final_ready" => ("Tayyorlik", "Готовность"),
        "in_final_ready_hint" => ("yopilgan shartlar ulushi", "доля закрытых условий"),
        "in_final_blocks" => ("To'siqlar", "Препятствий"),
        "in_final_of" => ("shartdan", "условий"),
        "in_final_all_clear" => (
            "Barcha shartlar yopilgan — obyektni topshirish mumkin.",
            "Все условия закрыты — объект можно предъявлять к приёмке."
        ),
        "fb_tasks" => ("Tugallanmagan ishlar", "Незавершённые работы"),
        "fb_docs" => ("Imzolanmagan ijro hujjatlari", "Неподписанные исполнительные документы"),
        "fb_defects" => ("Bartaraf etilmagan nuqsonlar", "Неустранённые дефекты"),
        "fb_lab" => ("Salbiy laboratoriya sinovlari", "Отрицательные лабораторные испытания"),
        "fb_inspections" => ("Yopilmagan tekshiruvlar", "Незакрытые проверки"),
        "fb_safety" => ("Yopilmagan xavfsizlik holatlari", "Незакрытые случаи по охране труда"),
        "fb_acceptance" => ("Qabul qilinmagan topshiruvlar", "Непринятые предъявления"),
        "in_checklist" => ("Tekshiruvga chiqishdan oldin", "Перед выходом на проверку"),
        "in_checklist_short" => ("chek-list", "чек-лист"),
        "cl_marks" => ("Markalar chizmaga mos", "Марки соответствуют чертежу"),
        "cl_drawing" => ("Oxirgi versiyadagi chizma qo'lda", "На руках чертёж последней версии"),
        "cl_docs" => ("Oldingi bosqich hujjatlari imzolangan", "Документы предыдущего этапа подписаны"),
        "cl_hidden_ready" => ("Ish to'liq bajarilgan va ochiq", "Работа выполнена полностью и открыта"),
        "cl_hidden_photo" => ("Yopishdan oldingi foto olingan", "Сделано фото до закрытия"),
        "cl_hidden_clean" => ("Ish joyi tozalangan", "Рабочее место убрано"),
        "cl_concrete_sample" => ("Namuna olingan va belgilangan", "Проба отобрана и промаркирована"),
        "cl_concrete_temp" => ("Havo va aralashma harorati o'lchangan", "Замерена температура воздуха и смеси"),
        "cl_concrete_care" => ("Parvarish rejimi belgilangan", "Назначен режим ухода"),
        "cl_geo_base" => ("Asos nuqtalari tekshirilgan", "Опорные точки проверены"),
        "cl_geo_tolerance" => ("Ruxsat etilgan chetlanish ma'lum", "Известен допуск отклонения"),
        "cl_mat_cert" => ("Sertifikat va muddat tekshirilgan", "Проверены сертификат и срок"),
        "cl_mat_batch" => ("Partiya raqami hujjatga mos", "Номер партии совпадает с документом"),
        "cl_mat_storage" => ("Saqlash sharti buzilmagan", "Условия хранения не нарушены"),
        "cl_vol_measure" => ("Hajm joyida o'lchangan", "Объём замерен на месте"),
        "cl_vol_journal" => ("Jurnaldagi yozuv bilan solishtirilgan", "Сверено с записью в журнале"),
        "cl_final_defects" => ("Barcha nuqsonlar bartaraf etilgan", "Все дефекты устранены"),
        "cl_final_docs" => ("Ijro hujjatlari to'liq", "Исполнительная документация полная"),
        "cl_final_tests" => ("Sinov natijalari ijobiy", "Результаты испытаний положительные"),
        "cl_phys_visual" => ("Ko'z bilan ko'rinadigan nuqsonlar yo'q", "Нет видимых дефектов"),
        "cl_sec_rebar" => ("Armatura diametri va qadami tekshirilgan", "Проверены диаметр и шаг арматуры"),
        "cl_sec_weld" => ("Payvand choklari ko'zdan kechirilgan", "Сварные швы осмотрены"),
        "cl_sec_pressure" => ("Bosim sinovi o'tkazilgan", "Проведено испытание давлением"),
        "cl_sec_insulation" => ("Izolyatsiya qarshiligi o'lchangan", "Замерено сопротивление изоляции"),
        "cl_sec_fire" => ("Yong'in tizimi ishga tushirib ko'rilgan", "Система пожаротушения опробована"),
        // Umumiy: izoh va biriktirma
        "nt_task" => ("Ish", "Работа"),
        "nt_issue" => ("Nomuvofiqlik", "Несоответствие"),
        "nt_request" => ("Ariza", "Заявка"),
        "nt_purchase" => ("Xarid", "Закупка"),
        "nt_exec_doc" => ("Ijro hujjati", "Исполнительный документ"),
        "nt_quality" => ("Sifat tekshiruvi", "Проверка качества"),
        "nt_safety" => ("Xavfsizlik holati", "Случай по охране труда"),
        "nt_inspection" => ("Texnik nazorat", "Технадзор"),
        "nt_machine" => ("Texnika", "Техника"),
        "nt_material" => ("Material", "Материал"),
        "nt_worker" => ("Xodim", "Сотрудник"),
        "nt_other" => ("Boshqa", "Прочее"),
        "ps_plain" => ("Oddiy", "Обычное"),
        "ps_before" => ("Oldin", "До"),
        "ps_after" => ("Keyin", "После"),
        "nt_panel" => ("izoh va fayllar", "комментарии и файлы"),
        "nt_close" => ("Yopish", "Закрыть"),
        "nt_add" => ("izoh", "коммент."),
        "nt_badge_hint" => (
            "Izoh va biriktirilgan fayllar: son — izohlar va fayllar soni.",
            "Комментарии и вложения: числа — количество комментариев и файлов.",
        ),
        "nt_notes" => ("Izohlar", "Комментарии"),
        "nt_no_notes" => ("Izoh yo'q.", "Комментариев нет."),
        "nt_unknown_author" => ("Muallif ko'rsatilmagan", "Автор не указан"),
        "nt_resolved" => ("hal qilindi", "решено"),
        "nt_resolve" => ("Hal qilindi", "Решено"),
        "nt_reopen" => ("Qayta ochish", "Открыть снова"),
        "nt_reply" => ("Javob", "Ответить"),
        "nt_replying_to" => ("Javob:", "Ответ:"),
        "nt_placeholder" => ("Izoh yozing…", "Напишите комментарий…"),
        "nt_send" => ("Yuborish", "Отправить"),
        "nt_files" => ("Fayllar", "Файлы"),
        "nt_no_files" => ("Fayl biriktirilmagan.", "Файлы не приложены."),
        "nt_add_file" => ("Fayl qo'shish", "Добавить файл"),
        "nt_caption" => ("izoh", "подпись"),
        "nt_open" => ("Ochish", "Открыть"),
        "nt_file_missing" => (
            "Fayl joyida topilmadi — ko'chirilgan yoki o'chirilgan.",
            "Файл не найден на месте — перемещён или удалён.",
        ),
        "nt_detach_hint" => (
            "Ro'yxatdan olib tashlanadi; faylning o'zi o'chirilmaydi.",
            "Убирается из списка; сам файл не удаляется.",
        ),
        // X.33, 37-38, 42. Obyektlar bo'yicha xaridlar
        "pf_central" => ("Obyektlar bo'yicha xaridlar", "Закупки по объектам"),
        "pf_central_hint" => (
            "Guruhlash nom bo'yicha: har obyektning o'z katalogi bor va kodlar mos kelmasligi mumkin.",
            "Группировка по названию: у каждого объекта свой каталог и коды могут не совпадать.",
        ),
        "pf_central_saving" => ("Markazlashtirishdan tejash", "Экономия от централизации"),
        "pf_central_saving_hint" => (
            "Bu yuqori chegara, kafolat emas: hajm va yetkazish sharti har xil bo'lishi mumkin.",
            "Это верхняя граница, а не гарантия: объём и условия поставки могут отличаться.",
        ),
        "pf_central_objects" => ("Obyektlar", "Объектов"),
        "pf_central_best" => ("Eng arzon narx", "Лучшая цена"),
        "pf_central_save" => ("Tejash", "Экономия"),
        "pf_central_now" => ("Hozir qanday olinyapti", "Как закупается сейчас"),
        "pf_central_split" => ("Markazlashtirilganda bo'linishi", "Разбивка при централизации"),
        // X.18-19, 22. Xaridlar tartibi nazorati
        "col_tech_ok" => ("Texnik kelishuv", "Техсогласование"),
        "col_tech_ok_hint" => (
            "Xarid loyihaga mos ekanini muhandis tasdiqlaydi. Belgilansa — kim tasdiqlagani yoziladi.",
            "Инженер подтверждает соответствие закупки проекту. При отметке записывается, кто согласовал.",
        ),
        "pu_control_title" => ("Tartib nazorati", "Контроль порядка"),
        "pu_control_hint" => (
            "Uch savol: buyurtma texnik kelishuvdan o'tganmi, almashtirish tasdiqlanganmi, shartnoma sharti buzilmayaptimi.",
            "Три вопроса: прошёл ли заказ техсогласование, утверждена ли замена, не нарушены ли условия договора.",
        ),
        "pu_control_none" => (
            "Xaridlar tartibi bo'yicha e'tiroz yo'q.",
            "Замечаний по порядку закупок нет.",
        ),
        "si_no_tech" => ("Texnik kelishuvsiz buyurtma", "Заказ без техсогласования"),
        "si_unapproved" => (
            "Almashtirish tasdiqlangan analoglar ro'yxatida yo'q",
            "Замена отсутствует в списке утверждённых аналогов",
        ),
        "si_sub_no_tech" => (
            "Analog tasdiqlangan, lekin texnik kelishuv olinmagan",
            "Аналог утверждён, но техсогласование не получено",
        ),
        "si_overrun" => ("Shartnoma summasidan oshib ketildi", "Превышена сумма договора"),
        "si_expired" => (
            "Shartnoma muddati tugagach buyurtma berilgan",
            "Заказ размещён после окончания срока договора",
        ),
        "si_no_contract" => ("Yirik xarid shartnomasiz", "Крупная закупка без договора"),
        // XVII.23-25, 35, 49. Kesimlar bo'yicha tahlil
        "an_tab_cuts" => ("Kesimlar", "Разрезы"),
        "an_week" => ("Haftalik boshqaruv hisoboti", "Недельный управленческий отчёт"),
        "an_w_progress" => ("Bajarilish", "Выполнение"),
        "an_w_done" => ("tugatilgan ish", "завершено работ"),
        "an_w_started" => ("boshlangan ish", "начато работ"),
        "an_w_inspections" => ("Tekshiruv", "Проверок"),
        "an_w_failed" => ("salbiy", "отрицательных"),
        "an_w_issues" => ("yangi nomuvofiqlik", "новых несоответствий"),
        "an_w_docs" => ("imzolangan hujjat", "подписано документов"),
        "an_w_journal" => ("Jurnal yozilgan kun", "Дней с записью в журнале"),
        "an_w_paid" => ("To'langan", "Оплачено"),
        "an_w_waiting" => ("Qaror kutmoqda: o'zgarish / qabul", "Ожидают решения: изменения / приёмки"),
        "day_short" => ("k", "д"),
        "an_exec_hint" => (
            "Umumiy ball yangi hisob qilmaydi: sifat va xavfsizlik ballari o'z modullaridan, qolganlari shu ekranlardagi sonlardan olinadi.",
            "Общий балл ничего не считает заново: качество и охрана труда берутся из своих модулей, остальное — из чисел этих же экранов.",
        ),
        "an_exec_total" => ("Umumiy ball", "Общий балл"),
        "an_exec_total_hint" => ("0-100, vaznlangan", "0-100, взвешенный"),
        "an_exec_safety" => ("Xavfsizlik", "Охрана труда"),
        "an_exec_quality" => ("Sifat", "Качество"),
        "an_exec_schedule" => ("Muddat", "Сроки"),
        "an_exec_money" => ("Pul", "Деньги"),
        "an_exec_supply" => ("Ta'minot", "Снабжение"),
        "an_exec_docs" => ("Hujjatlar", "Документы"),
        "an_exec_weight_25" => ("vazni 25", "вес 25"),
        "an_exec_weight_20" => ("vazni 20", "вес 20"),
        "an_exec_weight_15" => ("vazni 15", "вес 15"),
        "an_exec_weight_10" => ("vazni 10", "вес 10"),
        "an_exec_weight_5" => ("vazni 5", "вес 5"),
        "an_contractors" => ("Pudratchilar va mas'ullar", "Подрядчики и ответственные"),
        "an_no_data" => ("Ma'lumot yetarli emas.", "Недостаточно данных."),
        "an_c_tasks" => ("Ishlar", "Работ"),
        "an_c_done" => ("Tugatilgan", "Завершено"),
        "an_c_overdue" => ("Muddati o'tgan", "Просрочено"),
        "an_c_on_time" => ("Muddatida", "В срок"),
        "an_c_delay" => ("O'rtacha kechikish", "Средняя задержка"),
        "an_c_quality" => ("Sifat balli", "Балл качества"),
        "an_c_issues" => ("Nuqson / XT", "Дефекты / ОТ"),
        "an_suppliers" => ("Yetkazib beruvchilar", "Поставщики"),
        "an_suppliers_hint" => (
            "Narx ballga kirmaydi: arzon, lekin kechikadigan ta'minotchi yaxshi ko'rinib qolardi. Narx alohida ustunda.",
            "Цена не входит в балл: дешёвый, но опаздывающий поставщик выглядел бы хорошим. Цена — в отдельной колонке.",
        ),
        "an_s_deals" => ("Xaridlar", "Закупок"),
        "an_s_complete" => ("To'liq", "Полностью"),
        "an_s_on_time" => ("Muddatida", "В срок"),
        "an_s_rejected" => ("Rad etilgan", "Отклонено"),
        "an_s_price" => ("Narx farqi", "Разница цены"),
        "an_s_score" => ("Ishonchlilik", "Надёжность"),
        "an_corr" => ("Ko'rsatkichlar bog'liqligi", "Связь показателей"),
        "an_corr_hint" => (
            "Bu sabab emas, birgalikda o'zgarish: ikki son birga o'zgargani birinchisi ikkinchisini keltirib chiqargan degani emas.",
            "Это не причина, а совместное изменение: то, что два числа меняются вместе, не значит, что первое вызывает второе.",
        ),
        "an_corr_points" => ("juftlik", "пар"),
        "corr_crew_volume" => (
            "Brigada kattaligi va kunlik hajm",
            "Численность бригады и дневной объём",
        ),
        "corr_volume_cost" => ("Ish hajmi va tannarx", "Объём работ и себестоимость"),
        "corr_hours_volume" => ("Kunlik soat va kunlik hajm", "Часы за день и объём за день"),
        "an_changes" => ("Loyiha o'zgarishlari", "Изменения проекта"),
        "an_ch_total" => ("Jami", "Всего"),
        "an_ch_approved" => ("Tasdiqlangan", "Утверждено"),
        "an_ch_pending" => ("Qaror kutmoqda", "Ожидает решения"),
        "an_ch_days" => ("Qo'shilgan kun", "Добавлено дней"),
        "an_ch_decision" => ("O'rtacha qaror muddati", "Средний срок решения"),
        // XI.8, 13, 29, 31, 35, 37. Ombor nazorati
        "wh_tab_control" => ("Nazorat", "Контроль"),
        "wh_control_hint" => (
            "To'rt savol: kirim hujjatlanganmi, sarf smetaga bog'langanmi, harorat talabi buzilmayaptimi, yoqilg'i hisobi to'g'ri kelayaptimi.",
            "Четыре вопроса: оформлен ли приход, привязан ли расход к смете, не нарушено ли требование по температуре, сходится ли учёт топлива.",
        ),
        "wh_control_severe" => ("Jiddiy", "Серьёзных"),
        "wh_control_severe_hint" => ("hisobni buzadigan e'tiroz", "замечание, ломающее учёт"),
        "wh_control_total" => ("Jami e'tiroz", "Всего замечаний"),
        "wh_control_total_hint" => ("ombor yozuvlari bo'yicha", "по записям склада"),
        "wh_control_title" => ("Ombor yozuvlari", "Записи склада"),
        "wh_control_none" => ("Ombor yozuvlarida e'tiroz yo'q.", "Замечаний по записям склада нет."),
        "wh_si_no_doc" => ("Kirim hujjatsiz", "Приход без документа"),
        "wh_si_no_batch" => (
            "Sertifikatli material partiyasiz kirim qilingan",
            "Материал с сертификатом принят без партии",
        ),
        "wh_si_over" => ("Kirim buyurtmadan oshgan", "Приход больше заказа"),
        "wh_si_no_estimate" => (
            "Sarf smeta rasenkasiga bog'lanmagan",
            "Расход не привязан к расценке сметы",
        ),
        "wh_si_temp" => (
            "Harorat talabi bor material ochiq omborda",
            "Материал с требованием по температуре на открытом складе",
        ),
        "wh_si_fuel" => (
            "Yoqilg'i: omborda berilgan / texnikaga yozilgan",
            "Топливо: выдано со склада / списано на технику",
        ),
        "wh_ppe" => ("Ish kiyimi va SIZ", "Спецодежда и СИЗ"),
        "wh_ppe_hint" => (
            "Berilishi xavfsizlik modulida yuritiladi; bu yerda omborning ko'rinishi: kimga nima kerak.",
            "Выдача ведётся в модуле охраны труда; здесь взгляд склада: кому что нужно.",
        ),
        "wh_ppe_issued" => ("Berilgan SIZ yozuvlari", "Записей о выдаче СИЗ"),
        "wh_ppe_missing" => ("SIZ yetishmaydi", "Не хватает СИЗ"),
        "wh_ppe_missing_hint" => ("ishchi bo'yicha", "по рабочим"),
        "wh_ppe_need" => ("kerak", "нужно"),
        "wh_ppe_expired" => ("muddati o'tgan", "просрочено"),
        "wh_ppe_ok" => (
            "Barcha ishchilarga majburiy SIZ berilgan.",
            "Всем рабочим выданы обязательные СИЗ.",
        ),
        "wh_turnover" => ("Aylanma qaydnoma", "Оборотная ведомость"),
        "wh_turnover_hint" => (
            "Buxgalteriya uchun: boshlang'ich qoldiq, davr kirimi va chiqimi, oxirgi qoldiq — miqdorda va summada.",
            "Для бухгалтерии: начальный остаток, приход и расход за период, конечный остаток — в количестве и сумме.",
        ),
        "doc_turnover" => ("Materiallar bo'yicha aylanma qaydnoma", "Оборотная ведомость по материалам"),
        "doc_turnover_short" => ("Aylanma", "Оборотка"),
        "doc_tv_open_qty" => ("Boshda, miqdor", "На начало, кол-во"),
        "doc_tv_open_sum" => ("Boshda, summa", "На начало, сумма"),
        "doc_tv_in_qty" => ("Kirim, miqdor", "Приход, кол-во"),
        "doc_tv_in_sum" => ("Kirim, summa", "Приход, сумма"),
        "doc_tv_out_qty" => ("Chiqim, miqdor", "Расход, кол-во"),
        "doc_tv_out_sum" => ("Chiqim, summa", "Расход, сумма"),
        "doc_tv_close_qty" => ("Oxirida, miqdor", "На конец, кол-во"),
        "doc_tv_close_sum" => ("Oxirida, summa", "На конец, сумма"),
        // XVI.28, 31, 39, 46. Mexanik kabineti
        "mch_tab_mech" => ("Mexanik", "Механик"),
        "mch_mech_hint" => (
            "Ko'rik smena boshida o'tkaziladi va yozib qoldiriladi: og'zaki «hammasi joyida» hodisadan keyin hech narsani isbotlamaydi.",
            "Осмотр проводится в начале смены и фиксируется: устное «всё в порядке» после происшествия ничего не докажет.",
        ),
        "mch_mech_stop" => ("To'xtatish kerak", "Требует остановки"),
        "mch_mech_stop_hint" => ("texnika ishlashi mumkin emas", "техника не должна работать"),
        "mch_mech_issues" => ("Jami e'tiroz", "Всего замечаний"),
        "mch_mech_issues_hint" => ("park bo'yicha", "по парку"),
        "mch_mech_checked" => ("Bugun ko'rikdan o'tgan", "Осмотрено сегодня"),
        "mch_mech_checked_hint" => ("kunlik ko'rik yozuvi", "записей дневного осмотра"),
        "mch_mech_title" => ("E'tirozlar", "Замечания"),
        "mch_mech_none" => ("Park bo'yicha e'tiroz yo'q.", "Замечаний по парку нет."),
        "mi_no_check" => ("Bugun ishlagan, ko'rikdan o'tmagan", "Работала сегодня, осмотр не проведён"),
        "mi_fault" => ("Nosozlik topilgan, texnika ishlashda", "Найдена неисправность, техника в работе"),
        "mi_not_allowed" => ("Ruxsat berilmagan, lekin ishlatilgan", "Допуск не выдан, но техника использовалась"),
        "mi_inspection" => ("Texnik ko'rik muddati o'tgan", "Просрочен техосмотр"),
        "mi_service" => ("Rejali TX muddati o'tgan", "Просрочено плановое ТО"),
        "mi_no_operator" => ("Operator ko'rsatilmagan", "Оператор не указан"),
        "mi_no_permit" => (
            "Operatorning ko'targich ishlariga ruxsati yo'q",
            "У оператора нет допуска к работам с подъёмными механизмами",
        ),
        "mch_check_today" => ("Bugungi ko'rik", "Осмотр за сегодня"),
        "mch_check_state" => ("Holat", "Состояние"),
        "mch_check_by" => ("Kim ko'rdi", "Кто осмотрел"),
        "mch_check_fault" => ("Nosozlik", "Неисправность"),
        "mch_check_missing" => ("o'tkazilmagan", "не проведён"),
        "mch_check_blocked" => ("ruxsat berilmagan", "допуск не выдан"),
        "mch_check_ok" => ("to'liq, joyida", "полный, в порядке"),
        "mch_check_partial" => ("qisman", "частично"),
        "mch_check_add" => ("Ko'rik yozish", "Записать осмотр"),
        "mch_park_review" => ("Park bo'yicha ko'rib chiqish", "Обзор парка"),
        "mch_park_hint" => (
            "Tavsiya qaror emas: ijaraga olishmi yoki sotib olishmi — bu pul va muddat bo'yicha qaror, dastur faqat koeffitsiyentni ko'rsatadi.",
            "Рекомендация — не решение: брать в аренду или покупать решают деньги и сроки, программа лишь показывает коэффициент.",
        ),
        "mch_park_own" => ("Egaligi", "Владение"),
        "mch_park_owned" => ("o'z", "своя"),
        "mch_park_rented" => ("ijara", "аренда"),
        "mch_park_usage" => ("Foydalanish", "Использование"),
        "mch_park_hours" => ("Kunlik soat", "Часов в день"),
        "mch_park_idle" => ("Bo'sh kun", "Простой, дн."),
        "mch_park_cost" => ("Davr xarajati", "Расход за период"),
        "pa_own_idle" => (
            "O'z texnikasi kam ishlatilyapti — ijaraga berish yoki sotish mumkin",
            "Своя техника мало используется — можно сдать в аренду или продать",
        ),
        "pa_rented_idle" => (
            "Ijara texnikasi kam ishlatilyapti — qaytarishni ko'rib chiqish",
            "Арендованная техника мало используется — рассмотреть возврат",
        ),
        "pa_rented_busy" => (
            "Ijara texnikasi doim ishda — o'zini olish arzonroq bo'lishi mumkin",
            "Арендованная техника постоянно в работе — своя может выйти дешевле",
        ),
        // III.9, 11, 13, 16. Smetaning chuqur tekshiruvi
        "tab_deep" => ("Chuqur tekshiruv", "Глубокая проверка"),
        "es_deep_hint" => (
            "To'rt savol: kompleks rasenka ichidagi ish alohida hisoblanmadimi, ketma-ketlik buzilmadimi, marka ko'rsatilganmi, narx taklifdan uzoqlashmadimi.",
            "Четыре вопроса: не посчитана ли отдельно работа внутри комплексной расценки, не нарушена ли последовательность, указана ли марка, не ушла ли цена от предложения.",
        ),
        "es_deep_money" => ("Pulga tegishli", "Касается денег"),
        "es_deep_money_hint" => ("ikki marta to'lash yoki narx farqi", "двойная оплата или разница цены"),
        "es_deep_total" => ("Jami e'tiroz", "Всего замечаний"),
        "es_deep_total_hint" => ("smeta pozitsiyalari bo'yicha", "по позициям сметы"),
        "es_deep_none" => ("Chuqur tekshiruvda e'tiroz topilmadi.", "Глубокая проверка замечаний не нашла."),
        "ed_double" => ("Pozitsiya ichida takror hisob:", "Двойной счёт внутри позиции:"),
        "ed_same_code" => ("Bir xil rasenka turli narxda:", "Одна расценка по разным ценам:"),
        "ed_predecessor" => ("Oldingi ish smetada yo'q — pozitsiya", "Предшествующая работа не в смете — позиция"),
        "ed_no_mark" => ("Marka yoki standart ko'rsatilmagan — pozitsiya", "Не указана марка или стандарт — позиция"),
        "ed_quote_gap" => ("Narx tijorat taklifidan farq qiladi — pozitsiya", "Цена расходится с коммерческим предложением — позиция"),
        // IX.13, 27, 39. Ariza tekshiruvi va reyestr
        "rq_i_no_spec" => (
            "Loyiha spetsifikatsiyasiga havola yo'q — material loyihada ko'zda tutilganini tekshirib bo'lmaydi",
            "Нет ссылки на спецификацию проекта — нельзя проверить, предусмотрен ли материал",
        ),
        "rq_i_over_norm" => ("So'ralgan miqdor normadan ko'p", "Запрошено больше нормы"),
        "rq_i_no_profession" => (
            "Kasb ko'rsatilmagan: «odam kerak» degan ariza bo'yicha hech kimni topib bo'lmaydi",
            "Не указана профессия: по заявке «нужны люди» никого не найти",
        ),
        "rq_i_staff_enough" => (
            "Xodim ehtiyoji hisobda ko'rinmaydi (bor / kerak)",
            "Потребность в людях по расчёту не видна (есть / нужно)",
        ),
        "rq_register" => ("To'lov reyestri", "Реестр платежей"),
        "rq_register_hint" => (
            "Buxgalteriya uchun: tasdiqlangan pul arizalari va to'lovga qo'yilgan xaridlar.",
            "Для бухгалтерии: утверждённые денежные заявки и закупки, поставленные к оплате.",
        ),
        "doc_pay_register" => ("To'lovlar reyestri", "Реестр платежей"),
        "doc_pay_short" => ("Reyestr", "Реестр"),
        // VII.30, 32, VIII.24. Loyiha hujjati versiyalari
        "col_sheets" => ("Varaq", "Листов"),
        "doc_revision" => ("O'zgartirish", "Изменение"),
        "doc_revision_hint" => (
            "Chizmadagi o'zgartirish belgisi: «Izm. 2», «Rev. B». Uni loyihachi qo'yadi.",
            "Отметка об изменении на чертеже: «Изм. 2», «Rev. B». Её ставит проектировщик.",
        ),
        "doc_issued" => ("Topshirilgan", "Передан в работу"),
        "doc_issue" => ("Topshirish", "Передать"),
        "doc_issue_hint" => (
            "Chizmani qurilishga topshirish sanasi. Bo'sh bo'lsa obyektda hali eski versiya ishlatilyapti.",
            "Дата передачи чертежа на стройку. Пока пусто — на объекте работают по прежней версии.",
        ),
        "doc_new_version" => ("Yangi versiya", "Новая версия"),
        "doc_new_version_hint" => (
            "Eskisi arxivda qoladi va yangisiga bog'lanadi; yangi versiya avtomatik topshirilmaydi.",
            "Прежняя останется в архиве и свяжется с новой; новая версия не передаётся автоматически.",
        ),
        "doc_superseded" => ("arxivda", "в архиве"),
        "doc_change_note" => ("Nima o'zgardi", "Что изменилось"),
        "doc_ver_title" => ("Versiya nazorati", "Контроль версий"),
        "doc_ver_hint" => (
            "Asosiy xavf bitta: obyektda eski chizma bo'yicha ishlash.",
            "Главный риск один: работа на объекте по устаревшему чертежу.",
        ),
        "dv_not_issued" => (
            "Yangi versiya qurilishga topshirilmagan",
            "Новая версия не передана на стройку",
        ),
        "dv_not_issued_short" => ("topshirilmagan", "не передан"),
        "dv_work_before" => (
            "Yangi versiyadan oldin tugatilgan ish bor",
            "Есть работы, завершённые до новой версии",
        ),
        "dv_tasks" => ("ish", "работ"),
        "dv_no_note" => ("O'zgartirish izohi yozilmagan", "Не записано, что изменилось"),
        "dv_no_revision" => ("O'zgartirish belgisi ko'rsatilmagan", "Не указана отметка об изменении"),
        "dv_two_active" => (
            "Bir nomda ikkita amaldagi hujjat — qaysi biri to'g'ri ekani noma'lum",
            "Два действующих документа с одним именем — неясно, какой верный",
        ),
        "cl_versions" => ("Loyiha versiyalari", "Версии проекта"),
        "cl_versions_hint" => (
            "Dastur chizmaning ichini o'qimaydi: solishtirish kartochkadagi ma'lumotga tayanadi — varaq soni, belgi va loyihachi izohi.",
            "Программа не читает содержимое чертежа: сравнение опирается на данные карточки — число листов, отметку и примечание проектировщика.",
        ),
        "cl_versions_before" => (
            "Yangi versiyadan oldin tugatilgan ishlar",
            "Работ, завершённых до новой версии",
        ),
        // XIV.37. Haftalik sifat hisoboti
        "ql_tab_week" => ("Haftalik hisobot", "Недельный отчёт"),
        "ql_w_score" => ("Sifat balli", "Балл качества"),
        "ql_w_score_hint" => ("sifat modulidagi bilan bir xil", "тот же, что в модуле качества"),
        "ql_w_checks" => ("Tekshiruv", "Проверок"),
        "ql_w_opened" => ("Ochilgan nuqson", "Открыто дефектов"),
        "ql_w_opened_hint" => ("hafta ichida", "за неделю"),
        "ql_w_closed" => ("Yopilgan", "Закрыто"),
        "ql_w_closed_hint" => ("hafta ichida bartaraf etilgan", "устранено за неделю"),
        "ql_w_open" => ("Ochiq qolgan", "Осталось открытых"),
        "ql_w_overdue" => ("muddati o'tgan", "просрочено"),
        "ql_w_no_defects" => ("Nuqson qayd etilmagan.", "Дефектов не зафиксировано."),
        "ql_w_top" => ("Eng ko'p takrorlangan nuqsonlar", "Чаще всего повторяющиеся дефекты"),
        "ql_w_top_hint" => (
            "Takrorlanish sabab borligini bildiradi: bir xil nuqson uch marta chiqsa, gap ijrochida emas, jarayonda.",
            "Повторение указывает на причину: если один дефект встречается трижды, дело не в исполнителе, а в процессе.",
        ),
        "ql_w_times" => ("marta", "раз"),
        // V.24. Direktorning kunlik hisoboti
        "jr_dr_running" => ("Hajm yozilgan / ketayotgan", "С объёмом / идёт"),
        "jr_dr_running_hint" => ("bugungi ishlar", "работы за сегодня"),
        "jr_dr_crew" => ("Brigada", "Бригада"),
        "jr_dr_machines" => ("Texnika", "Техника"),
        "jr_dr_machines_hint" => ("bugun ishlagan", "работала сегодня"),
        "jr_dr_material" => ("Material sarfi", "Расход материала"),
        "jr_dr_material_hint" => ("bugungi chiqim qiymati", "стоимость сегодняшнего расхода"),
        "jr_dr_events" => ("XT / sifat", "ОТ / качество"),
        "jr_dr_events_hint" => ("bugun ochilgan holatlar", "случаев открыто сегодня"),
        "jr_dr_blockers" => ("Kun to'siqlari", "Препятствия дня"),
        "jr_dr_blockers_hint" => ("prorab ekranidagi bilan bir xil", "те же, что на экране прораба"),
        "jr_dr_docs" => ("Imzolangan hujjat", "Подписано документов"),
        "jr_dr_docs_hint" => ("bugun", "сегодня"),
        // V.17. Jurnaldan ariza
        "jr_req_title" => ("Jurnaldan ariza", "Заявка из журнала"),
        "jr_req_hint" => (
            "Qolgan hajmga norma qo'llanadi va ombordagi erkin qoldiq ayriladi. Qoldiq yetsa taklif berilmaydi.",
            "К остатку объёма применяется норма, затем вычитается свободный остаток на складе. Если хватает — предложения нет.",
        ),
        "jr_req_stock" => ("omborda", "на складе"),
        "jr_req_make" => ("Ariza ochish", "Создать заявку"),
        "jr_req_note" => (
            "Kunlik jurnal yozuvidan avtomatik taklif qilingan",
            "Предложено автоматически по записи дневного журнала",
        ),
        "jr_req_done" => ("Ariza qoralama sifatida ochildi", "Заявка создана как черновик"),
        // VII.18. Chizmadagi izoh
        "nt_document" => ("Chizma", "Чертёж"),
        // XIII.3, 12, 28. Obyektlar, grafik, ko'chirish
        "ts_tab_objects" => ("Obyektlar", "Объекты"),
        "ts_obj_hint" => (
            "Ishchi obyektga biriktirilgan: bitta odam bir vaqtda ikki obyektda bo'la olmaydi. Davr — oxirgi 30 kun.",
            "Рабочий закреплён за объектом: один человек не может одновременно быть на двух. Период — последние 30 дней.",
        ),
        "ts_obj_workers" => ("Ishchi", "Рабочих"),
        "ts_obj_idle" => ("Bo'sh turish", "Простой"),
        "ts_obj_per_worker" => ("Bir ishchiga soat", "Часов на рабочего"),
        "ts_obj_payroll" => ("Ish haqi fondi", "Фонд оплаты"),
        "ts_obj_move" => ("Xodimni ko'chirish", "Перевод сотрудника"),
        "ts_obj_move_hint" => (
            "Tabel yozuvlari ko'chirilmaydi: ular o'sha obyektda ishlangan soatning yozuvi bo'lib qoladi. Brigada bog'lanishi tushadi.",
            "Записи табеля не переносятся: они остаются записью отработанных часов на прежнем объекте. Привязка к бригаде снимается.",
        ),
        "ts_obj_move_to" => ("Boshqa obyektga", "На другой объект"),
        "ts_obj_moved" => ("Xodim ko'chirildi", "Сотрудник переведён"),
        "ts_obj_schedule" => ("Ish grafigi", "График работы"),
        "ts_obj_schedule_hint" => (
            "Dam olish kunidagi ish taqiq emas — qurilishda bu bo'ladi, lekin unga haq boshqacha to'lanadi va ko'rinib turishi kerak.",
            "Работа в выходной не запрещена — на стройке это бывает, но оплачивается иначе и должна быть видна.",
        ),
        "ts_obj_work_days" => ("Grafik bo'yicha ish kunlari", "Рабочих дней по графику"),
        "ts_obj_schedule_ok" => ("Grafik bo'yicha e'tiroz yo'q.", "Замечаний по графику нет."),
        "ts_sch_rest" => ("Dam olish kunida ish yozilgan", "Работа записана в выходной"),
        "ts_sch_empty" => ("Ish kunida hech kim belgilanmagan", "В рабочий день никто не отмечен"),
        "ts_sch_over" => ("Smena grafikdagidan uzun", "Смена длиннее графика"),
        // II.12, 14. Spetsifikatsiya va qurilish imkoniyati
        "rule_prj_spec_qty" => ("Spetsifikatsiya: miqdor", "Спецификация: количество"),
        "rule_prj_spec_unit" => ("Spetsifikatsiya: o'lchov birligi", "Спецификация: единица измерения"),
        "rule_prj_build_size" => ("Qurilish imkoniyati: teshik o'lchami", "Возможность стройки: размер проёма"),
        "rule_prj_build_level" => ("Qurilish imkoniyati: qavatda konstruksiya", "Возможность стройки: конструкции на этаже"),
        "chk_spec_qty_title" => ("Spetsifikatsiyada miqdor yo'q", "В спецификации нет количества"),
        "chk_spec_qty_desc" => (
            "uchun miqdor ko'rsatilmagan — buyurtma ham, smeta ham tuzib bo'lmaydi",
            "не указано количество — нельзя ни заказать, ни посчитать в смете",
        ),
        "chk_spec_qty_fix" => (
            "Element kartochkasida miqdorni to'ldiring.",
            "Заполните количество в карточке элемента.",
        ),
        "chk_spec_unit_title" => ("O'lchov birligi ko'rsatilmagan", "Не указана единица измерения"),
        "chk_spec_unit_desc" => (
            "son bor, birlik yo'q — «250» metrmi, millimetrmi yoki donami?",
            "число есть, единицы нет — «250» это метры, миллиметры или штуки?",
        ),
        "chk_spec_unit_fix" => ("O'lchov birligini ko'rsating.", "Укажите единицу измерения."),
        "chk_build_size_title" => ("Teshik konstruksiyadan katta", "Проём больше конструкции"),
        "chk_build_size_desc" => (">=", ">="),
        "chk_build_size_fix" => (
            "Teshik o'lchamini kamaytiring yoki konstruksiya kesimini oshiring.",
            "Уменьшите проём или увеличьте сечение конструкции.",
        ),
        "chk_build_level_title" => ("Qavatda konstruksiya yo'q", "На этаже нет конструкций"),
        "chk_build_level_desc" => ("Qavat", "Этаж"),
        "chk_build_level_count" => ("qurilma yoki tarmoq mahkamlanadi", "устройств или сетей крепятся"),
        "chk_build_level_fix" => (
            "KJ yoki KM bo'limida shu qavat konstruksiyalarini ko'rsating.",
            "Укажите конструкции этого этажа в разделе КЖ или КМ.",
        ),
        // IV.7, 24. Ijro sxemalari va mualliflik nazorati
        "ed_tab_schemes" => ("Ijro sxemalari", "Исполнительные схемы"),
        "ed_tab_author" => ("Mualliflik nazorati", "Авторский надзор"),
        "ed_schemes_hint" => (
            "Sxema — «qanday qurildi» degan hujjat: uning ortida o'lchangan nuqtalar turishi kerak. O'lchovsiz sxemada tasdiqlanadigan narsa yo'q.",
            "Схема — документ о том, «как построено»: за ней должны стоять замеренные точки. В схеме без замеров нечего утверждать.",
        ),
        "ed_schemes_empty" => (
            "Ijro sxemasi turidagi hujjat yo'q.",
            "Документов типа «исполнительная схема» нет.",
        ),
        "ed_schemes_ready" => ("Imzoga tayyor", "Готовы к подписи"),
        "ed_schemes_ready_hint" => ("o'lchov bor va dopusk ichida", "есть замеры и в пределах допуска"),
        "ed_schemes_out" => ("Dopuskdan chiqqan", "Вне допуска"),
        "ed_schemes_out_hint" => ("nuqtalar soni", "число точек"),
        "ed_schemes_points" => ("Nuqta", "Точек"),
        "ed_schemes_bad" => ("Dopuskdan tashqari", "Вне допуска"),
        "ed_schemes_max" => ("Eng katta chetlanish", "Макс. отклонение"),
        "ed_schemes_ok" => ("tayyor", "готова"),
        "ed_schemes_no_points" => ("o'lchov yo'q", "нет замеров"),
        "ed_schemes_deviation" => ("chetlanish bor", "есть отклонение"),
        "ed_author_hint" => (
            "Kabinet yangi ma'lumot yaratmaydi: boshqa modullardagi yozuvlardan loyihachiga tegishlilarini yig'adi.",
            "Кабинет не создаёт новых данных: собирает из других модулей то, что адресовано проектировщику.",
        ),
        "ed_author_total" => ("Javob kutmoqda", "Ожидают ответа"),
        "ed_author_total_hint" => ("loyihachi uchun", "для проектировщика"),
        "ed_author_late" => ("Muddati o'tgan", "Просрочено"),
        "ed_author_late_hint" => ("qaror kechikkan", "решение задерживается"),
        "ed_author_none" => (
            "Loyihachi javobini kutayotgan ish yo'q.",
            "Нет вопросов, ожидающих ответа проектировщика.",
        ),
        "ea_change" => ("O'zgarish", "Изменение"),
        "ea_version" => ("Topshirilmagan versiya:", "Непереданная версия:"),
        "ea_inspection" => ("Salbiy tekshiruv", "Отрицательная проверка"),
        "ed_archive" => ("Obyekt arxivi", "Архив объекта"),
        "ed_archive_hint" => (
            "Hujjatlar reyestri: fayllar ko'chirilmaydi — dastur o'zi joylashtirmagan faylni ko'chirishi noto'g'ri bo'lardi. Reyestrni papka bilan birga topshiriladi.",
            "Реестр документов: файлы не копируются — программе не следует копировать файлы, которые она не размещала. Реестр сдаётся вместе с папкой.",
        ),
        "doc_archive" => ("Obyekt arxivi reyestri", "Реестр архива объекта"),
        "doc_arch_project" => ("Loyiha", "Проект"),
        "doc_arch_exec" => ("Ijro", "Исполнительная"),
        "doc_arch_inspections" => ("Nazorat", "Надзор"),
        "col_path" => ("Fayl yo'li", "Путь к файлу"),
        // Rahbar ekrani (TZ IX.34, X.45, XI.40, XII.40, XIII.41-42, XIV.39, XV.39, XVI.47)
        "assist_open" => ("Yordamchi", "Помощник"),
        // II.3-8. Bo'limlar bo'yicha muhandislik hisobi
        "rule_prj_ar_door" => ("AR: eshik kengligi", "АР: ширина двери"),
        "rule_prj_ar_light" => ("AR: tabiiy yoritish", "АР: естественное освещение"),
        "rule_prj_kj_section" => ("KJ: kesim o'lchami", "КЖ: размер сечения"),
        "rule_prj_kj_class" => ("KJ: beton sinfi", "КЖ: класс бетона"),
        "rule_prj_km_steel" => ("KM: po'lat markasi", "КМ: марка стали"),
        "rule_prj_vk_velocity" => ("VK: suv tezligi", "ВК: скорость воды"),
        "rule_prj_ov_velocity" => ("OV: havo tezligi", "ОВ: скорость воздуха"),
        "rule_prj_eom_section" => ("EOM: kabel kesimi", "ЭОМ: сечение кабеля"),
        "unit_mm" => ("mm", "мм"),
        "unit_mm2" => ("mm²", "мм²"),
        "unit_ms" => ("m/s", "м/с"),
        "unit_a" => ("A", "А"),
        "chk_ar_door_title" => ("Eshik kengligi yetarli emas", "Недостаточная ширина двери"),
        "chk_ar_door_actual" => ("Chizmada", "По чертежу"),
        "chk_ar_door_min" => ("Eng kami", "Минимум"),
        "chk_ar_door_fix" => (
            "Eshik kengligini oshiring yoki chizmada o'lchovni aniqlashtiring.",
            "Увеличьте ширину двери или уточните размер на чертеже.",
        ),
        "chk_ar_light_title" => ("Tabiiy yoritish yetarli emas", "Недостаточное естественное освещение"),
        "chk_ar_light_actual" => ("Deraza / xona", "Окно / помещение"),
        "chk_ar_light_min" => ("Talab", "Требование"),
        "chk_ar_light_fix" => (
            "Deraza yuzasini oshiring yoki xona vazifasini qayta ko'ring.",
            "Увеличьте площадь окна или пересмотрите назначение помещения.",
        ),
        "chk_kj_section_title" => ("Kesim o'lchami juda kichik", "Слишком малое сечение"),
        "chk_kj_section_actual" => ("Chizmada", "По чертежу"),
        "chk_kj_section_min" => ("Eng kami", "Минимум"),
        "chk_kj_section_fix" => (
            "Kesimni kattalashtiring yoki hisobni biriktiring.",
            "Увеличьте сечение или приложите расчёт.",
        ),
        "chk_kj_class_title" => ("Beton sinfi ko'rsatilmagan", "Не указан класс бетона"),
        "chk_kj_class_desc" => (
            "uchun beton sinfi yozilmagan — armatura hisobini tekshirib bo'lmaydi",
            "не указан класс бетона — расчёт армирования не проверить",
        ),
        "chk_kj_class_fix" => (
            "Markada yoki izohda beton sinfini ko'rsating (masalan B25).",
            "Укажите класс бетона в марке или примечании (например B25).",
        ),
        "chk_km_steel_title" => ("Po'lat markasi ko'rsatilmagan", "Не указана марка стали"),
        "chk_km_steel_desc" => (
            "uchun po'lat markasi yozilmagan — payvand choki va elektrod tanlanmaydi",
            "не указана марка стали — не выбрать сварной шов и электрод",
        ),
        "chk_km_steel_fix" => (
            "Markada yoki izohda po'lat markasini ko'rsating (masalan S245).",
            "Укажите марку стали в марке или примечании (например С245).",
        ),
        "chk_velocity_actual" => ("Hisob bo'yicha tezlik", "Скорость по расчёту"),
        "chk_velocity_max" => ("Ruxsat etilgan", "Допустимо"),
        "chk_vk_velocity_title" => ("Suv tezligi chegaradan yuqori", "Скорость воды выше предела"),
        "chk_vk_velocity_note" => (
            "Hisob: v = Q / A, diametr chizmadan, sarf element parametridan olingan.",
            "Расчёт: v = Q / A, диаметр — из чертежа, расход — из параметра элемента.",
        ),
        "chk_vk_velocity_fix" => (
            "Diametrni oshiring yoki sarfni qayta ko'ring.",
            "Увеличьте диаметр или пересмотрите расход.",
        ),
        "chk_ov_velocity_title" => ("Havo tezligi chegaradan yuqori", "Скорость воздуха выше предела"),
        "chk_ov_velocity_note" => (
            "Hisob: v = L / (3600 · A), kesim chizmadan, sarf element parametridan olingan.",
            "Расчёт: v = L / (3600 · A), сечение — из чертежа, расход — из параметра элемента.",
        ),
        "chk_ov_velocity_fix" => (
            "Vozduxovod kesimini oshiring yoki sarfni qayta ko'ring.",
            "Увеличьте сечение воздуховода или пересмотрите расход.",
        ),
        "chk_eom_title" => ("Kabel kesimi yetarli emas", "Недостаточное сечение кабеля"),
        "chk_eom_current" => ("Hisob toki", "Расчётный ток"),
        "chk_eom_needed" => ("Kerakli kesim", "Требуемое сечение"),
        "chk_eom_actual" => ("Chizmada", "По чертежу"),
        "chk_eom_note" => (
            "Taxminlar: 380 V uch fazali tarmoq, cosφ 0.85, mis o'tkazgich, tok zichligi reyestrdan.",
            "Допущения: сеть 380 В трёхфазная, cosφ 0.85, медный проводник, плотность тока — из реестра.",
        ),
        "chk_eom_fix" => (
            "Kabel kesimini oshiring yoki hisobni biriktiring.",
            "Увеличьте сечение кабеля или приложите расчёт.",
        ),
        // XVII.3, 7, 10. Nega va keyin nima bo'ladi
        "an_tab_why" => ("Nega va keyin nima", "Почему и что дальше"),
        "an_why_hint" => (
            "Ikki savol bir ekranda: bugungi sabab ertangi risk bo'lib qaytadi. Sabab bazadagi yozuvdan, prognoz bugungi sur'atdan chiqadi.",
            "Два вопроса на одном экране: сегодняшняя причина возвращается завтрашним риском. Причина — из записей базы, прогноз — из текущего темпа.",
        ),
        "an_why_delays" => ("Kechikkan ish", "Отстающих работ"),
        "an_why_delays_hint" => ("muddati o'tgan", "просрочены"),
        "an_why_known" => ("Sababi ma'lum", "Причина известна"),
        "an_why_known_hint" => ("yozuvdan topilgan", "найдена по записям"),
        "an_why_cost" => ("Kechikish narxi", "Стоимость отставания"),
        "an_why_cost_hint" => ("brigadaning kunlik haqi bo'yicha", "по дневной оплате бригады"),
        "an_why_risks" => ("Yuqori risk", "Высоких рисков"),
        "an_why_risks_hint" => ("e'tibor talab qiladi", "требуют внимания"),
        "an_why_title" => ("Nega kechikdi", "Почему отстаём"),
        "an_why_none" => ("Kechikkan ish yo'q.", "Отстающих работ нет."),
        "an_why_days" => ("Kun", "Дней"),
        "an_why_cause" => ("Sabab", "Причина"),
        "an_why_price" => ("Narxi", "Стоимость"),
        "an_why_unknown_cost" => ("noma'lum", "неизвестно"),
        "dc_material" => ("Material yetishmaydi", "Не хватает материала"),
        "dc_no_crew" => ("Ishga hech kim yozilmagan", "На работу никто не записан"),
        "dc_quality" => ("Bartaraf etilmagan nuqson", "Неустранённые дефекты"),
        "dc_docs" => ("Imzolanmagan hujjat", "Неподписанные документы"),
        "dc_machine" => ("Texnika ta'mirda", "Техника в ремонте"),
        "dc_predecessor" => ("Oldingi ish kechikkan", "Предшествующая работа отстаёт"),
        "dc_unknown" => (
            "Yozuvlardan sabab topilmadi — buni odam aytishi kerak",
            "Причина по записям не найдена — её должен указать человек",
        ),
        "an_risk_title" => ("Keyin nima bo'ladi", "Что будет дальше"),
        "an_risk_hint" => (
            "Bu bashorat emas: hozirgi holat davom etsa nima bo'lishining hisobi. Sur'at o'zgarsa natija ham o'zgaradi.",
            "Это не предсказание, а расчёт того, что будет, если всё останется как есть. Изменится темп — изменится и результат.",
        ),
        "an_risk_none" => ("Yaqin xavf ko'rinmayapti.", "Ближайших угроз не видно."),
        "rk_schedule" => ("Muddat siljiydi", "Сроки сдвигаются"),
        "rk_stock" => ("Material tugaydi", "Материал закончится"),
        "rk_pending_money" => ("Kelishilmagan o'zgarishlar summasi", "Сумма несогласованных изменений"),
        "rk_quality" => ("Ochiq / muddati o'tgan nuqson", "Открытых / просроченных дефектов"),
        "rk_safety" => ("Yopilmagan xavfsizlik holati", "Незакрытых случаев по охране труда"),
        // X.23, 25-26, 47, XI.47. To'lov va ta'minot zanjiri
        "col_pay_due" => ("To'lov muddati", "Срок оплаты"),
        "col_pay_set" => ("Muddat", "Срок"),
        "col_pay_overdue" => (
            "To'lov muddati o'tgan va qarz qolgan",
            "Срок оплаты истёк, остался долг",
        ),
        "pu_tab_chain" => ("Zanjir", "Цепочка"),
        "pu_chain_hint" => (
            "Ariza → taklif → xarid → yetkazish → kirish nazorati → ombor → ish → to'lov. Har bosqich alohida modulda, bu yerda bir qatorda.",
            "Заявка → предложение → закупка → поставка → входной контроль → склад → работа → оплата. Каждый шаг в своём модуле, здесь — в одной строке.",
        ),
        "pu_chain_lines" => ("Zanjir qatorlari", "Строк цепочки"),
        "pu_chain_lines_hint" => ("qoralamadan tashqari xaridlar", "закупки, кроме черновиков"),
        "pu_chain_gaps" => ("Uzilishi bor", "С разрывами"),
        "pu_chain_gaps_hint" => ("bosqichlar orasida farq", "расхождение между шагами"),
        "pu_chain_paid" => ("To'langan / summa", "Оплачено / сумма"),
        "pu_chain_paid_hint" => ("zanjir bo'yicha jami", "итого по цепочке"),
        "pu_chain_overdue" => ("Muddati o'tgan to'lov", "Просроченная оплата"),
        "pu_chain_overdue_hint" => ("qarz qolgan summa", "остаток долга"),
        "pu_chain_empty" => ("Rasmiylashtirilgan xarid yo'q.", "Оформленных закупок нет."),
        "pu_ch_request" => ("Ariza", "Заявка"),
        "pu_ch_quotes" => ("Taklif", "КП"),
        "pu_ch_order" => ("Buyurtma", "Заказ"),
        "pu_ch_checks" => ("Nazorat", "Контроль"),
        "pu_ch_stock" => ("Omborga", "На склад"),
        "pu_ch_issued" => ("Ishga", "В работу"),
        "pu_ch_gaps" => ("Uzilish", "Разрывы"),
        "pu_ch_ok" => ("zanjir butun", "цепочка целая"),
        "cg_order_over" => ("buyurtma arizadan ko'p", "заказ больше заявки"),
        "cg_delivery_over" => ("yetkazilgan buyurtmadan ko'p", "поставлено больше заказа"),
        "cg_not_stocked" => ("omborga kirim qilinmagan", "не оприходовано"),
        "cg_issued_over" => ("omborda yo'q material berilgan", "выдано больше, чем на складе"),
        "cg_no_check" => ("kirish nazoratidan o'tmagan", "не прошло входной контроль"),
        "cg_no_quotes" => ("taklif olinmagan", "предложения не запрашивались"),
        "cg_overpaid" => ("ortiqcha to'langan", "переплата"),
        "cg_pay_overdue" => ("to'lov muddati o'tgan", "срок оплаты истёк"),
        // XVI.8, 21, 26, 42, 48. Texnika zanjiri
        "mch_tab_chain" => ("Zanjir", "Цепочка"),
        "mch_chain_hint" => (
            "Ariza → biriktirish → smena → yoqilg'i → ta'mir → tannarx. Ballga narx kirmaydi: qimmat, lekin doim ishlaydigan texnika yomon ko'rinib qolmasin.",
            "Заявка → закрепление → смена → топливо → ремонт → себестоимость. Цена не входит в балл: дорогая, но постоянно работающая техника не должна выглядеть плохой.",
        ),
        "mch_ch_requests" => ("Ariza", "Заявок"),
        "mch_ch_fuel" => ("Yoqilg'i", "Топливо"),
        "mch_ch_repairs" => ("Ta'mir", "Ремонтов"),
        "mch_ch_cost" => ("Jami xarajat", "Всего затрат"),
        "mch_ch_per_hour" => ("Soatiga", "За час"),
        "mch_ch_score" => ("Ball", "Балл"),
        "mch_forecast" => ("Ta'mir prognozi", "Прогноз ремонта"),
        "mch_forecast_hint" => (
            "Tarixga tayanadi: ta'mirlar orasidagi o'rtacha motosoat. Ikkitadan kam ta'miri bor texnikaga prognoz berilmaydi — bitta hodisadan qonuniyat chiqmaydi.",
            "Опирается на историю: средняя наработка между ремонтами. Технике с менее чем двумя ремонтами прогноз не даётся — по одному случаю закономерности нет.",
        ),
        "mch_forecast_none" => (
            "Prognoz uchun ta'mir tarixi yetarli emas.",
            "Истории ремонтов для прогноза недостаточно.",
        ),
        "mch_fc_mtbf" => ("o'rtacha oraliq", "средний интервал"),
        "mch_fc_left" => ("qoldi", "осталось"),
        "mch_fc_based" => ("Ta'mirlar soni", "Число ремонтов"),
        "mch_fc_since" => ("Oxirgi ta'mirdan beri", "С последнего ремонта"),
        // XIV.15, 19, 27. Bo'limlar, ustuvorlik, ketma-ketlik
        "ql_tab_sections" => ("Bo'limlar", "Разделы"),
        "ql_sec_hint" => (
            "Bo'lim balli umumiy ball bilan bir xil qoidada hisoblanadi — faqat shu bo'lim yozuvlari bo'yicha.",
            "Балл раздела считается по тем же правилам, что и общий — только по записям этого раздела.",
        ),
        "ql_sec_empty" => ("Bo'limga bog'langan tekshiruv yo'q.", "Нет проверок, привязанных к разделу."),
        "ql_sec_failed" => ("Salbiy", "Отрицательных"),
        "ql_priority" => ("Nuqsonlar ustuvorligi", "Приоритет дефектов"),
        "ql_priority_hint" => (
            "Har ball sababi ko'rsatiladi: nima uchun aynan shu nuqson birinchi ekani ko'rinib turishi kerak.",
            "Каждая причина балла показана: должно быть видно, почему именно этот дефект первый.",
        ),
        "ql_priority_none" => ("Ochiq nuqson yo'q.", "Открытых дефектов нет."),
        "dp_open" => ("ochiq", "открыт"),
        "dp_overdue" => ("muddati o'tgan", "просрочен"),
        "dp_failed" => ("salbiy natija", "отрицательный результат"),
        "dp_closing" => ("ish yopilmoqda", "работа закрывается"),
        "dp_hidden" => ("keyin ochib bo'lmaydi", "потом не вскрыть"),
        "ql_sequence" => ("Texnologik ketma-ketlik", "Технологическая последовательность"),
        "ql_sequence_hint" => (
            "Bu taqiq emas: qurilishda ishlar qisman ustma-ust ketadi. Lekin oldingi ish tekshiruvdan o'tmagan bo'lsa, keyingisi uni ko'mib yuboradi.",
            "Это не запрет: на стройке работы частично идут внахлёст. Но если предыдущая работа не проверена, следующая её закроет.",
        ),
        "ql_sequence_none" => ("Ketma-ketlik buzilmagan.", "Последовательность не нарушена."),
        "ql_seq_checked" => ("tekshirilgan", "проверена"),
        "ql_seq_unchecked" => ("tekshirilmagan", "не проверена"),
        // XVII.27-28, 42. Qarorlar markazi va benchmarking
        "dr_decisions" => ("Qarorlar markazi", "Центр решений"),
        "dr_decisions_hint" => (
            "Markaz yangi hisob qilmaydi: har qator boshqa modulda ko'rinadi. Bu yerda ular bir ro'yxatda va kutish vaqti bo'yicha tartiblangan.",
            "Центр ничего не считает заново: каждая строка видна и в своём модуле. Здесь они собраны в один список и упорядочены по времени ожидания.",
        ),
        "dr_decisions_none" => ("Qaror kutayotgan ish yo'q.", "Решений не ожидается."),
        "de_request" => ("Ariza tasdiqlanmagan", "Заявка не утверждена"),
        "de_change" => ("O'zgarish qaror kutmoqda", "Изменение ждёт решения"),
        "de_acceptance" => ("Topshiriq qabul qilinmagan", "Предъявление не принято"),
        "de_tech" => ("Texnik kelishuvsiz buyurtma", "Заказ без техсогласования"),
        "de_deadline" => ("Muddatsiz nuqson: kim va qachongacha", "Дефект без срока: кто и до когда"),
        "de_doc" => ("Imzoga qo'yilgan to'siqli hujjat", "Документ на подписи с препятствием"),
        "de_machine" => ("To'xtatish kerak bo'lgan texnika", "Техника, требующая остановки"),
        "dr_bench" => ("Obyektlar solishtiruvi", "Сравнение объектов"),
        "dr_bench_hint" => (
            "Solishtirish birlik hajmga keltiriladi: katta obyektning umumiy xarajati kichigidan har doim katta va bu hech narsani aytmaydi.",
            "Сравнение приводится к единице объёма: общие затраты крупного объекта всегда больше, и это ни о чём не говорит.",
        ),
        "dr_bench_gap" => ("Rejadan farq", "Отклонение от плана"),
        "dr_bench_cost" => ("Birlik hajmga xarajat", "Затраты на единицу"),
        "dr_bench_hours" => ("Birlik hajmga soat", "Часов на единицу"),
        "screen_director" => ("Rahbar", "Руководителю"),
        "dr_hint" => (
            "Ekran yangi hisob qilmaydi: har son o'z modulidagi funksiyadan olinadi, shuning uchun modul ekranidagi bilan farq qilmaydi.",
            "Экран ничего не считает заново: каждое число берётся из функции своего модуля, поэтому не расходится с экраном модуля.",
        ),
        "dr_progress" => ("Fakt / reja", "Факт / план"),
        "dr_overdue" => ("Muddati o'tgan ish", "Просроченных работ"),
        "dr_delay" => ("Kechikish", "Отставание"),
        "dr_open" => ("Ochiq ariza", "Открытых заявок"),
        "dr_late_supply" => ("Kechikkan ta'minot", "Опаздывает снабжение"),
        "dr_amount" => ("Xarid summasi", "Сумма закупок"),
        "dr_order_issues" => ("Tartib e'tirozi", "Замечаний по порядку"),
        "dr_severe" => ("Jiddiy", "Серьёзных"),
        "dr_stock_value" => ("Ombor qiymati", "Стоимость склада"),
        "dr_below_min" => ("Zaxiradan past", "Ниже запаса"),
        "dr_stock_issues" => ("Ombor e'tirozi", "Замечаний по складу"),
        "dr_material_critical" => ("Jiddiy moslik e'tirozi", "Серьёзных замечаний по соответствию"),
        "dr_kits" => ("To'liq emas / komplekt", "Неполных / комплектов"),
        "dr_workers" => ("Ishchi", "Рабочих"),
        "dr_staff_gap" => ("Yetishmovchilik", "Нехватка"),
        "dr_anomalies" => ("Tabel anomaliyasi", "Аномалий табеля"),
        "dr_score" => ("Ball", "Балл"),
        "dr_defects" => ("Ochiq nuqson", "Открытых дефектов"),
        "dr_defects_overdue" => ("Muddati o'tgan nuqson", "Просроченных дефектов"),
        "dr_open_cases" => ("Yopilmagan holat", "Незакрытых случаев"),
        "dr_stops" => ("To'xtatish kerak", "Требуют остановки"),
        "dr_idle_machines" => ("Kam ishlatilgan texnika", "Мало используется техники"),
        "dr_park_advice" => ("Park tavsiyasi", "Рекомендаций по парку"),
        "dr_critical" => ("Kritik nomuvofiqlik", "Критических несоответствий"),
        "dr_issues" => ("Jami nomuvofiqlik", "Всего несоответствий"),
        "dr_estimate" => ("Smeta summasi", "Сумма сметы"),
        "dr_deep" => ("Chuqur tekshiruv e'tirozi", "Замечаний глубокой проверки"),
        "dr_final_ready" => ("Qabulga tayyorlik", "Готовность к приёмке"),
        "dr_final_blocks" => ("To'siq", "Препятствий"),
        "ct_tab_contracts" => ("Shartnomalar", "Договоры"),
        "ct_tab_changes" => ("O'zgarishlar", "Изменения"),
        "ct_tab_stages" => ("To'lov jadvali", "График платежей"),
        "ct_tab_accept" => ("Ishlarni qabul qilish", "Приёмка работ"),

        "ct_kpi_sum" => ("Amaldagi summa", "Текущая сумма"),
        "ct_from_base" => ("dastlabkidan", "от первоначальной"),
        "ct_no_contracts" => ("shartnoma yo'q", "договоров нет"),
        "ct_kpi_pending" => ("Kelishuvda", "На согласовании"),
        "ct_kpi_pending_hint" => ("qaror kutayotgan o'zgarish", "изменений ждут решения"),
        "ct_kpi_paid" => ("To'langan", "Оплачено"),
        "ct_of_schedule" => ("jadvaldan", "от графика"),
        "ct_kpi_debt" => ("Muddati o'tgan qarz", "Просроченный долг"),
        "ct_days_late" => ("kun kechikdi", "дн. просрочки"),
        "ct_no_debt" => ("qarz yo'q", "долгов нет"),
        "ct_kpi_soon" => ("30 kun ichida", "В ближайшие 30 дн."),
        "ct_kpi_soon_hint" => ("to'lash kerak", "к оплате"),
        "ct_kpi_accept" => ("Qabul kutmoqda", "Ждут приёмки"),
        "ct_kpi_accept_hint" => ("topshirilgan hujjat", "предъявленных актов"),

        "ct_hint" => (
            "Amaldagi summa hisoblanadi: shartnoma plyus faqat kelishilgan o'zgarishlar.",
            "Текущая сумма считается: договор плюс только согласованные изменения.",
        ),
        "ct_add" => ("+ Shartnoma", "+ Договор"),
        "ct_empty" => ("Shartnoma yo'q", "Договоров нет"),
        "ct_signed" => ("Imzolangan", "Подписан"),
        "ct_end" => ("Tugash", "Окончание"),
        "ct_base" => ("Shartnoma summasi", "Сумма договора"),
        "ct_current" => ("Amaldagi summa", "Текущая сумма"),
        "ct_change" => ("O'zgarish", "Изменение"),
        "ct_paid" => ("To'langan", "Оплачено"),
        "ct_approved" => ("Kelishilgan", "Согласовано"),
        "ct_pending" => ("Kelishuvda", "На согласовании"),
        "ct_days_shift" => ("Muddat surilishi, kun", "Сдвиг срока, дн."),
        "ct_scheduled" => ("Jadval bo'yicha", "По графику"),
        "ct_debt" => ("Muddati o'tgan", "Просрочено"),
        "ct_gap" => ("Jadvalda ko'rsatilmagan", "Не разнесено по графику"),
        "ct_advance" => ("Avans", "Аванс"),
        "ct_retention" => ("Kafolat ushlanmasi", "Гарантийное удержание"),

        "ct_add_change" => ("+ O'zgarish", "+ Изменение"),
        "ct_changes_hint" => (
            "Faqat «Kelishildi» holatidagi o'zgarish shartnoma summasiga qo'shiladi.",
            "В сумму договора попадает только изменение в статусе «Согласовано».",
        ),
        "ct_changes_empty" => ("O'zgarish yo'q", "Изменений нет"),
        "ct_contract" => ("Shartnoma", "Договор"),
        "ct_amount" => ("Summa", "Сумма"),
        "ct_days" => ("Kun", "Дней"),
        "ct_decided" => ("Kim qaror qildi", "Кто решил"),

        "ct_add_stage" => ("+ Bosqich", "+ Этап"),
        "ct_stages_hint" => (
            "Kechikish to'langan sanadan o'lchanadi; to'lanmagan bosqich esa qarz.",
            "Просрочка считается от даты оплаты; неоплаченный этап — это долг.",
        ),
        "ct_stages_empty" => ("To'lov jadvali yo'q", "График платежей пуст"),
        "ct_basis" => ("Asos", "Основание"),
        "ct_due" => ("Muddat", "Срок"),
        "ct_left" => ("Qoldiq", "Остаток"),
        "ct_paid_at" => ("To'langan sana", "Дата оплаты"),
        "ct_state" => ("Holat", "Состояние"),
        "ct_s_closed" => ("to'langan", "оплачен"),
        "ct_s_waiting" => ("kutilmoqda", "ожидается"),

        "ct_add_accept" => ("+ Qabul hujjati", "+ Акт приёмки"),
        "ct_accept_hint" => (
            "Bu ilova ichidagi qaror: kim va qachon qabul qilgani yoziladi, elektron imzo emas.",
            "Это решение внутри программы: пишется кто и когда принял, электронной подписи нет.",
        ),
        "ct_accept_empty" => ("Qabul hujjati yo'q", "Актов приёмки нет"),
        "ct_accept" => ("Qabul qilish", "Принять"),
        "ct_reject" => ("Rad etish", "Отклонить"),
        "ct_reopen" => ("qaytarish", "вернуть"),
        "ct_comment" => ("Izoh", "Комментарий"),

        // ---------- VII. Texnik nazorat: tekshiruvlar ----------
        "screen_inspections" => ("Tekshiruvlar", "Проверки"),

        "ik_hidden" => ("Yashirin ish", "Скрытые работы"),
        "ik_physical" => ("Jismoniy ko'rik", "Визуальный осмотр"),
        "ik_concrete" => ("Beton", "Бетон"),
        "ik_geodesy" => ("Geodeziya", "Геодезия"),
        "ik_material" => ("Kirish nazorati", "Входной контроль"),
        "ik_volume" => ("Hajm", "Объём"),
        "ik_final" => ("Yakuniy qabul", "Приёмка"),

        "ir_waiting" => ("Kutilmoqda", "Ожидается"),
        "ir_pass" => ("Qabul qilindi", "Принято"),
        "ir_conditional" => ("Shartli", "Условно"),
        "ir_fail" => ("Rad etildi", "Не принято"),

        "in_tab_list" => ("Ro'yxat", "Список"),
        "in_tab_calendar" => ("Kalendar", "Календарь"),
        "in_tab_concrete" => ("Beton sinovi", "Испытания бетона"),
        "in_tab_geodesy" => ("Geodeziya", "Геодезия"),
        "in_tab_day" => ("Kunlik hisobot", "Дневной отчёт"),

        "in_kpi_today" => ("Bugun", "Сегодня"),
        "in_kpi_today_hint" => ("rejalashtirilgan tekshiruv", "запланировано проверок"),
        "in_kpi_week" => ("Bir haftada", "За неделю"),
        "in_kpi_week_hint" => ("kelgusi yetti kun", "ближайшие семь дней"),
        "in_kpi_overdue" => ("Muddati o'tgan", "Просрочено"),
        "in_kpi_overdue_hint" => ("chaqirilgan, o'tkazilmagan", "вызвано, не проведено"),
        "in_kpi_defects" => ("Bartaraf etilmagan", "Не устранено"),
        "in_kpi_defects_hint" => ("salbiy natijadan keyin", "после отрицательного результата"),
        "in_kpi_concrete" => ("Beton, o'rtacha", "Бетон, среднее"),
        "in_kpi_geodesy" => ("Dopuskdan chiqqan", "Вне допуска"),
        "in_kpi_geodesy_hint" => ("geodezik nuqta", "геодезических точек"),
        "in_tested" => ("sinov", "испытаний"),
        "in_failed" => ("yetmagan", "не прошло"),

        "in_hint" => (
            "Tekshiruv o'tkazilgani belgilangandan keyingina natija tanlanadi.",
            "Результат выбирается только после отметки о проведении проверки.",
        ),
        "in_empty" => ("Tekshiruv yozuvi yo'q", "Записей о проверках нет"),
        "in_planned" => ("Reja", "План"),
        "in_done" => ("O'tkazildi", "Проведено"),
        "in_requested_by" => ("Kim chaqirdi", "Кто вызвал"),
        "in_inspector" => ("Tekshirdi", "Проверил"),
        "in_fix" => ("Bartaraf etish", "Устранение"),
        "in_overdue" => ("muddati o'tdi", "просрочено"),
        "in_set_deadline" => ("muddat", "срок"),
        "in_close" => ("yopish", "закрыть"),

        "in_cal_hint" => (
            "Kalendarda faqat o'tkazilmagan tekshiruvlar — bu bajarilishi kerak bo'lgan ish ro'yxati.",
            "В календаре только непроведённые проверки — это список предстоящей работы.",
        ),
        "in_cal_empty" => ("Rejalashtirilgan tekshiruv yo'q", "Запланированных проверок нет"),
        "in_cal_today" => ("bugun", "сегодня"),
        "in_cal_days_ago" => ("kun oldin", "дн. назад"),
        "in_cal_days_left" => ("kundan keyin", "дн."),

        "in_add_sample" => ("+ Namuna", "+ Образец"),
        "in_concrete_hint" => (
            "Mustahkamlik hisoblanmaydi — laboratoriya natijasi kiritiladi va talab bilan solishtiriladi.",
            "Прочность не рассчитывается — вводится результат лаборатории и сравнивается с требованием.",
        ),
        "in_concrete_empty" => ("Beton namunasi yo'q", "Образцов бетона нет"),
        "in_sample" => ("Namuna", "Образец"),
        "in_grade" => ("Marka", "Марка"),
        "in_structure" => ("Konstruksiya", "Конструкция"),
        "in_poured" => ("Quyilgan", "Уложен"),
        "in_age" => ("Yosh", "Возраст"),
        "in_test_date" => ("Sinov sanasi", "Дата испытания"),
        "in_required" => ("Talab, MPa", "Требуется, МПа"),
        "in_actual" => ("Natija, MPa", "Результат, МПа"),
        "in_of_required" => ("Talabdan", "От требования"),
        "in_lab" => ("Laboratoriya", "Лаборатория"),
        "in_waiting" => ("natija yo'q", "нет результата"),

        "in_add_point" => ("+ Nuqta", "+ Точка"),
        "in_geodesy_hint" => (
            "Chetlanish o'lchovdan hisoblanadi: fakt minus loyiha. Dopuskdan chiqqani qizil.",
            "Отклонение считается от замера: факт минус проект. Вне допуска — красным.",
        ),
        "in_geodesy_empty" => ("Geodezik o'lchov yo'q", "Геодезических замеров нет"),
        "in_mark" => ("Marka", "Марка"),
        "in_level" => ("Qavat", "Этаж"),
        "in_design" => ("Loyiha", "Проект"),
        "in_fact" => ("Fakt", "Факт"),
        "in_deviation" => ("Chetlanish", "Отклонение"),
        "in_tolerance" => ("Dopusk", "Допуск"),
        "in_measured" => ("O'lchangan", "Замер"),
        "in_surveyor" => ("Geodezist", "Геодезист"),

        "in_day_hint" => (
            "Hisobot qo'lda yozilmaydi: oxirgi ikki haftadagi yozuvlardan yig'iladi.",
            "Отчёт не пишется вручную: собирается из записей за последние две недели.",
        ),
        "in_day_empty" => ("Bu davrda yozuv yo'q", "За этот период записей нет"),
        "in_d_checks" => ("Tekshiruv", "Проверок"),
        "in_d_failed" => ("Salbiy", "Отрицательных"),
        "in_d_issues_open" => ("Ochilgan izoh", "Открыто замечаний"),
        "in_d_issues_closed" => ("Yopilgan izoh", "Закрыто замечаний"),
        "in_d_quality" => ("Sifat / salbiy", "Качество / брак"),
        "in_d_docs" => ("Imzolangan", "Подписано"),
        "in_d_lab" => ("Beton / geodeziya", "Бетон / геодезия"),
        "in_fixed_at" => ("Bartaraf etilgan", "Устранено"),
        "in_open_defect" => ("Ochiq nuqson", "Открытый дефект"),

        // ---------- XVII.4. Obyektlar bo'yicha konsolidatsiya ----------
        "screen_portfolio" => ("Obyektlar", "Объекты"),
        "pf_hint" => (
            "Barcha obyektlar yonma-yon. Ro'yxat e'tibor talab qiladiganlardan boshlanadi, sabab yonida yozilgan.",
            "Все объекты рядом. Список начинается с требующих внимания, причина указана рядом.",
        ),
        "pf_projects" => ("Obyektlar", "Объектов"),
        "pf_projects_hint" => ("bazada", "в базе"),
        "pf_attention" => ("E'tibor kerak", "Требуют внимания"),
        "pf_attention_hint" => ("kechikish, kritik yoki xavfsizlik", "отставание, критичное или безопасность"),
        "pf_progress" => ("Bajarilish", "Выполнение"),
        "pf_progress_hint" => ("shartnoma summasi bo'yicha vaznlangan", "взвешенное по сумме договора"),
        "pf_contract" => ("Shartnomalar", "Договоры"),
        "pf_contract_hint" => ("jami summa", "общая сумма"),
        "pf_paid" => ("To'langan", "Оплачено"),
        "pf_paid_hint" => ("shartnomalardan", "от договоров"),
        "pf_stock" => ("Omborlar", "Склады"),
        "pf_stock_hint" => ("qoldiq qiymati", "стоимость остатков"),

        "pf_object" => ("Obyekt", "Объект"),
        "pf_fact" => ("Fakt", "Факт"),
        "pf_plan" => ("Reja", "План"),
        "pf_deviation" => ("Chetlanish", "Отклонение"),
        "pf_contract_short" => ("Shartnoma", "Договор"),
        "pf_paid_short" => ("To'langan", "Оплачено"),
        "pf_quality" => ("Sifat", "Качество"),
        "pf_safety" => ("Xavfsizlik", "Безопасность"),
        "pf_sales" => ("Sotilgan", "Продано"),
        "pf_reason" => ("Sabab", "Причина"),
        "pf_open" => ("Ochish", "Открыть"),
        "pf_current" => ("Joriy obyekt", "Текущий объект"),
        "pf_r_ok" => ("holat barqaror", "состояние стабильное"),
        "pf_r_delay" => ("kechikish, kun:", "отставание, дней:"),
        "pf_r_critical" => ("kritik nomuvofiqlik:", "критичных несоответствий:"),
        "pf_r_safety" => ("xavfsizlik balli past", "низкий балл безопасности"),
        "pf_r_late" => ("kechikkan yetkazish:", "просроченных поставок:"),
        "pf_r_overdue" => ("muddati o'tgan ish:", "просроченных работ:"),
        "pf_sales_paid" => ("tushum", "поступило"),
        "pf_no_sales" => ("sotuv bo'limi yuritilmagan", "продажи не ведутся"),
        "col_delay_days" => ("Kechikish, kun", "Отставание, дней"),
        "col_estimate" => ("Smeta", "Смета"),
        "col_critical_count" => ("Kritik", "Критичных"),
        "col_late_delivery" => ("Kechikkan yetkazish", "Просроченных поставок"),
        "col_units_total" => ("Jami xonadon", "Всего помещений"),
        "col_module" => ("Modul", "Модуль"),
        "col_recommendation" => ("Tavsiya", "Рекомендация"),
        "col_what" => ("Amal", "Действие"),
        "set_audit_shown" => (
            "Ro'yxatda oxirgi yozuvlar ko'rsatilgan:",
            "В списке показаны последние записи:",
        ),
        "col_table" => ("Jadval", "Таблица"),
        "col_row" => ("Yozuv", "Запись"),
        "no_user" => ("tanlanmagan", "не выбран"),
        "doc_saved" => ("Hujjat saqlandi:", "Документ сохранён:"),
        "doc_failed" => ("Hujjatni saqlab bo'lmadi", "Не удалось сохранить документ"),
        "doc_blank" => ("Blanka", "Бланк"),
        "doc_no_work" => (
            "Bu davrda bajarilgan ish yo'q — dalolatnoma tuzilmadi",
            "За этот период нет выполненных работ — акт не сформирован",
        ),
        "doc_ks2_hint" => (
            "Joriy oy uchun bajarilgan ishlar dalolatnomasi. Hajm ijro foizidan, narx smetadan olinadi.",
            "Акт выполненных работ за текущий месяц. Объём — из процента выполнения, цена — из сметы.",
        ),
        "doc_ks3_hint" => (
            "Bajarilgan ish qiymati haqida ma'lumotnoma: shartnoma, bajarilgan va to'langan summalar.",
            "Справка о стоимости выполненных работ: договор, выполнено и оплачено.",
        ),
        "doc_m29_hint" => (
            "Material sarfi hisoboti: normativ va haqiqiy sarf, ortiqcha sarf summasi.",
            "Отчёт о расходе материалов: норматив и факт, сумма перерасхода.",
        ),
        "doc_aosr_hint" => (
            "Yashirin ishlar dalolatnomasi: ish, ishlatilgan materiallar va sertifikatlari.",
            "Акт освидетельствования скрытых работ: работа, применённые материалы и их сертификаты.",
        ),
        "doc_only_hidden" => (
            "Blanka faqat yashirin ishlar hujjati uchun",
            "Бланк только для акта скрытых работ",
        ),
        "doc_code" => ("Rasenka", "Расценка"),
        "doc_work" => ("Ish nomi", "Наименование работ"),
        "doc_total" => ("Jami", "Итого"),
        "doc_kind" => ("Ko'rsatkich", "Показатель"),

        "doc_ks2" => (
            "Bajarilgan ishlarni qabul qilish dalolatnomasi (KS-2)",
            "Акт о приёмке выполненных работ (КС-2)",
        ),
        "doc_ks2_short" => ("KS-2", "КС-2"),
        "doc_plan_qty" => ("Shartnoma bo'yicha", "По договору"),
        "doc_done_qty" => ("Bajarilgan", "Выполнено"),
        "doc_no_price" => (
            "Smetada narxi topilmagan qatorlar bor:",
            "Есть строки, для которых цена в смете не найдена:",
        ),

        "doc_ks3" => (
            "Bajarilgan ish qiymati haqida ma'lumotnoma (KS-3)",
            "Справка о стоимости выполненных работ (КС-3)",
        ),
        "doc_ks3_short" => ("KS-3", "КС-3"),
        "doc_since_contract" => ("Shartnoma boshidan", "С начала договора"),
        "doc_since_year" => ("Yil boshidan", "С начала года"),
        "doc_period_col" => ("Shu davrda", "За период"),
        "doc_row_contract" => ("Shartnoma summasi", "Сумма договора"),
        "doc_row_done" => ("Bajarilgan ish qiymati", "Стоимость выполненных работ"),
        "doc_row_paid" => ("To'langan", "Оплачено"),
        "doc_row_unpaid" => ("To'lanmagan qoldiq", "Неоплаченный остаток"),
        "doc_estimate_check" => ("Smeta bilan solishtirish", "Сверка со сметой"),
        "doc_over_contract" => (
            "Diqqat: smeta summasi shartnoma summasidan oshgan.",
            "Внимание: сумма сметы превышает сумму договора.",
        ),

        "doc_m29" => (
            "Material sarfi hisoboti (M-29)",
            "Отчёт о расходе материалов (М-29)",
        ),
        "doc_m29_short" => ("M-29", "М-29"),

        "doc_aosr" => (
            "Yashirin ishlarni ko'zdan kechirish dalolatnomasi",
            "Акт освидетельствования скрытых работ",
        ),
        "doc_aosr_short" => ("AOSR", "АОСР"),
        "doc_materials_used" => (
            "Ishda ishlatilgan materiallar",
            "Материалы, применённые в работе",
        ),
        "doc_no_materials" => (
            "Ombordan bu ishga material berilmagan",
            "Материалы на эту работу со склада не выдавались",
        ),
        "doc_verdict" => ("Xulosa", "Заключение"),
        "doc_verdict_text" => (
            "Ishlar loyiha va normativ talablariga muvofiq bajarilgan, keyingi ishlarni bajarishga ruxsat beriladi.",
            "Работы выполнены в соответствии с проектом и нормативными требованиями, разрешается производство последующих работ.",
        ),

        // Eksport ustunlari
        "col_start" => ("Boshlanish", "Начало"),
        "col_fact_start" => ("Fakt boshlanish", "Факт начала"),
        "col_fact_end" => ("Fakt tugash", "Факт окончания"),
        "col_critical" => ("Kritik yo'l", "Критический путь"),
        "col_declared" => ("Hujjatda", "В документе"),
        "col_text" => ("Matn", "Текст"),
        "col_odo_start" => ("Spidometr chiqish", "Спидометр выезд"),
        "col_odo_end" => ("Spidometr qaytish", "Спидометр возврат"),
        "col_trips" => ("Reys", "Рейсы"),
        "col_cargo" => ("Yuk", "Груз"),
        "col_contract" => ("Shartnoma", "Договор"),
        "col_fact_short" => ("Fakt", "Факт"),
        "col_evidence" => ("Hisob", "Расчёт"),
        "col_action" => ("Tavsiya", "Рекомендация"),

        // ---------- XII.5-11, 19, 28-38. Bog'lanish, analog, tarix, brak ----------
        "mat_tab_alts" => ("Analoglar", "Аналоги"),
        "mat_tab_trace" => ("Kuzatuvchanlik", "Прослеживаемость"),
        "mat_tab_ready" => ("Ish tayyorligi", "Готовность к работам"),
        "mk_to_supplier" => ("Yetkazib beruvchiga qaytarish", "Возврат поставщику"),

        // Katalog ustunlari
        "col_estimate_code" => ("Rasenka", "Расценка"),
        "col_spec_ref" => ("Spetsifikatsiya", "Спецификация"),
        "col_special" => ("Maxsus talab", "Особое требование"),
        "col_banned" => ("Taqiq", "Запрет"),
        "material_banned" => ("ishlatib bo'lmaydi", "нельзя применять"),
        "ban_reason_hint" => (
            "Taqiq sababi yozilmagan — sababsiz taqiq bajarilmaydi",
            "Причина запрета не указана — запрет без причины не соблюдают",
        ),

        // Analoglar (TZ XII.9-11)
        "add_alt" => ("+ Analog", "+ Аналог"),
        "alts_hint" => (
            "Almashtiruvchi material tasdiqlangan bo'lishi kerak: kim va qachon ruxsat berganini bilmasak, almashtirish loyihadan chetga chiqish bo'lib qoladi.",
            "Замена должна быть согласована: если неизвестно, кто и когда разрешил, замена превращается в отступление от проекта.",
        ),
        "alts_empty" => (
            "Analog kiritilmagan — «+ Analog» bilan boshlang",
            "Аналоги не внесены — начните с «+ Аналог»",
        ),
        "col_alt" => ("Analog", "Аналог"),
        "col_alt_diff" => ("Narx farqi", "Разница цены"),
        "col_approved_by" => ("Kim tasdiqladi", "Кто согласовал"),
        "col_approved_at" => ("Qachon", "Когда"),
        "alt_approved" => ("tasdiqlangan", "согласован"),
        "alt_not_approved" => ("tasdiqlanmagan", "не согласован"),
        "alt_banned" => ("analog taqiqlangan", "аналог запрещён"),
        "alt_needs_two" => (
            "Analog uchun kamida ikkita material kerak",
            "Для аналога нужно минимум два материала",
        ),

        // Kuzatuvchanlik (TZ XII.19, 29-30, 35-36)
        "trace_hint" => (
            "Bitta material bo'yicha: qayerdan kelgan, qayerga ketgan, qaysi hujjatga kirgan va narxi qanday o'zgargan.",
            "По одному материалу: откуда пришёл, куда ушёл, в какой документ попал и как менялась цена.",
        ),
        "kpi_trace_in" => ("Kelgan", "Поступило"),
        "kpi_trace_in_hint" => ("jami kirim", "всего прихода"),
        "kpi_issued" => ("Berilgan", "Выдано"),
        "kpi_issued_hint" => ("ta ishga", "работам"),
        "kpi_deliveries" => ("Yetkazishlar", "Поставок"),
        "kpi_deliveries_hint" => ("narxi bor kirimlar", "приходов с ценой"),
        "kpi_price_change" => ("Narx o'zgarishi", "Изменение цены"),
        "kpi_price_change_hint" => ("birinchi kirimdan", "с первого прихода"),
        "kpi_input_pass" => ("Kirish nazorati", "Входной контроль"),
        "kpi_input_pass_hint" => ("o'tgan tekshiruvlar", "пройденных проверок"),
        "kpi_last_price" => ("oxirgi narx:", "последняя цена:"),
        "kpi_checks" => ("tekshiruv", "проверок"),
        "kpi_rejected" => ("rad etilgan", "отклонено"),
        "trace_chain" => ("Zanjir", "Цепочка"),
        "trace_suppliers" => ("Yetkazib beruvchilar", "Поставщики"),
        "trace_batches" => ("Partiyalar", "Партии"),
        "trace_tasks" => ("Qaysi ishlarga berilgan", "На какие работы выдан"),
        "trace_docs" => ("Ijro hujjatlari", "Исполнительные документы"),
        "trace_no_docs" => (
            "hujjat yo'q — zanjir uzilgan",
            "документов нет — цепочка разорвана",
        ),
        "trace_checks" => ("Sifat tekshiruvlari", "Проверок качества"),
        "trace_prices" => ("Narx tarixi", "История цены"),
        "trace_no_prices" => (
            "Narxi ko'rsatilgan kirim yo'q",
            "Нет приходов с указанной ценой",
        ),
        "col_change" => ("O'zgarish", "Изменение"),
        "trace_defects" => ("Brak", "Брак"),
        "trace_rejected" => ("Kirish nazoratida rad etilgan:", "Отклонено на входном контроле:"),
        "trace_unresolved" => (
            "Brak ombordan chiqmagan — u hali ham ishlatilishi mumkin",
            "Брак не выведен со склада — его всё ещё могут применить",
        ),

        // Ish tayyorligi (TZ XII.28)
        "ready_hint" => (
            "Ikki hafta ichida boshlanadigan ishlar uchun material yetarlimi. Tekshiruv butun hajmga qaraydi: yetishmasligini oldindan bilish kerak, buyurtma vaqt oladi.",
            "Хватает ли материалов на работы, начинающиеся в ближайшие две недели. Проверка смотрит на полный объём: о нехватке надо знать заранее, закупка занимает время.",
        ),
        "ready_ok" => (
            "Yaqin ishlar uchun material yetarli",
            "На ближайшие работы материалов хватает",
        ),
        "col_days_left" => ("Kun qoldi", "Дней"),
        "col_needed" => ("Kerak", "Нужно"),
        "col_short" => ("Yetishmaydi", "Не хватает"),

        // ---------- XIII. Tabel ----------
        "add_worker" => ("+ Ishchi", "+ Рабочий"),
        "worker_new_name" => ("Yangi ishchi", "Новый рабочий"),
        "workers_empty" => (
            "Ishchilar kiritilmagan — «+ Ishchi» bilan boshlang",
            "Рабочие не внесены — начните с «+ Рабочий»",
        ),
        "timesheet_hint" => (
            "Katakka soat yozing. Ish haqi soat va stavkadan hisoblanadi.",
            "Впишите часы в ячейку. Зарплата считается из часов и ставки.",
        ),
        "prev_week" => ("Oldingi hafta", "Предыдущая неделя"),
        "next_week" => ("Keyingi hafta", "Следующая неделя"),
        "this_week" => ("Shu hafta", "Текущая неделя"),
        "overtime_hint" => (
            "Kunlik normadan (8 soat) oshgan",
            "Больше дневной нормы (8 часов)",
        ),
        "col_worker" => ("Ishchi", "Рабочий"),
        "col_hourly_rate" => ("Soatlik stavka", "Ставка за час"),
        "col_total_hours" => ("Jami soat", "Всего часов"),
        "col_wage" => ("Ish haqi", "Зарплата"),
        "wd_mon" => ("Du", "Пн"),
        "wd_tue" => ("Se", "Вт"),
        "wd_wed" => ("Ch", "Ср"),
        "wd_thu" => ("Pa", "Чт"),
        "wd_fri" => ("Ju", "Пт"),
        "wd_sat" => ("Sh", "Сб"),
        "wd_sun" => ("Ya", "Вс"),
        "kpi_workers" => ("Faol ishchilar", "Активных рабочих"),
        "kpi_workers_hint" => ("ta ro'yxatda", "в списке"),
        "kpi_hours_week" => ("Haftalik soat", "Часов за неделю"),
        "kpi_hours_week_hint" => ("tabel bo'yicha", "по табелю"),
        "kpi_avg_day" => ("Kunlik o'rtacha", "В среднем за день"),
        "kpi_avg_day_hint" => ("ishlangan kunlarda", "в отработанные дни"),
        "kpi_payroll" => ("Ish haqi fondi", "Фонд зарплаты"),
        "kpi_payroll_hint" => ("shu hafta uchun", "за эту неделю"),

        // ---------- XIII. Brigadalar, smenalar, yo'qliklar, tannarx ----------
        "ts_tab_sheet" => ("Tabel", "Табель"),
        "ts_tab_brigades" => ("Brigadalar", "Бригады"),
        "ts_tab_cost" => ("Tannarx", "Себестоимость"),

        "add_brigade" => ("+ Brigada", "+ Бригада"),
        "add_brigade_hint" => (
            "Brigada — ishchilar guruhi va uning brigadiri. Soat, ish haqi va bo'sh turish brigada bo'yicha yig'iladi.",
            "Бригада — группа рабочих и её бригадир. Часы, зарплата и простои собираются по бригаде.",
        ),
        "brigade_new_name" => ("brigada", "бригада"),
        "no_brigade" => ("Brigadasiz", "Без бригады"),
        "brigades_empty" => (
            "Brigada yaratilmagan — «+ Brigada» bilan boshlang",
            "Бригады не созданы — начните с «+ Бригада»",
        ),
        "brigades_hint" => (
            "Ko'rsatkichlar tanlangan hafta bo'yicha. Bo'sh turish ulushi 10% dan oshsa qizil bo'ladi.",
            "Показатели за выбранную неделю. Доля простоев выше 10% выделяется красным.",
        ),
        "col_brigade" => ("Brigada", "Бригада"),
        "col_foreman" => ("Brigadir", "Бригадир"),
        "col_people" => ("Odam", "Человек"),
        "col_downtime" => ("Bo'sh turish", "Простой"),
        "col_cost_per_hour" => ("Soatning tannarxi", "Себестоимость часа"),

        // Katakda nima tahrirlanadi
        "cell_shows" => ("Katakda:", "В ячейке:"),
        "cell_hours" => ("Soat", "Часы"),
        "cell_kind" => ("Kun turi", "Тип дня"),
        "cell_shift" => ("Smena", "Смена"),
        "cell_task" => ("Ish", "Работа"),
        "cell_overtime" => ("ortiqcha ish", "сверхурочные"),

        // Smenalar (TZ XIII.11, 14)
        "sh_day" => ("Kunduzgi", "Дневная"),
        "sh_evening" => ("Kechki", "Вечерняя"),
        "sh_night" => ("Tungi", "Ночная"),
        "shs_day" => ("K", "Д"),
        "shs_evening" => ("Kc", "В"),
        "shs_night" => ("T", "Н"),

        // Kun turlari (TZ XIII.16-22)
        "dk_work" => ("Ish kuni", "Рабочий день"),
        "dk_downtime" => ("Bo'sh turish", "Простой"),
        "dk_vacation" => ("Ta'til", "Отпуск"),
        "dk_sick" => ("Kasallik varaqasi", "Больничный"),
        "dk_trip" => ("Xizmat safari", "Командировка"),
        "dk_absent" => ("Sababsiz yo'qlik", "Прогул"),
        "dks_work" => ("I", "Р"),
        "dks_downtime" => ("BT", "П"),
        "dks_vacation" => ("T", "О"),
        "dks_sick" => ("K", "Б"),
        "dks_trip" => ("XS", "К"),
        "dks_absent" => ("Y", "Н"),

        "kpi_overtime" => ("Ortiqcha ish", "Сверхурочные"),
        "kpi_overtime_hint" => ("normadan oshgan soat", "часов сверх нормы"),
        "kpi_downtime" => ("Bo'sh turish", "Простой"),
        "kpi_downtime_hint" => ("soat, ishchi aybsiz", "часов не по вине рабочего"),
        "kpi_absences" => ("Yo'qliklar", "Отсутствия"),
        "kpi_absences_hint" => ("kun: ta'til, kasallik, safar", "дней: отпуск, больничный, командировка"),

        // Tannarx (TZ XIII.30-31)
        "cost_empty" => (
            "Tannarx uchun tabelda ish ko'rsatilishi kerak — katakni «Ish» rejimida to'ldiring",
            "Для себестоимости в табеле нужно указать работу — заполните ячейки в режиме «Работа»",
        ),
        "cost_hint" => (
            "Tannarx tabeldagi soat va omborga berilgan materialdan yig'iladi — alohida kiritilmaydi.",
            "Себестоимость собирается из часов табеля и выданного со склада материала — отдельно не вводится.",
        ),
        "col_total" => ("Jami", "Итого"),
        "col_labour" => ("Ish haqi", "Зарплата"),
        "col_material_cost" => ("Material", "Материал"),
        "col_per_volume" => ("Bir birlikka", "На единицу"),
        "kpi_labour_cost" => ("Ish haqi", "Зарплата"),
        "kpi_labour_cost_hint" => ("ishlarga taqsimlangan", "распределено по работам"),
        "kpi_material_cost" => ("Material", "Материал"),
        "kpi_material_cost_hint" => ("ishlarga berilgan", "выдано на работы"),
        "kpi_task_machine" => ("Texnika", "Техника"),
        "kpi_task_machine_hint" => ("ishlarda ishlagan", "работала на работах"),
        "kpi_total_cost" => ("Jami tannarx", "Итого себестоимость"),
        "kpi_total_cost_hint" => ("ish haqi va material", "зарплата и материал"),

        // ---------- XIV.8, 10, 30-35. Chek-listlar, ball, bloklash ----------
        "ql_tab_checks" => ("Tekshiruvlar", "Проверки"),
        "ql_tab_checklists" => ("Chek-listlar", "Чек-листы"),
        "ql_tab_defects" => ("Brak tahlili", "Анализ брака"),
        "ql_tab_blocks" => ("Bloklangan ishlar", "Заблокированные работы"),

        "kpi_quality_score" => ("Sifat balli", "Балл качества"),
        "kpi_quality_score_hint" => ("0-100, ochiq nuqsonlar hisobga olinadi", "0-100, с учётом открытых дефектов"),
        "kpi_score_open" => ("ochiq nuqson", "открытых дефектов"),
        "kpi_quality_blocked" => ("Bloklangan", "Заблокировано"),
        "kpi_quality_blocked_hint" => ("ish yopilmaydi", "работ нельзя закрыть"),

        "col_fixed" => ("Bartaraf etildi", "Устранено"),
        "fixed_hint" => (
            "Nuqson bartaraf etilgan sanani belgilang — ochiq nuqsonlar ballni pasaytiradi.",
            "Отметьте дату устранения дефекта — открытые дефекты снижают балл.",
        ),
        "col_points" => ("Nazorat nuqtalari", "Точки контроля"),
        "points_none" => ("chek-list yo'q", "чек-листа нет"),
        "points_open_hint" => (
            "Nazorat nuqtalarini ochish",
            "Открыть точки контроля",
        ),

        // Nazorat nuqtalari paneli
        "points_title" => ("Nazorat nuqtalari", "Точки контроля"),
        "points_no_checklist" => (
            "Bu bosqich uchun chek-list namunasi yo'q — «Chek-listlar» ko'rinishida yarating",
            "Для этого этапа нет шаблона чек-листа — создайте его во вкладке «Чек-листы»",
        ),
        "points_apply" => ("Namunadan:", "Из шаблона:"),
        "points_apply_hint" => (
            "Bandlar ko'chirib olinadi: namuna keyin o'zgarsa ham bu tekshiruv o'zgarmaydi.",
            "Пункты копируются: если шаблон потом изменится, эта проверка не изменится.",
        ),
        "points_applied" => ("Nazorat nuqtalari qo'shildi:", "Добавлено точек контроля:"),
        "points_empty" => (
            "Nuqta yo'q — namunadan oling yoki qo'lda qo'shing",
            "Точек нет — возьмите из шаблона или добавьте вручную",
        ),
        "points_add" => ("+ Nuqta", "+ Точка"),

        "pt_pending" => ("Tekshirilmagan", "Не проверено"),
        "pt_pass" => ("Mos", "Соответствует"),
        "pt_fail" => ("Mos emas", "Не соответствует"),
        "pt_na" => ("Taalluqli emas", "Не применимо"),
        "pts_pending" => ("—", "—"),
        "pts_pass" => ("Mos", "Да"),
        "pts_fail" => ("Yo'q", "Нет"),
        "pts_na" => ("N/A", "N/A"),

        // Chek-list namunalari
        "add_checklist" => ("+ Chek-list", "+ Чек-лист"),
        "checklist_new_name" => ("Chek-list", "Чек-лист"),
        "checklist_add_item" => ("+ Band", "+ Пункт"),
        "checklist_no_items" => (
            "Band yo'q — «+ Band» bilan qo'shing",
            "Пунктов нет — добавьте кнопкой «+ Пункт»",
        ),
        "checklists_hint" => (
            "Namuna bo'lim va nazorat bosqichiga bog'lanadi. Tekshiruvga biriktirilganda bandlar ko'chirib olinadi.",
            "Шаблон привязан к разделу и этапу контроля. При привязке к проверке пункты копируются.",
        ),
        "checklists_empty" => (
            "Chek-list namunasi yo'q — «+ Chek-list» bilan boshlang",
            "Шаблонов чек-листов нет — начните с «+ Чек-лист»",
        ),
        "col_norm_doc" => ("Hujjat", "Документ"),
        "col_norm_clause" => ("Band", "Пункт"),

        // Brak tahlili
        "defects_hint" => (
            "Bir xil nuqson necha marta takrorlangani. Uch martadan ko'pi — tizimli muammo, sababini izlash kerak.",
            "Сколько раз повторился один и тот же дефект. Больше трёх раз — системная проблема, нужно искать причину.",
        ),
        "defects_empty" => (
            "Nuqson yozilmagan — brak tahlili uchun ma'lumot yo'q",
            "Дефекты не зафиксированы — нет данных для анализа брака",
        ),
        "col_times" => ("Marta", "Раз"),
        "col_open" => ("Ochiq", "Открыто"),
        "col_overdue" => ("Muddati o'tgan", "Просрочено"),
        "col_last" => ("Oxirgi", "Последний"),
        "col_tasks" => ("Ishlar", "Работы"),

        // Bloklash
        "blocks_hint" => (
            "Yopilishga yaqin (95% dan yuqori) ishlar sifat bo'yicha tekshiriladi. Bu taqiq emas — nima yopilmaganini ko'rsatadi.",
            "Работы близкие к закрытию (выше 95%) проверяются по качеству. Это не запрет — показывает, что не закрыто.",
        ),
        "blocks_empty" => (
            "Sifat bo'yicha to'siq yo'q",
            "Блокировок по качеству нет",
        ),
        "col_open_defects" => ("Ochiq nuqson", "Открытых дефектов"),
        "col_pending_points" => ("Tekshirilmagan", "Не проверено точек"),
        "col_reason" => ("Sabab", "Причина"),
        "block_no_acceptance" => ("qabul nazorati yo'q", "нет приёмочного контроля"),
        "block_open_defects" => ("nuqson bartaraf etilmagan", "дефект не устранён"),
        "block_pending_points" => ("nazorat nuqtalari to'ldirilmagan", "точки контроля не заполнены"),

        // ---------- XIV. Sifat ----------
        "quality_hint" => (
            "Kirish, operatsion va qabul nazorati. Nuqson bo'lsa muddat qo'ying.",
            "Входной, операционный и приемочный контроль. При дефекте укажите срок.",
        ),
        "quality_empty" => (
            "Nazorat yozuvi yo'q — bosqichni tanlab qo'shing",
            "Записей контроля нет — добавьте, выбрав этап",
        ),
        "quality_new_subject" => ("Nazorat", "Контроль"),
        "col_stage" => ("Bosqich", "Этап"),
        "col_result" => ("Natija", "Результат"),
        "col_subject" => ("Nazorat obyekti", "Объект контроля"),
        "col_inspector" => ("Tekshiruvchi", "Проверяющий"),
        "col_defect" => ("Nuqson", "Дефект"),
        "col_fix_deadline" => ("Bartaraf etish muddati", "Срок устранения"),
        "kpi_quality_total" => ("Nazorat yozuvlari", "Записей контроля"),
        "kpi_quality_total_hint" => ("jami", "всего"),
        "kpi_quality_pass" => ("Talabga mos", "Соответствует"),
        "kpi_quality_pass_hint" => ("yozuvlarning ulushi", "доля записей"),
        "kpi_quality_fail" => ("Mos emas", "Не соответствует"),
        "kpi_quality_cond_hint" => ("ta shartli ruxsat", "условно допущено"),
        "kpi_quality_overdue" => ("Muddati o'tgan nuqson", "Просроченный дефект"),
        "kpi_quality_overdue_hint" => ("bartaraf etilmagan", "не устранен"),

        // ---------- XV.4-12, 33. Ruxsatlar, SIZ, naryad-dopusk, ball ----------
        "sf_tab_events" => ("Hodisalar", "События"),
        "sf_tab_permits" => ("Ruxsatlar", "Допуски"),
        "sf_tab_ppe" => ("SIZ", "СИЗ"),
        "sf_tab_work_permits" => ("Naryad-dopusk", "Наряд-допуск"),

        "kpi_safety_score" => ("Xavfsizlik balli", "Балл безопасности"),
        "kpi_score_permits" => ("ruxsat muddati o'tgan", "просроченных допусков"),
        "kpi_score_ppe" => ("SIZ yo'q", "без СИЗ"),
        "kpi_score_violations" => ("buzilish", "нарушений"),
        "kpi_score_near_miss" => ("near miss", "near miss"),
        "kpi_score_bad_permits" => ("kamchilikli naryad", "нарядов с замечаниями"),

        // Ruxsat turlari (TZ XV.4-5)
        "pk_induction" => ("Kirish instruktaji", "Вводный инструктаж"),
        "pk_height" => ("Balandlikda", "Высотные"),
        "pk_electric" => ("Elektr", "Электро"),
        "pk_hot_work" => ("O't ishlari", "Огневые"),
        "pk_lifting" => ("Yuk ko'tarish", "Грузоподъёмные"),
        "pk_confined" => ("Yopiq idish", "Замкнутое пространство"),
        "pk_excavation" => ("Yer ishlari", "Земляные"),
        "pk_medical" => ("Tibbiy ko'rik", "Медосмотр"),

        "permits_hint" => (
            "Katakda ruxsat muddati. Bosilsa ruxsat bir yilga ochiladi yoki uzaytiriladi; raqam va aniq sana keyin tahrirlanadi.",
            "В ячейке — срок допуска. По нажатию допуск открывается или продлевается на год; номер и точную дату можно уточнить позже.",
        ),
        "permits_no_workers" => (
            "Ishchi yo'q — avval «Tabel» bo'limida ishchi qo'shing",
            "Нет рабочих — сначала добавьте их в разделе «Табель»",
        ),
        "permit_issue" => ("Ruxsat ochish", "Открыть допуск"),
        "permit_extend" => ("Bir yilga uzaytirish", "Продлить на год"),
        "worker_ok" => ("ishga ruxsat bor", "допуск есть"),
        "worker_blocked" => ("ishga qo'yib bo'lmaydi", "нельзя допускать"),
        "worker_expired" => ("muddati o'tgan ruxsat bor", "есть просроченный допуск"),
        "worker_expiring" => ("muddati tugayapti", "срок истекает"),

        // SIZ (TZ XV.8-9)
        "ppe_helmet" => ("Kaska", "Каска"),
        "ppe_vest" => ("Jilet", "Жилет"),
        "ppe_boots" => ("Poyabzal", "Обувь"),
        "ppe_gloves" => ("Qo'lqop", "Перчатки"),
        "ppe_glasses" => ("Ko'zoynak", "Очки"),
        "ppe_harness" => ("Arqon", "Страховочная привязь"),
        "ppe_mask" => ("Niqob", "Респиратор"),
        "ppe_ears" => ("Quloqchin", "Наушники"),
        "ppe_hint" => (
            "Qora sarlavhali SIZ har bir ishchida bo'lishi shart, kulrangi ish turiga qarab beriladi. Katak bosilsa SIZ beriladi va xizmat muddati boshlanadi.",
            "СИЗ с тёмным заголовком обязательны для каждого рабочего, серые выдаются по виду работ. По нажатию СИЗ выдаётся и начинается срок службы.",
        ),
        "ppe_required" => ("Har bir ishchida bo'lishi shart", "Обязателен для каждого"),
        "ppe_by_work" => ("Ish turiga qarab beriladi", "Выдаётся по виду работ"),
        "ppe_none" => ("berilmagan", "не выдан"),
        "ppe_no_limit" => ("muddatsiz", "бессрочно"),
        "ppe_missing" => ("yetishmaydi:", "не хватает:"),
        "ppe_issue_hint" => (
            "Bosilsa SIZ beriladi, xizmat muddati bugundan boshlanadi.",
            "По нажатию СИЗ выдаётся, срок службы начинается с сегодня.",
        ),

        // Naryad-dopusk (TZ XV.10-12)
        "wps_draft" => ("Loyiha", "Черновик"),
        "wps_open" => ("Ochiq", "Открыт"),
        "wps_closed" => ("Yopilgan", "Закрыт"),
        "wps_stopped" => ("To'xtatilgan", "Приостановлен"),
        "add_work_permit" => ("+ Naryad", "+ Наряд"),
        "work_permits_hint" => (
            "Naryad aniq ish, aniq muddat va aniq odamlar uchun beriladi. Kamchiliklar ustuni imzolashdan oldin nima yetishmayotganini ko'rsatadi.",
            "Наряд выдаётся на конкретную работу, срок и людей. Графа замечаний показывает, чего не хватает до подписания.",
        ),
        "work_permits_empty" => (
            "Naryad-dopusk yo'q — yuqori xavfli ish uchun «+ Naryad» bilan oching",
            "Нарядов нет — для работ повышенной опасности откройте «+ Наряд»",
        ),
        "col_from" => ("Boshlanish", "Начало"),
        "col_to" => ("Tugash", "Окончание"),
        "col_issuer" => ("Bergan", "Выдал"),
        "col_supervisor" => ("Nazorat qiluvchi", "Ответственный"),
        "col_executors" => ("Bajaruvchilar", "Исполнители"),
        "col_permit_issues" => ("Kamchiliklar", "Замечания"),
        "workers_none" => ("tanlanmagan", "не выбраны"),
        "workers_count" => ("ishchi", "чел."),
        "permit_ok" => ("kamchilik yo'q", "замечаний нет"),

        "pi_no_workers" => ("bajaruvchilar ko'rsatilmagan", "не указаны исполнители"),
        "pi_no_measures" => ("chora-tadbirlar yozilmagan", "не указаны мероприятия"),
        "pi_no_issuer" => ("mas'ul ko'rsatilmagan", "не указан ответственный"),
        "pi_bad_period" => ("muddat noto'g'ri", "неверный срок"),
        "pi_overdue" => ("muddati o'tgan, yopilmagan", "срок истёк, не закрыт"),
        "pi_not_allowed" => ("bu ishga ruxsati yo'q", "нет допуска на эти работы"),
        "pi_no_ppe" => ("SIZ yetishmaydi", "не хватает СИЗ"),

        // ---------- XV. Xavfsizlik ----------
        "safety_hint" => (
            "Hodisa qayd etiladi, aybdor belgilanmaydi. Chora va muddat ko'rsatiling.",
            "Событие фиксируется, виновный не назначается. Укажите меру и срок.",
        ),
        "safety_empty" => (
            "Yozuv yo'q — turini tanlab qo'shing",
            "Записей нет — добавьте, выбрав вид",
        ),
        "col_place" => ("Joy", "Место"),
        "col_description" => ("Tavsif", "Описание"),
        "col_measure" => ("Ko'rilgan chora", "Принятая мера"),
        "kpi_safety_events" => ("Hodisalar", "Событий"),
        "kpi_safety_events_hint" => ("buzilish, xavf, hodisa", "нарушения, риски, случаи"),
        "kpi_safety_incidents" => ("Baxtsiz hodisa", "Несчастных случаев"),
        "kpi_safety_incidents_hint" => ("qayd etilgan", "зафиксировано"),
        "kpi_safety_open" => ("Yopilmagan", "Не закрыто"),
        "kpi_safety_open_hint" => ("chora kutmoqda", "ожидает мер"),
        "kpi_safety_overdue" => ("Muddati o'tgan", "Просрочено"),
        "kpi_safety_overdue_hint" => ("chora ko'rilmagan", "меры не приняты"),
        "kpi_safety_training" => ("Instruktajlar", "Инструктажей"),
        "kpi_safety_training_hint" => ("o'tkazilgan", "проведено"),

        // ---------- XVI.14, 17-27, 32-38. Yo'l varaqasi, TX, foydalanish ----------
        "mch_tab_usage" => ("Foydalanish", "Использование"),

        "col_fuel_norm" => ("Norma, l/soat", "Норма, л/м-ч"),
        "fuel_norm_hint" => (
            "Bir motosoatga yoqilg'i sarf normasi. Nol — norma yuritilmaydi.",
            "Норма расхода топлива на один машино-час. Ноль — норма не ведётся.",
        ),
        "col_service_hours" => ("TX oralig'i", "Интервал ТО"),
        "service_hours_hint" => (
            "Rejali texnik xizmat oralig'i, motosoatda. Nol — reja yuritilmaydi.",
            "Интервал планового ТО в машино-часах. Ноль — план не ведётся.",
        ),
        "col_service_left" => ("TX gacha qoldi", "До ТО осталось"),
        "col_rented" => ("Ijara", "Аренда"),
        "rented_hint" => (
            "Ijaraga olingan texnika — o'z texnikasi bilan solishtirish uchun.",
            "Арендованная техника — для сравнения со своей.",
        ),
        "machine_blocked" => ("ishlatib bo'lmaydi", "нельзя использовать"),
        "machine_ok" => ("ishlatish mumkin", "можно использовать"),
        "block_inspection" => ("texnik ko'rik muddati o'tgan", "техосмотр просрочен"),
        "block_service" => ("TX muddati o'tgan", "ТО просрочено"),
        "block_repair" => ("ta'mirda", "в ремонте"),

        // Yo'l varaqasi (TZ XVI.19-22)
        "col_driver" => ("Haydovchi", "Водитель"),
        "col_route_way" => ("Marshrut", "Маршрут"),
        "route_hint" => ("qayerdan — qayerga", "откуда — куда"),
        "col_odometer" => ("Spidometr: chiqish / qaytish", "Спидометр: выезд / возврат"),
        "col_distance" => ("Masofa", "Пробег"),
        "distance_hint" => (
            "Spidometr farqidan hisoblanadi — alohida kiritilmaydi.",
            "Считается из разницы спидометра — отдельно не вводится.",
        ),
        "odo_back_hint" => (
            "Qaytish ko'rsatkichi chiqishdan kam — raqamda xato bor",
            "Показание на возврате меньше, чем на выезде — ошибка в цифрах",
        ),
        "fuel_over_hint" => ("Normadan ortiqcha:", "Сверх нормы:"),
        "col_trips_cargo" => ("Reys / yuk", "Рейсы / груз"),
        "trips_hint" => ("Reys soni", "Количество рейсов"),
        "cargo_hint" => ("Tashilgan yuk", "Перевезённый груз"),

        // Foydalanish (TZ XVI.36-38)
        "usage_machines_hint" => (
            "Oxirgi 30 kun bo'yicha. Foydalanish koeffitsiyenti ish kunlariga nisbatan: 60% dan past — texnika bekor turibdi.",
            "За последние 30 дней. Коэффициент использования — к рабочим дням: ниже 60% техника простаивает.",
        ),
        "col_work_days" => ("Ishlagan kun", "Рабочих дней"),
        "col_idle_days" => ("Bo'sh kun", "Простой, дней"),
        "col_utilization" => ("Foydalanish", "Использование"),
        "col_fuel_fact_norm" => ("Yoqilg'i: fakt / norma", "Топливо: факт / норма"),

        // ---------- XVI. Mashinalar ----------
        "add_machine" => ("+ Texnika", "+ Техника"),
        "add_machine_log" => ("+ Smena", "+ Смена"),
        "machine_new_name" => ("Yangi texnika", "Новая техника"),
        "machines_hint" => (
            "Motosoat va xarajat smenalardan hisoblanadi — alohida kiritilmaydi.",
            "Моточасы и затраты считаются из смен — отдельно не вводятся.",
        ),
        "machines_empty" => (
            "Texnika kiritilmagan — «+ Texnika» bilan boshlang",
            "Техника не внесена — начните с «+ Техника»",
        ),
        "machine_logs_empty" => ("Smena yozilmagan", "Смены не внесены"),
        "mch_tab_park" => ("Park", "Парк"),
        "mch_tab_logs" => ("Smenalar", "Смены"),
        "col_machine" => ("Texnika", "Техника"),
        "col_reg_no" => ("Davlat raqami", "Госномер"),
        "col_owner" => ("Egasi", "Владелец"),
        "col_operator" => ("Operator", "Оператор"),
        "col_hour_rate" => ("Soatlik stavka", "Ставка за час"),
        "col_inspection" => ("Texnik ko'rik", "Техосмотр"),
        "col_hours_30" => ("Motosoat, 30 kun", "Моточасы, 30 дн."),
        "col_hours" => ("Soat", "Часов"),
        "col_fuel" => ("YoMM, litr", "ГСМ, литр"),
        "col_machine_cost" => ("Smena qiymati", "Стоимость смены"),
        "kpi_machines" => ("Texnika", "Техники"),
        "kpi_machine_hours" => ("Motosoat", "Моточасы"),
        "kpi_machine_hours_hint" => ("oxirgi 30 kun", "за 30 дней"),
        "kpi_machine_fuel" => ("YoMM, litr", "ГСМ, литр"),
        "kpi_machine_fuel_hint" => ("oxirgi 30 kun", "за 30 дней"),
        "kpi_machine_cost" => ("Texnika xarajati", "Затраты на технику"),
        "kpi_machine_cost_hint" => ("oxirgi 30 kun", "за 30 дней"),
        "kpi_inspection" => ("Texnik ko'rik o'tgan", "Техосмотр просрочен"),
        "kpi_inspection_hint" => ("tasi yaqinda tugaydi", "истекают скоро"),

        // ---------- XIX–XX. Sotuv ----------
        "screen_sales" => ("Sotuv — shaxmatka", "Продажи — шахматка"),
        "screen_deals" => ("Shartnomalar va to'lovlar", "Договоры и платежи"),
        "nav_sales" => ("SOTUV", "ПРОДАЖИ"),

        // Birlik turlari va holatlari
        "uk_flat" => ("Kvartira", "Квартира"),
        "uk_comm" => ("Tijorat", "Коммерция"),
        "uk_office" => ("Ofis", "Офис"),
        "uk_parking" => ("Avtoturargoh", "Паркинг"),
        "uk_storage" => ("Ombor", "Кладовая"),
        "uks_flat" => ("kv", "кв"),
        "uks_comm" => ("tij", "комм"),
        "uks_office" => ("ofis", "офис"),
        "uks_parking" => ("park", "парк"),
        "uks_storage" => ("omb", "клад"),
        "us_free" => ("Bo'sh", "Свободна"),
        "us_reserved" => ("Band qilingan", "Забронирована"),
        "us_contract" => ("Shartnoma", "Договор"),
        "us_sold" => ("Sotilgan", "Продана"),
        "us_off" => ("Sotuvda emas", "Не в продаже"),

        // To'lov turlari va shartnoma holatlari
        "pk_cash" => ("Naqd", "Наличные"),
        "pk_installment" => ("Muddatli to'lov", "Рассрочка"),
        "pk_credit" => ("Kredit / ipoteka", "Кредит / ипотека"),
        "pk_subsidy" => ("Subsidiya", "Субсидия"),
        "pk_barter" => ("Barter", "Бартер"),
        "pk_mixed" => ("Aralash", "Смешанная"),
        "ds_reserved" => ("Band qilingan", "Бронь"),
        "ds_signed" => ("Imzolangan", "Подписан"),
        "ds_completed" => ("To'liq to'langan", "Полностью оплачен"),
        "ds_cancelled" => ("Bekor qilingan", "Расторгнут"),

        // Shaxmatka ekrani
        "add_block" => ("+ Blok", "+ Блок"),
        "block_short" => ("blok", "блок"),
        "generate_units" => ("Kvartiralarni yaratish", "Создать квартиры"),
        "sales_hint" => (
            "Vertikal — qavatlar, gorizontal — qavatdagi kvartiralar. Katakni bosing.",
            "По вертикали — этажи, по горизонтали — квартиры на этаже. Нажмите на ячейку.",
        ),
        "sales_no_block" => ("Blok qo'shilmagan", "Блоки не добавлены"),
        "sales_no_block_hint" => (
            "«+ Blok» bilan podez yarating, keyin kvartiralarni bir bosishda hosil qiling.",
            "Создайте подъезд через «+ Блок», затем сформируйте квартиры одной кнопкой.",
        ),
        "sales_no_units" => (
            "Bu blokda kvartira yo'q — «Kvartiralarni yaratish» tugmasini bosing",
            "В этом блоке нет квартир — нажмите «Создать квартиры»",
        ),
        "sales_tab_board" => ("Shaxmatka", "Шахматка"),
        "sales_tab_list" => ("Ro'yxat", "Список"),
        "rooms_short" => ("x", "к"),
        "units_created" => ("Yangi kvartira yaratildi:", "Создано квартир:"),

        // Generator
        "gen_per_floor" => ("Qavatdagi soni", "На этаже"),
        "gen_first_number" => ("Boshlang'ich raqam", "Начальный номер"),
        "gen_area" => ("Maydon, m²", "Площадь, м²"),
        "gen_price_m2" => ("1 m² narxi", "Цена за м²"),
        "gen_rooms" => ("Xonalar", "Комнат"),
        "gen_run" => ("Yaratish", "Создать"),
        "gen_hint" => (
            "Blokning har bir qavati uchun bir xil kvartiralar yaratiladi; raqami band bo'lgani o'tkazib yuboriladi.",
            "Для каждого этажа блока создаются одинаковые квартиры; занятые номера пропускаются.",
        ),

        // Kvartira kartochkasi
        "unit_card" => ("Kvartira", "Квартира"),
        "unit_deal" => ("Shartnoma", "Договор"),
        "unit_no_deal" => ("Shartnoma yo'q", "Договора нет"),
        "create_deal" => ("Band qilish / shartnoma", "Бронь / договор"),
        "open_deal" => ("Shartnomani ochish", "Открыть договор"),
        "delete_unit" => ("Kvartirani o'chirish", "Удалить квартиру"),
        "open_short" => ("Ochish", "Открыть"),
        "paid_short" => ("To'landi", "Оплачено"),
        "overdue_short" => ("Muddati o'tgan", "Просрочено"),

        // Ustunlar
        "col_block" => ("Blok", "Блок"),
        "col_floor" => ("Qavat", "Этаж"),
        "floor_short" => ("qavat", "этаж"),
        "col_position" => ("O'rni", "Позиция"),
        "col_rooms" => ("Xonalar", "Комнат"),
        "col_area" => ("Maydon, m²", "Площадь, м²"),
        "col_area_living" => ("Yashash", "Жилая"),
        "col_price_m2" => ("1 m² narxi", "Цена за м²"),
        "col_price_total" => ("Umumiy narx", "Полная цена"),
        "col_layout" => ("Planirovka", "Планировка"),
        "col_flat" => ("Kvartira", "Квартира"),
        "col_client" => ("Mijoz", "Клиент"),
        "col_client_doc" => ("Hujjat", "Документ"),
        "col_pay_kind" => ("To'lov turi", "Форма оплаты"),
        "col_deal_total" => ("Shartnoma summasi", "Сумма договора"),
        "col_paid" => ("To'langan", "Оплачено"),
        "col_discount" => ("Chegirma", "Скидка"),
        "col_prepayment" => ("Boshlang'ich to'lov", "Первый взнос"),
        "col_months" => ("Muddat, oy", "Срок, мес."),
        "col_manager" => ("Menejer", "Менеджер"),
        "col_due" => ("Muddat", "Срок"),
        "col_planned" => ("Reja", "План"),
        "col_fact" => ("Fakt", "Факт"),

        // Shartnoma kartochkasi
        "deal_card" => ("Shartnoma", "Договор"),
        "deal_total" => ("Shartnoma summasi", "Сумма договора"),
        "deal_paid" => ("To'langan", "Оплачено"),
        "deal_remaining" => ("Qoldiq qarz", "Остаток долга"),
        "deal_overdue" => ("Muddati o'tgan", "Просрочено"),
        "deal_next_due" => ("Keyingi to'lov", "Следующий платеж"),
        "deal_schedule" => ("Grafik (qator · reja)", "График (строк · план)"),
        "delete_deal" => ("Shartnomani o'chirish", "Удалить договор"),
        "payment_plan" => ("To'lov grafigi", "График платежей"),
        "rebuild_schedule" => ("Grafikni qayta qurish", "Пересобрать график"),
        "rebuild_schedule_hint" => (
            "Eski grafik o'chiriladi va shartnoma shartlaridan yangisi quriladi: boshlang'ich to'lov + teng oylik ulushlar.",
            "Старый график удаляется, новый строится из условий договора: первый взнос + равные ежемесячные доли.",
        ),
        "add_payment" => ("+ Qator", "+ Строка"),
        "schedule_empty" => (
            "Grafik bo'sh — «Grafikni qayta qurish» tugmasini bosing",
            "График пуст — нажмите «Пересобрать график»",
        ),
        "schedule_rebuilt" => ("To'lov grafigi qayta qurildi", "График платежей пересобран"),
        "schedule_mismatch" => (
            "Grafik summasi shartnoma summasiga to'g'ri kelmaydi.",
            "Сумма графика не совпадает с суммой договора.",
        ),
        "mark_paid" => ("To'liq to'langan deb belgilash", "Отметить полностью оплаченным"),
        "total_row" => ("Jami", "Итого"),
        "deals_empty" => (
            "Shartnoma yo'q — shaxmatkadan kvartira tanlab band qiling",
            "Договоров нет — выберите квартиру в шахматке и оформите бронь",
        ),
        "deals_no_units" => ("Avval kvartiralarni yarating", "Сначала создайте квартиры"),
        "deals_no_units_hint" => (
            "Shartnoma kvartiraga bog'lanadi — «XIX Sotuv» bo'limiga o'ting.",
            "Договор привязывается к квартире — перейдите в раздел «XIX Продажи».",
        ),

        // Ko'rsatkichlar
        "kpi_units" => ("Birliklar", "Единиц"),
        "kpi_sales_value" => ("Katalog qiymati", "Стоимость по каталогу"),
        "kpi_contracted" => ("Shartnomalar summasi", "Сумма договоров"),
        "kpi_deals_hint" => ("ta shartnoma", "договоров"),
        "kpi_received" => ("Tushgan pul", "Поступило"),
        "kpi_received_hint" => ("shartnomalardan", "от договоров"),
        "kpi_received_hint2" => ("haqiqiy to'lovlar", "фактические платежи"),
        "kpi_debt" => ("Qoldiq qarz", "Остаток долга"),
        "kpi_debt_hint" => ("hali to'lanmagan", "еще не оплачено"),
        "kpi_overdue_pay" => ("Muddati o'tgan", "Просрочено"),
        "kpi_overdue_pay_hint" => ("to'lov sanasi o'tdi", "срок платежа прошел"),
        "kpi_avg_m2" => ("O'rtacha 1 m²", "Средняя за м²"),

        // ---------- Oy nomlari / Названия месяцев ----------
        "mon_1" => ("yan", "янв"),
        "mon_2" => ("fev", "фев"),
        "mon_3" => ("mar", "мар"),
        "mon_4" => ("apr", "апр"),
        "mon_5" => ("may", "май"),
        "mon_6" => ("iyn", "июн"),
        "mon_7" => ("iyl", "июл"),
        "mon_8" => ("avg", "авг"),
        "mon_9" => ("sen", "сен"),
        "mon_10" => ("okt", "окт"),
        "mon_11" => ("noy", "ноя"),
        "mon_12" => ("dek", "дек"),

        _ => return None,
    };
    Some(pair)
}

/// «… gacha» / «до …» — ko'makchi o'zbekchada sanadan keyin, ruschada oldin keladi.
pub fn until(date: &str) -> String {
    match lang() {
        Lang::Uz => format!("{date} gacha"),
        Lang::Ru => format!("до {date}"),
    }
}

/// Qisqartirilgan oy nomi.
pub fn month(m: u32) -> &'static str {
    match m {
        1 => t("mon_1"),
        2 => t("mon_2"),
        3 => t("mon_3"),
        4 => t("mon_4"),
        5 => t("mon_5"),
        6 => t("mon_6"),
        7 => t("mon_7"),
        8 => t("mon_8"),
        9 => t("mon_9"),
        10 => t("mon_10"),
        11 => t("mon_11"),
        _ => t("mon_12"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Kodda ishlatilgan har bir kalit tarjima ro'yxatida bo'lishi kerak.
    ///
    /// Bu sinov `?` belgisining interfeysga chiqib ketishini oldini oladi:
    /// yangi ekran qo'shilganda tarjima unutilsa, yig'ish emas — sinov yiqiladi.
    #[test]
    fn every_key_used_in_the_code_has_a_translation() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        collect(&root, &mut files);
        assert!(files.len() > 10, "manba fayllar topilmadi");

        let mut missing: Vec<String> = Vec::new();
        for f in &files {
            // Tarjimalar fayli o'zi tekshirilmaydi: izohlarda namuna sifatida
            // yozilgan `t("...")` haqiqiy kalit emas.
            if f.file_name().is_some_and(|x| x == "i18n.rs") {
                continue;
            }
            let text = std::fs::read_to_string(f).unwrap_or_default();
            for key in keys_in(&text) {
                if lookup(&key).is_none() && !missing.contains(&key) {
                    missing.push(format!(
                        "{}: {key}",
                        f.file_name().unwrap_or_default().to_string_lossy()
                    ));
                }
            }
        }
        assert!(missing.is_empty(), "tarjimasi yo'q kalitlar: {missing:?}");
    }

    /// Bir kalit ikki marta yozilmasin — ikkinchisi hech qachon ishlamaydi.
    #[test]
    fn keys_are_not_duplicated() {
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/i18n.rs"),
        )
        .expect("i18n.rs");
        let mut seen: Vec<&str> = Vec::new();
        let mut dupes: Vec<&str> = Vec::new();
        for line in text.lines() {
            let line = line.trim_start();
            let Some(rest) = line.strip_prefix('"') else {
                continue;
            };
            let Some(end) = rest.find('"') else { continue };
            let key = &rest[..end];
            if !rest[end + 1..].trim_start().starts_with("=>") {
                continue;
            }
            if seen.contains(&key) {
                dupes.push(key);
            } else {
                seen.push(key);
            }
        }
        assert!(dupes.is_empty(), "takrorlangan kalitlar: {dupes:?}");
        assert!(seen.len() > 500, "kalitlar juda kam: {}", seen.len());
    }

    /// `.rs` fayllarni yig'adi.
    fn collect(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                collect(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }

    /// Matndan `t("...")` ko'rinishidagi kalitlarni ajratadi.
    ///
    /// Faqat to'g'ridan-to'g'ri yozilgan satrlar olinadi: o'zgaruvchi orqali
    /// uzatilgan kalitni statik tekshirib bo'lmaydi.
    fn keys_in(text: &str) -> Vec<String> {
        let mut out = Vec::new();
        let b = text.as_bytes();
        let mut i = 0;
        while let Some(pos) = text[i..].find("t(\"") {
            let start = i + pos;
            // Oldingi belgi harf yoki `_` bo'lsa — bu boshqa funksiya
            // (masalan `format!`, `insert`), `t(` emas.
            let prev = if start == 0 { b' ' } else { b[start - 1] };
            i = start + 3;
            if prev.is_ascii_alphanumeric() || prev == b'_' {
                continue;
            }
            let Some(end) = text[i..].find('"') else {
                break;
            };
            let key = &text[i..i + end];
            // Yopuvchi qavs darrov kelmasa — bu `t("x")` emas.
            if text[i + end..].starts_with("\")") && !key.is_empty() {
                out.push(key.to_string());
            }
            i += end + 1;
        }
        out
    }
}

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
        "col_phone" => ("Telefon", "Телефон"),
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
            "Bu yig'ilishda tarmoq qismi yo'q: ilova `llm` xususiyatisiz yig'ilgan, shuning uchun ulanish umuman mumkin emas.",
            "В этой сборке нет сетевой части: программа собрана без функции `llm`, поэтому подключение невозможно в принципе.",
        ),
        "set_llm_enabled" => ("Yoqilgan", "Включено"),
        "set_llm_endpoint" => ("Xizmat manzili", "Адрес сервиса"),
        "set_llm_model" => ("Model nomi", "Название модели"),
        "set_llm_key" => ("API kaliti", "API-ключ"),
        "set_llm_ready" => (
            "Yoqilgan: yordamchida «Modeldan so'rash» tugmasi chiqadi.",
            "Включено: в помощнике появится кнопка «Спросить модель».",
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

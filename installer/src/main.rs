//! QURAi o'rnatuvchisi.
//!
//! Bitta fayl: ichida dastur, server va hujjatlar turadi. Ishga tushirilganda
//! ular foydalanuvchining o'z papkasiga yoziladi, yorliq qo'yiladi va
//! «Dasturlar va imkoniyatlar» ro'yxatiga qo'shiladi.
//!
//! **Administrator huquqi so'ralmaydi.** Dastur `Program Files` ga emas,
//! `%LOCALAPPDATA%\Programs\QURAi` ga tushadi — shuning uchun o'rnatish
//! korxona kompyuterida ham, ruxsatsiz hisobda ham ishlaydi.
//!
//! Ma'lumot bazasi dastur yonidagi `data` papkasida bo'ladi va
//! o'chirishda **so'ralmasdan o'chirilmaydi**: kod qaytadi, ma'lumot
//! qaytmaydi.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Dastur fayllari shu yerga joylashtiriladi.
///
/// `include_bytes!` yig'ilish paytida tayyor fayllarni talab qiladi,
/// shuning uchun o'rnatuvchi ish maydonidan tashqarida va **ikkinchi
/// qadamda** yig'iladi.
const APP: &[u8] = include_bytes!("../../target/release/qurai.exe");
const SERVER: &[u8] = include_bytes!("../../target/release/qurai-server.exe");
const READ_ME: &[u8] = include_bytes!("../OQING.txt");
const CHECKLIST: &[u8] = include_bytes!("../TEKSHIRUV.txt");

const NAME: &str = "QURAi";
const VERSION: &str = "1.1.0";

fn main() {
    let code = run();
    // Oyna yopilib ketmasin: odam natijani o'qishi kerak.
    println!();
    println!("  Davom etish uchun Enter ni bosing...");
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);
    std::process::exit(code);
}

fn run() -> i32 {
    println!();
    println!("  {NAME} {VERSION} - Qurilish intellektual platformasi");
    println!("  ------------------------------------------------");

    let target = match target_dir() {
        Some(t) => t,
        None => {
            say_error("papka aniqlanmadi: LOCALAPPDATA yo'q");
            return 1;
        }
    };
    println!("  Joy: {}", target.display());
    println!();

    // Ochiq dastur fayl almashishiga yo'l qo'ymaydi.
    stop_running();

    if let Err(e) = std::fs::create_dir_all(&target) {
        say_error(&format!("papka yaratilmadi: {e}"));
        return 1;
    }

    let files: [(&str, &[u8]); 4] = [
        ("QURAi.exe", APP),
        ("qurai-server.exe", SERVER),
        ("OQING.txt", READ_ME),
        ("TEKSHIRUV.txt", CHECKLIST),
    ];
    for (name, bytes) in files {
        print!("  {name} ... ");
        let _ = std::io::stdout().flush();
        if let Err(e) = write_file(&target.join(name), bytes) {
            println!("xato");
            say_error(&format!("{name}: {e}"));
            return 1;
        }
        println!("tayyor");
    }

    if let Err(e) = write_file(&target.join("ochirish.cmd"), uninstall_script().as_bytes()) {
        say_error(&format!("ochirish.cmd: {e}"));
        return 1;
    }

    let exe = target.join("QURAi.exe");
    shortcuts(&exe, &target);
    register(&target, &exe);

    println!();
    println!("  O'rnatildi.");
    println!();
    println!("  Dastur:  {}", exe.display());
    println!("  Yorliq:  ish stolida va \"Boshlash\" menyusida");
    println!(
        "  Baza:    {}",
        target.join("data").join("qurai.db").display()
    );
    println!("  O'chirish: {}", target.join("ochirish.cmd").display());
    println!();
    println!("  Birinchi ochilishda namuna obyekti yaratiladi - hamma");
    println!("  bo'limni darhol ko'rib chiqsangiz bo'ladi.");
    println!();
    println!("  Nimani tekshirish kerakligi: TEKSHIRUV.txt");

    // Dastur o'zi ochiladi: odam natijani darhol ko'rsin.
    let _ = Command::new(&exe).current_dir(&target).spawn();
    0
}

/// `%LOCALAPPDATA%\Programs\QURAi`, yoki buyruqda ko'rsatilgan papka.
fn target_dir() -> Option<PathBuf> {
    if let Some(arg) = std::env::args().nth(1) {
        return Some(PathBuf::from(arg));
    }
    let base = std::env::var_os("LOCALAPPDATA")?;
    Some(PathBuf::from(base).join("Programs").join(NAME))
}

/// Faylni yozadi. Band bo'lsa - eski nom bilan chetga suriladi.
///
/// Windows ishlab turgan dastur faylini almashtirmaydi, lekin **qayta
/// nomlashga** ruxsat beradi. Shuning uchun yangilash ochiq dasturda ham
/// ishlaydi va eski nusxa keyingi o'rnatishda tozalanadi.
fn write_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if path.exists() {
        let old = path.with_extension("eski");
        let _ = std::fs::remove_file(&old);
        if std::fs::write(path, bytes).is_ok() {
            return Ok(());
        }
        std::fs::rename(path, &old)?;
    }
    std::fs::write(path, bytes)
}

/// Ochiq QURAi ni yopadi.
fn stop_running() {
    let out = Command::new("taskkill")
        .args(["/f", "/im", "QURAi.exe"])
        .output();
    if let Ok(o) = out {
        if o.status.success() {
            println!("  Ochiq dastur yopildi.");
            std::thread::sleep(std::time::Duration::from_millis(1500));
        }
    }
}

/// Ish stoli va «Boshlash» menyusida yorliq.
///
/// Yorliq COM obyekti orqali yaratiladi; uni chaqirishning eng qisqa yo'li
/// PowerShell orqali o'tadi. Bajarilmasa o'rnatish to'xtamaydi: dastur
/// baribir o'z papkasidan ochiladi va buni aytamiz.
fn shortcuts(exe: &Path, dir: &Path) {
    let script = format!(
        "$s=New-Object -ComObject WScript.Shell; \
         foreach($p in @(\"$env:USERPROFILE\\Desktop\\{NAME}.lnk\", \
         \"$env:APPDATA\\Microsoft\\Windows\\Start Menu\\Programs\\{NAME}.lnk\")){{ \
         $l=$s.CreateShortcut($p); $l.TargetPath='{exe}'; $l.WorkingDirectory='{dir}'; \
         $l.Description='{NAME} - qurilish boshqaruvi'; $l.Save() }}",
        exe = exe.display(),
        dir = dir.display(),
    );
    let ok = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        println!("  Eslatma: yorliq qo'yilmadi - dasturni o'z papkasidan oching.");
    }
}

/// «Dasturlar va imkoniyatlar» ro'yxatiga qo'shadi.
///
/// Faqat `HKCU` ga yoziladi - administrator huquqi kerak emas.
fn register(dir: &Path, exe: &Path) {
    let key = format!("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{NAME}");
    let values: [(&str, &str); 6] = [
        ("DisplayName", NAME),
        ("DisplayVersion", VERSION),
        ("Publisher", NAME),
        ("InstallLocation", &dir.display().to_string()),
        ("DisplayIcon", &exe.display().to_string()),
        (
            "UninstallString",
            &dir.join("ochirish.cmd").display().to_string(),
        ),
    ];
    for (name, value) in values {
        let _ = Command::new("reg")
            .args(["add", &key, "/v", name, "/t", "REG_SZ", "/d", value, "/f"])
            .output();
    }
}

/// O'chirish buyrug'i.
///
/// Ma'lumot bazasi **so'ralmasdan o'chirilmaydi**: dastur qayta
/// o'rnatiladi, kiritilgan ma'lumot esa qaytmaydi.
fn uninstall_script() -> String {
    let body = format!(
        "@echo off\r\n\
chcp 65001 >nul 2>&1\r\n\
echo.\r\n\
echo   {NAME} o'chirilmoqda...\r\n\
echo.\r\n\
taskkill /f /im QURAi.exe >nul 2>&1\r\n\
ping -n 3 127.0.0.1 >nul 2>&1\r\n\
del /q \"%~dp0QURAi.exe\" 2>nul\r\n\
del /q \"%~dp0QURAi.eski\" 2>nul\r\n\
del /q \"%~dp0qurai-server.exe\" 2>nul\r\n\
del /q \"%~dp0OQING.txt\" 2>nul\r\n\
del /q \"%~dp0TEKSHIRUV.txt\" 2>nul\r\n\
del /q \"%USERPROFILE%\\Desktop\\{NAME}.lnk\" 2>nul\r\n\
del /q \"%APPDATA%\\Microsoft\\Windows\\Start Menu\\Programs\\{NAME}.lnk\" 2>nul\r\n\
reg delete \"HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{NAME}\" /f >nul 2>&1\r\n\
echo   Dastur o'chirildi.\r\n\
echo.\r\n\
if exist \"%~dp0data\" (\r\n\
    echo   DIQQAT: ma'lumot bazasi joyida qoldi:\r\n\
    echo     %~dp0data\r\n\
    echo.\r\n\
    echo   Unda kiritilgan hamma narsa bor. Kerak bo'lmasa o'zingiz\r\n\
    echo   o'chiring - biz so'ramasdan o'chirmaymiz.\r\n\
    echo.\r\n\
)\r\n\
pause\r\n"
    );
    body
}

fn say_error(text: &str) {
    println!();
    println!("  XATO: {text}");
}

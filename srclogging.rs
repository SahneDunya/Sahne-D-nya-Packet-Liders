use log::{LevelFilter};
use std::env;

// Assuming 'Sahne64' modules are in the same crate
use crate::kernel;

// Define a mapping from log::LevelFilter to Sahne64 log levels (assuming u32)
fn level_filtre_to_sahne_level(level: LevelFilter) -> u32 {
    match level {
        LevelFilter::Trace => 0,
        LevelFilter::Debug => 1,
        LevelFilter::Info => 2,
        LevelFilter::Warn => 3,
        LevelFilter::Error => 4,
        LevelFilter::Off => 5, // Assuming Off is also a level
    }
}

pub fn baslat_gunlukleme() {
    let mut log_seviyesi_filtresi = LevelFilter::Info; // Varsayılan günlük seviyesi

    // Ortam değişkeninden günlük seviyesini oku
    match env::var("PAKET_YONETICISI_LOG") {
        Ok(log_seviyesi_str) => {
            kernel::log(1, &format!("PAKET_YONETICISI_LOG ortam değişkeni bulundu: {}", log_seviyesi_str)); // Ortam değişkeninin bulunduğunu debug seviyesinde logla
            match log_seviyesi_str.to_lowercase().as_str() {
                "trace" => {
                    log_seviyesi_filtresi = LevelFilter::Trace;
                    kernel::log(1, "Günlük seviyesi 'trace' olarak ayarlandı."); // Ayarlanan seviyeyi debug seviyesinde logla
                },
                "debug" => {
                    log_seviyesi_filtresi = LevelFilter::Debug;
                    kernel::log(1, "Günlük seviyesi 'debug' olarak ayarlandı."); // Ayarlanan seviyeyi debug seviyesinde logla
                },
                "info" => {
                    log_seviyesi_filtresi = LevelFilter::Info;
                    kernel::log(1, "Günlük seviyesi 'info' olarak ayarlandı."); // Ayarlanan seviyeyi debug seviyesinde logla
                },
                "warn" => {
                    log_seviyesi_filtresi = LevelFilter::Warn;
                    kernel::log(1, "Günlük seviyesi 'warn' olarak ayarlandı."); // Ayarlanan seviyeyi debug seviyesinde logla
                },
                "error" => {
                    log_seviyesi_filtresi = LevelFilter::Error;
                    kernel::log(1, "Günlük seviyesi 'error' olarak ayarlandı."); // Ayarlanan seviyeyi debug seviyesinde logla
                },
                gecersiz_seviye => {
                    kernel::log(3, &format!(
                        "Geçersiz günlük seviyesi belirtildi: '{}', varsayılan 'info' seviyesi kullanılıyor.", // Uyarı mesajı
                        gecersiz_seviye
                    ));
                }
            };
        }
        Err(_e) => {
            kernel::log(1, "PAKET_YONETICISI_LOG ortam değişkeni bulunamadı. Varsayılan 'info' seviyesi kullanılıyor."); // Ortam değişkeninin bulunamadığını debug seviyesinde logla
        }
    }

    kernel::log(2, "Günlükleme sistemi başlatıldı."); // Günlükleme sisteminin başladığını info seviyesinde logla

    // Örnek günlük mesajları (farklı seviyelerde)
    if log_seviyesi_filtresi <= LevelFilter::Trace {
        kernel::log(0, "İzleme seviyesi günlüğü (sadece 'trace' seviyesinde görünür)");
    }
    if log_seviyesi_filtresi <= LevelFilter::Debug {
        kernel::log(1, "Hata ayıklama seviyesi günlüğü (trace, debug ve daha düşük seviyelerde görünür)");
    }
    if log_seviyesi_filtresi <= LevelFilter::Info {
        kernel::log(2, "Bilgi seviyesi günlüğü (trace, debug, info ve daha düşük seviyelerde görünür)");
    }
    if log_seviyesi_filtresi <= LevelFilter::Warn {
        kernel::log(3, "Uyarı seviyesi günlüğü (trace, debug, info, warn ve daha düşük seviyelerde görünür)");
    }
    if log_seviyesi_filtresi <= LevelFilter::Error {
        kernel::log(4, "Hata seviyesi günlüğü (tüm seviyelerde görünür)");
    }
}
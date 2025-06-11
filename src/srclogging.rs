#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // String, format! için

use alloc::string::{String, ToString};
use alloc::format; // format! makrosu için
use alloc::borrow::ToOwned; // &str -> String için
use alloc::boxed::Box; // set_logger için Box<dyn Log> gerekebilir

// log crate'i ve gerekli traitler
use log::{self, Level, LevelFilter, Log, Metadata, Record};

// Sahne64 resource modülü (çıktı için)
use crate::resource;
use crate::SahneError; // Hata yönetimi için
use crate::Handle; // Kaynak Handle'ları

// Yapılandırma struct'ı (log seviyesini okumak için varsayım)
use crate::srcconfig::Yapilandirma; // Varsayım: Yapilandirma srcconfig.rs'de tanımlı

// no_std uyumlu print makroları (logger backend olarak kullanılabilir)
 use crate::print_macros::{println, eprintln}; // Logger doğrudan resource'a yazacak


// Sahne64 ortamına özel loglama backend implementasyonu.
// log crate'inin `Log` trait'ini implement eder.
struct Sahne64Logger {
    // Log çıktısının yazılacağı Sahne64 Kaynağının Handle'ı.
    // Logger statik olduğu için Handle'ın 'static lifetime'ı olmalı veya güvenli erişim sağlanmalı.
    // Statik Handle yönetimi no_std'de zordur. Geçici olarak Handle'ı field olarak tutmayalım,
    // Loglama sırasında konsol Kaynağını isimle bulmayı veya sabit bir Handle kullanmayı varsayalım.
    // Daha temiz bir çözüm, logger'ın başlangıçta console output handle'ını almasıdır.
    // Ancak set_logger &dyn Log ister.

    // Basitlik adına, logger'ın çıktı için standart bir Sahne64 Kaynağına (örn. "sahne://dev/console")
    // isimle eriştiğini varsayalım. Bu, Kaynak acquire/release overhead'i getirir.
    // Alternatif: Console output handle'ı global static mut olarak tutulur (unsafe gerektirir).

    // Logger struct'ının field'a ihtiyacı yoksa boş bırakabiliriz.
     output_resource_id: String, // Çıktı Kaynağı ID'si
}

impl Log for Sahne64Logger {
    // Log mesajının etkin olup olmadığını kontrol eder.
    fn enabled(&self, metadata: &Metadata) -> bool {
        // log::set_max_level tarafından kontrol edilir, genellikle burada ekstra filtreleme yapılmaz.
        metadata.level() <= log::max_level()
    }

    // Log mesajını formatlar ve çıktı Kaynağına yazar.
    fn log(&self, record: &Record) {
        // Sadece etkin olan mesajları işle
        if self.enabled(record.metadata()) {
            // Log mesajını formatla (timestamp, seviye, modül yolu, mesaj)
            // format! makrosu alloc gerektirir.
            let formatted_message = format!(
                "[{}] {}: {}",
                record.level(), // Level enum'ı Display implement eder
                record.module_path().unwrap_or(""), // Modül yolu (opsiyonel)
                record.args() // Format argümanları
            );

            // Sahne64 çıktı Kaynağını acquire et (örn. "sahne://dev/console")
            // Her log mesajı için acquire/release overhead'i yüksektir.
            // Daha iyi bir backend, handle'ı cache'ler.
            let console_output_resource_id = "sahne://dev/console"; // Varsayımsal konsol Kaynak ID'si
            match resource::acquire(console_output_resource_id, resource::MODE_WRITE) { // Yazma izni
                Ok(handle) => {
                    // Formatlanmış mesajı Kaynağa yaz (byte dilimi olarak)
                    let write_result = resource::write(handle, formatted_message.as_bytes()); // String -> &[u8]

                    // Handle'ı serbest bırak
                    let release_result = resource::release(handle);

                    // Hata durumunda loglama yapamayız (zaten log fonksiyonundayız), hata akışını yönetmek zor.
                    // Basitçe hataları görmezden gelebiliriz veya çekirdeğe doğrudan bir debug çıktısı göndermeye çalışabiliriz.
                    if let Err(_e) = write_result {
                        // Log yazma hatası (örn. Kaynak kapalı, izin yok).
                         eprintln!("Logger yazma hatası: {:?}", e); // Bu recursive çağrıya neden olabilir!
                        // Güvenli bir şekilde çekirdeğe temel bir hata bildirme syscall'u olabilir.
                    }
                     if let Err(_e) = release_result {
                         // Handle release hatası.
                     }
                }
                Err(_e) => {
                    // Kaynak acquire hatası (örn. Kaynak bulunamadı).
                     eprintln!("Logger Kaynak acquire hatası: {:?}", e); // Recursive olabilir!
                }
            }
        }
    }

    // Log mesajını temizler (genellikle bir şey yapmaz)
    fn flush(&self) {
        // resource::write muhtemelen veriyi hemen gönderir veya buffer'lar.
        // Eğer Kaynak buffering yapıyorsa ve flush komutu destekliyorsa, resource::control ile flush yapılabilir.
        // Şimdilik bir şey yapmaya gerek yok.
    }
}


// Loglama sistemini başlatır.
// config: Yapılandırma struct'ına referans (log seviyesini okumak için).
// Dönüş değeri: Başarı veya hata (Logger kurulumu hata verebilir).
pub fn baslat_gunlukleme(config: &Yapilandirma) -> Result<(), Box<dyn log::SetLoggerError>> { // log::SetLoggerError standart bir hata türüdür, Box içinde dönebilir
    // Log seviyesini yapılandırmadan oku.
    // Yapilandirma struct'ında log_level: Option<String> alanı olduğunu varsayalım.
    let log_seviyesi_str = config.log_level.as_deref().unwrap_or("info"); // Default "info"

    // String'i LevelFilter'a dönüştür.
    let log_seviyesi_filtresi = log_seviyesi_str.parse::<LevelFilter>()
        .unwrap_or_else(|_| {
            // Ayrıştırma hatası durumunda uyarı logla (ama logger henüz tam kurulmamış olabilir!)
            // veya sadece varsayılan değeri kullan.
            // Logger kurulmadan loglamak zor. Sadece varsayılanı kullanalım ve kurulumdan sonra uyarı loglayalım.
            LevelFilter::Info
        });

    // Logger implementasyonunu ve maksimum log seviyesini ayarla.
    // Sahne64Logger'ın 'static olması gerekir. Bir static mut kullanmak veya Box'ı leak etmek gerekebilir.
    // Statik mut kullanımı unsafe gerektirir ve dikkatli senkronizasyon ister.
    // Box::leak en basit no_std çözümüdür ama bellek sızıntısına neden olur (tek seferlik kurulum için kabul edilebilir).
    static LOGGER: Sahne64Logger = Sahne64Logger { /* fields if any */ };

    // set_logger sadece bir kez çağrılmalıdır.
    log::set_logger(&LOGGER)?; // LOGGER'ın static reference'ı alınır

    // Maksimum log seviyesini ayarla
    log::set_max_level(log_seviyesi_filtresi);

    // Logger kurulduktan sonra uyarı loglayabiliriz.
     if log_seviyesi_str.parse::<LevelFilter>().is_err() {
         warn!("Geçersiz günlük seviyesi belirtildi: '{}', varsayılan '{}' seviyesi kullanılıyor.",
               log_seviyesi_str,
               log_seviyesi_filtresi);
     }


    info!("Günlükleme sistemi başlatıldı. Seviye: {}", log_seviyesi_filtresi); // no_std log

    Ok(()) // Başarı
}


// --- Yapilandirma struct tanımının güncellenmesi (srcconfig.rs) ---
// Log seviyesi için alan eklenmeli.

// srcconfig.rs (Güncellenmiş - log_level alanı eklenmiş)
#![no_std]
extern crate alloc;

// ... diğer importlar ...

use serde::{Deserialize, Serialize};

// ... diğer struct'lar ...

#[derive(Serialize, Deserialize, Debug)]
pub struct Yapilandirma {
    // ... diğer alanlar ...

    // Loglama seviyesi (örn. "info", "debug", "trace")
    pub log_level: Option<String>, // Option<String> olarak saklamak ayrıştırmayı başlatma sırasında yapar.

    // ... diğer özellik bayrakları ...
}

// ... impl Yapilandirma { ... }

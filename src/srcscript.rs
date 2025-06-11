#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // String, Vec, format! için

use alloc::string::{String, ToString}; // std::string::String yerine
use alloc::vec::Vec; // std::vec::Vec yerine
use alloc::format; // format! makrosu için

// Özel hata enum'ımızı içe aktar (no_std uyumlu hali)
use crate::paket_yoneticisi_hatasi::PaketYoneticisiHatasi;
// SahneError'dan dönüşüm From implementasyonu ile sağlanacak

// log kütüphanesini içe aktar (no_std uyumlu backend varsayımıyla)
use log::{info, error};

// Sahne64 API modülleri
use crate::task; // Görev yönetimi (spawn, exit)
use crate::resource; // Betik Kaynağını acquire etmek için
use crate::SahneError; // Sahne64 hata türü
use crate::Handle; // Kaynak Handle'ları

// Verilen betik Kaynağını (executable code resource) yeni bir Sahne64 görevi olarak çalıştırır.
// Not: Sahne64 API'sında task::wait ve task çıktısını (stdout/stderr) yakalama mekanizmaları
// henüz tanımlanmamıştır. Bu nedenle bu fonksiyon sadece betik görevini başlatır,
// tamamlanmasını beklemez, çıkış kodunu veya çıktısını alamaz.
// betik_kaynagi_id: Çalıştırılacak betik veya yürütülebilir dosya Kaynağının ID'si (örn. "sahne://system/package_scripts/my_package/install.sh").
// args: Betik görevine geçilecek argümanlar (byte dilimi olarak).
// Dönüş değeri: Başarı (görevin başlatılması) veya PaketYoneticisiHatasi.
pub fn betik_calistir(betik_kaynagi_id: &str, args: &[u8]) -> Result<(), PaketYoneticisiHatasi> { // betik_yolu -> betik_kaynagi_id: &str, args eklendi
    info!("Betik çalıştırılıyor (task başlatılıyor): {}", betik_kaynagi_id); // no_std log

    // Betik Kaynağını acquire et.
    // Yürütülebilir kaynaklar genellikle MODE_READ izniyle acquire edilir.
    match resource::acquire(betik_kaynagi_id, resource::MODE_READ) { // Kaynak ID kullanılıyor
        Ok(script_handle) => {
            // Yeni bir görev (task) olarak betiği başlat.
            // task::spawn function signature: spawn(code_handle: Handle, args: &[u8]) -> Result<TaskId, SahneError>
            match task::spawn(script_handle, args) { // args pass ediliyor
                Ok(new_tid) => {
                    info!("Betik görevi başlatıldı, TaskId: {:?}", new_tid); // no_std log
                    // Betik görevi başlatıldıktan sonra Handle'ı serbest bırakabiliriz.
                    // Betik kendi Kaynaklarına kendisi erişmelidir.
                    let _ = resource::release(script_handle); // Handle'ı bırak

                    // !!! Sahne64 API eksikliği: task::wait(new_tid) ve task çıktısını yakalama burada yapılamaz.
                    // Betiğin başarıyla tamamlandığını veya hata verdiğini bilemeyiz.
                    // Gerçek bir paket yöneticisi için bu kritik bir eksikliktir.

                    println!("UYARI: Betik görevinin tamamlanması beklenmiyor ve çıktısı yakalanmıyor. Bu fonksiyon sadece görevi başlatır."); // no_std print

                    Ok(()) // Görev başlatıldı olarak başarı dön
                }
                Err(e) => {
                    // Görev başlatılamadı hatası.
                    let _ = resource::release(script_handle); // Hata durumunda handle'ı temizle
                    let hata_mesaji = format!( // format! alloc
                        "Betik görevi başlatılamadı (Kaynak: {}): {:?}",
                        betik_kaynagi_id, e
                    );
                    error!("{}", hata_mesaji); // no_std log
                    Err(PaketYoneticisiHatasi::BetikCalistirmaHatasi(hata_mesaji)) // SahneError -> PaketYoneticisiHatasi
                }
            }
        }
        Err(e) => {
            // Betik Kaynağı acquire hatası (örn. bulunamadı, izin yok).
            let hata_mesaji = format!( // format! alloc
                "Betik Kaynağı acquire hatası (Kaynak: {}): {:?}",
                betik_kaynagi_id, e
            );
            error!("{}", hata_mesaji); // no_std log
            Err(PaketYoneticisiHatasi::BetikCalistirmaHatasi(hata_mesaji)) // SahneError -> PaketYoneticisiHatasi
        }
    }
}

// #[cfg(test)] bloğu std test runner'ı ve Sahne64 task/resource mock'ları gerektirir.
// Bu blok no_std ortamında derlenmeyecektir eğer std feature aktif değilse ve test ortamı yoksa.

#[cfg(test)]
mod tests {
    // std::io, std::process kullandığı için no_std'de doğrudan çalışmaz.
    // Mock task::spawn, resource::acquire/release ve çıktı yakalama/kontrol mekanizması gerektirir.
}

// --- PaketYoneticisiHatasi enum tanımının no_std uyumlu hale getirilmesi ---
// (paket_yoneticisi_hata.rs dosyasında veya ilgili modülde olmalı)
// BetikCalistirmaHatasi varyantı eklenmeli.
// SahneError'dan PaketYoneticisiHatasi'na dönüşüm için From implementasyonu gereklidir.

// paket_yoneticisi_hata.rs (Örnek - no_std uyumlu, Betik çalıştırma hatası eklenmiş)
#![no_std]
extern crate alloc;

use alloc::string::String;
// ... diğer importlar ...

#[derive(Debug)]
pub enum PaketYoneticisiHatasi {
    // ... diğer hatalar ...

    // Betik çalıştırma sırasında oluşan hatalar
    BetikCalistirmaHatasi(String), // Hata detayını string olarak tutmak alloc gerektirir. Altında yatan SahneError detayını içerebilir.

    // ... diğer hatalar ...
}

// SahneError'dan PaketYoneticisiHatasi'na dönüşüm
 impl From<SahneError> for PaketYoneticisiHatasi { ... } // Bu implementasyon başka bir yerde olmalı ve genel olmalı.

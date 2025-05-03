#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // String, format! için

use alloc::string::{String, ToString};
use alloc::format; // format! makrosu için

// Sahne64 API modülleri
use crate::resource; // Ağ ve dosya sistemi benzeri işlemler için
use crate::SahneError; // Sahne64 hata türü
use crate::Handle; // Kaynak Handle'ları

// Özel hata enum'ımızı içe aktar (no_std uyumlu hali)
use crate::paket_yoneticisi_hata::PaketYoneticisiHatasi;
// SahneError'dan dönüşüm From implementasyonu ile sağlanacak

// no_std uyumlu print makroları
use crate::print_macros::{println, eprintln};


// Sahne64'e özgü ağ (network) işlemleri için Kaynak tabanlı arayüz kullanılıyor.
// Bir URL, belirli bir şemaya sahip bir Kaynak ID'si olarak kabul edilir
// (örn. "http://example.com/file.zip").
// resource::acquire(url, MODE_READ) isteği başlatır, resource::read cevabı okur.

// URL'den hedef Kaynağa dosya indirir.
// url: İndirilecek dosyanın Kaynak ID'si (URL formatında olabilir).
// hedef_kaynak_id: Dosyanın kaydedileceği yerel Kaynağın ID'si.
// Dönüş değeri: Başarı veya PaketYoneticisiHatasi.
pub fn dosya_indir(url: &str, hedef_kaynak_id: &str) -> Result<(), PaketYoneticisiHatasi> { // Result türü PaketYoneticisiHatasi olmalı
    println!("Dosya indirme başlatılıyor: {} -> {}", url, hedef_kaynak_id); // no_std print

    // Uzak (ağ) Kaynağı okuma izniyle acquire et. Bu isteği başlatır.
    let kaynak_handle = resource::acquire(url, resource::MODE_READ) // URL Kaynak ID'si olarak kullanılıyor
        .map_err(|e| {
             eprintln!("Uzak Kaynak acquire hatası ({}): {:?}", url, e); // no_std print
            PaketYoneticisiHatasi::from(e) // SahneError -> PaketYoneticisiHatasi (Ağ hatası olabilir)
        })?;

    // Hedef (yerel) Kaynağı yazma izniyle aç/oluştur/sil.
    // MODE_CREATE varsa Kaynak yoksa oluşturur, MODE_TRUNCATE varsa içeriği siler.
    // hedef_kaynak_id'nin parent resource'larının oluşturulması gerekebilir.
    // srcconfig.rs/srcinstaller.rs'deki resource oluşturma mantığına bakılabilir.
    let hedef_handle = resource::acquire(
        hedef_kaynak_id,
        resource::MODE_WRITE | resource::MODE_CREATE | resource::MODE_TRUNCATE
    ).map_err(|e| {
         eprintln!("Hedef Kaynak acquire hatası ({}): {:?}", hedef_kaynak_id, e); // no_std print
        PaketYoneticisiHatasi::from(e) // SahneError -> PaketYoneticisiHatasi
    })?;

    // Kaynaktan oku ve hedefe yaz döngüsü
    let mut buffer = [0u8; 4096]; // Okuma/yazma buffer'ı (stack'te)
    loop {
        // Kaynak Kaynağından oku (ağ üzerinden)
        match resource::read(kaynak_handle, &mut buffer) {
            Ok(0) => break, // Kaynak sonu (indirme tamamlandı)
            Ok(bytes_read) => {
                // Hedef Kaynağa yaz (yerel depolama)
                match resource::write(hedef_handle, &buffer[..bytes_read]) {
                    Ok(_) => {
                        // Başarılı yazma
                    }
                    Err(e) => {
                        eprintln!("Hedef Kaynak yazma hatası ({}): {:?}", hedef_kaynak_id, e); // no_std print
                        let _ = resource::release(kaynak_handle); // Handle'ları temizle
                        let _ = resource::release(hedef_handle);
                        return Err(PaketYoneticisiHatasi::from(e)); // SahneError -> PaketYoneticisiHatasi
                    }
                }
            }
            Err(e) => {
                eprintln!("Uzak Kaynak okuma hatası ({}): {:?}", url, e); // no_std print
                let _ = resource::release(kaynak_handle); // Handle'ları temizle
                let _ = resource::release(hedef_handle);
                return Err(PaketYoneticisiHatasi::from(e)); // SahneError -> PaketYoneticisiHatasi
            }
        }
    }

    // Handle'ları serbest bırak
    let release_kaynak_result = resource::release(kaynak_handle);
     if let Err(e) = release_kaynak_result { eprintln!("Uzak Kaynak release hatası ({}): {:?}", url, e); } // Logla

    let release_hedef_result = resource::release(hedef_handle);
     if let Err(e) = release_hedef_result { eprintln!("Hedef Kaynak release hatası ({}): {:?}", hedef_kaynak_id, e); } // Logla


    println!("Dosya başarıyla indirildi: {}", hedef_kaynak_id); // no_std print
    Ok(())
}

// URL'den hedef Kaynağa dosyayı indirir ve ilerleme raporlar (Placeholder).
// Sahne64 API'sında indirme ilerlemesini almak için özel bir mekanizma gereklidir
// (örn. resource::control komutu veya read syscall'undan dönen özel durumlar).
pub fn dosya_indir_ilerleme(url: &str, hedef_kaynak_id: &str) -> Result<(), PaketYoneticisiHatasi> { // Result türü PaketYoneticisiHatasi olmalı
    println!("İlerlemeli dosya indirme başlatılıyor (ilerleme raporlama implemente edilmedi): {} -> {}", url, hedef_kaynak_id); // no_std print

    // Temel indirme mantığı 'dosya_indir' fonksiyonu ile aynı olacaktır.
    // Fark, okuma döngüsü sırasında toplam boyutu (eğer biliniyorsa) ve
    // o ana kadar okunan bayt sayısını kullanarak ilerlemeyi hesaplamak ve raporlamaktır.
    // Raporlama, konsola yazdırma, loglama veya başka bir göreve mesaj gönderme şeklinde olabilir.

    // Toplam boyutu almak için:
    // resource::control(kaynak_handle, RESOURCE_CONTROL_CMD_GET_SIZE, &[]) -> Result<Vec<u8>, SahneError> gibi bir API gerekebilir.
     let mut total_size = None;
     if let Ok(size_bytes) = resource::control(kaynak_handle, RESOURCE_CONTROL_CMD_GET_SIZE, &[]) {
         if size_bytes.len() >= 8 { // Varsayım: boyut u64 olarak dönüyor
             total_size = Some(u64::from_le_bytes(size_bytes[..8].try_into().unwrap())); // try_into alloc gerektirebilir? Veya manuel okuma.
         }
     }

    // Okuma döngüsü içinde:
     let mut downloaded_bytes = 0;
     loop {
         match resource::read(...) {
             Ok(bytes_read) => {
                 downloaded_bytes += bytes_read as u64;
                 if let Some(total) = total_size {
                     let progress_percent = (downloaded_bytes as f64 / total as f64) * 100.0;
    //                 // İlerlemeyi raporla (log, print, messaging)
                      println!("İndiriliyor: {:.2}%", progress_percent);
                 } else {
                      println!("İndiriliyor: {} bayt...", downloaded_bytes);
                 }
    //             // ... yazma ...
             }
    //         // ... hatalar ve bitiş ...
         }
     }

    // Şu anki API eksikliği nedeniyle, sadece temel indirme fonksiyonunu çağırıyoruz.
    dosya_indir(url, hedef_kaynak_id) // Temel indirme işlevini kullan
}

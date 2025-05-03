#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // Bellek ayırma için alloc crate'i

use alloc::string::{String, ToString};
use alloc::format; // format! makrosu için

use md5::{Md5, Digest}; // md5 crate'i (alloc özellikli no_std uyumlu olduğunu varsayıyoruz)

// Sahne64 API modüllerini içe aktarın
use crate::resource;
use crate::SahneError;
use crate::Handle;

// Özel hata enum'ımızı içe aktar (no_std uyumlu ve SahneError'ı içeren haliyle)
use crate::paket_yoneticisi_hata::PaketYoneticisiHatasi;
// SahneError'dan dönüşüm helper'ı veya From implementasyonu
use crate::paket_yoneticisi_hata::from_sahne_error; // Eğer özel helper varsa
// Veya doğrudan From<SahneError> for PaketYoneticisiHatasi implementasyonunu kullanacağız.


// -- Helper fonksiyon: SahneError'ı ChecksumResourceError'a çevir --
// Bu hata türü muhtemelen PaketYoneticisiHatasi içinde tanımlanmalı.
// Örnek: #[derive(Debug)] pub enum PaketYoneticisiHatasi { ChecksumResourceError(SahneError), ... }
fn map_sahne_error_to_checksum_resource_error(err: SahneError) -> PaketYoneticisiHatasi {
    // PaketYoneticisiHatasi::from(err) çağrısı SahneApiHatasi(err) dönecek.
    // Checksum'a özgü bir varyant istiyorsak PaketYoneticisiHatasi enum'ını güncelleyip
    // burada o varyantı kullanmalıyız.
    // Geçici olarak genel SahneApiHatasi'nı kullanalım.
    PaketYoneticisiHatasi::from(err)
}


// Verilen Kaynak ID'sinin MD5 özetini hesaplar.
// resource_id: MD5 özeti hesaplanacak Kaynağın Sahne64 Kaynak ID'si (örn. "sahne://installed_packages/my_package/file.bin")
pub fn hesapla_md5(resource_id: &str) -> Result<String, PaketYoneticisiHatasi> {
    // Kaynağı oku (sadece okuma izniyle)
    let handle = resource::acquire(resource_id, resource::MODE_READ)
        .map_err(|e| {
             eprintln!("MD5 hesaplama: Kaynak acquire hatası ({}): {:?}", resource_id, e);
             map_sahne_error_to_checksum_resource_error(e)
        })?; // SahneError'ı paket yöneticisi hatasına çevir

    let mut md5 = Md5::new(); // Yeni bir MD5 hesaplayıcısı oluştur (alloc gerektirir)
    let mut buffer = [0u8; 4096]; // Okuma için bir buffer oluştur (stack'te)

    loop {
        // Kaynaktan veri oku
        match resource::read(handle, &mut buffer) {
            Ok(0) => break, // Kaynağın sonuna gelindi
            Ok(bytes_read) => {
                // Okunan veriyi MD5 hesaplayıcısına ekle
                md5.update(&buffer[..bytes_read]); // update Digest trait'inden gelir, alloc gerektirir
            }
            Err(e) => {
                // Okuma hatası durumunda handle'ı serbest bırakıp hata dön
                let _ = resource::release(handle);
                eprintln!("MD5 hesaplama: Kaynak okuma hatası ({}): {:?}", resource_id, e);
                return Err(map_sahne_error_to_checksum_resource_error(e));
            }
        }
    }

    // Okuma bitti, handle'ı serbest bırak
    let release_result = resource::release(handle);
     if let Err(e) = release_result {
          eprintln!("MD5 hesaplama: Kaynak release hatası ({}): {:?}", resource_id, e);
          // Release hatası kritik olmayabilir, loglayıp devam edebiliriz veya hata dönebiliriz.
          // Checksum zaten hesaplandı, bu yüzden sadece loglamak yeterli olabilir.
     }


    let sonuc = md5.finalize(); // MD5 özetini hesapla (Digest trait'inden gelir, alloc gerektirir)

    // Hesaplanan MD5 özetini hex string olarak döndür (format! alloc gerektirir)
    // format!("{:x}", sonuc) Result döndürmez, panikleyebilir.
    // Güvenli formatlama için alternatifler değerlendirilebilir, ama burada alloc varsayımıyla format! kullanalım.
    Ok(format!("{:x}", sonuc))
}

// Verilen Kaynağın MD5 özetini hesaplar ve beklenen MD5 özeti ile karşılaştırır.
// resource_id: Doğrulanacak Kaynağın Sahne64 Kaynak ID'si.
// beklenen_md5: Beklenen MD5 özeti (hex string).
pub fn dogrula_md5(resource_id: &str, beklenen_md5: &str) -> Result<bool, PaketYoneticisiHatasi> {
    // Kaynağın MD5 özetini hesapla
    let hesaplanan_md5 = hesapla_md5(resource_id)?; // hata PaketYoneticisiHatasi olarak yayılır

    // Hesaplanan özet ile beklenen özeti karşılaştır
    if hesaplanan_md5 == beklenen_md5 {
        Ok(true) // Eşleşiyorsa true döndür
    } else {
        Ok(false) // Eşleşmiyorsa false döndür
    }
}

// --- PaketYoneticisiHatasi enum tanımının güncellenmesi ---
// (paket_yoneticisi_hata.rs dosyasında veya ilgili modülde olmalı)
// paket_yoneticisi_hata.rs (Örnek - no_std uyumlu, Checksum için varyant eklenmiş)
#![no_std]
extern crate alloc;

use alloc::string::String;
use alloc::format;
use crate::SahneError;

// ... (ZipError, diğer no_std uyumlu hatalar) ...

#[derive(Debug)]
pub enum PaketYoneticisiHatasi {
    // ... (ZipHatasi, PaketBulunamadi, PaketKurulumHatasi, OnbellekHatasi, GecersizParametre, PathTraversalHatasi) ...

    // Sahne64 API'sından gelen genel hatalar
    SahneApiHatasi(SahneError),

    // Checksum hesaplama veya doğrulama sırasında Sahne64 kaynak erişimi hatası
    ChecksumResourceError(SahneError),

    // Checksum hesaplama kütüphanesinden gelen hata (örn. md5 kütüphanesinin kendi spesifik hatası, nadir)
     ChecksumCalculationError(...), // Eğer md5 veya Digest trait'i hata dönebiliyorsa

    // ... diğer hatalar ...
    BilinmeyenHata,
}

// SahneError'dan PaketYoneticisiHatasi'na dönüşüm
// Bu From implementasyonu genel SahneApiHatasi için kullanılabilir.
impl From<SahneError> for PaketYoneticisiHatasi {
    fn from(err: SahneError) -> Self {
        // SahneError'ın kaynağını inceleyip daha spesifik hatalara yönlendirme yapılabilir.
        // Match guard veya pattern matching kullanılabilir.
        match err {
            // Kaynakla ilgili hataları daha spesifik ChecksumResourceError'a yönlendir
            SahneError::ResourceNotFound |
            SahneError::PermissionDenied |
            SahneError::InvalidHandle |
            SahneError::ResourceBusy |
            SahneError::InvalidOperation => PaketYoneticisiHatasi::ChecksumResourceError(err),

            // Diğer SahneError'ları genel SahneApiHatasi olarak ele al
            _ => PaketYoneticisiHatasi::SahneApiHatasi(err),
        }
    }
}

#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // String, Vec, format! için

// Sahne64 API modülleri
use crate::resource; // Kaynak işlemleri (okuma, yazma, acquire, release)
use crate::SahneError; // Sahne64 hata türü
use crate::Handle; // Kaynak Handle'ları

// Özel hata enum'ımızı içe aktar (no_std uyumlu hali)
use crate::paket_yoneticisi_hatasi::PaketYoneticisiHatasi;
// SahneError'dan dönüşüm From implementasyonu ile sağlanacak.
// SecurityError, ParsingError gibi diğer hatalar zaten PaketYoneticisiHatasi'na mapleniyor.

// log kütüphanesini içe aktar (no_std uyumlu backend varsayımıyla)
use log::{info, warn, error, debug};

// String ve Vec from alloc
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format; // format! makrosu için
use alloc::borrow::ToOwned; // to_string() için

// no_std uyumlu print makroları (örnek çıktılar için)
use crate::print_macros::{println, eprintln};


// Helper fonksiyon: Sahne64 Kaynağından tüm içeriği Vec<u8> olarak oku.
// Utils modülünden yeniden kullanıldı veya buraya kopyalandı.
// SecurityError yerine PaketYoneticisiHatasi dönecek şekilde güncellendi.
fn read_resource_to_vec(resource_id: &str) -> Result<Vec<u8>, PaketYoneticisiHatasi> { // Result<Vec<u8>, PaketYoneticisiHatasi> olmalı
    let handle = resource::acquire(resource_id, resource::MODE_READ)
        .map_err(|e| PaketYoneticisiHatasi::from(e))?; // SahneError -> PaketYoneticisiHatasi

    let mut buffer = Vec::new(); // alloc
    let mut temp_buffer = [0u8; 512]; // Stack buffer

    loop {
        match resource::read(handle, &mut temp_buffer) {
            Ok(0) => break,
            Ok(bytes_read) => {
                buffer.extend_from_slice(&temp_buffer[..bytes_read]); // alloc
            }
            Err(e) => {
                let _ = resource::release(handle);
                return Err(PaketYoneticisiHatasi::from(e)); // SahneError -> PaketYoneticisiHatasi
            }
        }
    }

    let release_result = resource::release(handle);
     if let Err(_e) = release_result {
          // Log error, but continue
          error!("Helper: Kaynak release hatası ({}): {:?}", resource_id, _e); // no_std log
     }

    Ok(buffer) // Vec<u8> (alloc)
}


// Paket yönetim işlemlerini izlemek ve geri almak için bir işlem günlüğü tutar.
// İşlem günlüğü, bir Sahne64 Kaynağı olarak saklanır.
pub struct IslemYoneticisi {
    // İşlem günlüğü dosyasının Kaynak ID'si (örn. "sahne://system/pkgmgr_transaction.log")
    log_resource_id: String, // String alloc gerektirir.
}

impl IslemYoneticisi {
    // Yeni bir IslemYoneticisi örneği oluşturur.
    // log_resource_id: İşlem günlüğü olarak kullanılacak Kaynağın ID'si.
    pub fn yeni(log_resource_id: &str) -> Self { // &str log_resource_id
        IslemYoneticisi {
            log_resource_id: log_resource_id.to_owned(), // to_owned() alloc
        }
    }

    // İşlem günlüğüne bir giriş yazar.
    // message: Günlüğe yazılacak mesaj (örn. "ISLEM BASLADI", "Paket yukleniyor: X").
    // Dönüş değeri: Başarı veya PaketYoneticisiHatasi.
    fn log_entry(&self, message: &str) -> Result<(), PaketYoneticisiHatasi> {
        let full_message = format!("{}\n", message); // format! alloc
        let buffer = full_message.as_bytes(); // String -> &[u8]

        // İşlem günlüğü Kaynağını yazma ve ekleme (append) izniyle acquire et.
        // MODE_CREATE varsa Kaynak yoksa oluşturur.
        let handle = resource::acquire(
            &self.log_resource_id,
            resource::MODE_WRITE | resource::MODE_CREATE | resource::MODE_APPEND // MODE_APPEND eklendi
        ).map_err(|e| {
             error!("İşlem günlüğü Kaynağı acquire hatası ({}): {:?}", self.log_resource_id, e); // no_std log
            PaketYoneticisiHatasi::IslemYoneticisiHatasi(format!( // format! alloc
                "İşlem günlüğü açılırken/oluşturulurken hata oluştu ({}): {:?}",
                self.log_resource_id, e
            ))
        })?;

        // Mesajı Kaynağa yaz.
        // resource::write tek seferde yazmayabilir, loop gerekebilir.
        let mut written = 0;
        while written < buffer.len() {
            match resource::write(handle, &buffer[written..]) {
                 Ok(bytes_written) => {
                     if bytes_written == 0 {
                          // Hiçbir şey yazılamadı, Kaynak hatası olabilir
                          let _ = resource::release(handle);
                           error!("İşlem günlüğü Kaynağı yazma hatası ({}): Kaynak yazmayı durdurdu.", self.log_resource_id); // no_std log
                          return Err(PaketYoneticisiHatasi::IslemYoneticisiHatasi(format!( // alloc
                              "İşlem günlüğü Kaynağı yazmayı durdurdu ({}).", self.log_resource_id
                          )));
                     }
                     written += bytes_written;
                 }
                 Err(e) => {
                      // Yazma hatası
                      let _ = resource::release(handle);
                       error!("İşlem günlüğü Kaynağı yazma hatası ({}): {:?}", self.log_resource_id, e); // no_std log
                      return Err(PaketYoneticisiHatasi::from(e)); // SahneError -> PaketYoneticisiHatasi
                 }
            }
        }

        // Handle'ı serbest bırak
        let release_result = resource::release(handle);
         if let Err(_e) = release_result {
              error!("İşlem günlüğü Kaynağı release hatası ({}): {:?}", self.log_resource_id, _e); // no_std log
         }

        Ok(()) // Başarı
    }


    // İşlem günlüğüne "ISLEM BASLADI" kaydını yazar ve işlemi başlatır.
    pub fn baslat_islem(&self) -> Result<(), PaketYoneticisiHatasi> {
        info!("İşlem başlatılıyor. Günlük Kaynağı: {}", self.log_resource_id); // no_std log
        self.log_entry("ISLEM BASLADI") // log_entry helper'ını kullan
    }

    // İşlem günlüğüne bir işlem adımı kaydeder.
    // adim: İşlem adımını tanımlayan mesaj.
    pub fn islem_adimi(&self, adim: &str) -> Result<(), PaketYoneticisiHatasi> {
        debug!("İşlem adımı kaydediliyor: '{}'. Günlük Kaynağı: {}", adim, self.log_resource_id); // no_std log
        self.log_entry(adim) // log_entry helper'ını kullan
    }

    // İşlem günlüğüne "ISLEM TAMAMLANDI" kaydını yazar ve işlemi tamamlar.
    pub fn tamamla_islem(&self) -> Result<(), PaketYoneticisiHatasi> {
        info!("İşlem tamamlanıyor. Günlük Kaynağı: {}", self.log_resource_id); // no_std log
        self.log_entry("ISLEM TAMAMLANDI") // log_entry helper'ını kullan
    }

    // İşlemi geri alır. İşlem günlüğünü okur ve adımları tersine çevirmeye çalışır.
    // Bu basit implementasyonda sadece günlük dosyasını siler/temizler.
    // Gerçek bir geri alma, günlüktteki adımlara göre yapılan değişiklikleri geri almayı gerektirir.
    pub fn geri_al_islem(&self) -> Result<(), PaketYoneticisiHatasi> {
        info!("İşlem geri alma başlatılıyor. Günlük Kaynağı: {}", self.log_resource_id); // no_std log

        // İşlem günlüğü içeriğini oku (Vec<u8> olarak).
        // Kaynak bulunamazsa hata dönmez (geri alınacak işlem yoktur).
        let islem_gunlugu_icerigi_bytes = match read_resource_to_vec(&self.log_resource_id) {
             Ok(bytes) => bytes,
             Err(PaketYoneticisiHatasi::SahneApiError(SahneError::ResourceNotFound)) => {
                 warn!("İşlem günlüğü Kaynağı bulunamadı ({}). Geri alınacak bir işlem yok.", self.log_resource_id); // no_std log
                 return Ok(()); // Kaynak yoksa, geri alma başarılı sayılır.
             }
             Err(e) => {
                 error!("İşlem günlüğü okunurken hata oluştu ({}): {:?}", self.log_resource_id, e); // no_std log
                 return Err(e); // Diğer okuma hatalarını yay
             }
        }; // read_resource_to_vec PaketYoneticisiHatasi döner.


        // Vec<u8>'i String'e çevir (UTF-8). Hatalı karakterler olabilir, lossy çeviri kullanalım veya hata dönelim.
        let islem_gunlugu_icerigi = String::from_utf8_lossy(&islem_gunlugu_icerigi_bytes); // from_utf8_lossy no_std+alloc

        // Satırları ayır ve son satırı kontrol et.
        let islem_adimlari: Vec<&str> = islem_gunlugu_icerigi.lines().collect(); // lines() Iter<'_> over &str no_std, collect Vec<&str> alloc

        // İşlem zaten tamamlanmışsa geri almayı reddet.
        if islem_adimlari.last() == Some(&"ISLEM TAMAMLANDI") { // last() Option<& &str>, Some(&"...") comp no_std
            let hata_mesaji = format!("İşlem geri alınamaz, zaten tamamlandı. Günlük Kaynağı: {}", self.log_resource_id); // format! alloc
            warn!("{}", hata_mesaji); // no_std log
            return Err(PaketYoneticisiHatasi::IslemYoneticisiHatasi(hata_mesaji)); // PaketYoneticisiHatasi::IslemYoneticisiHatasi alloc
        }

        // --- Gerçek Geri Alma Mantığı Eksikliği ---
        // Bu noktada, gerçek bir geri alma mekanizması, `islem_adimlari` listesini
        // tersten okuyarak her adımda yapılan değişikliği geri almalıdır.
        // Örneğin: "Paket yuklendi: X" adımını gördüğünde, paketi kaldırma işlemini başlatmalıdır.
        // Bu, paket yöneticisindeki tüm kurulum, kaldırma gibi eylemlerin geri alınabilir şekilde
        // implement edilmesini (örn. bir geri alma betiği veya bilgisi tutarak) gerektirir.
        // Bu modül sadece günlüğü yönetir, adımları nasıl geri alacağını bilmez.

        // Basitlik adına, sadece işlem günlüğü Kaynağını temizleyelim (trunc).
        // Bu, bir sonraki işlem için temiz bir başlangıç sağlar, ancak önceki
        // başarısız işlemin neden olduğu yarım kalmış değişiklikleri geri almaz.
        info!("İşlem günlüğü Kaynağı temizleniyor (geri alma simülasyonu): {}", self.log_resource_id); // no_std log

        // Kaynağı yazma ve truncate izniyle acquire et.
        let handle_temizle = resource::acquire(
            &self.log_resource_id,
            resource::MODE_WRITE | resource::MODE_TRUNCATE // TRUNCATE Kaynak içeriğini siler
        ).map_err(|e| {
             error!("İşlem günlüğü Kaynağı temizleme acquire hatası ({}): {:?}", self.log_resource_id, e); // no_std log
            PaketYoneticisiHatasi::IslemYoneticisiHatasi(format!( // format! alloc
                "İşlem günlüğü temizlenemedi ({}): {:?}",
                self.log_resource_id, e
            ))
        })?;

        // Handle'ı serbest bırakmak temizleme işlemini tamamlar.
        let release_result = resource::release(handle_temizle);
         if let Err(_e) = release_result {
              error!("İşlem günlüğü Kaynağı temizleme release hatası ({}): {:?}", self.log_resource_id, _e); // no_std log
         }


        info!("İşlem geri alma tamamlandı (günlük temizlendi). Günlük Kaynağı: {}", self.log_resource_id); // no_std log
        Ok(()) // Başarı
    }
}

// #[cfg(test)] bloğu std test runner'ı ve std bağımlılıkları gerektirir.
// Testler için mock resource veya Sahne64 simülasyonu gereklidir.

#[cfg(test)]
mod tests {
    // std::io, std::path, std::fs, tempfile kullandığı için no_std'de doğrudan çalışmaz.
    // Mock resource::acquire/read/write/release ve test dosyası oluşturma/okuma helper'ları gerektirir.
}

// --- PaketYoneticisiHatasi enum tanımı ---
// srcerror.rs modülünde tanımlanmıştır ve no_std uyumludur.
// IslemYoneticisiHatasi varyantı eklenmelidir.
// SahneError'dan PaketYoneticisiHatasi'na dönüşüm için From implementasyonu gereklidir.
// ParsingError varyantı read_resource_to_vec ve process_line'da kullanılabilir.

// paket_yoneticisi_hata.rs (Örnek - no_std uyumlu, İşlem hatası eklenmiş)
#![no_std]
extern crate alloc;

use alloc::string::String;
// ... diğer importlar ...

#[derive(Debug)]
pub enum PaketYoneticisiHatasi {
    // ... diğer hatalar ...

    // İşlem Yönetimi sırasında oluşan hatalar
    IslemYoneticisiHatasi(String), // Hata detayını string olarak tutmak alloc gerektirir. Altında yatan SahneError detayını içerebilir.

    // Parsing hataları (örn. log dosyasından okuma sırasında)
    ParsingError(String), // String alloc gerektirir

    // ... diğer hatalar ...
}

// SahneError'dan PaketYoneticisiHatasi'na dönüşüm (genel implementasyon)
 impl From<SahneError> for PaketYoneticisiHatasi { ... }

// ParsingError varyantı için From implementasyonu String'den
 impl From<String> for PaketYoneticisiHatasi {
     fn from(err_msg: String) -> Self {
         PaketYoneticisiHatasi::ParsingError(err_msg)
     }
 }

#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // String, format! için

use alloc::string::{String, ToString};
use alloc::format; // format! makrosu için
use alloc::borrow::ToOwned; // &str -> String için

// Özel hata enum'ımızı içe aktar (no_std uyumlu hali)
use crate::paket_yoneticisi_hata::PaketYoneticisiHatasi;
// SahneError'dan dönüşüm From implementasyonu ile sağlanacak

// log kütüphanesini içe aktar (no_std uyumlu backend varsayımıyla)
use log::{info, debug, error, warn};

// Sahne64 API modülleri
use crate::resource; // Kaynak işlemleri (açma, kontrol)
use crate::SahneError; // Sahne64 hata türü
use crate::Handle; // Kaynak Handle'ları

// Kilit Yönetimi için Sahne64 Kaynak Kontrol Komutları (Varsayımsal)
// Gerçek Sahne64 API'sında tanımlanmalıdır.
const RESOURCE_CONTROL_CMD_LOCK_EXCLUSIVE: u33 = 1; // Exclusive kilit al
const RESOURCE_CONTROL_CMD_UNLOCK: u33 = 2;       // Kilidi serbest bırak


// Dosya veya Kaynak tabanlı kilit yönetimini sağlar.
// Sahne64 Kaynak kontrol mekanizmasını kullanır.
pub struct KilitYoneticisi {
    // Kilit dosyasının/kaynağının Handle'ı. Kilitleme/serbest bırakma Handle üzerinden yapılır.
    kilit_kaynagi_handle: Handle,
    // Kilit dosyasının/kaynağının Kaynak ID'si (loglama ve hata mesajları için).
    kilit_kaynagi_id: String, // String alloc gerektirir.
    // Kilidin bu örnek tarafından alınıp alınmadığını takip et.
    kilit_tutuldu: bool,
}

impl KilitYoneticisi {
    // Yeni bir KilitYoneticisi örneği oluşturur ve kilit kaynağını açar.
    // kilit_kaynagi_id: Kilit olarak kullanılacak Sahne64 Kaynağının ID'si (örn. "sahne://system/pkgmgr.lock").
    // Dönüş değeri: Yeni KilitYoneticisi örneği veya PaketYoneticisiHatasi.
    pub fn yeni(kilit_kaynagi_id: &str) -> Result<Self, PaketYoneticisiHatasi> { // Path yerine &str Kaynak ID
        debug!("Kilit yöneticisi oluşturuluyor. Kaynak ID: {}", kilit_kaynagi_id); // no_std log

        // Kilit kaynağını okuma ve yazma izniyle aç veya oluştur.
        // MODE_CREATE: Kaynak yoksa oluştur.
        // MODE_READ/WRITE: Handle üzerinden okuma/yazma yetkisi (Kilitleme için gerekli olabilir).
        let kilit_kaynagi_handle = resource::acquire(
            kilit_kaynagi_id,
            resource::MODE_READ | resource::MODE_WRITE | resource::MODE_CREATE
        ).map_err(|e| {
            // SahneError'dan PaketYoneticisiHatasi::KilitYoneticisiHatasi'na çevir.
            PaketYoneticisiHatasi::KilitYoneticisiHatasi(format!( // format! alloc
                "Kilit Kaynağı açılırken hata oluştu: {:?}. Kaynak ID: {}",
                e, kilit_kaynagi_id
            ))
        })?;

        info!("Kilit Kaynağı başarıyla açıldı. Kaynak ID: {}", kilit_kaynagi_id); // no_std log

        Ok(KilitYoneticisi {
            kilit_kaynagi_handle,
            kilit_kaynagi_id: kilit_kaynagi_id.to_owned(), // Kaynak ID'sini String olarak sakla (alloc)
            kilit_tutuldu: false, // Başlangıçta kilit tutulmuyor
        })
    }

    // Kaynak üzerinde exclusive kilit almaya çalışır. Bloklayıcı olabilir veya hemen dönebilir (try_lock gibi).
    // Varsayım: resource::control ile kilitleme yapılır ve çakışma durumunda hata döner (try_lock benzeri).
    pub fn kilit_al(&mut self) -> Result<(), PaketYoneticisiHatasi> {
        debug!("Kilit alınmaya çalışılıyor. Kaynak ID: {}", self.kilit_kaynagi_id); // no_std log

        // resource::control(handle, command, args: &[u8]) -> Result<Vec<u8>, SahneError>
        // Kilitleme komutuna argüman gerekebilir (örn. kilitleme tipi - exclusive/shared).
        // Varsayım: Komut numarası yeterli ve argüman gerekmiyor veya 0 argüman.
        match resource::control(self.kilit_kaynagi_handle, RESOURCE_CONTROL_CMD_LOCK_EXCLUSIVE, &[]) {
            Ok(_) => {
                // Kontrol başarılıysa kilit alındı.
                self.kilit_tutuldu = true; // Kilit alındı olarak işaretle
                info!("Kilit başarıyla alındı. Kaynak ID: {}", self.kilit_kaynagi_id); // no_std log
                Ok(())
            }
            Err(e) => {
                // Kontrol hatası (örn. Kaynak meşgul - ResourceBusy).
                eprintln!("Kilit alınamadı (exclusive lock) (Kaynak ID: {}): {:?}", self.kilit_kaynagi_id, e); // no_std log
                // SahneError'dan KilitYoneticisiHatasi'na çevir.
                // ResourceBusy gibi spesifik SahneError'ları burada mapleyebiliriz.
                match e {
                    SahneError::ResourceBusy => {
                         // Kaynak meşgul, kilit zaten tutuluyor. Özel bir hata varyantı eklenebilir (LockAlreadyHeld?).
                         // Veya genel KilitYoneticisiHatasi içinde detay verilir.
                         Err(PaketYoneticisiHatasi::KilitYoneticisiHatasi(format!( // format! alloc
                             "Kaynak meşgul, kilit zaten başka biri tarafından tutuluyor. Kaynak ID: {}",
                             self.kilit_kaynagi_id
                         )))
                    }
                    _ => {
                         // Diğer SahneError'lar
                         Err(PaketYoneticisiHatasi::KilitYoneticisiHatasi(format!( // format! alloc
                             "Kilit alınamadı (Kaynak ID: {}): {:?}",
                             self.kilit_kaynagi_id, e
                         )))
                    }
                }
            }
        }
    }

    // Kilidi serbest bırakır.
    // Varsayım: resource::control ile kilit serbest bırakılır.
    pub fn kilidi_serbest_birak(&mut self) -> Result<(), PaketYoneticisiHatasi> { // self immutable &Self idi, kilit_tutuldu değiştiği için &mut self olmalı
        if self.kilit_tutuldu { // Sadece kilit bu örnek tarafından tutuluyorsa serbest bırakmayı dene
            debug!("Kilit serbest bırakılmaya çalışılıyor. Kaynak ID: {}", self.kilit_kaynagi_id); // no_std log

            match resource::control(self.kilit_kaynagi_handle, RESOURCE_CONTROL_CMD_UNLOCK, &[]) {
                 Ok(_) => {
                    // Kontrol başarılıysa kilit serbest bırakıldı.
                    self.kilit_tutuldu = false; // Kilit tutulmuyor olarak işaretle
                    info!("Kilit başarıyla serbest bırakıldı. Kaynak ID: {}", self.kilit_kaynagi_id); // no_std log
                    Ok(())
                 }
                 Err(e) => {
                     eprintln!("Kilit serbest bırakılamadı (Kaynak ID: {}): {:?}", self.kilit_kaynagi_id, e); // no_std log
                     // SahneError'dan KilitYoneticisiHatasi'na çevir.
                     Err(PaketYoneticisiHatasi::KilitYoneticisiHatasi(format!( // format! alloc
                         "Kilit serbest bırakılamadı (Kaynak ID: {}): {:?}",
                         self.kilit_kaynagi_id, e
                     )))
                 }
            }
        } else {
            // Kilit zaten tutulmuyordu, bir şey yapmaya gerek yok.
            // Loglayıp başarı dönebiliriz.
            warn!("Kilit serbest bırakma çağrısı yapıldı, ancak kilit zaten bu örnek tarafından tutulmuyordu. Kaynak ID: {}", self.kilit_kaynagi_id); // no_std log
            Ok(())
        }
    }
}

// RAII (Resource Acquisition Is Initialization) prensibi ile kilidin otomatik serbest bırakılmasını sağlar.
impl Drop for KilitYoneticisi {
    fn drop(&mut self) { // self immutable &self idi, kilidi_serbest_birak çağrıldığı için &mut self olmalı
        if self.kilit_tutuldu { // Eğer kilit hala bu örnek tarafından tutuluyorsa
            debug!("KilitYoneticisi Drop trait çağrıldı, kilit serbest bırakılıyor. Kaynak ID: {}", self.kilit_kaynagi_id); // no_std log

            // resource::control ile kilidi serbest bırak. Drop içinde hata dönemeyiz, bu yüzden logla.
            if let Err(e) = resource::control(self.kilit_kaynagi_handle, RESOURCE_CONTROL_CMD_UNLOCK, &[]) {
                 error!(
                     "Kilit Drop trait içinde serbest bırakılırken hata oluştu (Kaynak ID: {}): {:?}",
                     self.kilit_kaynagi_id, e
                 ); // no_std log
            } else {
                 // Başarılı olursa, kilit_tutuldu bayrağını güncelle (burada drop içindeyiz, nesne zaten yok olacak).
                 // self.kilit_tutuldu = false; // Gerek yok
                 info!("Kilit Drop trait içinde başarıyla serbest bırakıldı. Kaynak ID: {}", self.kilit_kaynagi_id); // no_std log
            }

            // Ayrıca, drop içinde handle'ı da serbest bırakmak gerekebilir (eğer resource::acquire kaynağı açık tutuyorsa).
            // Eğer kilit Kaynağının kendisi kapatıldığında kilit otomatik serbest kalıyorsa, sadece resource::release yeterli olabilir.
            // Varsayım: resource::release Handle'ı kapatır.
            let release_result = resource::release(self.kilit_kaynagi_handle);
             if let Err(e) = release_result {
                 error!("Kilit Kaynağı Handle Drop trait içinde serbest bırakılırken hata oluştu (Kaynak ID: {}): {:?}", self.kilit_kaynagi_id, e); // no_std log
             }

        } else {
             // Kilit tutulmuyordu, sadece handle'ı serbest bırak.
             let release_result = resource::release(self.kilit_kaynagi_handle);
              if let Err(e) = release_result {
                  error!("Kilit Kaynağı Handle Drop trait çağrıldı (kilit tutulmuyordu), serbest bırakılırken hata oluştu (Kaynak ID: {}): {:?}", self.kilit_kaynagi_id, e); // no_std log
              }
        }
    }
}

// --- PaketYoneticisiHatasi enum tanımının no_std uyumlu hale getirilmesi ---
// (paket_yoneticisi_hata.rs dosyasında veya ilgili modülde olmalı)
// KilitYoneticisiHatasi varyantı eklenmeli.

// paket_yoneticisi_hata.rs (Örnek - no_std uyumlu, Kilit hatası eklenmiş)
#![no_std]
extern crate alloc;

use alloc::string::String;
use alloc::format;
use crate::SahneError;
// ... diğer hata türleri ...

#[derive(Debug)]
pub enum PaketYoneticisiHatasi {
    // ... diğer hatalar ...

    // Kilit Yönetimi sırasında oluşan hatalar (açma, alma, serbest bırakma)
    KilitYoneticisiHatasi(String), // Hata detayını string olarak tutmak alloc gerektirir. Altında yatan SahneError detayını içerebilir.

    // Sahne64 API'sından gelen genel hatalar (KilitYoneticisiHatasi bunu sarmalayabilir veya ayrı tutulur)
     SahneApiError(SahneError),

    // ... diğer hatalar ...
}

// SahneError'dan PaketYoneticisiHatasi'na dönüşüm (KilitYoneticisiHatasi varyantına maplenmiyor genellikle buradan)
 impl From<SahneError> for PaketYoneticisiHatasi { ... }

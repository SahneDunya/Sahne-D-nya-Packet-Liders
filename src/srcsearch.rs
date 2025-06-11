#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // String, Vec, format! için

// no_std ve alloc uyumlu regex crate'i (feature'ları etkinleştirilmiş olmalı)
use regex::Regex;
use regex::Error as RegexError; // regex hata türü

// Paket struct tanımını içeren modül
use crate::package::Paket; // Varsayım: Paket struct'ı srcpackage.rs'de tanımlı ve no_std uyumlu

// Özel hata enum'ımızı içe aktar (no_std uyumlu hali)
use crate::paket_yoneticisi_hatasi::PaketYoneticisiHatasi;
// RegexError'dan dönüşüm From implementasyonu ile sağlanacak

// log kütüphanesini içe aktar (no_std uyumlu backend varsayımıyla)
use log::{debug, error, trace};

// String ve Vec from alloc
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format; // format! makrosu için

// Paket arama işlemlerini yöneten yapı.
// Düzenli ifadeler kullanarak paket adı veya açıklamasında arama yapar.
pub struct AramaYoneticisi {}

impl AramaYoneticisi {
    // Yeni bir AramaYoneticisi örneği oluşturur.
    pub fn yeni() -> Self {
        AramaYoneticisi {}
    }

    // Verilen paket listesinde, arama desenine göre paket arar.
    // paketler: Aranacak paketlerin dilimi (&[Paket]). Vec yerine dilim daha esnektir.
    // arama_deseni: Regex formatında arama deseni.
    // Dönüş değeri: Arama desenine uyan paketlere referansların listesi veya PaketYoneticisiHatasi (geçersiz regex deseni durumunda).
    pub fn paket_ara<'a>(paketler: &'a [Paket], arama_deseni: &str) -> Result<Vec<&'a Paket>, PaketYoneticisiHatasi> { // &Vec<Paket> yerine &'a [Paket]
        debug!("Paket araması başlatılıyor. Arama deseni: '{}'", arama_deseni); // no_std log

        // Arama desenini kullanarak bir Regex nesnesi oluşturmaya çalış.
        // regex::Regex::new alloc gerektirir.
        let regex = Regex::new(arama_deseni).map_err(|e| {
            // regex::Error'dan PaketYoneticisiHatasi::AramaYoneticisiHatasi'na çevir.
            let hata_mesaji = format!("Geçersiz arama deseni: {}. Hata: {}", arama_deseni, e); // format! alloc
            error!("{}", hata_mesaji); // no_std log
            PaketYoneticisiHatasi::AramaYoneticisiHatasi(hata_mesaji) // PaketYoneticisiHatasi::AramaYoneticisiHatasi alloc
        })?; // Hata durumunda ? ile yay

        trace!("Regex deseni derlendi: '{}'", arama_deseni); // no_std log

        // Paket listesini filtrele ve arama desenine uyan paketleri topla.
        // iter(), filter(), collect() no_std+alloc uyumludur.
        let sonuclar: Vec<&Paket> = paketler
            .iter() // Iteratör over &[Paket] -> &Paket
            .filter(|paket| {
                // Paket adı veya açıklaması (varsa) regex desenine uyuyor mu kontrol et.
                let eslesme_bulundu = regex.is_match(&paket.ad) || // is_match &str alır
                                     paket.aciklama.as_ref().map_or(false, |aciklama| regex.is_match(aciklama.as_str())); // Option<&String> -> Option<&str>, map_or no_std

                trace!("Paket '{}' için arama yapılıyor. Eşleşme bulundu: {}", paket.ad, eslesme_bulundu); // no_std log
                eslesme_bulundu
            })
            .collect(); // Vec<&Paket> oluştur (alloc)

        debug!("Arama tamamlandı. {} paket bulundu. Arama deseni: '{}'", sonuclar.len(), arama_deseni); // no_std log
        Ok(sonuclar) // Arama sonuçlarını (referans listesi) döndür. Vec alloc gerektirir.
    }

    // Kaldırılan placeholder fonksiyonlar (paket yükleme ve Sahne64 loglama)
    // Paket yükleme logic'i Depo Yönetimi veya Veritabanı Yönetimi modüllerine aittir.
    // Loglama, log crate'i ve Sahne64 backend'i tarafından handled edilir.
}


// #[cfg(test)] bloğu std test runner'ı ve std bağımlılıkları gerektirir.
// Regex crate'i testleri için özel bir test ortamı veya feature flag'i gerekebilir.

#[cfg(test)]
mod tests {
    // std::string, std::vec, regex crate testleri vb. std gerektirir.
    // Sahne64 ortamında testler için özel test runner ve mock paket listesi gerektirir.
}
// paket_yoneticisi_hata.rs (Örnek - no_std uyumlu, Arama hatası eklenmiş)
#![no_std]
extern crate alloc;

use alloc::string::String;
// ... diğer importlar ...

use regex::Error as RegexError; // regex hata türü

#[derive(Debug)]
pub enum PaketYoneticisiHatasi {
    // ... diğer hatalar ...

    // Arama Yönetimi (Regex) sırasında oluşan hatalar
    AramaYoneticisiHatasi(String), // Hata detayını string olarak tutmak alloc gerektirir. Altında yatan RegexError detayını içerebilir.

    // ... diğer hatalar ...
}

// regex::Error'dan PaketYoneticisiHatasi'na dönüşüm için From implementasyonu
// AramaYoneticisiHatasi varyantını kullanarak.
impl From<RegexError> for PaketYoneticisiHatasi {
    fn from(err: RegexError) -> Self {
        // RegexError'ın Display implementasyonu PaketYoneticisiHatasi içindeki string için kullanılabilir.
        PaketYoneticisiHatasi::AramaYoneticisiHatasi(format!("Regex hatası: {}", err)) // format! alloc
    }
}

// PaketYoneticisiHatasi::AramaYoneticisiHatasi(String) için From<String> de implement edilebilir.
 impl From<String> for PaketYoneticisiHatasi {
     fn from(err_msg: String) -> Self {
         PaketYoneticisiHatasi::AramaYoneticisiHatasi(err_msg)
     }
 }

// ... diğer From implementasyonları (örn. SahneError, PostcardError, ZipError, DependencyResolverError, ScriptError) ...

#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // HashMap, String, Vec, format! için

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format; // format! makrosu için
use alloc::boxed::Box; // Hata sarmalamak için gerekebilir
use alloc::collections::HashMap; // Cache için

// serde ve no_std uyumlu serileştirme/deserileştirme kütüphanesi
use serde::{Deserialize, Serialize};
use postcard; // no_std uyumlu binary serileştirme
use postcard::Error as PostcardError; // Postcard hata türü

// Paket struct tanımını içeren modül
use crate::package::Paket; // Varsayım: Paket struct'ı srcpackage.rs'de tanımlı ve no_std uyumlu

// Sahne64 API modülleri
use crate::resource; // Ağ ve dosya sistemi benzeri işlemler için
use crate::SahneError; // Sahne64 hata türü
use crate::Handle; // Kaynak Handle'ları

// Özel hata enum'ımızı içe aktar (no_std uyumlu hali)
use crate::paket_yoneticisi_hata::PaketYoneticisiHatasi;
// SahneError ve PostcardError'dan dönüşüm From implementasyonları ile sağlanacak

// no_std uyumlu print makroları
use crate::print_macros::{println, eprintln};

// Helper function to read resource content into a Vec<u8> (reused from previous refactoring)
fn read_resource_to_vec(resource_id: &str) -> Result<Vec<u8>, PaketYoneticisiHatasi> {
    let handle = resource::acquire(resource_id, resource::MODE_READ)
        .map_err(|e| {
             eprintln!("Helper: Kaynak acquire hatası ({}): {:?}", resource_id, e);
             PaketYoneticisiHatasi::from(e) // SahneError -> PaketYoneticisiHatasi
        })?;

    let mut buffer = Vec::new();
    let mut temp_buffer = [0u8; 512]; // Stack buffer

    loop {
        match resource::read(handle, &mut temp_buffer) {
            Ok(0) => break,
            Ok(bytes_read) => {
                buffer.extend_from_slice(&temp_buffer[..bytes_read]);
            }
            Err(e) => {
                let _ = resource::release(handle);
                 eprintln!("Helper: Kaynak okuma hatası ({}): {:?}", resource_id, e);
                return Err(PaketYoneticisiHatasi::from(e));
            }
        }
    }

    let release_result = resource::release(handle);
     if let Err(e) = release_result {
          eprintln!("Helper: Kaynak release hatası ({}): {:?}", resource_id, e);
     }

    Ok(buffer)
}

// Depo Yöneticisi Yapısı (Paket Deposunu Yönetir)
pub struct DepoYoneticisi {
    // Paket deposunun temel Kaynak ID'si (örn. "sahne://remotepkgrepo/packages/")
    pub depo_base_resource_id: String,
    // Yerel depolama veya önbellek dizininin Kaynak ID'si (örn. "sahne://cache/repo/")
    pub yerel_depo_base_resource_id: String,
    // Paket listesi önbelleği (bellek içi)
    paket_listesi_cache: Option<Vec<Paket>>,
}

impl DepoYoneticisi {
    pub fn yeni(depo_base_resource_id: String, yerel_depo_base_resource_id: String) -> Self {
        DepoYoneticisi {
            depo_base_resource_id,
            yerel_depo_base_resource_id,
            paket_listesi_cache: None, // Başlangıçta önbellek boş
        }
    }

    // Paket Listesini Alır (Bellek içi cache -> Yerel depo Kaynağı -> Uzak depo Kaynağı).
    // İlk başarılı kaynaktan veriyi yükler ve önbelleğe alır.
    pub fn paket_listesini_al(&mut self) -> Result<Vec<Paket>, PaketYoneticisiHatasi> {
        // 1. Bellek içi cache kontrolü
        if let Some(ref paketler) = self.paket_listesi_cache {
            println!("Bellek içi önbellekten paket listesi kullanılıyor.");
            return Ok(paketler.clone());
        }

        // 2. Yerel depo Kaynağı kontrolü (önbellekteki paketler.bin gibi dosya)
        let yerel_paket_listesi_id = format!("{}/paketler.bin", self.yerel_depo_base_resource_id);
        match read_resource_to_vec(&yerel_paket_listesi_id) {
             Ok(buffer) => {
                 match postcard::from_bytes_copy::<Vec<Paket>>(&buffer) {
                     Ok(paketler) => {
                         println!("Yerel depo Kaynağından paket listesi yüklendi: {}", yerel_paket_listesi_id);
                         self.paket_listesi_cache = Some(paketler.clone());
                         return Ok(paketler);
                     }
                     Err(e) => {
                         eprintln!("Yerel paket listesi deserialize hatası (Kaynak: {}): {:?}", yerel_paket_listesi_id, e);
                         // Deserialize hatası durumunda uzak depodan indirmeye devam edelim.
                     }
                 }
             }
             Err(PaketYoneticisiHatasi::SahneApiError(SahneError::ResourceNotFound)) => {
                 // Yerel depo Kaynağı bulunamadı. Uzak depodan indirmeye devam et.
                 println!("Yerel paket listesi Kaynağı bulunamadı ({}).", yerel_paket_listesi_id);
             }
             Err(e) => {
                  // Diğer kaynak okuma hataları. Hata logla ve uzak depodan indirmeyi dene.
                  eprintln!("Yerel paket listesi Kaynağı okuma hatası ({}): {:?}", yerel_paket_listesi_id, e);
             }
        }

        // 3. Yerel önbellekte yoksa, uzak depodan indir
        println!("Uzak depodan paket listesi indiriliyor: {}", self.depo_base_resource_id);
        let uzak_paket_listesi_id = format!("{}/paketler.bin", self.depo_base_resource_id);

        let buffer = read_resource_to_vec(&uzak_paket_listesi_id)?;

        match postcard::from_bytes_copy::<Vec<Paket>>(&buffer) {
            Ok(paketler) => {
                println!("Paket listesi uzak depodan başarıyla indirildi ve çözümlendi.");
                self.paket_listesi_cache = Some(paketler.clone());

                // Başarıyla indirildiyse, yerel depo Kaynağına da kaydet (güncelle)
                let yerel_depo_dosyasi_id = format!("{}/paketler.bin", self.yerel_depo_base_resource_id);
                 let serialized_data = postcard::to_postcard(&paketler)
                     .map_err(|e| {
                          eprintln!("Yerel depo için paket listesi serileştirme hatası: {:?}", e);
                          PaketYoneticisiHatasi::SerializationError(e)
                     })?;

                 match resource::acquire(
                     &yerel_depo_dosyasi_id,
                     resource::MODE_WRITE | resource::MODE_CREATE | resource::MODE_TRUNCATE
                 ) {
                     Ok(handle) => {
                         let write_result = resource::write(handle, &serialized_data);
                         let release_result = resource::release(handle);
                         if let Err(e) = write_result { eprintln!("Yerel depo Kaynağı yazma hatası: {:?}", e); }
                         if let Err(e) = release_result { eprintln!("Yerel depo Kaynağı release hatası: {:?}", e); }
                         println!("Paket listesi yerel depo Kaynağına kaydedildi: {}", yerel_depo_dosyasi_id);
                     }
                     Err(e) => {
                         eprintln!("Yerel depo Kaynağı acquire hatası ({}): {:?}", yerel_depo_dosyasi_id, e);
                         // Hata durumunda da listeyi yine de döndürelim.
                     }
                 }

                Ok(paketler)
            }
            Err(e) => {
                eprintln!("Uzak depodan indirilen paket listesi deserialize hatası: {:?}", e);
                Err(PaketYoneticisiHatasi::DeserializationError(e))
            }
        }
    }

    // Yerel Depoyu Güncelleme (Paket listesini indirip yerel depoya kaydeder).
    pub fn yerel_depoyu_guncelle(&mut self) -> Result<(), PaketYoneticisiHatasi> {
        println!("Yerel depo güncelleniyor: {}", self.yerel_depo_base_resource_id);

        let paketler = self.paket_listesini_al()?;

        let yerel_depo_dosyasi_id = format!("{}/paketler.bin", self.yerel_depo_base_resource_id);

        let serialized_data = postcard::to_postcard(&paketler)
             .map_err(|e| {
                  eprintln!("Yerel depo için paket listesi serileştirme hatası: {:?}", e);
                  PaketYoneticisiHatasi::SerializationError(e)
             })?;

         match resource::acquire(
             &yerel_depo_dosyasi_id,
             resource::MODE_WRITE | resource::MODE_CREATE | resource::MODE_TRUNCATE
         ) {
             Ok(handle) => {
                 let write_result = resource::write(handle, &serialized_data);
                 let release_result = resource::release(handle);
                 if let Err(e) = write_result { eprintln!("Yerel depo Kaynağı yazma hatası: {:?}", e); return Err(PaketYoneticisiHatasi::from(e)); }
                 if let Err(e) = release_result { eprintln!("Yerel depo Kaynağı release hatası: {:?}", e); }
                 println!("Yerel depo başarıyla güncellendi: {}", yerel_depo_dosyasi_id);
                 Ok(())
             }
             Err(e) => {
                 eprintln!("Yerel depo Kaynağı acquire hatası ({}): {:?}", yerel_depo_dosyasi_id, e);
                 Err(PaketYoneticisiHatasi::from(e))
             }
         }
    }

    // Paket Arama (Paket Adına Göre).
    pub fn paket_ara(&mut self, paket_adi: &str) -> Result<Option<Paket>, PaketYoneticisiHatasi> {
        let paketler = self.paket_listesini_al()?;

        let bulunan_paket = paketler.into_iter().find(|paket| paket.ad == paket_adi);

        Ok(bulunan_paket)
    }
}

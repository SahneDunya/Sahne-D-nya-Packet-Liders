#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // HashMap, String, Vec, format! için

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format; // format! makrosu için
use alloc::boxed::Box; // Hata sarmalamak için gerekebilir (std::error::Error yerine PaketYoneticisiHatasi)
use alloc::collections::HashMap; // std::collections::HashMap yerine

// serde ve no_std uyumlu serileştirme/deserileştirme kütüphanesi
use serde::{Deserialize, Serialize};
// serde_json std gerektirir, postcard kullanacağız.
// use serde_json;
use postcard; // no_std uyumlu binary serileştirme
use postcard::Error as PostcardError; // Postcard hata türü

// Paket struct tanımını içeren modül
use crate::package::Paket; // Varsayım: Paket struct'ı srcpackage.rs'de tanımlı

// Sahne64 API modülleri
use crate::resource; // Ağ ve dosya sistemi benzeri işlemler için
use crate::SahneError; // Sahne64 hata türü
use crate::Handle; // Kaynak Handle'ları

// Özel hata enum'ımızı içe aktar (no_std uyumlu hali)
use crate::paket_yoneticisi_hata::PaketYoneticisiHatasi;
// SahneError ve PostcardError'dan dönüşüm From implementasyonları ile sağlanacak

// no_std uyumlu print makroları
use crate::print_macros::{println, eprintln};

// Helper fonksiyon: Kaynaktan tüm içeriği Vec<u8> olarak oku.
// read_resource_to_string vardı, şimdi binary okuyan lazım.
// Kaynaktan tüm içeriği Vec<u8> olarak oku.
fn read_resource_to_vec(resource_id: &str) -> Result<Vec<u8>, PaketYoneticisiHatasi> {
    let handle = resource::acquire(resource_id, resource::MODE_READ)
        .map_err(|e| {
             eprintln!("Helper: Kaynak acquire hatası ({}): {:?}", resource_id, e);
             PaketYoneticisiHatasi::from(e) // SahneError -> PaketYoneticisiHatasi
        })?;

    let mut buffer = Vec::new(); // Kaynak içeriğini tutacak tampon (alloc::vec::Vec)
    let mut temp_buffer = [0u8; 512]; // Okuma tamponu (stack'te)

    // Kaynağın tüm içeriğini oku (parça parça okuma döngüsü)
    loop {
        match resource::read(handle, &mut temp_buffer) {
            Ok(0) => break, // Kaynak sonu
            Ok(bytes_read) => {
                buffer.extend_from_slice(&temp_buffer[..bytes_read]);
            }
            Err(e) => {
                // Okuma hatası durumunda handle'ı serbest bırakıp hata dön
                let _ = resource::release(handle);
                 eprintln!("Helper: Kaynak okuma hatası ({}): {:?}", resource_id, e);
                return Err(PaketYoneticisiHatasi::from(e)); // SahneError -> PaketYoneticisiHatasi
            }
        }
    }

    // Handle'ı serbest bırak
    let release_result = resource::release(handle);
     if let Err(e) = release_result {
          eprintln!("Helper: Kaynak release hatası ({}): {:?}", resource_id, e);
          // Release hatası kritik olmayabilir, loglayıp devam edebiliriz.
     }

    Ok(buffer) // Vec<u8> olarak döndür
}


// Depo Yöneticisi Yapısı (Paket Deposunu Yönetir)
pub struct DepoYoneticisi {
    // Paket deposunun temel Kaynak ID'si (örn. "sahne://remotepkgrepo/packages/")
    pub depo_base_resource_id: String,
    // Yerel depolama veya önbellek dizininin Kaynak ID'si (örn. "sahne://cache/repo/")
    pub yerel_depo_base_resource_id: String,
    // Paket listesi önbelleği (bellek içi)
    paket_listesi_cache: Option<Vec<Paket>>, // Vec<Paket> alloc gerektirir
}

impl DepoYoneticisi {
    pub fn yeni(depo_base_resource_id: String, yerel_depo_base_resource_id: String) -> Self {
        DepoYoneticisi {
            depo_base_resource_id,
            yerel_depo_base_resource_id,
            paket_listesi_cache: None, // Başlangıçta önbellek boş
        }
    }

    // Paket Listesini İndirme (Uzak Kaynaktan) ve Önbelleğe Alma.
    // Öncelik: Bellek içi cache -> Yerel depo Kaynağı -> Uzak depo Kaynağı.
    // Önbellek mekanizması ile birlikte çalışır.
    // Dönüş değeri: Paketlerin listesi veya hata.
    pub fn paket_listesini_al(&mut self) -> Result<Vec<Paket>, PaketYoneticisiHatasi> { // Result türü PaketYoneticisiHatasi olmalı
        // 1. Bellek içi cache kontrolü
        if let Some(ref paketler) = self.paket_listesi_cache {
            println!("Bellek içi önbellekten paket listesi kullanılıyor.");
            return Ok(paketler.clone()); // Vec<Paket> klonlama (alloc gerektirir)
        }

        // 2. Yerel depo Kaynağı kontrolü (önbellekteki paketler.bin gibi dosya)
        let yerel_paket_listesi_id = format!("{}/paketler.bin", self.yerel_depo_base_resource_id); // format! alloc gerektirir
        match read_resource_to_vec(&yerel_paket_listesi_id) { // Helper fonksiyonu kullan
             Ok(buffer) => {
                 // Kaynak bulundu, deserialize et.
                 match postcard::from_bytes_copy::<Vec<Paket>>(&buffer) { // Postcard deserialize (alloc gerektirir)
                     Ok(paketler) => {
                         println!("Yerel depo Kaynağından paket listesi yüklendi: {}", yerel_paket_listesi_id);
                         self.paket_listesi_cache = Some(paketler.clone()); // Bellek içi önbelleğe kaydet (alloc)
                         return Ok(paketler);
                     }
                     Err(e) => {
                         // Deserialize hatası. Hata logla ve uzak depodan indirmeyi dene.
                         eprintln!("Yerel paket listesi deserialize hatası (Kaynak: {}): {:?}", yerel_paket_listesi_id, e); // no_std print
                         // Deserialize hatası durumunda uzak depodan indirmeye devam edelim.
                     }
                 }
             }
             Err(PaketYoneticisiHatasi::SahneApiError(SahneError::ResourceNotFound)) => {
                 // Yerel depo Kaynağı bulunamadı. Uzak depodan indirmeye devam et.
                 println!("Yerel paket listesi Kaynağı bulunamadı ({}).", yerel_paket_listesi_id); // no_std print
             }
             Err(e) => {
                  // Diğer kaynak okuma hataları. Hata logla ve uzak depodan indirmeyi dene.
                  eprintln!("Yerel paket listesi Kaynağı okuma hatası ({}): {:?}", yerel_paket_listesi_id, e); // no_std print
                  // Hata durumunda uzak depodan indirmeye devam edelim.
             }
        }


        // 3. Yerel önbellekte yoksa, uzak depodan indir
        println!("Uzak depodan paket listesi indiriliyor: {}", self.depo_base_resource_id); // no_std print
        // Paket listesi genellikle belirli bir isimde bir dosyadır (örn. paketler.json veya paketler.bin)
        let uzak_paket_listesi_id = format!("{}/paketler.bin", self.depo_base_resource_id); // Uzak Kaynak ID'si (binary varsayalım)

        // Uzak Kaynaktan binary veriyi oku (ağ Kaynağı varsayımı)
        // read_resource_to_vec helper'ı kullan
        let buffer = read_resource_to_vec(&uzak_paket_listesi_id)?; // Hata otomatik PaketYoneticisiHatasi'na maplenir

        // İndirilen binary veriyi Vec<Paket> struct'ına deserialize et (postcard ile)
        match postcard::from_bytes_copy::<Vec<Paket>>(&buffer) { // Deserialize (alloc gerektirir)
            Ok(paketler) => {
                println!("Paket listesi uzak depodan başarıyla indirildi ve çözümlendi."); // no_std print
                self.paket_listesi_cache = Some(paketler.clone()); // Bellek içi önbelleğe kaydet (alloc)

                // Başarıyla indirildiyse, yerel depo Kaynağına da kaydet (güncelle)
                // Bu, yerel_depoyu_guncelle fonksiyonunun mantığına benzer.
                let yerel_depo_dosyasi_id = format!("{}/paketler.bin", self.yerel_depo_base_resource_id); // format! alloc
                // Yapılandırma verisi yazma mantığına benzer: resource::acquire(WRITE|CREATE|TRUNCATE), resource::write
                 let serialized_data = match postcard::to_postcard(&paketler) { // Serialize (alloc)
                     Ok(data) => data,
                     Err(e) => {
                         eprintln!("Yerel depo için paket listesi serileştirme hatası: {:?}", e); // Logla
                         // Serileştirme hatası kritik değil, listeyi yine de döndürelim.
                         // Ama hatayı da döndürebiliriz, hata yönetimi stratejisine bağlı.
                         // Loglayıp devam edelim.
                         return Err(PaketYoneticisiHatasi::from(e)); // Eğer hata durumunda işlemi durduracaksak
                         let empty_vec: Vec<u8> = Vec::new(); // Boş Vec<u8> alloc
                         empty_vec // Hata durumunda boş data kullan (veya logla)
                     }
                 };

                 // Yerel depo Kaynağını yazma izniyle aç/oluştur/sil
                 match resource::acquire(
                     &yerel_depo_dosyasi_id,
                     resource::MODE_WRITE | resource::MODE_CREATE | resource::MODE_TRUNCATE
                 ) {
                     Ok(handle) => {
                         let write_result = resource::write(handle, &serialized_data); // Yaz (alloc)
                         let release_result = resource::release(handle); // Serbest bırak
                         if let Err(e) = write_result { eprintln!("Yerel depo Kaynağı yazma hatası: {:?}", e); } // Logla
                         if let Err(e) = release_result { eprintln!("Yerel depo Kaynağı release hatası: {:?}", e); } // Logla
                         println!("Paket listesi yerel depo Kaynağına kaydedildi: {}", yerel_depo_dosyasi_id); // no_std print
                     }
                     Err(e) => {
                         eprintln!("Yerel depo Kaynağı acquire hatası ({}): {:?}", yerel_depo_dosyasi_id, e); // Logla
                         // Kaynak acquire hatası durumunda da listeyi yine de döndürelim.
                          return Err(PaketYoneticisiHatasi::from(e)); // Eğer hata durumunda işlemi durduracaksak
                     }
                 }

                Ok(paketler) // Başarılı
            }
            Err(e) => {
                eprintln!("Uzak depodan indirilen paket listesi deserialize hatası: {:?}", e); // no_std print
                // Deserialize hatasını PaketYoneticisiHatasi türünde döndür.
                Err(PaketYoneticisiHatasi::DeserializationError(e)) // PostcardError -> PaketYoneticisiHatasi
            }
        }
    }

    // Yerel Depoyu Güncelleme (Şimdi paket_listesini_al fonksiyonunun içinde yapılıyor)
    // Bu fonksiyon ayrı bir işlem olarak paket listesini indirip yerel depoya kaydetmek için kullanılabilir.
    pub fn yerel_depoyu_guncelle(&mut self) -> Result<(), PaketYoneticisiHatasi> { // Result türü PaketYoneticisiHatasi olmalı
        println!("Yerel depo güncelleniyor: {}", self.yerel_depo_base_resource_id); // no_std print

        // Paket listesini indir veya önbellekten al (bu aynı zamanda belleğe yükler)
        let paketler = self.paket_listesini_al()?; // Hata otomatik PaketYoneticisiHatasi'na maplenir

        // İndirilen (veya cache'teki) listeyi yerel depo Kaynağına kaydet
        let yerel_depo_dosyasi_id = format!("{}/paketler.bin", self.yerel_depo_base_resource_id); // format! alloc

        let serialized_data = postcard::to_postcard(&paketler) // Serialize (alloc)
             .map_err(|e| {
                  eprintln!("Yerel depo için paket listesi serileştirme hatası: {:?}", e); // Logla
                  PaketYoneticisiHatasi::SerializationError(e) // Hata dön
             })?;

         match resource::acquire(
             &yerel_depo_dosyasi_id,
             resource::MODE_WRITE | resource::MODE_CREATE | resource::MODE_TRUNCATE
         ) {
             Ok(handle) => {
                 let write_result = resource::write(handle, &serialized_data); // Yaz (alloc)
                 let release_result = resource::release(handle); // Serbest bırak
                 if let Err(e) = write_result { eprintln!("Yerel depo Kaynağı yazma hatası: {:?}", e); return Err(PaketYoneticisiHatasi::from(e)); } // Logla ve hata dön
                 if let Err(e) = release_result { eprintln!("Yerel depo Kaynağı release hatası: {:?}", e); } // Logla
                 println!("Yerel depo başarıyla güncellendi: {}", yerel_depo_dosyasi_id); // no_std print
                 Ok(())
             }
             Err(e) => {
                 eprintln!("Yerel depo Kaynağı acquire hatası ({}): {:?}", yerel_depo_dosyasi_id, e); // Logla
                 Err(PaketYoneticisiHatasi::from(e)) // SahneError -> PaketYoneticisiHatasi
             }
         }
    }


    // Paket Arama (Paket Adına Göre)
    // Paket listesini alır (bellek içi cache, yerel veya uzaktan) ve arama yapar.
    pub fn paket_ara(&mut self, paket_adi: &str) -> Result<Option<Paket>, PaketYoneticisiHatasi> { // Result türü PaketYoneticisiHatasi olmalı
        let paketler = self.paket_listesini_al()?; // Paket listesini al (hata otomatik maplenir)

        // Paketin adını arayarak listeyi gez
        let bulunan_paket = paketler.into_iter().find(|paket| paket.ad == paket_adi); // find iteratör metodu
        // find metodu ownership'i alır, eğer paketler listesini sonra tekrar kullanacaksak klonlamalıyız.
        // veya paketlere referans alıp find(|paket| paket.ad == paket_adi) yapmalıyız.
         let bulunan_paket = paketler.iter().find(|paket| paket.ad == paket_adi).cloned(); // Referans alıp klonla

        Ok(bulunan_paket) // Option<Paket> döndürülür
    }
}

// --- Paket Struct Tanımı ---
// Orijinal srclib.rs'deki Paket struct tanımı buraya kopyalanmalı veya srcpackage.rs modülünde olmalı.
// srcpackage.rs modülünde olduğunu varsayalım.

#![no_std]
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::HashMap; // Eğer Paket içinde HashMap varsa

use serde::{Deserialize, Serialize}; // Serde derive'lar (no_std uyumlu backend ile çalışır)

#[derive(Serialize, Deserialize, Debug, Clone)] // Gerekli derive'lar (alloc ile no_std'de çalışır)
pub struct Paket {
    pub ad: String,
    pub surum: String, // Sürüm string olarak saklanıyor
    pub bagimliliklar: Vec<String>, // Bu paketin ihtiyaç duyduğu diğer paketlerin ADLARI listesi
    pub aciklama: Option<String>, // Açıklama isteğe bağlı
    pub dosya_adi: Option<String>, // Paketin arşiv dosyasının adı (örn. "my_package-1.0.0.zip")
    // Diğer meta veriler eklenebilir (yazar, lisans, sağlama toplamı vb.)
    // pub checksum_md5: Option<String>, // Checksum için
    // pub dosyalar: Vec<String>, // Kurulum sırasında kopyalanacak dosyaların listesi
    // pub kurulum_scripti: Option<String>, // Kurulum scripti Kaynak ID'si veya içeriği
}

impl Paket {
    pub fn yeni(ad: String, surum: String, bagimliliklar: Vec<String>) -> Self {
        Paket {
            ad,
            surum,
            bagimliliklar,
            aciklama: None,
            dosya_adi: None,
            // ... diğer alanlar default/None
        }
    }
}

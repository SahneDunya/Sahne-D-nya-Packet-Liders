#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc ve postcard kullanacağız)
extern crate alloc; // String, Vec, format! için

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use alloc::boxed::Box; // postcard'ın dönüş türleri Box kullanabilir

use serde::{Deserialize, Serialize};
use postcard; // no_std uyumlu serileştirme/deserileştirme
use postcard::Error as PostcardError; // Postcard hatasını yeniden adlandır

// Sahne64 API modüllerini içe aktarın
use crate::resource;
use crate::SahneError;
use crate::Handle;

// Özel hata enum'ımızı içe aktar (no_std uyumlu ve SahneError'ı içeren haliyle)
use crate::paket_yoneticisi_hata::PaketYoneticisiHatasi;
// SahneError ve PostcardError'dan dönüşüm From implementasyonları ile sağlanacak

// Yapılandırma verilerini tutan struct. Serileştirme/Deserileştirme yapılabilir.
#[derive(Serialize, Deserialize, Debug)] // Serde derive makroları (no_std uyumlu serde backend ile çalışır)
pub struct Yapilandirma {
    pub depo_url: String,
    pub yerel_depo_yolu: String, // Bu artık Sahne64 Kaynak ID formatında olmalı
    pub kurulum_dizini: String, // Bu artık Sahne64 Kaynak ID formatında olmalı
    pub onbellek_dizini: String, // Bu artık Sahne64 Kaynak ID formatında olmalı
}

impl Yapilandirma {
    pub fn yeni(depo_url: String, yerel_depo_yolu: String, kurulum_dizini: String, onbellek_dizini: String) -> Yapilandirma {
        Yapilandirma {
            depo_url,
            // Bu yollar artık Sahne64 Kaynak ID'leri olarak düşünülmeli
            yerel_depo_yolu,
            kurulum_dizini,
            onbellek_dizini,
        }
    }

    // Yapılandırma verilerini belirtilen Kaynak ID'sinden okur ve deserialize eder.
    // resource_id: Yapılandırma verilerini içeren Kaynağın ID'si (örn. "sahne://config/paket_yoneticisi.bin")
    // Result türü PaketYoneticisiHatasi olmalı
    pub fn oku(resource_id: &str) -> Result<Yapilandirma, PaketYoneticisiHatasi> {
        // Kaynağı oku (sadece okuma izniyle)
        let handle = resource::acquire(resource_id, resource::MODE_READ)
            .map_err(|e| {
                 eprintln!("Yapılandırma Kaynağı acquire hatası ({}): {:?}", resource_id, e);
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
                     eprintln!("Yapılandırma Kaynağı okuma hatası ({}): {:?}", resource_id, e);
                    return Err(PaketYoneticisiHatasi::from(e)); // SahneError -> PaketYoneticisiHatasi
                }
            }
        }

        // Handle'ı serbest bırak
        let release_result = resource::release(handle);
         if let Err(e) = release_result {
              eprintln!("Yapılandırma Kaynağı release hatası ({}): {:?}", resource_id, e);
              // Release hatası kritik olmayabilir, loglayıp devam edebiliriz.
         }


        // Tampondaki binary veriyi Yapilandirma struct'ına deserialize et (postcard ile)
        // postcard::from_bytes Result<'a, T, Error> döner. T Yapilandirma struct'ımız.
        // Lifetime 'a, buffer'ın lifetime'ı olmalı. Veya from_bytes_copy kullanmalı.
        // from_bytes_copy daha uygundur no_std'de.
        match postcard::from_bytes_copy::<Yapilandirma>(&buffer) {
            Ok(yapilandirma) => Ok(yapilandirma),
            Err(e) => {
                eprintln!("Yapılandırma verisi deserialize hatası ({}): {:?}", resource_id, e);
                Err(PaketYoneticisiHatasi::from(e)) // PostcardError -> PaketYoneticisiHatasi
            }
        }
    }

    // Yapılandırma verilerini serialize eder ve belirtilen Kaynak ID'sine yazar.
    // resource_id: Yapılandırma verilerinin yazılacağı Kaynağın ID'si.
    // Result türü PaketYoneticisiHatasi olmalı
    pub fn yaz(&self, resource_id: &str) -> Result<(), PaketYoneticisiHatasi> {
        // Yapılandırma struct'ını binary veriye serialize et (postcard ile)
        // to_postcard Result<Vec<u8>, Error> veya Result<Box<[u8]>, Error> döner.
        // Vec kullanmak alloc gerektirir.
        let serialized_data = match postcard::to_postcard(self) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Yapılandırma verisi serialize hatası: {:?}", e);
                return Err(PaketYoneticisiHatasi::from(e)); // PostcardError -> PaketYoneticisiHatasi
            }
        };

        // Kaynağın parent dizinlerini/kaynaklarını oluştur (Sahne64 modeline göre)
        // Yapılandırma dosyası genellikle belirli bir yerde olur ("sahne://config/"),
        // bu yolun varlığını sağlamak gerekebilir.
        // sahne_create_resource_recursive helper fonksiyonu kullanılabilir.
        // Varsayım: Yapılandırma Kaynağının parent'ı ("sahne://config/") zaten mevcuttur
        // veya resource::acquire(..., MODE_CREATE) parent'ı da oluşturur.
        // Eğer oluşturmuyorsa:
         let parent_resource_id = resource_id.rfind('/').map(|idx| &resource_id[..idx]).unwrap_or("");
         if !parent_resource_id.is_empty() {
              sahne_create_resource_recursive(resource_id)?; // resource_id kendisi değil, parent'ları için
        //     // Doğrudan parent'ı acquire etmeyi deneyelim MODE_CREATE ile:
              match resource::acquire(parent_resource_id, resource::MODE_CREATE) {
                   Ok(handle) => { let _ = resource::release(handle); },
                   Err(e) => { eprintln!("Yapılandırma Kaynağı parent acquire hatası ({}): {:?}", parent_resource_id, e); return Err(PaketYoneticisiHatasi::from(e)); }
              }
         }


        // Kaynağı yazma izniyle oluştur/aç (varsa sil)
        // MODE_TRUNCATE içeriği siler, bu yeni yapılandırmayı yazmak için uygun.
        match resource::acquire(
            resource_id,
            resource::MODE_WRITE | resource::MODE_CREATE | resource::MODE_TRUNCATE
        ) {
            Ok(handle) => {
                // Binary veriyi Kaynağa yaz
                let write_result = resource::write(handle, &serialized_data); // Vec<u8> dilime dönüştürüldü

                // Handle'ı serbest bırak
                let release_result = resource::release(handle);

                // Yazma veya release hatalarını kontrol et
                if let Err(e) = write_result {
                    eprintln!("Yapılandırma Kaynağı yazma hatası ({}): {:?}", resource_id, e);
                    // Yazma başarısızsa release'in sonucunu yoksayabiliriz.
                    return Err(PaketYoneticisiHatasi::from(e)); // SahneError -> PaketYoneticisiHatasi
                }
                 if let Err(e) = release_result {
                      eprintln!("Yapılandırma Kaynağı release hatası ({}): {:?}", resource_id, e);
                      // Release hatası loglanmalı ama write başarılıysa Ok dönebiliriz?
                      // Hata yönetim stratejisine bağlı. Şimdilik loglayalım.
                 }


                Ok(()) // Hem yazma hem release başarılıysa
            }
            Err(e) => {
                 eprintln!("Yapılandırma Kaynağı acquire hatası ({}): {:?}", resource_id, e);
                Err(PaketYoneticisiHatasi::from(e)) // SahneError -> PaketYoneticisiHatasi
            }
        }
    }
}

// --- PaketYoneticisiHatasi enum tanımının no_std uyumlu hale getirilmesi ---
// (paket_yoneticisi_hata.rs dosyasında veya ilgili modülde olmalı)

// paket_yoneticisi_hata.rs (Örnek - no_std uyumlu, Postcard hatası eklenmiş)
#![no_std]
extern crate alloc;

use alloc::string::String;
use alloc::format;
use crate::SahneError;
use postcard::Error as PostcardError; // Postcard hata türü

// ... (ZipHatasi, PaketBulunamadi, PaketKurulumHatasi, OnbellekHatasi, ChecksumResourceError, GecersizParametre, PathTraversalHatasi) ...

#[derive(Debug)]
pub enum PaketYoneticisiHatasi {
    // ... (Çeşitli hatalar) ...

    // Sahne64 API'sından gelen genel hatalar
    SahneApiHatasi(SahneError),

    // Serileştirme/Deserileştirme hataları için
    SerializationError(PostcardError),
    DeserializationError(PostcardError),

    // ... diğer hatalar ...
    BilinmeyenHata,
}

// SahneError'dan PaketYoneticisiHatasi::SahneApiHatasi'na dönüşüm
// Bu From implementasyonu genel SahneApiHatasi için kullanılabilir.
impl From<SahneError> for PaketYoneticisiHatasi {
    fn from(err: SahneError) -> Self {
        // SahneError'ın kaynağını inceleyip daha spesifik hatalara yönlendirme yapılabilir.
        // Örn: match err { SahneError::ResourceNotFound => PaketYoneticisiHatasi::YapilandirmaDosyasiBulunamadi }
        match err {
            // Kaynakla ilgili hataları (özellikle config dosyası okurken/yazarken)
            // daha spesifik bir YapilandirmaResourceError'a yönlendirebiliriz,
            // veya genel SahneApiHatasi içinde bırakabiliriz.
            // Basitlik adına genel SahneApiHatasi içinde tutalım ve loglarken detay verelim.
             _ => PaketYoneticisiHatasi::SahneApiHatasi(err),
        }
    }
}

// PostcardError'dan PaketYoneticisiHatasi'na dönüşüm
impl From<PostcardError> for PaketYoneticisiHatasi {
    fn from(err: PostcardError) -> Self {
        // Postcard hata türleri genellikle serileştirme veya deserileştirme sırasında
        // yaşanan sorunları belirtir.
        match err {
            // Postcard'ın iç hata tiplerine göre daha detaylı ayırabiliriz.
            // Şimdilik genel olarak ayırıyoruz.
            PostcardError::Serialize(_) => PaketYoneticisiHatasi::SerializationError(err),
            _ => PaketYoneticisiHatasi::DeserializationError(err), // Kalanlar deserialization hatası sayılır
        }
    }
}

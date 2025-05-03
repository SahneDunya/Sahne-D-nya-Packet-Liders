#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // String, Vec, format! için

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format; // Hata mesajları için
use alloc::borrow::ToOwned; // &str -> String için

// Paket struct tanımını içeren modül
use crate::package::Paket;
// Sahne64 resource modülü
use crate::resource;
use crate::SahneError; // Sahne64 hata türü
use crate::Handle; // Kaynak Handle'ları

// Özel hata enum'ımızı içe aktar (no_std uyumlu hali)
use crate::paket_yoneticisi_hata::PaketYoneticisiHatasi;
// SahneError'dan dönüşüm From implementasyonu ile sağlanacak

// ZIP arşiv işlemleri modülü
use crate::srcarchive;

// no_std uyumlu print makroları
use crate::print_macros::{println, eprintln};


// Paket kurulum ve indirme işlemlerini yöneten yapı.
pub struct KurulumYoneticisi {
    // Paket deposunun temel Kaynak ID'si (örn. "sahne://remotepkgrepo/packages/")
    pub paket_deposu_base_resource_id: String,
    // Kurulu paketlerin temel Kaynak ID'si (örn. "sahne://installed_packages/")
    pub kurulum_base_resource_id: String,
    // Önbellek temel Kaynak ID'si (örn. "sahne://cache/packages/") - İndirilen paketler buraya kaydedilecek
    pub onbellek_base_resource_id: String,
}

impl KurulumYoneticisi {
    // Yeni bir KurulumYoneticisi oluşturur.
    // paket_deposu_base_resource_id: Uzak deponun ID'si.
    // kurulum_base_resource_id: Paketlerin kurulacağı yerin ID'si.
    // onbellek_base_resource_id: Paketlerin indirileceği/saklanacağı önbellek ID'si.
    pub fn yeni(
        paket_deposu_base_resource_id: String,
        kurulum_base_resource_id: String,
        onbellek_base_resource_id: String,
    ) -> Self {
        KurulumYoneticisi {
            paket_deposu_base_resource_id,
            kurulum_base_resource_id,
            onbellek_base_resource_id,
        }
    }

    // Paketi uzak depodan önbelleğe indirir.
    // paket: İndirilecek paketin meta verisi (Paket struct'ı).
    // Dönüş değeri: Başarı veya PaketYoneticisiHatasi.
    pub fn paketi_indir(&self, paket: &Paket) -> Result<(), PaketYoneticisiHatasi> { // Result türü PaketYoneticisiHatasi olmalı
        // Paketin dosya adını al (Paket struct'ında Option<String> olduğunu varsayarak)
        if let Some(dosya_adi) = &paket.dosya_adi { // dosya_adi Option<String>
            // Uzak depodaki kaynak ID'sini oluştur (örn. "sahne://remotepkgrepo/packages/my_package.zip")
            let paket_kaynak_id = format!("{}/{}", self.paket_deposu_base_resource_id, dosya_adi); // format! alloc gerektirir

            // Önbellekteki hedef kaynak ID'sini oluştur (örn. "sahne://cache/packages/my_package.zip")
            let onbellek_hedef_id = format!("{}/{}", self.onbellek_base_resource_id, dosya_adi); // format! alloc gerektirir

            println!("Paket indirme başlatılıyor: {} -> {}", paket_kaynak_id, onbellek_hedef_id);

            // Uzak Kaynağı oku (ağ resource tipi varsayımı)
            // Sahne64 API'sında ağ iletişimi resource::acquire(URL, MODE_READ) ile mi yapılıyor?
            // Veya özel bir ağ resource tipi mi var?
            // Varsayım: paket_kaynak_id bir ağ Kaynağı ID'sidir ve resource::acquire ile Handle alınabilir.
            let kaynak_handle = resource::acquire(&paket_kaynak_id, resource::MODE_READ)
                .map_err(|e| {
                     eprintln!("Paket Kaynağı acquire hatası ({}): {:?}", paket_kaynak_id, e);
                    PaketYoneticisiHatasi::from(e) // SahneError -> PaketYoneticisiHatasi (Network hatası da olabilir SahneError içinde)
                })?;

            // Önbellekteki Hedef Kaynağı yaz (dosya sistemi benzeri resource tipi)
            // MODE_CREATE varsa Kaynak yoksa oluşturur, MODE_TRUNCATE varsa içeriği siler.
            // onbellek_hedef_id'nin parent resource'larının oluşturulması gerekebilir.
            // srcconfig.rs'deki sahne_create_resource_recursive helper'ı burada kullanılabilir.
            // Veya resource::acquire(..., MODE_CREATE) parent'ları otomatik oluşturur varsayılır.
            // Şimdilik otomatik oluştuğunu varsayalım veya helper'ı kullanmayalım.
              let _ = super::sahne_create_resource_recursive(&onbellek_hedef_id)?; // Eğer helper varsa

            let hedef_handle = resource::acquire(
                &onbellek_hedef_id,
                resource::MODE_WRITE | resource::MODE_CREATE | resource::MODE_TRUNCATE
            ).map_err(|e| {
                 eprintln!("Önbellek Hedef Kaynağı acquire hatası ({}): {:?}", onbellek_hedef_id, e);
                PaketYoneticisiHatasi::from(e) // SahneError -> PaketYoneticisiHatasi
            })?;

            // Kaynaktan oku ve hedefe yaz döngüsü
            let mut buffer = [0u8; 4096]; // Okuma/yazma buffer'ı (stack'te)
            loop {
                // Kaynak Kaynağından oku
                match resource::read(kaynak_handle, &mut buffer) {
                    Ok(0) => break, // Kaynak sonu (indirme tamamlandı)
                    Ok(bytes_read) => {
                        // Hedef Kaynağa yaz
                        match resource::write(hedef_handle, &buffer[..bytes_read]) {
                            Ok(_) => {
                                // Başarılı yazma
                            }
                            Err(e) => {
                                eprintln!("Önbellek Hedef Kaynağı yazma hatası ({}): {:?}", onbellek_hedef_id, e);
                                let _ = resource::release(kaynak_handle); // Handle'ları temizle
                                let _ = resource::release(hedef_handle);
                                return Err(PaketYoneticisiHatasi::from(e)); // SahneError -> PaketYoneticisiHatasi
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Paket Kaynağı okuma hatası ({}): {:?}", paket_kaynak_id, e);
                        let _ = resource::release(kaynak_handle); // Handle'ları temizle
                        let _ = resource::release(hedef_handle);
                        return Err(PaketYoneticisiHatasi::from(e)); // SahneError -> PaketYoneticisiHatasi
                    }
                }
            }

            // Handle'ları serbest bırak
            let release_kaynak_result = resource::release(kaynak_handle);
             if let Err(e) = release_kaynak_result { eprintln!("Paket Kaynağı release hatası ({}): {:?}", paket_kaynak_id, e); } // Logla

            let release_hedef_result = resource::release(hedef_handle);
             if let Err(e) = release_hedef_result { eprintln!("Önbellek Hedef Kaynağı release hatası ({}): {:?}", onbellek_hedef_id, e); } // Logla


            println!("Paket indirildi ve önbelleğe kaydedildi: {}", onbellek_hedef_id);
            Ok(())
        } else {
            eprintln!("Paket meta verisinde dosya adı belirtilmemiş: {:?}", paket.ad);
            // Dosya adı belirtilmemişse hata dönelim.
            Err(PaketYoneticisiHatasi::InvalidParameter(format!("Paket '{}' için dosya adı belirtilmemiş.", paket.ad))) // alloc gerektirir
        }
    }

    // Paketi önbellekten kurulum dizinine kurar (çıkarma ve kopyalama).
    // paket: Kurulacak paketin meta verisi.
    // Dönüş değeri: Başarı veya PaketYoneticisiHatasi.
    pub fn paketi_kur(&self, paket: &Paket) -> Result<(), PaketYoneticisiHatasi> { // Result türü PaketYoneticisiHatasi olmalı
        // Paketin dosya adını al
        if let Some(dosya_adi) = &paket.dosya_adi { // dosya_adi Option<String>
            // Önbellekteki paket dosyasının kaynak ID'sini oluştur (örn. "sahne://cache/packages/my_package.zip")
            let onbellek_paket_id = format!("{}/{}", self.onbellek_base_resource_id, dosya_adi); // format! alloc gerektirir

            // Kurulum dizini hedef kaynak ID'sini oluştur (örn. "sahne://installed_packages/my_package/")
            // Zip dosyasının içeriği bu ana dizin altına çıkarılacak.
            // Paket ismi, kurulum_base_resource_id altına bir alt dizin oluşturmak için kullanılabilir.
            let kurulum_hedef_base_id = format!("{}/{}/", self.kurulum_base_resource_id, paket.ad); // format! alloc gerektirir. Sonuna '/' eklemek dizin anlamı katabilir.


            println!("Paket kurulumuna başlanıyor: {:?}", paket.ad);
            println!("Paket önbellek yolu: {}", onbellek_paket_id);
            println!("Kurulum hedef yolu: {}", kurulum_hedef_base_id);

            // Zip arşivini önbellek konumundan kurulum hedef dizinine çıkar (srcarchive modülü kullanılarak)
             srcarchive::zip_ac(arsiv_resource_id: &str, cikartma_base_resource_id: &str)
            match srcarchive::zip_ac(&onbellek_paket_id, &kurulum_hedef_base_id) {
                Ok(_) => {
                    println!("Paket içeriği çıkarıldı ve kuruldu: {:?}", paket.ad);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Paket içeriği çıkarma/kurulum hatası (Kaynak: {}): {:?}", onbellek_paket_id, e);
                    // srcarchive'dan gelen hata zaten PaketYoneticisiHatasi türünde.
                    Err(e)
                }
            }
        } else {
            eprintln!("Paket meta verisinde dosya adı belirtilmemiş: {:?}", paket.ad);
             // Dosya adı belirtilmemişse hata dönelim.
            Err(PaketYoneticisiHatasi::InvalidParameter(format!("Paket '{}' için dosya adı belirtilmemiş.", paket.ad))) // alloc gerektirir
        }
    }

     // Paketi kaldırma fonksiyonu (Eksik fonksiyonellik: resource silme)
     // package_name: Kaldırılacak paketin adı.
     // Dönüş değeri: Başarı veya PaketYoneticisiHatasi.
      pub fn paketi_kaldir(&self, paket_adi: &str) -> Result<(), PaketYoneticisiHatasi> {
          println!("Paket kaldırma başlatılıyor: {}", paket_adi);
     //     // Kurulu paketlerin dosyalarının bulunduğu temel Kaynak ID'sini oluştur (örn. "sahne://installed_packages/my_package/")
          let kurulum_paket_base_id = format!("{}/{}/", self.kurulum_base_resource_id, paket_adi); // format! alloc gerektirir
     //
     //     // Kurulu dosyaları silme mantığı burada olacak.
     //     // Bu, resource::delete veya resource::control kullanımı gerektirir (Sahne64 API'sında eksik).
     //     // Ayrıca, silinecek dosyaların listesi bilinmelidir (paket veritabanından veya paket meta verilerinden).
     //
          eprintln!("UYARI: Sahne64 API'sında resource silme mekanizması bulunmamaktadır. Paket kaldırma tamamlanmamıştır.");
          eprintln!("Silinmek istenen temel Kaynak ID: {}", kurulum_paket_base_id);
     //
     //     // Placeholder: resource::delete(kurulum_paket_base_id)? gibi bir çağrı yapılacak.
     //
     //     // Eğer kaldırma scripti varsa, onu çalıştır (task::spawn).
           const PACKAGE_SCRIPT_BASE_RESOURCE: &str = "sahne://system/package_scripts/"; // srccli.rs'den hatırlatma
           let uninstall_script_resource_id = format!("{}{}/uninstall.sh", PACKAGE_SCRIPT_BASE_RESOURCE, paket_adi);
           match resource::acquire(&uninstall_script_resource_id, resource::MODE_READ) {
                Ok(script_handle) => {
                    let _ = task::spawn(script_handle, b"");
                    let _ = resource::release(script_handle);
                }
                Err(SahneError::ResourceNotFound) => { /* Script yoksa devam et */ }
                Err(e) => { /* Hata yönetimi */ }
           }
     //
     //     // Paket veritabanı kaydını silme (paketi kurulu listesinden çıkarma).
     //     // Bu da paket veritabanını yöneten modül tarafından yapılmalıdır.
     
          Err(PaketYoneticisiHatasi::SahneApiHatasi(SahneError::NotSupported)) // Silme desteği yok hatası
      }
}

// --- Paket struct tanımı (crate::package modülünde olmalı) ---
// Bu struct'ın no_std ve alloc uyumlu olması gerekir.
// `dosya_adi` alanı eklenmeli.

// srcpackage.rs (Güncellenmiş - dosya_adi alanı eklenmiş)
#![no_std]
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Paket {
    pub ad: String,
    pub versiyon: String,
    pub bagimliliklar: Vec<String>,
    pub dosya_adi: Option<String>, // Paketin arşiv dosyasının adı (örn. "my_package-1.0.0.zip")
    // Diğer meta veriler...
}

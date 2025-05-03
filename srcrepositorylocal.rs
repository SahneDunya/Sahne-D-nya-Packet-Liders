#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // String, Vec, format! için

use alloc::string::{String, ToString}; // std::string::String yerine
use alloc::vec::Vec; // std::vec::Vec yerine
use alloc::format; // format! makrosu için
use alloc::borrow::ToOwned; // &str -> String için

// Sahne64 API modülleri
use crate::resource; // Kaynak işlemleri
use crate::SahneError; // Sahne64 hata türü
use crate::Handle; // Kaynak Handle'ları

// Paket struct tanımını içeren modül (package_name, version, file_name için)
use crate::package::Paket; // Varsayım: Paket struct'ı srcpackage.rs'de tanımlı

// Özel hata enum'ımızı içe aktar (no_std uyumlu hali)
use crate::paket_yoneticisi_hatasi::PaketYoneticisiHatasi;
// SahneError'dan dönüşüm From implementasyonu ile sağlanacak

// no_std uyumlu print makroları
use crate::print_macros::{println, eprintln};

// Helper fonksiyon: Sahne64 Kaynağını recursive olarak oluşturur (srcconfig.rs veya utils modülünden)
// Kaynak ID'si bir dizini temsil ediyorsa alt dizinleri de oluşturur.
// srcconfig.rs'den kopyalanmıştır.
fn sahne_create_resource_recursive(resource_id: &str) -> Result<(), PaketYoneticisiHatasi> {
    let mut current_path = String::new(); // alloc gerektirir
    let mut parts = resource_id.split('/'); // &str split no_std
    
    if let Some(scheme) = parts.next() {
        current_path.push_str(scheme); // push_str alloc
        current_path.push_str("://"); // push_str alloc
        
        while let Some(part) = parts.next() {
            if part.is_empty() { continue; }

            current_path.push_str(part); // push_str alloc
            
            // Mevcut path'i resource olarak oluşturmaya çalış
            // Eğer resource zaten varsa hata vermez (MODE_CREATE yetmez, resource::create_container gerekebilir)
            // Varsayım: resource::acquire(..., MODE_CREATE) parent'ları oluşturmaz, sadece son elementi.
            // Veya resource::create_container recursive olabilir.
            // Basitlik adına, her adımı resource::acquire(..., MODE_CREATE) ile deneyelim ve SahneError::ResourceAlreadyExists hatasını görmezden gelelim.
            // Bu tam olarak create_dir_all gibi çalışmaz.
            // Doğru implementasyon için kernel/API seviyesinde recursive create_container veya benzeri gerekir.

            // Daha iyi bir yaklaşım: resource::create_container syscall'ı olduğunu varsayalım.
             let current_path_str = current_path.as_str();
             if let Err(e) = resource::create_container(current_path_str, resource::MODE_READ | resource::MODE_WRITE) { // Mode gerekebilir mi?
                 if e != SahneError::ResourceAlreadyExists {
                     eprintln!("Recursive resource oluşturma hatası ({}): {:?}", current_path_str, e);
                     return Err(PaketYoneticisiHatasi::from(e));
                 }
             }

            // Geçici Çözüm (SahneError::ResourceAlreadyExists'ı görmezden gelerek acquire deneme):
            // resource::acquire(..., MODE_CREATE) sadece son elemanı oluşturur. Directory oluşturmak için özel bir Kaynak tipi gerekebilir.
            // Varsayım: Bir Kaynak ID'sinin sonuna '/' eklemek dizin olduğunu belirtir ve resource::acquire(..., MODE_CREATE) ile oluşturulabilir.
             let container_path = if current_path.ends_with('/') { current_path.clone() } else { format!("{}/", current_path) }; // format! alloc
             match resource::acquire(&container_path, resource::MODE_CREATE | resource::MODE_READ | resource::MODE_WRITE) {
                 Ok(handle) => {
                     let _ = resource::release(handle); // Handle'ı hemen bırak
                 }
                 Err(e) => {
                     // Eğer Kaynak zaten varsa bu bir hata değildir, devam et.
                     if e != SahneError::ResourceAlreadyExists {
                         eprintln!("Recursive resource oluşturma hatası ({}): {:?}", container_path, e); // no_std print
                         return Err(PaketYoneticisiHatasi::from(e)); // SahneError -> PaketYoneticisiHatasi
                     }
                 }
             }


            current_path.push('/'); // "/" ekle
        }
    }

    Ok(()) // Başarı
}


// Yerel paket deposunu yönetir.
// Paketler genellikle `base_resource_id/paket_adi/surum/dosya_adi` yapısında saklanır.
pub struct LocalRepository {
    // Yerel deponun temel Kaynak ID'si (örn. "sahne://cache/repo/")
    pub base_resource_id: String, // String alloc gerektirir.
}

impl LocalRepository {
    // Yeni bir LocalRepository örneği oluşturur.
    // base_resource_id: Yerel deponun kök Kaynak ID'si.
    pub fn new(base_resource_id: String) -> Self {
        LocalRepository { base_resource_id } // String ownership'i alınır.
    }

    // Yerel depoda belirtilen isim ve sürüme sahip bir paketin dosyasının olup olmadığını kontrol eder.
    // package_name: Paketin adı.
    // version: Paketin sürümü.
    // package_file_name: Paketin arşiv dosyasının adı (Paket struct'ından gelir).
    // Dönüş değeri: Boolean (varsa true, yoksa false) veya PaketYoneticisiHatasi.
    pub fn has_package(
        &self,
        package_name: &str,
        version: &str,
        package_file_name: &str, // Dosya adını da bilmek gerekir
    ) -> Result<bool, PaketYoneticisiHatasi> { // Result türü PaketYoneticisiHatasi olmalı
        // Paket dosyasının tam Kaynak ID'sini oluştur.
        let package_file_resource_id = format!("{}/{}/{}/{}", self.base_resource_id, package_name, version, package_file_name); // format! alloc

        debug!("Yerel depoda paket kontrol ediliyor: {}", package_file_resource_id); // no_std log

        // Kaynağı okuma izniyle acquire etmeyi dene.
        // Eğer SahneError::ResourceNotFound dönerse paket yoktur. Diğer hatalar gerçek bir hata.
        match resource::acquire(&package_file_resource_id, resource::MODE_READ) {
            Ok(handle) => {
                // Kaynak bulundu, Handle'ı serbest bırak ve true dön.
                let _ = resource::release(handle);
                Ok(true)
            }
            Err(SahneError::ResourceNotFound) => {
                // Kaynak bulunamadı, paket yok.
                Ok(false)
            }
            Err(e) => {
                // Diğer Sahne64 hataları (izin yok, bozuk resource vb.)
                 eprintln!("Yerel depoda paket kontrol hatası (Kaynak: {}): {:?}", package_file_resource_id, e); // no_std print
                Err(PaketYoneticisiHatasi::from(e)) // SahneError -> PaketYoneticisiHatasi
            }
        }
    }

    // Yerel depodan belirtilen isim ve sürüme sahip bir paketin dosyasının Kaynak ID'sini alır.
    // package_name: Paketin adı.
    // version: Paketin sürümü.
    // package_file_name: Paketin arşiv dosyasının adı.
    // Dönüş değeri: Paketin Kaynak ID'si (varsa) veya PaketYoneticisiHatasi.
    pub fn get_package(
        &self,
        package_name: &str,
        version: &str,
        package_file_name: &str, // Dosya adını da bilmek gerekir
    ) -> Result<Option<String>, PaketYoneticisiHatasi> { // Option<PathBuf> yerine Option<String>
        // Paket dosyasının tam Kaynak ID'sini oluştur.
        let package_file_resource_id = format!("{}/{}/{}/{}", self.base_resource_id, package_name, version, package_file_name); // format! alloc

        debug!("Yerel depodan paket yolu alınıyor: {}", package_file_resource_id); // no_std log

        // Kaynağın varlığını kontrol et (has_package gibi).
        match resource::acquire(&package_file_resource_id, resource::MODE_READ) {
             Ok(handle) => {
                 let _ = resource::release(handle);
                 // Kaynak bulundu, ID'sini String olarak dön.
                 Ok(Some(package_file_resource_id)) // String alloc
             }
             Err(SahneError::ResourceNotFound) => {
                 // Kaynak bulunamadı.
                 Ok(None)
             }
             Err(e) => {
                 // Diğer Sahne64 hataları.
                  eprintln!("Yerel depodan paket yolu alma hatası (Kaynak: {}): {:?}", package_file_resource_id, e); // no_std print
                 Err(PaketYoneticisiHatasi::from(e)) // SahneError -> PaketYoneticisiHatasi
             }
         }
    }


    // Belirtilen kaynak Kaynağından paketi okur ve yerel depoya kopyalar.
    // Paketler `base_resource_id/paket_adi/surum/dosya_adi` yapısında kaydedilir.
    // source_resource_id: Kopyalanacak paket arşiv dosyasının Kaynak ID'si.
    // paket: Paketin meta verisi (ad, sürüm, dosya_adi içerir).
    // Dönüş değeri: Başarı veya PaketYoneticisiHatasi.
    pub fn add_package(&self, source_resource_id: &str, paket: &Paket) -> Result<(), PaketYoneticisiHatasi> { // Path yerine &str Kaynak ID, Paket struct'ı
        // Paket adı ve sürümden hedef dizin Kaynak ID'sini oluştur.
        let destination_dir_resource_id = format!("{}/{}/{}/", self.base_resource_id, paket.ad, paket.surum); // format! alloc. Sonuna '/' eklemek dizin olduğunu belirtebilir.

        // Hedef dizini recursive olarak oluştur (Sahne64 API desteği gerekiyor veya helper kullan).
        println!("Yerel depo için hedef dizin oluşturuluyor: {}", destination_dir_resource_id); // no_std print
        sahne_create_resource_recursive(&destination_dir_resource_id)?; // Helper fonksiyonu kullan

        // Paketin dosya adını al (Paket struct'ında Option<String> olduğunu varsayarak)
        let package_file_name = paket.dosya_adi.as_deref().ok_or_else(|| { // as_deref Option<&String> -> Option<&str>
            eprintln!("Paket '{}' meta verisinde dosya adı belirtilmemiş.", paket.ad); // no_std print
             PaketYoneticisiHatasi::InvalidParameter(format!("Paket '{}' için dosya adı belirtilmemiş.", paket.ad)) // format! alloc
        })?; // Dosya adı yoksa hata dön


        // Hedef dosya Kaynak ID'sini oluştur (Hedef dizin + dosya adı).
        let destination_file_resource_id = format!("{}{}", destination_dir_resource_id, package_file_name); // format! alloc

        println!("Paket yerel depoya ekleniyor: {} -> {}", source_resource_id, destination_file_resource_id); // no_std print

        // Kaynak ve hedef Kaynakları acquire et.
        let source_handle = resource::acquire(source_resource_id, resource::MODE_READ) // Okuma izni
            .map_err(|e| {
                 eprintln!("Kaynak acquire hatası ({}): {:?}", source_resource_id, e); // no_std print
                PaketYoneticisiHatasi::from(e) // SahneError -> PaketYoneticisiHatasi
            })?;

        let destination_handle = resource::acquire(
            &destination_file_resource_id,
            resource::MODE_WRITE | resource::MODE_CREATE | resource::MODE_TRUNCATE // Yazma, oluştur, sil
        ).map_err(|e| {
             eprintln!("Hedef Kaynak acquire hatası ({}): {:?}", destination_file_resource_id, e); // no_std print
            PaketYoneticisiHatasi::from(e) // SahneError -> PaketYoneticisiHatasi
        })?;


        // Kaynaktan oku ve hedefe yaz döngüsü (dosya kopyalama).
        let mut buffer = [0u8; 4096]; // Okuma/yazma buffer'ı (stack'te)
        loop {
            // Kaynak Kaynağından oku
            match resource::read(source_handle, &mut buffer) {
                Ok(0) => break, // Kaynak sonu
                Ok(bytes_read) => {
                    // Hedef Kaynağa yaz
                    match resource::write(destination_handle, &buffer[..bytes_read]) {
                        Ok(_) => {
                            // Başarılı yazma
                        }
                        Err(e) => {
                             eprintln!("Kopyalama yazma hatası ({} -> {}): {:?}", source_resource_id, destination_file_resource_id, e); // no_std print
                            let _ = resource::release(source_handle); // Handle'ları temizle
                            let _ = resource::release(destination_handle);
                            return Err(PaketYoneticisiHatasi::from(e)); // SahneError -> PaketYoneticisiHatasi
                        }
                    }
                }
                Err(e) => {
                     eprintln!("Kopyalama okuma hatası ({} -> {}): {:?}", source_resource_id, destination_file_resource_id, e); // no_std print
                    let _ = resource::release(source_handle); // Handle'ları temizle
                    let _ = resource::release(destination_handle);
                    return Err(PaketYoneticisiHatasi::from(e)); // SahneError -> PaketYoneticisiHatasi
                }
            }
        }

        // Handle'ları serbest bırak
        let release_source_result = resource::release(source_handle);
         if let Err(_e) = release_source_result {
              eprintln!("Kaynak Handle release hatası ({}): {:?}", source_resource_id, _e); // no_std print
         }

        let release_destination_result = resource::release(destination_handle);
         if let Err(_e) = release_destination_result {
             eprintln!("Hedef Handle release hatası ({}): {:?}", destination_file_resource_id, _e); // no_std print
         }


        println!("Paket başarıyla yerel depoya eklendi: {}", destination_file_resource_id); // no_std print
        Ok(()) // Başarı
    }

     // Yerel depodan bir paketi kaldırır (Eksik fonksiyonellik: resource silme).
     // package_name: Kaldırılacak paketin adı.
     // version: Kaldırılacak paketin sürümü.
     // package_file_name: Paketin arşiv dosyasının adı.
     // Dönüş değeri: Başarı veya PaketYoneticisiHatasi.
      pub fn remove_package(
          &self,
          package_name: &str,
          version: &str,
          package_file_name: &str,
      ) -> Result<(), PaketYoneticisiHatasi> {
          println!("Yerel depodan paket siliniyor: {} {}", package_name, version); // no_std print
     //     // Paketin dosyasının ve dizinlerinin Kaynak ID'lerini oluştur.
          let package_file_resource_id = format!("{}/{}/{}/{}", self.base_resource_id, package_name, version, package_file_name);
          let package_version_dir_id = format!("{}/{}/{}/", self.base_resource_id, package_name, version);
          let package_name_dir_id = format!("{}/{}/", self.base_resource_id, package_name);
     //
     //     // Dosyayı sil (resource::delete veya kontrol komutu gerekiyor).
           if let Err(e) = resource::delete(&package_file_resource_id) { // Varsayımsal delete API'sı
                if e != SahneError::ResourceNotFound { // Eğer dosya yoksa hata değil
                     eprintln!("Paket dosyası silinirken hata ({}): {:?}", package_file_resource_id, e);
                     return Err(PaketYoneticisiHatasi::from(e));
                }
           }
     //
     //     // Boş kalan versiyon dizinini sil (resource::remove_dir veya delete gerektirir).
     //     // Eğer dizin boş değilse bu komut başarısız olmalı.
           if let Err(e) = resource::remove_dir(&package_version_dir_id) { // Varsayımsal remove_dir API'sı
                if e != SahneError::ResourceNotFound && e != SahneError::ResourceNotEmpty { // Yoksa veya boş değilse hata değil
                     eprintln!("Paket versiyon dizini silinirken hata ({}): {:?}", package_version_dir_id, e);
                     return Err(PaketYoneticisiHatasi::from(e));
                }
           }
     //
     //     // Boş kalan paket adı dizinini sil (resource::remove_dir veya delete gerektirir).
           if let Err(e) = resource::remove_dir(&package_name_dir_id) { // Varsayımsal remove_dir API'sı
                if e != SahneError::ResourceNotFound && e != SahneError::ResourceNotEmpty {
                     eprintln!("Paket adı dizini silinirken hata ({}): {:?}", package_name_dir_id, e);
                     return Err(PaketYoneticisiHatasi::from(e));
                }
           }
     
          eprintln!("UYARI: Sahne64 API'sında resource silme mekanizması bulunmamaktadır. Yerel depodan paket silme tam olarak desteklenmiyor."); // no_std print
          Err(PaketYoneticisiHatasi::SahneApiHatasi(SahneError::NotSupported)) // Silme desteği yok hatası
      }
}

// #[cfg(test)] bloğu std test runner'ı gerektirir ve Sahne64 resource mock'ları gerektirir.
// Bu blok no_std ortamında derlenmeyecektir eğer std feature aktif değilse ve test ortamı yoksa.
/
#[cfg(test)]
mod tests {
    // std::fs, std::path, std::io, tempfile kullandığı için no_std'de doğrudan çalışmaz.
    // Mock resource veya Sahne64 simülasyonu gerektirir.
    // Sahne64'te resource silme desteği eksikliği testleri zorlaştırır.
}

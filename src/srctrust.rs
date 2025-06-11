#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // HashSet, String, Vec, format! için

use alloc::collections::HashSet; // std::collections::HashSet yerine
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format; // format! makrosu için
use alloc::borrow::ToOwned; // to_owned() için

// Sahne64 API modülleri
use crate::resource; // Kaynak işlemleri (acquire, read, write, release)
use crate::SahneError; // Sahne64 hata türü
use crate::Handle; // Kaynak Handle'ları

// Özel hata enum'ımızı içe aktar (no_std uyumlu hali)
use crate::paket_yoneticisi_hatasi::PaketYoneticisiHatasi;
// SahneError'dan dönüşüm From implementasyonu ile sağlanacak.
// TrustError'dan dönüşüm eklenecek.
use crate::srcerror::PaketYoneticisiHatasi as ErrorMapping; // From impl için yeniden adlandır

// log kütüphanesini içe aktar (no_std uyumlu backend varsayımıyla)
use log::{info, warn, error, debug};

// no_std uyumlu print makroları (kullanıcı arayüzü çıktıları için, loglama ayrı)
 use crate::print_macros::{println, eprintln};

// Güven Yönetimi hatalarını temsil eden enum (no_std uyumlu)
// Debug ve Display manuel implementasyonları.
#[derive(Debug)] // Debug derive'ı no_std'de çalışır
pub enum TrustError {
    // Sahne64 Kaynak Hatası
    Sahne64ResourceError(SahneError), // SahneError'ı sarmalar

    // Parsing Hatası (örn. güvenilenler dosyasından okuma sırasında)
    ParsingError(String), // String alloc gerektirir

    // Diğer güven yönetimi hataları
    // ...
}

// core::fmt::Display implementasyonu
impl core::fmt::Display for TrustError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TrustError::Sahne64ResourceError(e) => write!(f, "Sahne64 Kaynak hatası: {:?}", e),
            TrustError::ParsingError(s) => write!(f, "Ayrıştırma hatası: {}", s),
        }
    }
}

// From implementasyonları
impl From<SahneError> for TrustError {
    fn from(err: SahneError) -> Self {
        TrustError::Sahne64ResourceError(err)
    }
}

impl From<core::str::Utf8Error> for TrustError {
    fn from(err: core::str::Utf8Error) -> Self {
        TrustError::ParsingError(format!("UTF-8 hatası: {:?}", err)) // format! alloc
    }
}


// Helper fonksiyon: Sahne64 Kaynağından tüm içeriği Vec<u8> olarak oku.
// Utils modülünden yeniden kullanıldı. Result türü TrustError olarak güncellendi.
fn read_resource_to_vec(resource_id: &str) -> Result<Vec<u8>, TrustError> { // Result<Vec<u8>, TrustError> olmalı
    let handle = resource::acquire(resource_id, resource::MODE_READ)
        .map_err(|e| TrustError::from(e))?; // SahneError -> TrustError

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
                return Err(TrustError::from(e)); // SahneError -> TrustError
            }
        }
    }

    let release_result = resource::release(handle);
     if let Err(_e) = release_result {
          error!("Helper: Kaynak release hatası ({}): {:?}", resource_id, _e); // no_std log
     }

    Ok(buffer) // Vec<u8> (alloc)
}

// Helper fonksiyon: String içeriğini Sahne64 Kaynağına yazar (truncate ederek).
// resource_id: Yazılacak Kaynağın ID'si.
// content: Yazılacak string içerik.
fn write_string_to_resource(resource_id: &str, content: &str) -> Result<(), TrustError> { // Result<(), TrustError> olmalı
     let handle = resource::acquire(
         resource_id,
         resource::MODE_WRITE | resource::MODE_CREATE | resource::MODE_TRUNCATE
     ).map_err(|e| {
          error!("Helper: Kaynak acquire hatası ({}): {:?}", resource_id, e); // no_std log
          TrustError::from(e) // SahneError -> TrustError
     })?;

     let buffer = content.as_bytes(); // &str -> &[u8]

     let mut written = 0;
     while written < buffer.len() {
          match resource::write(handle, &buffer[written..]) {
               Ok(bytes_written) => {
                   if bytes_written == 0 {
                        // Hiçbir şey yazılamadı
                        let _ = resource::release(handle);
                         error!("Helper: Kaynak yazma hatası ({}): Kaynak yazmayı durdurdu.", resource_id); // no_std log
                        return Err(TrustError::Sahne64ResourceError(SahneError::InvalidOperation)); // Veya daha uygun bir hata
                   }
                    written += bytes_written;
               }
               Err(e) => {
                    let _ = resource::release(handle);
                     error!("Helper: Kaynak yazma hatası ({}): {:?}", resource_id, e); // no_std log
                    return Err(TrustError::from(e)); // SahneError -> TrustError
               }
          }
     }

     let release_result = resource::release(handle);
     if let Err(_e) = release_result {
          error!("Helper: Kaynak release hatası ({}): {:?}", resource_id, _e); // no_std log
     }

    Ok(()) // Başarı
}


// Güvenilen yayıncıları ve paketleri yönetir.
// Güvenilen listeleri Sahne64 Kaynaklarında saklar.
pub struct TrustManager {
    trusted_publishers: HashSet<String>, // alloc
    trusted_packages: HashSet<String>, // alloc
    // Güvenilen listelerin Kaynak ID'leri
    publishers_resource_id: String, // alloc
    packages_resource_id: String, // alloc
}

impl TrustManager {
    // Yeni bir TrustManager örneği oluşturur ve güvenilen verileri yükler.
    // publishers_resource_id: Güvenilen yayıncı listesi Kaynağının ID'si.
    // packages_resource_id: Güvenilen paket listesi Kaynağının ID'si.
    pub fn new(publishers_resource_id: &str, packages_resource_id: &str) -> Self { // &str Kaynak ID'leri
        let mut manager = TrustManager {
            trusted_publishers: HashSet::new(), // alloc
            trusted_packages: HashSet::new(), // alloc
            publishers_resource_id: publishers_resource_id.to_owned(), // to_owned alloc
            packages_resource_id: packages_resource_id.to_owned(), // to_owned alloc
        };
        // Yükleme hatasını burada logluyoruz, çünkü constructor Result döndürmez.
        if let Err(e) = manager.load_trusted_data() {
             error!("Güvenilen veri yüklenirken hata oluştu: {:?}", e); // no_std log
             // Hata durumunda boş bir yönetici ile devam et.
        }
        manager
    }

    // Güvenilen yayıncı ve paket listelerini Sahne64 Kaynaklarından yükler.
    fn load_trusted_data(&mut self) -> Result<(), TrustError> { // Result<(), TrustError> olmalı
        debug!("Güvenilen veri yükleniyor."); // no_std log
        self.load_trusted_publishers()?; // TrustError yayar
        self.load_trusted_packages()?; // TrustError yayar
        info!("Güvenilen veri yükleme tamamlandı."); // no_std log
        Ok(()) // Başarı
    }

    // Güvenilen yayıncı listesini Kaynaktan yükler.
    fn load_trusted_publishers(&mut self) -> Result<(), TrustError> { // Result<(), TrustError> olmalı
        debug!("Güvenilen yayıncılar yükleniyor: {}", self.publishers_resource_id); // no_std log
        self.trusted_publishers.clear(); // Önceki veriyi temizle (alloc)

        match read_resource_to_vec(&self.publishers_resource_id) { // Helper fonksiyonu kullan
            Ok(buffer) => {
                 // Byte içeriğini UTF-8 stringe çevir.
                 let content = core::str::from_utf8(&buffer) // from_utf8 Result<_, Utf8Error> no_std
                    .map_err(|e| TrustError::from(e))?; // Utf8Error -> TrustError

                // Satırları işle ve HashSet'e ekle.
                for line in content.lines() { // lines() Iter over &str no_std
                    let publisher = line.trim(); // trim() &str metodu no_std
                    if !publisher.is_empty() { // is_empty() &str metodu no_std
                        self.trusted_publishers.insert(publisher.to_owned()); // insert, to_owned alloc
                    }
                }
                 debug!("{} güvenilen yayıncı yüklendi.", self.trusted_publishers.len()); // no_std log
                 Ok(()) // Başarı
            }
            Err(TrustError::Sahne64ResourceError(SahneError::ResourceNotFound)) => {
                // Kaynak bulunamadıysa hata değil, boş liste ile başla.
                warn!("Güvenilen yayıncılar Kaynağı bulunamadı ({}). Boş liste ile başlanıyor.", self.publishers_resource_id); // no_std log
                Ok(()) // Başarı (boş HashSet ile)
            }
            Err(e) => {
                // Diğer okuma hatalarını yay.
                 error!("Güvenilen yayıncılar yüklenirken hata oluştu ({}): {:?}", self.publishers_resource_id, e); // no_std log
                Err(e) // TrustError yayılır
            }
        }
    }

    // Güvenilen paket listesini Kaynaktan yükler.
    fn load_trusted_packages(&mut self) -> Result<(), TrustError> { // Result<(), TrustError> olmalı
        debug!("Güvenilen paketler yükleniyor: {}", self.packages_resource_id); // no_std log
        self.trusted_packages.clear(); // Önceki veriyi temizle (alloc)

        match read_resource_to_vec(&self.packages_resource_id) { // Helper fonksiyonu kullan
            Ok(buffer) => {
                 // Byte içeriğini UTF-8 stringe çevir.
                 let content = core::str::from_utf8(&buffer)
                     .map_err(|e| TrustError::from(e))?; // Utf8Error -> TrustError

                // Satırları işle ve HashSet'e ekle.
                for line in content.lines() {
                    let package = line.trim();
                    if !package.is_empty() {
                        self.trusted_packages.insert(package.to_owned()); // alloc
                    }
                }
                 debug!("{} güvenilen paket yüklendi.", self.trusted_packages.len()); // no_std log
                 Ok(()) // Başarı
            }
            Err(TrustError::Sahne64ResourceError(SahneError::ResourceNotFound)) => {
                // Kaynak bulunamadıysa hata değil, boş liste ile başla.
                warn!("Güvenilen paketler Kaynağı bulunamadı ({}). Boş liste ile başlanıyor.", self.packages_resource_id); // no_std log
                Ok(()) // Başarı (boş HashSet ile)
            }
            Err(e) => {
                // Diğer okuma hatalarını yay.
                 error!("Güvenilen paketler yüklenirken hata oluştu ({}): {:?}", self.packages_resource_id, e); // no_std log
                Err(e) // TrustError yayılır
            }
        }
    }


    // Güvenilen yayıncı listesine bir yayıncı ekler ve Kaynağa kaydeder.
    // publisher_name: Eklenecek yayıncının adı.
    // Dönüş değeri: Ekleme başarılı olursa Ok(()), zaten varsa Ok(()), hata olursa PaketYoneticisiHatasi.
    pub fn add_trusted_publisher(&mut self, publisher_name: &str) -> Result<(), PaketYoneticisiHatasi> { // Result<(), PaketYoneticisiHatasi> olmalı
        // HashSet'e ekle. Eğer zaten varsa false döner.
        if self.trusted_publishers.insert(publisher_name.to_owned()) { // alloc, insert bool döner
             // Yeni eklendiyse Kaynağa kaydet.
             debug!("Yeni güvenilen yayıncı eklendi, Kaynağa kaydediliyor: {}", publisher_name); // no_std log
             // append yerine Kaynağa yazma helper'ı kullanalım ve append moda ayarlayalım.
             let line_to_append = format!("{}\n", publisher_name); // format! alloc
             match self.append_string_to_resource(&self.publishers_resource_id, &line_to_append) { // append_string_to_resource TrustError dönsün
                 Ok(_) => {
                      info!("Güvenilen yayıncı Kaynağa kaydedildi: {}", publisher_name); // no_std log
                     Ok(()) // Başarı
                 }
                 Err(e) => {
                      error!("Güvenilen yayıncı Kaynağa kaydedilirken hata oluştu ({}): {:?}", publisher_name, e); // no_std log
                      // HashSet'ten geri çıkarmayı düşünebiliriz, ama hata durumunda tutarlılık zor.
                     Err(PaketYoneticisiHatasi::from(e)) // TrustError -> PaketYoneticisiHatasi
                 }
             }
        } else {
             // Zaten varsa bir şey yapmaya gerek yok, başarı dön.
             debug!("Güvenilen yayıncı zaten listede: {}", publisher_name); // no_std log
            Ok(())
        }
    }

    // Helper fonksiyon: String içeriğini Sahne64 Kaynağına append eder.
    // resource_id: Yazılacak Kaynağın ID'si.
    // content: Yazılacak string içerik.
    fn append_string_to_resource(&self, resource_id: &str, content: &str) -> Result<(), TrustError> { // Result<(), TrustError> olmalı
         let handle = resource::acquire(
             resource_id,
             resource::MODE_WRITE | resource::MODE_CREATE | resource::MODE_APPEND // MODE_APPEND
         ).map_err(|e| {
              error!("Helper: Kaynak acquire hatası ({}): {:?}", resource_id, e); // no_std log
              TrustError::from(e) // SahneError -> TrustError
         })?;

         let buffer = content.as_bytes(); // &str -> &[u8]

         let mut written = 0;
         while written < buffer.len() {
              match resource::write(handle, &buffer[written..]) {
                   Ok(bytes_written) => {
                       if bytes_written == 0 {
                           // Hiçbir şey yazılamadı
                            let _ = resource::release(handle);
                             error!("Helper: Kaynak yazma hatası ({}): Kaynak yazmayı durdurdu.", resource_id); // no_std log
                            return Err(TrustError::Sahne64ResourceError(SahneError::InvalidOperation)); // Veya daha uygun bir hata
                       }
                        written += bytes_written;
                   }
                   Err(e) => {
                        let _ = resource::release(handle);
                         error!("Helper: Kaynak yazma hatası ({}): {:?}", resource_id, e); // no_std log
                        return Err(TrustError::from(e)); // SahneError -> TrustError
                   }
              }
         }

         let release_result = resource::release(handle);
         if let Err(_e) = release_result {
              error!("Helper: Kaynak release hatası ({}): {:?}", resource_id, _e); // no_std log
         }

        Ok(()) // Başarı
    }


    // Güvenilen yayıncı listesinden bir yayıncıyı kaldırır ve Kaynağı günceller.
    // publisher_name: Kaldırılacak yayıncının adı.
    // Dönüş değeri: Kaldırma başarılı olursa true, yoksa false. Hata durumunda PaketYoneticisiHatasi.
    // Not: Bu implementasyon, tüm listeyi yeniden yazarak güncellemeyi yapar.
    pub fn remove_trusted_publisher(&mut self, publisher_name: &str) -> Result<bool, PaketYoneticisiHatasi> { // Result<bool, PaketYoneticisiHatasi> olmalı
         // HashSet'ten kaldır. Eğer bulunduysa ve kaldırıldıysa true döner.
         let removed = self.trusted_publishers.remove(publisher_name); // remove &str alır no_std

         if removed {
              // HashSet'ten kaldırıldıysa, Kaynağı yeniden yazarak güncelle.
              debug!("Güvenilen yayıncı listeden kaldırıldı, Kaynak güncelleniyor: {}", publisher_name); // no_std log
              if let Err(e) = self.persist_trusted_publishers() { // TrustError dönsün
                   error!("Güvenilen yayıncılar Kaynağı güncellenirken hata oluştu ({}): {:?}", self.publishers_resource_id, e); // no_std log
                   // Hata durumunda ne yapılmalı? HashSet güncellendi ama dosya güncellenmedi. Tutarlılık sorunu.
                   // Geri alma mekanizması (örn. orijinal dosyayı geri yükleme) burada düşünülebilir.
                  return Err(PaketYoneticisiHatasi::from(e)); // TrustError -> PaketYoneticisiHatasi
              }
             info!("Güvenilen yayıncı Kaynaktan kaldırıldı: {}", publisher_name); // no_std log
         } else {
             debug!("Güvenilen yayıncı listede bulunamadı: {}", publisher_name); // no_std log
         }

         Ok(removed) // Kaldırılıp kaldırılmadığını döndür.
    }

    // Tüm güvenilen yayıncıları Kaynağa yeniden yazar (truncate ve yaz).
    // Bu basit ama potansiyel olarak büyük listeler için verimsiz bir yöntemdir.
    fn persist_trusted_publishers(&self) -> Result<(), TrustError> { // Result<(), TrustError> olmalı
         debug!("Tüm güvenilen yayıncılar Kaynağa yeniden yazılıyor: {}", self.publishers_resource_id); // no_std log
         let mut content_to_write = String::new(); // alloc
         for publisher in &self.trusted_publishers {
              content_to_write.push_str(&format!("{}\n", publisher)); // push_str, format! alloc
         }
         write_string_to_resource(&self.publishers_resource_id, &content_to_write) // Helper fonksiyonu kullan
    }

    // Belirtilen yayıncının güvenilir olup olmadığını kontrol eder.
    // publisher_name: Kontrol edilecek yayıncının adı.
    // Dönüş değeri: Güvenilir ise true, değilse false.
    pub fn is_trusted_publisher(&self, publisher_name: &str) -> bool {
        self.trusted_publishers.contains(publisher_name) // contains &str alır no_std
    }

    // Güvenilen paket listesine bir paket ekler ve Kaynağa kaydeder.
    // package_name: Eklenecek paketin adı.
    // Dönüş değeri: Ekleme başarılı olursa Ok(()), zaten varsa Ok(()), hata olursa PaketYoneticisiHatasi.
    pub fn add_trusted_package(&mut self, package_name: &str) -> Result<(), PaketYoneticisiHatasi> { // Result<(), PaketYoneticisiHatasi> olmalı
        // HashSet'e ekle. Eğer zaten varsa false döner.
        if self.trusted_packages.insert(package_name.to_owned()) { // alloc, insert bool döner
             // Yeni eklendiyse Kaynağa kaydet.
             debug!("Yeni güvenilen paket eklendi, Kaynağa kaydediliyor: {}", package_name); // no_std log
             let line_to_append = format!("{}\n", package_name); // format! alloc
             match self.append_string_to_resource(&self.packages_resource_id, &line_to_append) { // append_string_to_resource TrustError dönsün
                 Ok(_) => {
                      info!("Güvenilen paket Kaynağa kaydedildi: {}", package_name); // no_std log
                     Ok(()) // Başarı
                 }
                 Err(e) => {
                      error!("Güvenilen paket Kaynağa kaydedilirken hata oluştu ({}): {:?}", package_name, e); // no_std log
                     Err(PaketYoneticisiHatasi::from(e)) // TrustError -> PaketYoneticisiHatasi
                 }
             }
        } else {
             // Zaten varsa bir şey yapmaya gerek yok, başarı dön.
             debug!("Güvenilen paket zaten listede: {}", package_name); // no_std log
            Ok(())
        }
    }

    // Güvenilen paket listesinden bir paketi kaldırır ve Kaynağı günceller.
    // package_name: Kaldırılacak paketin adı.
    // Dönüş değeri: Kaldırma başarılı olursa true, yoksa false. Hata durumunda PaketYoneticisiHatasi.
    // Not: Bu implementasyon, tüm listeyi yeniden yazarak güncellemeyi yapar.
    pub fn remove_trusted_package(&mut self, package_name: &str) -> Result<bool, PaketYoneticisiHatasi> { // Result<bool, PaketYoneticisiHatasi> olmalı
        // HashSet'ten kaldır. Eğer bulunduysa ve kaldırıldıysa true döner.
         let removed = self.trusted_packages.remove(package_name); // remove &str alır no_std

         if removed {
              // HashSet'ten kaldırıldıysa, Kaynağı yeniden yazarak güncelle.
              debug!("Güvenilen paket listeden kaldırıldı, Kaynak güncelleniyor: {}", package_name); // no_std log
              if let Err(e) = self.persist_trusted_packages() { // TrustError dönsün
                   error!("Güvenilen paketler Kaynağı güncellenirken hata oluştu ({}): {:?}", self.packages_resource_id, e); // no_std log
                   // Hata durumunda ne yapılmalı? Tutarlılık sorunu.
                  return Err(PaketYoneticisiHatasi::from(e)); // TrustError -> PaketYoneticisiHatasi
              }
             info!("Güvenilen paket Kaynaktan kaldırıldı: {}", package_name); // no_std log
         } else {
             debug!("Güvenilen paket listede bulunamadı: {}", package_name); // no_std log
         }

         Ok(removed) // Kaldırılıp kaldırılmadığını döndür.
    }

    // Tüm güvenilen paketleri Kaynağa yeniden yazar (truncate ve yaz).
    fn persist_trusted_packages(&self) -> Result<(), TrustError> { // Result<(), TrustError> olmalı
        debug!("Tüm güvenilen paketler Kaynağa yeniden yazılıyor: {}", self.packages_resource_id); // no_std log
        let mut content_to_write = String::new(); // alloc
        for package in &self.trusted_packages {
            content_to_write.push_str(&format!("{}\n", package)); // push_str, format! alloc
        }
        write_string_to_resource(&self.packages_resource_id, &content_to_write) // Helper fonksiyonu kullan
    }


    // Belirtilen paketin güvenilir olup olmadığını kontrol eder.
    // package_name: Kontrol edilecek paketin adı.
    // Dönüş değeri: Güvenilir ise true, değilse false.
    pub fn is_trusted_package(&self, package_name: &str) -> bool {
        self.trusted_packages.contains(package_name) // contains &str alır no_std
    }
}

// #[cfg(test)] bloğu std test runner'ı ve std bağımlılıkları gerektirir.
// Testler için mock resource veya Sahne64 simülasyonu gereklidir.

#[cfg(test)]
mod tests {
    // std::collections, std::io, std::path, std::fs, tempfile kullandığı için no_std'de doğrudan çalışmaz.
    // Mock resource::acquire/read/write/release ve test dosyası oluşturma/okuma helper'ları gerektirir.
}

// --- TrustError enum tanımı ---
// Bu dosyanın başında tanımlanmıştır ve no_std uyumludur.


// --- PaketYoneticisiHatasi enum tanımı ---
// srcerror.rs modülünde tanımlanmıştır ve no_std uyumludur.
// TrustError'dan PaketYoneticisiHatasi'na dönüşüm için From implementasyonu eklenmelidir.

// srcerror.rs (Örnek - no_std uyumlu, TrustError'dan dönüşüm eklenmiş)
#![no_std]
extern crate alloc;

// ... diğer importlar ...

use crate::srctrust::TrustError; // TrustError'ı içe aktar

#[derive(Debug)]
pub enum PaketYoneticisiHatasi {
    // ... diğer hatalar ...

    // Güven Yönetimi sırasında oluşan hatalar
    TrustError(TrustError), // TrustError'ı sarmalar

    // ... diğer hatalar ...
}

// TrustError'dan PaketYoneticisiHatasi'na dönüşüm
impl From<TrustError> for PaketYoneticisiHatasi {
    fn from(err: TrustError) -> Self {
        PaketYoneticisiHatasi::TrustError(err)
    }
}

#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // HashMap, String, Vec, format! için

use alloc::collections::HashMap; // std::collections::HashMap yerine
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format; // format! makrosu için
use alloc::boxed::Box; // Hata sarmalamak için gerekebilir

// serde ve no_std uyumlu serileştirme/deserileştirme kütüphanesi
use serde::{Deserialize, Serialize};
use postcard; // no_std uyumlu binary serileştirme
use postcard::Error as PostcardError; // Postcard hata türü

// Sahne64 resource modülü
use crate::resource;
use crate::SahneError; // Sahne64 hata türü
use crate::Handle; // Kaynak Handle'ları

// Özel hata enum'ımızı içe aktar (no_std uyumlu hali)
use crate::paket_yoneticisi_hata::PaketYoneticisiHatasi;
// SahneError ve PostcardError'dan dönüşüm From implementasyonları ile sağlanacak

// Helper function to read resource content into a Vec<u8> (reused from srcrepository.rs)
// Note: This helper should ideally be in a common utility module.
fn read_resource_to_vec(resource_id: &str) -> Result<Vec<u8>, PaketYoneticisiHatasi> {
    let handle = resource::acquire(resource_id, resource::MODE_READ)
        .map_err(|e| PaketYoneticisiHatasi::from(e))?;

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
                return Err(PaketYoneticisiHatasi::from(e));
            }
        }
    }

    let release_result = resource::release(handle);
     if let Err(_e) = release_result {
          // Log error, but continue
     }

    Ok(buffer)
}


// Paket deposu indeksi hatalarını temsil eden enum (no_std uyumlu)
// thiserror::Error yerine Debug ve Display manuel implementasyonları.
#[derive(Debug)] // Debug derive'ı no_std'de çalışır
pub enum IndexError {
    // Sahne64 Kaynak (Dosya Sistemi benzeri) Hatası
    Sahne64ResourceError(SahneError), // SahneError'ı sarmalar

    // Serileştirme/Deserileştirme hataları
    SerializationError(PostcardError),
    DeserializationError(PostcardError),

    // Geçersiz Kaynak ID'si veya yol hatası
    InvalidResourceID(String), // String alloc gerektirir

    // Diğer beklenmedik hatalar
     UnknownError(String), // Daha spesifik hata varyantları tercih edilir.
}

// core::fmt::Display implementasyonu (kullanıcı dostu mesajlar için)
impl core::fmt::Display for IndexError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            IndexError::Sahne64ResourceError(e) => write!(f, "Sahne64 Kaynak hatası: {:?}", e),
            IndexError::SerializationError(e) => write!(f, "İndeks serileştirme hatası: {:?}", e),
            IndexError::DeserializationError(e) => write!(f, "İndeks seriden çıkarma hatası: {:?}", e),
            IndexError::InvalidResourceID(s) => write!(f, "Geçersiz Kaynak ID'si: {}", s),
        }
    }
}

// From implementasyonları
impl From<SahneError> for IndexError {
    fn from(err: SahneError) -> Self {
        IndexError::Sahne64ResourceError(err)
    }
}

impl From<PostcardError> for IndexError {
    fn from(err: PostcardError) -> Self {
        // Postcard hatasının türüne göre Serialization veya Deserialization olarak ayırabiliriz.
        match err {
            PostcardError::Serialize(_) => IndexError::SerializationError(err),
            _ => IndexError::DeserializationError(err),
        }
    }
}

// Helper type for results within this module
type IndexResult<T> = Result<T, IndexError>;


// Paket deposu indeksini temsil eden yapı (no_std uyumlu)
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)] // Debug, PartialEq, Eq, Serialize, Deserialize derive'ları no_std'de çalışır
struct PackageIndex {
    // Paket adı -> Sürümler listesi. HashMap ve Vec<String> alloc gerektirir.
    packages: HashMap<String, Vec<String>>,
}

impl PackageIndex {
    // Yeni bir boş PaketIndeksi oluşturur.
    fn new() -> Self {
        PackageIndex {
            packages: HashMap::new(), // HashMap::new() alloc gerektirir
        }
    }

    // Paketi indekse ekler.
    // Eğer paket zaten varsa, verilen sürüm listesine eklenir.
    fn add_package(&mut self, package_name: &str, version: &str) {
        self.packages
            .entry(package_name.to_owned()) // package_name &str -> String (alloc), entry alloc
            .or_default() // or_default alloc
            .push(version.to_owned()); // version &str -> String (alloc), push alloc
    }

    // Paketin indekste olup olmadığını kontrol eder.
    fn has_package(&self, package_name: &str) -> bool {
        self.packages.contains_key(package_name) // contains_key (&str)
    }

    // Paketin sürümlerini döndürür.
    // Eğer paket bulunamazsa `None` döndürür.
    fn get_versions(&self, package_name: &str) -> Option<&Vec<String>> {
        self.packages.get(package_name) // get (&str)
    }

    // İndeksi belirtilen Kaynak ID'sine kaydeder.
    // Binary (postcard) formatında Kaynağa yazma işlemini gerçekleştirir.
    // index_resource_id: İndeksin kaydedileceği Kaynak ID'si.
    fn save_to_resource(&self, index_resource_id: &str) -> IndexResult<()> { // save_to_file yerine save_to_resource
        // İndeks yapısını binary formatına serileştir (postcard)
        let serialized_data = postcard::to_postcard(self) // Serileştirme (alloc gerektirir)
            .map_err(|e| {
                 eprintln!("İndeks serileştirme hatası: {:?}", e); // no_std print
                 IndexError::SerializationError(e) // PostcardError -> IndexError
            })?; // Hata durumunda ? ile yay

        // Hedef Kaynağı yazma izniyle aç/oluştur/sil.
        // MODE_CREATE varsa Kaynak yoksa oluşturur, MODE_TRUNCATE varsa içeriği siler.
        // Parent resource'ların oluşturulması resource::acquire'a bağlıdır.
        let handle = resource::acquire(
            index_resource_id,
            resource::MODE_WRITE | resource::MODE_CREATE | resource::MODE_TRUNCATE
        ).map_err(|e| {
             eprintln!("İndeks Kaynağı acquire hatası ({}): {:?}", index_resource_id, e); // no_std print
             IndexError::from(e) // SahneError -> IndexError
        })?; // Hata durumunda ? ile yay

        // Serileştirilmiş veriyi Kaynağa yaz.
        // resource::write tek seferde yazmayabilir, loop gerekebilir.
        let buffer = &serialized_data; // &[u8]
        let mut written = 0;
        while written < buffer.len() {
            match resource::write(handle, &buffer[written..]) {
                 Ok(bytes_written) => {
                     if bytes_written == 0 {
                          // Hiçbir şey yazılamadı, Kaynak hatası olabilir
                          let _ = resource::release(handle);
                           eprintln!("İndeks Kaynağı yazma hatası ({}): Kaynak yazmayı durdurdu.", index_resource_id); // no_std print
                          return Err(IndexError::Sahne64ResourceError(SahneError::InvalidOperation)); // Veya daha uygun bir hata
                     }
                     written += bytes_written;
                 }
                 Err(e) => {
                      // Yazma hatası
                      let _ = resource::release(handle);
                       eprintln!("İndeks Kaynağı yazma hatası ({}): {:?}", index_resource_id, e); // no_std print
                      return Err(IndexError::from(e)); // SahneError -> IndexError
                 }
            }
        }


        // Handle'ı serbest bırak
        let release_result = resource::release(handle);
         if let Err(_e) = release_result {
             // Log error, but continue
              eprintln!("İndeks Kaynağı release hatası ({}): {:?}", index_resource_id, _e); // no_std print
         }

        Ok(()) // Başarı
    }

    // Belirtilen Kaynak ID'sinden indeksi yükler.
    // Binary (postcard) formatındaki Kaynaktan okuma ve deserializasyon işlemini yapar.
    // index_resource_id: İndeksin yükleneceği Kaynak ID'si.
    fn load_from_resource(index_resource_id: &str) -> IndexResult<Self> { // load_from_file yerine load_from_resource
        // Kaynak içeriğini oku (Vec<u8> olarak)
        let buffer = read_resource_to_vec(index_resource_id) // Helper fonksiyonu kullanır (PaketYoneticisiHatasi döner)
            .map_err(|e| {
                // read_resource_to_vec'ten gelen PaketYoneticisiHatasi'nı IndexError'a çevir.
                // PaketYoneticisiHatasi'nın SahneApiError veya ParsingError varyantları gelebilir.
                match e {
                     PaketYoneticisiHatasi::SahneApiError(se) => IndexError::Sahne64ResourceError(se),
                     PaketYoneticisiHatasi::ParsingError(s) => IndexError::InvalidResourceID(s), // UTF-8 hatasını burada InvalidResourceID'ye mapleyelim
                     _ => {
                          // Diğer beklenmedik hatalar
                          eprintln!("load_from_resource: Beklenmedik helper hatası: {:?}", e); // no_std print
                         IndexError::Sahne64ResourceError(SahneError::UnknownSystemCall) // Generic hata
                     }
                }
            })?; // Hata durumunda ? ile yay

        // Okunan binary veriyi PackageIndex yapısına deserialize et (postcard)
        postcard::from_bytes_copy::<Self>(&buffer) // Deserileştirme (alloc gerektirir)
            .map_err(|e| {
                 eprintln!("İndeks seriden çıkarma hatası (Kaynak: {}): {:?}", index_resource_id, e); // no_std print
                 IndexError::DeserializationError(e) // PostcardError -> IndexError
            }) // Hata durumunda map_err ile IndexError'a çevir
    }
}

// İndeks Kaynağının ID'sini oluşturur.
// Repo Kaynak ID'sini temel alarak `index.bin` dosyasının Kaynak ID'sini birleştirir.
// repo_resource_id: Paket deposu temel Kaynak ID'si.
// Dönüş değeri: İndeks Kaynağı ID'si String olarak (alloc gerektirir).
fn get_index_resource_id(repo_resource_id: &str) -> String { // Path yerine &str Kaynak ID, PathBuf yerine String
    // Kaynak ID'sini birleştirme. format! alloc gerektirir.
    format!("{}/index.bin", repo_resource_id) // .json yerine .bin (binary format)
}

// İndeksi oluşturur veya yükler.
// Eğer indeks Kaynağı varsa yükler, yoksa yeni bir indeks oluşturur.
// repo_resource_id: Paket deposu temel Kaynak ID'si.
// Dönüş değeri: Yüklenen veya oluşturulan PackageIndex veya hata.
fn get_or_create_index(repo_resource_id: &str) -> IndexResult<PackageIndex> { // Path yerine &str Kaynak ID
    let index_resource_id = get_index_resource_id(repo_resource_id); // String (alloc)

    // İndeksi Kaynaktan yüklemeye çalış.
    match PackageIndex::load_from_resource(&index_resource_id) { // String'e referans geçeriz
        Ok(index) => {
             // Başarıyla yüklendi
             println!("İndeks Kaynağından yüklendi: {}", index_resource_id); // no_std print
            Ok(index)
        }
        Err(IndexError::Sahne64ResourceError(SahneError::ResourceNotFound)) => {
            // Kaynak bulunamadı, bu normal (indeks ilk kez oluşturuluyor).
             println!("İndeks Kaynağı bulunamadı ({}). Yeni indeks oluşturuluyor.", index_resource_id); // no_std print
            Ok(PackageIndex::new()) // Yeni, boş bir indeks oluştur (alloc)
        }
        Err(e) => {
            // Diğer yükleme hataları (acquire hatası, deserialize hatası vb.)
             eprintln!("İndeks yüklenirken hata oluştu ({}): {:?}", index_resource_id, e); // no_std print
            Err(e) // Hatayı yay
        }
    }
}

// #[cfg(test)] bloğu std test runner'ı gerektirir ve Sahne64 resource/postcard mock'ları gerektirir.
// Bu blok no_std ortamında derlenmeyecektir eğer std feature aktif değilse ve test ortamı yoksa.

#[cfg(test)]
mod tests {
    // std::fs, std::path, std::io, tempfile kullandığı için no_std'de doğrudan çalışmaz.
    // Mock resource veya Sahne64 simülasyonu gerektirir.
}



// --- PaketYoneticisiHatasi enum tanımı ---
// srcerror.rs modülünde tanımlanmıştır ve no_std uyumludur.
// IndexError'dan PaketYoneticisiHatasi'na dönüşüm için From implementasyonu eklenmelidir.

// srcerror.rs (Güncellenmiş - IndexError'dan dönüşüm eklenmiş)
#![no_std]
extern crate alloc;

// ... diğer importlar ...

use crate::srcrepositoryindex::IndexError; // IndexError'ı içe aktar

#[derive(Debug)]
pub enum PaketYoneticisiHatasi {
    // ... diğer hatalar ...

    // İndeks yönetimi sırasında oluşan hatalar
    IndexError(IndexError), // IndexError'ı sarmalar

    // ... diğer hatalar ...
}

// IndexError'dan PaketYoneticisiHatasi'na dönüşüm
impl From<IndexError> for PaketYoneticisiHatasi {
    fn from(err: IndexError) -> Self {
        PaketYoneticisiHatasi::IndexError(err)
    }
}

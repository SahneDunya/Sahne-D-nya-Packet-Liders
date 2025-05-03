#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // String, format!, Box için

use alloc::string::{String, ToString};
use alloc::format;
// zip crate'inin no_std hatasını varsayalım
use zip::result::ZipError; // zip crate'inin no_std+alloc uyumlu ZipError'ını varsayıyoruz

// postcard hata türü
use postcard::Error as PostcardError; // postcard crate'inin no_std+alloc uyumlu Error'ını varsayıyoruz

// Sahne64 API'sının hata türü
use crate::SahneError;
// Paket Yöneticisi Hata Türleri (no_std uyumlu)
// thiserror::Error yerine Debug ve Display manuel implementasyonları.
#[derive(Debug)] // Debug derive'ı no_std'de çalışır
pub enum PaketYoneticisiHatasi {
    // Düşük seviye Sahne64 API hatalarını sarmalar
    SahneApiError(SahneError),

    // Serileştirme/Deserileştirme hataları
    SerializationError(PostcardError),
    DeserializationError(PostcardError),

    // ZIP arşivi işlenirken oluşan hatalar
    ZipError(ZipError),

    // Ağ iletişimi sırasında oluşan hatalar (Varsayımsal, eğer ağ Sahne64'te Kaynak ise SahneApiError olabilir)
    // Eğer özel bir ağ stack'i varsa, onun hata türü burada sarmalanır.
    // Şimdilik String detaylı basit bir placeholder. String alloc gerektirir.
    NetworkError(String),

    // Kaynak (dosya, vs.) içeriğini ayrıştırma (parsing) sırasında oluşan hatalar
    ParsingError(String), // Hata detayını string olarak tutmak alloc gerektirir.

    // Bağımlılık çözümleme sırasında bir bağımlılık bulunamadı
    BagimlilikBulunamadi(String), // Bulunamayan bağımlılık adı (String alloc gerektirir)

    // Bağımlılık çözümleme sırasında (veya başka yerde) bir paket adı bulunamadı
    PaketBulunamadi(String), // Bulunamayan paket adı (String alloc gerektirir)

    // Bağımlılık çakışması tespit edildiğinde (çözülemeyen durumda)
    ConflictError(String), // Çakışma detayını string olarak tutmak alloc gerektirir.

    // Checksum doğrulama hatası (Paket bütünlüğü doğrulanamadı)
    ChecksumVerificationError, // Enum varyantı olarak sabit, alloc gerektirmez.

    // Paket kurulumu veya kaldırma sırasında oluşan genel hatalar
    // Betik çalıştırma hatası gibi daha spesifik hatalar bu varyanta girebilir veya ayrı tutulur.
    // Genel bir hata mesajı veya nedeni tutabilir.
    InstallationError(String), // Detay String (alloc gerektirir)
    RemovalError(String), // Detay String (alloc gerektirir)

    // Önbellek işlemleri sırasında oluşan hatalar (SahneApiError dışındaki cache mantığı hataları)
    CacheError(String), // Detay String (alloc gerektirir)

    // Fonksiyona geçersiz parametre geçilmesi
    InvalidParameter(String), // Detay String (alloc gerektirir)

    // Beklenmedik veya eşlenmemiş hatalar
    UnknownError(String), // Detay String (alloc gerektirir)

    // Diğer spesifik hatalar eklenebilir...
}

// core::fmt::Display implementasyonu (kullanıcı dostu mesajlar için)
// Bu implementasyon format! makrosunu kullanır, bu da alloc gerektirir eğer hata detayları String ise.
impl core::fmt::Display for PaketYoneticisiHatasi {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PaketYoneticisiHatasi::SahneApiError(e) => write!(f, "Sahne64 API hatası: {:?}", e), // Debug formatı yeterli olabilir
            PaketYoneticisiHatasi::SerializationError(e) => write!(f, "Serileştirme hatası: {:?}", e),
            PaketYoneticisiHatasi::DeserializationError(e) => write!(f, "Seriden çıkarma hatası: {:?}", e),
            PaketYoneticisiHatasi::ZipError(e) => write!(f, "ZIP hatası: {:?}", e),
            PaketYoneticisiHatasi::NetworkError(s) => write!(f, "Ağ hatası: {}", s),
            PaketYoneticisiHatasi::ParsingError(s) => write!(f, "Ayrıştırma hatası: {}", s),
            PaketYoneticisiHatasi::BagimlilikBulunamadi(s) => write!(f, "Bağımlılık bulunamadı: {}", s),
            PaketYoneticisiHatasi::PaketBulunamadi(s) => write!(f, "Paket bulunamadı: {}", s),
            PaketYoneticisiHatasi::ConflictError(s) => write!(f, "Paket çakışması: {}", s),
            PaketYoneticisiHatasi::ChecksumVerificationError => write!(f, "Checksum doğrulama hatası: Paket bütünlüğü doğrulanamadı."),
            PaketYoneticisiHatasi::InstallationError(s) => write!(f, "Kurulum hatası: {}", s),
            PaketYoneticisiHatasi::RemovalError(s) => write!(f, "Kaldırma hatası: {}", s),
            PaketYoneticisiHatasi::CacheError(s) => write!(f, "Önbellek hatası: {}", s),
            PaketYoneticisiHatasi::InvalidParameter(s) => write!(f, "Geçersiz parametre: {}", s),
            PaketYoneticisiHatasi::UnknownError(s) => write!(f, "Beklenmedik hata: {}", s),
        }
    }
}

// From implementasyonları, diğer hata türlerinden PaketYoneticisiHatasi'na kolay dönüşüm sağlar.

impl From<SahneError> for PaketYoneticisiHatasi {
    fn from(err: SahneError) -> Self {
        // Burada SahneError'ın spesifik varyantlarına göre daha detaylı maplemeler yapılabilir.
        // Örn: SahneError::PermissionDenied -> PaketYoneticisiHatasi::YetkiHatasi gibi (eğer YetkiHatasi varyantını tutuyorsak)
        // Ama SahneApiError(err) en basit ve bilgi kaybetmeyen yoldur.
        match err {
            // ResourceNotFound gibi hatalar PaketBulunamadi veya BagimlilikBulunamadi gibi
            // daha spesifik hatalara maplenebilir, ancak bu genellikle hatanın oluştuğu bağlamda yapılır (get_dependencies gibi).
            // Burada genel dönüşümü yapalım.
            _ => PaketYoneticisiHatasi::SahneApiError(err),
        }
    }
}

impl From<PostcardError> for PaketYoneticisiHatasi {
    fn from(err: PostcardError) -> Self {
        // Postcard hatasının türüne göre Serialization veya Deserialization olarak ayırabiliriz.
        match err {
            PostcardError::Serialize(_) => PaketYoneticisiHatasi::SerializationError(err),
            _ => PaketYoneticisiHatasi::DeserializationError(err),
        }
    }
}

// ZipError'dan dönüşüm. zip crate'inin no_std hatasını varsayıyoruz.
impl From<ZipError> for PaketYoneticisiHatasi {
    fn from(err: ZipError) -> Self {
        // ZipError'ın içindeki IO hataları (ki bunlar no_std'de farklıdır)
        // PaketYoneticisiHatasi::SahneApiError'a maplenemez doğrudan.
        // ZipError'ı olduğu gibi sarmalamak en uygunu.
        PaketYoneticisiHatasi::ZipError(err)
    }
}

// Eğer ağ Kaynakları için özel bir hata türü varsa, onun için de From implementasyonu eklenir.
impl From<YourNoStdNetworkError> for PaketYoneticisiHatasi { ... }

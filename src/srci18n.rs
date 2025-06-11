#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // HashMap, String, Vec, format! için

use alloc::collections::HashMap; // std::collections::HashMap yerine
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format; // Hata mesajları için
use alloc::boxed::Box; // Debug implementasyonu için gerekebilir (thiserror kullanmadığımızdan)

// fluent-bundle ve unic-langid crate'lerinin no_std+alloc uyumlu olduğunu varsayıyoruz.
use fluent_bundle::{FluentBundle, FluentResource, LocalizedString, FluentValue};
use unic_langid::LanguageIdentifier;

// Sahne64 API modüllerini içe aktarın
use crate::resource; // fs yerine
use crate::SahneError;

// Uluslararasılaştırma (i18n) Hata Türleri (no_std uyumlu)
// thiserror::Error yerine Debug ve Display manuel implementasyonları.
#[derive(Debug)] // Debug derive'ı no_std'de çalışır
pub enum I18nError {
    // Dizin okuma hatası (Sahne64 API'sında dizin okuma yoksa bu variant kullanılamaz)
    // DirectoryReadError(String), // Şu anki new fonksiyonu directory okumuyor.

    // Kaynak okuma hatası (dosya yerine)
     FileReadError(String), // Bu hatayı Sahne64FileSystemError içinde sarmalayacağız

    // Dil tanımlayıcısı ayrıştırma hatası
    LanguageIdentifierParseError(String), // String alloc gerektirir

    // Fluent kaynağı (.ftl içeriği) ayrıştırma hatası
    FluentResourceParseError(String), // String alloc gerektirir

    // Kaynağı FluentBundle'a ekleme hatası
    ResourceAdditionError(String), // String alloc gerektirir

    // Belirtilen dil paketi (bundle) bulunamadı
    BundleNotFound(String), // String alloc gerektirir

    // İstenen mesaj anahtarı (key) bulunamadı
    MessageNotFound(String), // String alloc gerektirir

    // Mesaj değeri (pattern) bulunamadı
    MessageValueError(String), // String alloc gerektirir

    // Mesaj değerlendirme (evaluate) hatası (argümanlarla birlikte)
    MessageEvaluationError(String), // String alloc gerektirir

    // Sahne64 Kaynak (Dosya Sistemi benzeri) Hatası
    Sahne64ResourceError(SahneError), // SahneError'ı sarmalar

    // Kaynak içeriğinin UTF-8 olmaması hatası
    ResourceUtf8Error(String), // Kaynak ID'sini tutmak için String (alloc gerektirir)

    // Diğer beklenmedik veya eşlenmemiş hatalar
     UnknownError(String), // Daha spesifik hata varyantları tercih edilir.
}

// core::fmt::Display implementasyonu (kullanıcı dostu mesajlar için)
// format! makrosunu kullanır, bu da alloc gerektirir eğer hata detayları String ise.
impl core::fmt::Display for I18nError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
             I18nError::DirectoryReadError(s) => write!(f, "Dizin okuma hatası: {}", s),
             I18nError::FileReadError(s) => write!(f, "Dosya okuma hatası: {}", s),
            I18nError::LanguageIdentifierParseError(s) => write!(f, "Dil tanımlayıcısı ayrıştırma hatası: {}", s),
            I18nError::FluentResourceParseError(s) => write!(f, "Fluent Kaynak ayrıştırma hatası: {}", s),
            I18nError::ResourceAdditionError(s) => write!(f, "Kaynağı Bundle'a ekleme hatası: {}", s),
            I18nError::BundleNotFound(s) => write!(f, "Dil paketi (bundle) bulunamadı: {}", s),
            I18nError::MessageNotFound(s) => write!(f, "Mesaj bulunamadı: {}", s),
            I18nError::MessageValueError(s) => write!(f, "Mesaj değeri bulunamadı: {}", s),
            I18nError::MessageEvaluationError(s) => write!(f, "Mesaj değerlendirme hatası: {}", s),
            I18nError::Sahne64ResourceError(e) => write!(f, "Sahne64 Kaynak hatası: {:?}", e), // SahneError'ın Debug çıktısını kullan
            I18nError::ResourceUtf8Error(s) => write!(f, "Kaynak içeriği UTF-8 değil: {}", s),
        }
    }
}

// From implementasyonu, SahneError'dan I18nError'a kolay dönüşüm sağlar.
impl From<SahneError> for I18nError {
    fn from(err: SahneError) -> Self {
        // SahneError'ı Sahne64ResourceError varyantı içinde sarmalar.
        // Daha spesifik hataları burada maplemek isterseniz match kullanın.
        I18nError::Sahne64ResourceError(err)
    }
}


// Uluslararasılaştırma (i18n) yöneticisi yapısı. Farklı diller için FluentBundle'ları tutar.
pub struct I18n {
    // HashMap<Dil Tanımlayıcısı, FluentBundle<FluentResource>>
    bundles: HashMap<LanguageIdentifier, FluentBundle<FluentResource>>, // alloc::collections::HashMap
    default_language: Option<LanguageIdentifier>, // Varsayılan dil (Option<LanguageIdentifier>)
}

impl I18n {
    // Yeni bir I18n yöneticisi oluşturur ve dil dosyalarını yükler.
    // locale_resources: HashMap<Dil Tanımlayıcısı, Vec<&str>> - dil tanımlayıcılarını ve ilgili Kaynak ID'lerinin listesini eşler.
    // default_language: Varsayılan dil tanımlayıcısı.
    pub fn new(
        locale_resources: HashMap<LanguageIdentifier, Vec<&str>>, // &str Kaynak ID'leri
        default_language: Option<LanguageIdentifier>
    ) -> Result<Self, I18nError> {
        let mut bundles = HashMap::new(); // alloc::collections::HashMap

        for (lang_id, resource_ids) in locale_resources { // Iterasyon
            // Her dil için yeni bir FluentBundle oluştur.
            let mut bundle = FluentBundle::new(vec![lang_id.clone()]); // vec! ve clone alloc gerektirir. FluentBundle::new de alloc kullanır.

            // Dil paketindeki her bir Kaynak (.ftl dosyası gibi) için
            for resource_id in resource_ids { // Iterasyon
                 if resource_id.ends_with(".ftl") { // Kaynak ID'sinin .ftl ile bittiğini kontrol et (no_std &str metodu)
                    // Kaynak içeriğini oku (resource::*, read_resource_to_string helper kullanılabilir)
                    let source = match super::read_resource_to_string(resource_id) { // srcconflict.rs'deki helper fonksiyonu kullanalım
                         Ok(s) => s,
                         Err(e) => {
                             // Kaynak okuma veya UTF-8 hatasını I18nError'a çevir
                             // read_resource_to_string PaketYoneticisiHatasi döner.
                             // Onu I18nError'a çevirmeliyiz. PaketYoneticisiHatasi'ndan I18nError'a From implementasyonu eklenebilir mi?
                             // Veya burada match ile PaketYoneticisiHatasi'nın türüne bakıp I18nError'a mapleyelim.
                             // PaketYoneticisiHatasi'nın SahneApiError ve ParsingError varyantlarını I18nError'a mapleyelim.
                             match e {
                                PaketYoneticisiHatasi::SahneApiError(se) => return Err(I18nError::Sahne64ResourceError(se)),
                                PaketYoneticisiHatasi::ParsingError(s) => return Err(I18nError::ResourceUtf8Error(s)), // ParsingError genellikle UTF8 hatasıdır burada
                                _ => {
                                    // Diğer PaketYoneticisiHatasi türleri beklenmemeli burada. Loglayalım.
                                    eprintln!("Beklenmedik hata Kaynak okunurken ({}): {:?}", resource_id, e);
                                    // Unknown error olarak dönebiliriz, ama I18nError'da UnknownError yok.
                                    // En yakın hata ResourceError olabilir, ama SahneError içermez.
                                    // Yeni bir I18nError varyantı gerekebilir: UnexpectedError(String)?
                                    return Err(I18nError::Sahne64ResourceError(SahneError::UnknownSystemCall)); // Veya uygun bir SahneError simülasyonu
                                }
                             }
                             // Alternatif: read_resource_to_string direkt Result<String, I18nError> dönecek şekilde refaktore edilebilir.
                         }
                    };

                    // Kaynak içeriğini FluentResource'a ayrıştır.
                    match FluentResource::try_new(source) { // source String, FluentResource::try_new String'i consume eder.
                        Ok(resource) => { // resource FluentResource
                            // Ayrıştırılan kaynağı bundle'a ekle.
                            if let Err(errors) = bundle.add_resource(resource) { // resource consume edilir. add_resource Result<(), Vec<FluentError>> döner
                                for error in errors { // Hata listesi üzerinde iterasyon
                                    eprintln!("Kaynak {} bundle'a eklenirken hata: {:?}", resource_id, error);
                                    // FluentError'ları string'e çevirip ResourceAdditionError'a eklemek alloc gerektirir.
                                    // Sadece loglayıp genel hata dönelim.
                                }
                                // resource_id.to_string() String klonlama (alloc gerektirir).
                                return Err(I18nError::ResourceAdditionError(resource_id.to_string()));
                            }
                        }
                        // try_new hata döndürürse (Source, ParseError) tuple döner.
                        Err((_, e)) => { // e ParseError
                             eprintln!("Kaynak {} ayrıştırılırken hata: {:?}", resource_id, e);
                            // ParseError'ı string'e çevirip FluentResourceParseError'a eklemek alloc gerektirir.
                            return Err(I18nError::FluentResourceParseError(format!("Kaynak {} ayrıştırma hatası", resource_id))); // format! alloc gerektirir. e.to_string() daha iyi
                        }
                    }
                }
            }
            bundles.insert(lang_id, bundle); // Bundle'ı HashMap'e ekle (alloc gerektirir)
        }

        Ok(I18n { bundles, default_language }) // Yeni I18n struct'ını döndür
    }

    // Belirtilen dil için mesajı alır.
    // lang_id: İstenen dilin tanımlayıcısı.
    // key: Mesajın anahtarı.
    // args: Mesaj formatlama için argümanlar (isteğe bağlı). HashMap<&str, FluentValue> no_std uyumlu.
    // Dönüş değeri: Lokalize edilmiş String veya hata.
    pub fn get_message(
        &self,
        lang_id: &LanguageIdentifier, // &LanguageIdentifier (no_std uyumlu)
        key: &str, // &str (no_std uyumlu)
        args: Option<&HashMap<&str, FluentValue>>, // Option<&HashMap<...>> (no_std uyumlu)
    ) -> Result<LocalizedString, I18nError> { // LocalizedString (fluent-bundle'dan, no_std uyumlu olmalı)
        // İstenen dil için bundle'ı bul. Bulamazsa varsayılan dili dene.
        let bundle = self.bundles.get(lang_id) // HashMap get (&LanguageIdentifier, LanguageIdentifier için Hash ve Eq kullanır)
            .or_else(|| {
                // Varsayılan dil ayarlanmışsa ve o dil için bundle varsa onu al.
                self.default_language.as_ref().and_then(|default_lang| self.bundles.get(default_lang)) // Option as_ref, and_then, HashMap get
            })
            // Bundle bulunamazsa hata dön. lang_id.to_string() alloc gerektirir.
            .ok_or_else(|| I18nError::BundleNotFound(lang_id.to_string()))?; // ok_or_else (closure içinde to_string), BundleNotFound (String)

        // Bundle'dan mesajı al.
        let message = bundle.get_message(key) // FluentBundle get_message (&str)
            // Mesaj bulunamazsa hata dön. key.to_string() alloc gerektirir.
            .ok_or_else(|| I18nError::MessageNotFound(key.to_string()))?; // ok_or_else, MessageNotFound (String)

        // Mesajın değerini (pattern) al.
        let pattern = message.value() // FluentMessage value() -> Option<&FluentPattern>
            // Değer bulunamazsa hata dön. key.to_string() alloc gerektirir.
            .ok_or_else(|| I18nError::MessageValueError(key.to_string()))?; // ok_or_else, MessageValueError (String)

        // Pattern'ı argümanlarla birlikte değerlendir (formatla).
        pattern.evaluate(bundle, args) // FluentPattern evaluate
            // Değerlendirme hatası olursa hata dön. e.to_string() alloc gerektirir.
            .map_err(|e| I18nError::MessageEvaluationError(e.to_string())) // map_err, MessageEvaluationError (String)
    }

    // Helper: resource::read_resource_to_string fonksiyonunu kullanıyoruz.
    // Bu fonksiyon muhtemelen srcconflict.rs veya başka bir helper modülünde tanımlıdır.
     use super::read_resource_to_string; // Eğer aynı crate'telerse
}

// --- I18nError enum tanımının no_std uyumlu hali ---
// (paket_yoneticisi_hata.rs dosyasında veya ilgili modülde olmalıydı, ama burada tanımlandı)

// Debug ve Display implementasyonları yukarıda manuel olarak yapıldı.
// From<SahneError> implementasyonu yukarıda manuel olarak yapıldı.

// Artık thiserror crate'ine ve std::io importuna gerek yok.

// --- read_resource_to_string helper fonksiyonu ---
// Bu fonksiyon srcconflict.rs'de tanımlanmıştı. Burada kullanabilmek için
// ya bu dosyaya kopyalanmalı ya da ayrı bir helper modülünde tanımlanıp import edilmeli.
// Kopyalama, modüller arası bağımlılığı azaltır ama kod tekrarı yaratır.
// Ayrı helper modülü (örn. src/utils/resource_helpers.rs) daha iyidir.
// Şimdilik srcconflict.rs'den kopyalandığını varsayalım veya path düzeltilsin (super::).

// src/utils/resource_helpers.rs (Örnek helper modülü)
#![no_std]
extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use alloc::borrow::ToOwned;

use crate::resource;
use crate::SahneError;
use crate::Handle;
use crate::paket_yoneticisi_hata::PaketYoneticisiHatasi; // Hata dönüşümü için

// Kaynaktan tüm içeriği String olarak oku.
pub fn read_resource_to_string(resource_id: &str) -> Result<String, PaketYoneticisiHatasi> {
    // ... (srcconflict.rs'deki implementasyonun aynısı) ...
    let handle = resource::acquire(resource_id, resource::MODE_READ)?; // ? operator PaketYoneticisiHatasi döner
    let mut buffer = Vec::new();
    let mut temp_buffer = [0u8; 512];

    loop {
        match resource::read(handle, &mut temp_buffer) {
            Ok(0) => break,
            Ok(bytes_read) => { buffer.extend_from_slice(&temp_buffer[..bytes_read]); }
            Err(e) => { let _ = resource::release(handle); return Err(PaketYoneticisiHatasi::from(e)); } // SahneError -> PaketYoneticisiHatasi
        }
    }
    let release_result = resource::release(handle);
     if let Err(e) = release_result { eprintln!("Helper: Kaynak release hatası ({}): {:?}", resource_id, e); }

    core::str::from_utf8(&buffer)
        .map(|s| s.to_owned()) // &str -> String
        .map_err(|_| {
             eprintln!("Helper: Kaynak içeriği geçerli UTF-8 değil ({})", resource_id);
             PaketYoneticisiHatasi::ParsingError(format!("Geçersiz UTF-8 Kaynak içeriği: {}", resource_id))
        })
}

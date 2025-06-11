#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // String, Vec, HashMap için

use alloc::string::String; // std::string::String yerine
use alloc::vec::Vec; // std::vec::Vec yerine
use alloc::collections::HashMap; // Checksums gibi ek alanlar için gerekebilir

// serde derive'lar (no_std uyumlu backend ile çalışır)
use serde::{Deserialize, Serialize};

// Paket Verilerini Temsil Eden Yapı.
// Debug, Clone, PartialEq, Eq, Hash derive'ları no_std'de çalışır (alloc ile).
// Hash derive'ı, eğer Paket struct'ını HashMap veya HashSet içinde kullanacaksak gereklidir.
// Örneğin, kurulu paketlerin bir kümesini veya map'ini tutmak için.
// Checksum ve dosyalar gibi alanlar eklendi (varsayımsal ihtiyaçlara göre).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Paket {
    pub ad: String, // Paketin adı (örn. "coreutils")
    pub surum: String, // Paketin sürümü (örn. "8.32") - Semantik versiyonlama için ayrıştırma gerekebilir
    pub bagimliliklar: Vec<String>, // Bu paketin ihtiyaç duyduğu diğer paketlerin ADLARI listesi (örn. ["libc", "zlib"])
    pub aciklama: Option<String>, // Paketin kısa açıklaması
    pub dosya_adi: Option<String>, // Uzak depoda veya önbellekte bulunan arşiv dosyasının adı (örn. "coreutils-8.32.tar.gz")

    // --- Ek Alanlar (Varsayımsal olarak paket formatı veya depo meta verilerinde bulunur) ---

    // Arşiv dosyasının bütünlüğünü doğrulamak için sağlama toplamı.
    // Birden fazla algoritma desteklenebilir. HashMap<Algoritma Adı, Checksum Değeri>.
    // Checksum doğrulama mantığı srcchecksum.rs'de bulunur, değeri burada saklarız.
    pub checksums: HashMap<String, String>, // HashMap alloc gerektirir.

    // Paketin kurulduğunda içereceği dosyaların listesi.
    // Kurulum sırasında nereye kopyalanacağını veya çıkarılacağını belirlemek için kullanılır.
    // Kaldırma sırasında hangi dosyaların silineceğini bilmek için de gereklidir (Sahne64'te resource silme eksik).
    pub dosyalar: Vec<String>, // Kurulum dizinine göre dosya yolları (örn. "bin/ls", "share/man/ls.1") Vec alloc gerektirir.

    // Kurulum ve kaldırma için betikler (Kaynak ID'si veya içerik)
    // resource::acquire ile çalıştırılabilir dosyalar veya betikler olabilir.
    pub kurulum_scripti: Option<String>, // Kurulum betiği Kaynak ID'si veya içeriği (String alloc gerektirir)
    pub kaldirma_scripti: Option<String>, // Kaldırma betiği Kaynak ID'si veya içeriği (String alloc gerektirir)

    // Lisans bilgisi, yazar vb. diğer meta veriler eklenebilir.
     pub lisans: Option<String>,
     pub yazar: Option<String>,
}

impl Paket {
    // Yeni bir temel Paket örneği oluşturur.
    // Diğer alanlar varsayılan/boş değerlerle başlatılır.
    pub fn yeni(ad: String, surum: String, bagimliliklar: Vec<String>) -> Paket {
        Paket {
            ad,
            surum,
            bagimliliklar, // Vec<String> alloc gerektirir.
            aciklama: None, // Option alloc gerektirmez
            dosya_adi: None, // Option alloc gerektirmez
            checksums: HashMap::new(), // HashMap::new() alloc gerektirir.
            dosyalar: Vec::new(), // Vec::new() alloc gerektirir.
            kurulum_scripti: None, // Option alloc gerektirmez
            kaldirma_scripti: None, // Option alloc gerektirmez
            // ... diğer alanlar default/None ...
        }
    }

    // Paket struct'ının kimliğini (ad ve sürüm) temsil eden bir method (Hash ve Eq derive'ları ile aynı bilgiyi verir).
    // Kullanım örneği: Başka bir struct içinde Paket'e referans yerine PaketId tutmak.
     #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
     pub struct PaketId { pub ad: String, pub surum: String }
     impl From<Paket> for PaketId { ... }
     impl From<&Paket> for PaketId { ... }

    // Semantik versiyon karşılaştırması için bir method (semver crate'i no_std uyumluysa kullanılabilir)
     pub fn surum_karsilastir(&self, diger_surum: &str) -> Option<core::cmp::Ordering> { ... }
}

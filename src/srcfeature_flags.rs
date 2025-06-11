#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // String için

use alloc::string::String;
// std::env yerine yapılandırma kaynağından okuyacağız
// use std::env;

// Yapılandırma struct'ını içe aktarın
use crate::srcconfig::Yapilandirma;

// Özel hata enum'ımızı içe aktar (no_std uyumlu hali) - aslında bu modül hata dönmeyebilir.
 use crate::paket_yoneticisi_hata::PaketYoneticisiHatasi;

// no_std uyumlu print makroları
use crate::print_macros::{println, eprintln};


// Özellik bayraklarını tutan yapı. Yapılandırma dosyasından yüklenir.
// Debug derive'ı no_std'de çalışır.
#[derive(Debug)]
pub struct FeatureFlags {
    pub compression: bool,
    pub network: bool,
    pub security: bool,
    // ... diğer özellik bayrakları
}

impl FeatureFlags {
    // Yapılandırma struct'ından özellik bayraklarını yükler.
    // Eğer yapılandırmada bir bayrak yoksa veya geçersizse varsayılan değeri kullanır.
    // config: Yüklenmiş Yapilandirma struct'ına referans.
    pub fn from_config(config: &Yapilandirma) -> Self {
        // Yapılandırma struct'ında özellik bayrakları için alanlar olduğunu varsayıyoruz.
        // Örn: Yapilandirma { ..., pub features: FeatureConfig { compression: bool, network: bool, ... }, ... }
        // Eğer doğrudan Yapilandirma içinde bool alanları varsa:
        FeatureFlags {
            // Yapılandırma struct'ından değeri oku, yoksa varsayılanı kullan.
            // Yapilandirma struct'ı Option<bool> veya bool tutabilir. Bool tutuyorsa default olmaz.
            // Option<bool> tuttuğunu varsayalım veya doğrudan Yapilandirma'da alanlar olduğunu.
            // Daha esnek bir Yapilandirma için ayrı bir features alanı olabilir.
            // Örn: config.features.compression.unwrap_or(false) veya config.compression
            // Şimdilik doğrudan Yapilandirma içinde bool alanları olduğunu varsayalım.
            // Yapilandirma struct'ı güncellenmeli.

            // Geçici olarak varsayılan değerler ile manuel initialize edelim,
            // Yapilandirma'dan okuma kısmı, Yapilandirma struct'ı netleşince yapılır.
            // Veya Yapilandirma içinde Option<bool> olduğunu varsayalım.
            compression: config.compression, // Varsayım: config.compression bool veya Option<bool>
            network: config.network, // Varsayım: config.network bool veya Option<bool>
            security: config.security, // Varsayım: config.security bool veya Option<bool>
            // ... diğer özellik bayrakları ...
        }
    }

    // Alternatif: Varsayılan değerlerle yeni bir FeatureFlags oluşturur (yapılandırma okunamadığında vb.)
    pub fn default() -> Self {
        FeatureFlags {
            compression: false, // Varsayılan: kapalı
            network: false,     // Varsayılan: kapalı
            security: false,    // Varsayılan: kapalı
            // ... diğer özellik bayrakları (varsayılan değerleri ile)
        }
    }

}

// main fonksiyonu bu dosyada olmamalıdır. srccli.rs veya lib.rs'de olmalı.
 #[cfg(feature = "std")] // main std gerektirir
 fn main() {
     let features = FeatureFlags::new(); // new yerine from_config kullanılmalı
     println!("Compression: {}", features.compression);
     println!("Network: {}", features.network);
     println!("Security: {}", features.security);
 }


// --- Yapilandirma struct tanımının güncellenmesi (srcconfig.rs) ---
// Feature flag'leri için alanlar eklenmeli.

// srcconfig.rs (Güncellenmiş - feature flag alanları eklenmiş)
#![no_std]
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec; // Postcard için

use serde::{Deserialize, Serialize};
use postcard::Error as PostcardError;

use crate::resource;
use crate::SahneError;
use crate::Handle;
use crate::paket_yoneticisi_hata::PaketYoneticisiHatasi;

// Yapılandırma verilerini tutan struct.
#[derive(Serialize, Deserialize, Debug)]
pub struct Yapilandirma {
    pub depo_url: String, // Paket deposu URL'i (Sahne64 Kaynak ID'si?)
    pub yerel_depo_yolu: String, // Yerel cache/repo Kaynak ID'si
    pub kurulum_dizini: String, // Kurulu paketlerin temel Kaynak ID'si
    pub onbellek_dizini: String, // Önbellek temel Kaynak ID'si

    // --- Yeni Alanlar: Özellik Bayrakları ---
    // Bunlar Yapilandirma dosyasından okunacak. Option<bool> kullanmak esneklik sağlar.
    pub compression: Option<bool>,
    pub network: Option<bool>,
    pub security: Option<bool>,
    // ... diğer özellik bayrakları ...
}

impl Yapilandirma {
    // ... yeni constructor, oku, yaz fonksiyonları (resource ve postcard kullanan) ...

    // Örnek: Varsayılan Yapılandırma oluşturma
    pub fn varsayilan() -> Self {
        Yapilandirma {
            depo_url: String::from("sahne://remotepkgrepo/"), // Varsayılan repo Kaynak ID'si
            yerel_depo_yolu: String::from("sahne://cache/repo/"), // Varsayılan yerel repo Kaynak ID'si
            kurulum_dizini: String::from("sahne://installed_packages/"), // Varsayılan kurulum Kaynak ID'si
            onbellek_dizini: String::from("sahne://cache/packages/"), // Varsayılan önbellek Kaynak ID'si
            // Varsayılan özellik bayrakları: None (yapılandırmada belirtilmemiş)
            compression: None,
            network: None,
            security: None,
            // ...
        }
    }
}

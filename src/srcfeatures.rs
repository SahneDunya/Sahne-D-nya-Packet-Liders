#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // HashSet, String, Vec, format! için

use alloc::collections::HashSet; // std::collections::HashSet yerine
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use core::str::FromStr; // std::str::FromStr yerine
use alloc::borrow::ToOwned; // to_lowercase() sonrası as_str() yerine to_owned() kullanabiliriz (veya to_lowercase().collect::<Vec<u8>>()). as_str() sonrası &str karşılaştırması en iyisi.


// Farklı paket yöneticisi özelliklerini temsil eden ana enum.
// Debug, PartialEq, Eq, Hash, Clone, Copy derive'ları no_std'de çalışır. Hash, alloc gerektirir.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Feature {
    Compression(CompressionAlgorithm),
    Network(NetworkProtocol),
    Security(SecurityFeature),
    Logging(LoggingFramework),
    // ... diğer özellik kategorileri
}

// Sıkıştırma algoritmalarını temsil eden enum.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum CompressionAlgorithm {
    Gzip,
    Bzip2,
    Zstd,
    Lz4,
    Brotli,
}

// Ağ protokollerini temsil eden enum.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum NetworkProtocol {
    Http,
    Https,
    Ftp,
    Tcp,
    Udp,
    Websocket,
    Smtp,
    Pop3,
    Imap,
}

// Güvenlik özelliklerini temsil eden enum.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum SecurityFeature {
    SignatureVerification,
    Sandbox,
    Firewall,
    Encryption,
    Authorization,
    Authentication,
    DataMasking,
    RateLimiting,
}

// Logging frameworklerini temsil eden enum.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum LoggingFramework {
    File,
    Console,
    Database,
    Remote,
    Syslog,
    EventTracing,
}

// String'den Feature enum'ına dönüşüm implementasyonu.
// type Err = String; // Hata tipi String, alloc gerektirir.
impl FromStr for Feature {
    type Err = String; // core::str::FromStr trait'inin hatası String olabilir (alloc gerektirir)

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Küçük harfe çevirme ve karşılaştırma. to_lowercase() alloc gerektirir.
        match s.to_lowercase().as_str() {
            "gzip" => Ok(Feature::Compression(CompressionAlgorithm::Gzip)),
            "bzip2" => Ok(Feature::Compression(CompressionAlgorithm::Bzip2)),
            "zstd" => Ok(Feature::Compression(CompressionAlgorithm::Zstd)),
            "lz4" => Ok(Feature::Compression(CompressionAlgorithm::Lz4)),
            "brotli" => Ok(Feature::Compression(CompressionAlgorithm::Brotli)),

            "http" => Ok(Feature::Network(NetworkProtocol::Http)),
            "https" => Ok(Feature::Network(NetworkProtocol::Https)),
            "ftp" => Ok(Feature::Network(NetworkProtocol::Ftp)),
            "tcp" => Ok(Feature::Network(NetworkProtocol::Tcp)),
            "udp" => Ok(Feature::Network(NetworkProtocol::Udp)),
            "websocket" => Ok(Feature::Network(NetworkProtocol::Websocket)),
            "smtp" => Ok(Feature::Network(NetworkProtocol::Smtp)),
            "pop3" => Ok(Feature::Network(NetworkProtocol::Pop3)),
            "imap" => Ok(Feature::Network(NetworkProtocol::Imap)),

            "signature_verification" => Ok(Feature::Security(SecurityFeature::SignatureVerification)),
            "sandbox" => Ok(Feature::Security(SecurityFeature::Sandbox)),
            "firewall" => Ok(Feature::Security(SecurityFeature::Firewall)),
            "encryption" => Ok(Feature::Security(SecurityFeature::Encryption)),
            "authorization" => Ok(Feature::Security(SecurityFeature::Authorization)),
            "authentication" => Ok(Feature::Security(SecurityFeature::Authentication)),
            "data_masking" => Ok(Feature::Security(SecurityFeature::DataMasking)),
            "rate_limiting" => Ok(Feature::Security(SecurityFeature::RateLimiting)),

            "file_logging" => Ok(Feature::Logging(LoggingFramework::File)),
            "console_logging" => Ok(Feature::Logging(LoggingFramework::Console)),
            "database_logging" => Ok(Feature::Logging(LoggingFramework::Database)),
            "remote_logging" => Ok(Feature::Logging(LoggingFramework::Remote)),
            "syslog_logging" => Ok(Feature::Logging(LoggingFramework::Syslog)),
            "event_tracing" => Ok(Feature::Logging(LoggingFramework::EventTracing)),

            // Bilinmeyen özellik durumunda hata dön. format! alloc gerektirir.
            _ => Err(format!("Bilinmeyen özellik: {}", s)),
        }
    }
}

// Etkin özelliklerin kümesini yöneten yapı.
pub struct FeatureSet {
    features: HashSet<Feature>, // alloc::collections::HashSet (alloc gerektirir)
}

impl FeatureSet {
    pub fn new() -> Self {
        FeatureSet {
            features: HashSet::new(), // HashSet::new() (alloc gerektirebilir, iç tahsis)
        }
    }

    // Bir özelliği etkinleştirir (kümeye ekler).
    pub fn enable(&mut self, feature: Feature) {
        self.features.insert(feature); // HashSet::insert (alloc gerektirebilir)
    }

    // Bir özelliği devre dışı bırakır (kümeden siler).
    pub fn disable(&mut self, feature: Feature) {
        self.features.remove(&feature); // HashSet::remove
    }

    // Bir özelliğin etkin olup olmadığını kontrol eder.
    pub fn is_enabled(&self, feature: &Feature) -> bool {
        self.features.contains(feature) // HashSet::contains
    }

    // String diliminden bir FeatureSet oluşturur.
    // features: Özellik isimlerinin string dilimi (örn. &["gzip", "http"]).
    // Result türü String, alloc gerektirir.
    pub fn from_strs(features: &[&str]) -> Result<Self, String> { // Hata tipi String (alloc gerektirir)
        let mut feature_set = FeatureSet::new();
        for feature_str in features {
            // String'i Feature enum'ına ayrıştır. Ayrıştırma hatası olursa fonksiyon hata döner.
            let feature = Feature::from_str(feature_str)?; // ? operatörü String hatasını yayar
            feature_set.enable(feature); // Etkinleştir
        }
        Ok(feature_set) // Başarılı sonuç (FeatureSet yapısı alloc kullanır)
    }

    // Etkinleştirilmiş özelliklerin bir listesini döndürür.
    pub fn enabled_features(&self) -> Vec<Feature> { // Dönüş tipi Vec<Feature>, alloc gerektirir
        self.features.iter().cloned().collect() // HashSet iter, clone (Feature Copy olduğu için etkisiz), collect (Vec'e dönüştürme, alloc gerektirir)
    }

    // FeatureSet yapısını Package struct'ında kullanabilmek için gerekli olabilir
     pub fn iter(&self) -> impl Iterator<Item = &Feature> { self.features.iter() }
}

// #[cfg(test)] bloğu std test runner'ı gerektirir.
// No_std ortamında testler için özel bir test runner veya std feature flag'i gerekir.
// Bu blok no_std ortamında derlenmeyecektir eğer std feature aktif değilse.
#[cfg(test)]
mod tests {
    // Eğer std feature kullanılıyorsa, bu testler çalışır.
    // Eğer no_std test runner kullanılıyorsa, onun gereksinimleri karşılanmalıdır.
    use super::*;
    // std::collections::HashSet std feature ile kullanılır.
     use std::collections::HashSet;

    #[test]
    fn test_from_str() {
        assert_eq!(Feature::from_str("gzip").unwrap(), Feature::Compression(CompressionAlgorithm::Gzip));
        assert_eq!(Feature::from_str("Bzip2").unwrap(), Feature::Compression(CompressionAlgorithm::Bzip2));
        assert_eq!(Feature::from_str("lz4").unwrap(), Feature::Compression(CompressionAlgorithm::Lz4));
        assert_eq!(Feature::from_str("HTTP").unwrap(), Feature::Network(NetworkProtocol::Http));
        assert_eq!(Feature::from_str("https").unwrap(), Feature::Network(NetworkProtocol::Https));
        assert_eq!(Feature::from_str("websocket").unwrap(), Feature::Network(NetworkProtocol::Websocket));
        assert_eq!(Feature::from_str("signature_verification").unwrap(), Feature::Security(SecurityFeature::SignatureVerification));
        assert_eq!(Feature::from_str("sandbox").unwrap(), Feature::Security(SecurityFeature::Sandbox));
        assert_eq!(Feature::from_str("firewall").unwrap(), Feature::Security(SecurityFeature::Firewall));
        assert_eq!(Feature::from_str("file_logging").unwrap(), Feature::Logging(LoggingFramework::File));
        assert_eq!(Feature::from_str("console_logging").unwrap(), Feature::Logging(LoggingFramework::Console));

        assert!(Feature::from_str("unknown_feature").is_err());
        // Hata mesajını kontrol etmek isteyebiliriz:
         assert_eq!(Feature::from_str("unknown_feature").unwrap_err(), "Bilinmeyen özellik: unknown_feature".to_string());
    }

    #[test]
    fn test_feature_set() {
        let mut feature_set = FeatureSet::new();

        let gzip_feature = Feature::Compression(CompressionAlgorithm::Gzip);
        let https_feature = Feature::Network(NetworkProtocol::Https);

        feature_set.enable(gzip_feature);
        feature_set.enable(https_feature);

        assert!(feature_set.is_enabled(&gzip_feature));
        assert!(feature_set.is_enabled(&https_feature));

        feature_set.disable(gzip_feature);
        assert!(!feature_set.is_enabled(&gzip_feature));
        assert!(feature_set.is_enabled(&https_feature));

        let enabled_features = feature_set.enabled_features();
        assert_eq!(enabled_features.len(), 1);
         assert!(enabled_features.contains(&https_feature)); // contains method is on Vec, needs Eq/PartialEq
        assert!(enabled_features.iter().any(|&f| f == https_feature)); // Vec<Feature> üzerinde contains kontrolü
    }

    #[test]
    fn test_from_strs() {
        let features_str = &["gzip", "https", "sandbox", "file_logging"];
        let feature_set = FeatureSet::from_strs(features_str).unwrap();

        assert!(feature_set.is_enabled(&Feature::Compression(CompressionAlgorithm::Gzip)));
        assert!(feature_set.is_enabled(&Feature::Network(NetworkProtocol::Https)));
        assert!(feature_set.is_enabled(&Feature::Security(SecurityFeature::Sandbox)));
        assert!(feature_set.is_enabled(&Feature::Logging(LoggingFramework::File)));

        // Geçersiz özellik içeren durumu test et
        let features_with_invalid = &["gzip", "invalid_feature", "https"];
        let result = FeatureSet::from_strs(features_with_invalid);
        assert!(result.is_err());
        // Hata mesajını kontrol etmek isteyebiliriz:
         assert_eq!(result.unwrap_err(), "Bilinmeyen özellik: invalid_feature".to_string());
    }
}

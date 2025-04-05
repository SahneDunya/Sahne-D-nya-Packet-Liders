use std::collections::HashSet;
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Feature {
    Compression(CompressionAlgorithm),
    Network(NetworkProtocol),
    Security(SecurityFeature),
    Logging(LoggingFramework), // Yeni özellik kategorisi eklendi
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum CompressionAlgorithm {
    Gzip,
    Bzip2,
    Zstd,
    Lz4,        // Yeni sıkıştırma algoritması eklendi
    Brotli,     // Yeni sıkıştırma algoritması eklendi
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum NetworkProtocol {
    Http,
    Https,
    Ftp,
    Tcp,        // Yeni ağ protokolü eklendi
    Udp,        // Yeni ağ protokolü eklendi
    Websocket,  // Yeni ağ protokolü eklendi
    Smtp,       // Yeni ağ protokolü eklendi
    Pop3,       // Yeni ağ protokolü eklendi
    Imap,       // Yeni ağ protokolü eklendi
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum SecurityFeature {
    SignatureVerification,
    Sandbox,
    Firewall,             // Yeni güvenlik özelliği eklendi
    Encryption,           // Yeni güvenlik özelliği eklendi
    Authorization,        // Yeni güvenlik özelliği eklendi
    Authentication,       // Yeni güvenlik özelliği eklendi
    DataMasking,          // Yeni güvenlik özelliği eklendi
    RateLimiting,         // Yeni güvenlik özelliği eklendi
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum LoggingFramework { // Yeni enum: LoggingFramework
    File,
    Console,
    Database,
    Remote,
    Syslog,      // Yeni logging framework eklendi
    EventTracing, // Yeni logging framework eklendi
}

impl FromStr for Feature {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "gzip" => Ok(Feature::Compression(CompressionAlgorithm::Gzip)),
            "bzip2" => Ok(Feature::Compression(CompressionAlgorithm::Bzip2)),
            "zstd" => Ok(Feature::Compression(CompressionAlgorithm::Zstd)),
            "lz4" => Ok(Feature::Compression(CompressionAlgorithm::Lz4)), // Yeni özellik eklendi
            "brotli" => Ok(Feature::Compression(CompressionAlgorithm::Brotli)), // Yeni özellik eklendi

            "http" => Ok(Feature::Network(NetworkProtocol::Http)),
            "https" => Ok(Feature::Network(NetworkProtocol::Https)),
            "ftp" => Ok(Feature::Network(NetworkProtocol::Ftp)),
            "tcp" => Ok(Feature::Network(NetworkProtocol::Tcp)),       // Yeni özellik eklendi
            "udp" => Ok(Feature::Network(NetworkProtocol::Udp)),       // Yeni özellik eklendi
            "websocket" => Ok(Feature::Network(NetworkProtocol::Websocket)), // Yeni özellik eklendi
            "smtp" => Ok(Feature::Network(NetworkProtocol::Smtp)),      // Yeni özellik eklendi
            "pop3" => Ok(Feature::Network(NetworkProtocol::Pop3)),      // Yeni özellik eklendi
            "imap" => Ok(Feature::Network(NetworkProtocol::Imap)),      // Yeni özellik eklendi

            "signature_verification" => Ok(Feature::Security(SecurityFeature::SignatureVerification)),
            "sandbox" => Ok(Feature::Security(SecurityFeature::Sandbox)),
            "firewall" => Ok(Feature::Security(SecurityFeature::Firewall)), // Yeni özellik eklendi
            "encryption" => Ok(Feature::Security(SecurityFeature::Encryption)), // Yeni özellik eklendi
            "authorization" => Ok(Feature::Security(SecurityFeature::Authorization)), // Yeni özellik eklendi
            "authentication" => Ok(Feature::Security(SecurityFeature::Authentication)),// Yeni özellik eklendi
            "data_masking" => Ok(Feature::Security(SecurityFeature::DataMasking)),  // Yeni özellik eklendi
            "rate_limiting" => Ok(Feature::Security(SecurityFeature::RateLimiting)), // Yeni özellik eklendi

            "file_logging" => Ok(Feature::Logging(LoggingFramework::File)),    // Yeni özellik eklendi (Logging -> File)
            "console_logging" => Ok(Feature::Logging(LoggingFramework::Console)), // Yeni özellik eklendi (Logging -> Console)
            "database_logging" => Ok(Feature::Logging(LoggingFramework::Database)),// Yeni özellik eklendi (Logging -> Database)
            "remote_logging" => Ok(Feature::Logging(LoggingFramework::Remote)),  // Yeni özellik eklendi (Logging -> Remote)
            "syslog_logging" => Ok(Feature::Logging(LoggingFramework::Syslog)),    // Yeni özellik eklendi (Logging -> Syslog)
            "event_tracing" => Ok(Feature::Logging(LoggingFramework::EventTracing)),// Yeni özellik eklendi (Logging -> EventTracing)


            _ => Err(format!("Bilinmeyen özellik: {}", s)),
        }
    }
}

pub struct FeatureSet {
    features: HashSet<Feature>,
}

impl FeatureSet {
    pub fn new() -> Self {
        FeatureSet {
            features: HashSet::new(),
        }
    }

    pub fn enable(&mut self, feature: Feature) {
        self.features.insert(feature);
    }

    pub fn disable(&mut self, feature: Feature) {
        self.features.remove(&feature);
    }

    pub fn is_enabled(&self, feature: &Feature) -> bool {
        self.features.contains(feature)
    }

    pub fn from_strs(features: &[&str]) -> Result<Self, String> {
        let mut feature_set = FeatureSet::new();
        for feature_str in features {
            let feature = Feature::from_str(feature_str)?;
            feature_set.enable(feature);
        }
        Ok(feature_set)
    }

    pub fn enabled_features(&self) -> Vec<Feature> { // Yeni fonksiyon eklendi
        self.features.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(enabled_features.contains(&https_feature));
    }

    #[test]
    fn test_from_strs() {
        let features_str = &["gzip", "https", "sandbox", "file_logging"];
        let feature_set = FeatureSet::from_strs(features_str).unwrap();

        assert!(feature_set.is_enabled(&Feature::Compression(CompressionAlgorithm::Gzip)));
        assert!(feature_set.is_enabled(&Feature::Network(NetworkProtocol::Https)));
        assert!(feature_set.is_enabled(&Feature::Security(SecurityFeature::Sandbox)));
        assert!(feature_set.is_enabled(&Feature::Logging(LoggingFramework::File)));
    }
}
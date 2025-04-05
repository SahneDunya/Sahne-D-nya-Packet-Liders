use std::env;

pub struct FeatureFlags {
    pub compression: bool,
    pub network: bool,
    pub security: bool,
    // ... diğer özellik bayrakları
}

impl FeatureFlags {
    pub fn new() -> Self {
        FeatureFlags {
            compression: Self::get_feature_flag("COMPRESSION", false), // Varsayılan: kapalı
            network: Self::get_feature_flag("NETWORK", false),     // Varsayılan: kapalı
            security: Self::get_feature_flag("SECURITY", false),    // Varsayılan: kapalı
            // ... diğer özellik bayrakları (varsayılan değerleri ile)
        }
    }

    // Daha esnek ve okunabilir özellik bayrağı alma fonksiyonu
    fn get_feature_flag(name: &str, default_value: bool) -> bool {
        match env::var(format!("PKG_FEATURE_{}", name)) {
            Ok(val) => {
                // Çevresel değişken değerini küçük harfe dönüştürerek karşılaştırmayı kolaylaştırıyoruz
                match val.to_lowercase().as_str() {
                    "true" | "1" | "yes" => true,
                    "false" | "0" | "no" | "" => false, // Boş dize de 'false' olarak kabul edilir
                    _ => {
                        eprintln!(
                            "Uyarı: '{}' özellik bayrağı için geçersiz değer: '{}'. 'true', 'false', '1', '0', 'yes', 'no' değerlerinden birini bekliyorduk. Varsayılan değer ({}) kullanılıyor.",
                            name, val, default_value
                        );
                        default_value // Geçersiz değer durumunda varsayılan değeri döndür
                    }
                }
            }
            Err(_) => {
                // Çevresel değişken bulunamadığında da varsayılan değeri döndür
                default_value
            }
        }
    }
}

fn main() {
    let features = FeatureFlags::new();
    println!("Compression: {}", features.compression);
    println!("Network: {}", features.network);
    println!("Security: {}", features.security);
}
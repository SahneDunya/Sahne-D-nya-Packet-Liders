use serde::{Deserialize, Serialize};
use std::fs;
use toml;

#[derive(Serialize, Deserialize, Debug)]
pub struct Yapilandirma {
    pub depo_url: String,
    pub yerel_depo_yolu: String,
    pub kurulum_dizini: String,
    pub onbellek_dizini: String,
}

impl Yapilandirma {
    pub fn yeni(depo_url: String, yerel_depo_yolu: String, kurulum_dizini: String, onbellek_dizini: String) -> Yapilandirma {
        Yapilandirma {
            depo_url,
            yerel_depo_yolu,
            kurulum_dizini,
            onbellek_dizini,
        }
    }

    pub fn oku(dosya_yolu: &str) -> Result<Yapilandirma, String> {
        let dosya_icerigi = match fs::read_to_string(dosya_yolu) {
            Ok(icerik) => icerik,
            Err(e) => return Err(format!("Yapılandırma dosyası okunamadı: {}", e)),
        };

        match toml::from_str(&dosya_icerigi) {
            Ok(yapilandirma) => Ok(yapilandirma),
            Err(e) => Err(format!("Yapılandırma dosyası çözümlenemedi: {}", e)),
        }
    }

    pub fn yaz(&self, dosya_yolu: &str) -> Result<(), String> {
        let dosya_icerigi = match toml::to_string_pretty(self) {
            Ok(icerik) => icerik,
            Err(e) => return Err(format!("Yapılandırma serileştirilemedi: {}", e)),
        };

        match fs::write(dosya_yolu, dosya_icerigi) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Yapılandırma dosyası yazılamadı: {}", e)),
        }
    }
}
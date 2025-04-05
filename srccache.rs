use disk_cache::{Cache, Error as DiskCacheError}; // DiskCacheError'ı yeniden adlandır
use std::path::Path;
use crate::paket_yoneticisi_hata::PaketYoneticisiHata; // Özel hata enum'ımızı içe aktar

// Assuming 'Sahne64' modules are in the same crate
use crate::fs;
use crate::SahneError;

pub struct PaketOnbellek {
    cache: Cache,
}

impl PaketOnbellek {
    pub fn yeni(onbellek_dizini: &Path) -> Result<PaketOnbellek, PaketYoneticisiHata> {
        let onbellek_dizini_str = onbellek_dizini.to_str().ok_or(PaketYoneticisiHata::OnbellekHatasi(format!("Geçersiz önbellek dizini: {:?}", onbellek_dizini)))?;

        // Sahne64 özel fonksiyonlarını kullanarak önbellek dizinini oluştur
        if !fs::path_exists(onbellek_dizini_str) {
            fs::create_dir_recursive(onbellek_dizini_str, 0o755).map_err(|e| {
                PaketYoneticisiHata::OnbellekHatasi(format!(
                    "Önbellek dizini oluşturulamadı: {}, Hata: {}",
                    onbellek_dizini_str, e
                ))
            })?;
        }

        // disk_cache crate'ini kullanarak önbelleği oluştur
        let cache = Cache::new(onbellek_dizini).map_err(|e| {
            PaketYoneticisiHata::OnbellekHatasi(format!(
                "Önbellek oluşturulamadı: {:?}, Hata: {}",
                onbellek_dizini, e
            ))
        })?;
        Ok(PaketOnbellek { cache })
    }

    pub fn paket_verisini_al(&self, anahtar: &str) -> Result<Option<Vec<u8>>, PaketYoneticisiHata> {
        self.cache.get(anahtar).map_err(|e| {
            PaketYoneticisiHata::OnbellekHatasi(format!(
                "Önbellekten veri alınamadı (anahtar: {}): {}",
                anahtar, e
            ))
        })
    }

    pub fn paket_verisini_kaydet(&self, anahtar: &str, veri: &[u8]) -> Result<(), PaketYoneticisiHata> {
        self.cache.insert(anahtar, veri).map_err(|e| {
            PaketYoneticisiHata::OnbellekHatasi(format!(
                "Önbelleğe veri kaydedilemedi (anahtar: {}): {}",
                anahtar, e
            ))
        })?;
        Ok(())
    }

    pub fn paket_verisini_sil(&self, anahtar: &str) -> Result<(), PaketYoneticisiHata> {
        self.cache.remove(anahtar).map_err(|e| {
            PaketYoneticisiHata::OnbellekHatasi(format!(
                "Önbellekten veri silinemedi (anahtar: {}): {}",
                anahtar, e
            ))
        })?;
        Ok(())
    }

    pub fn onbellegi_temizle(&self) -> Result<(), PaketYoneticisiHata> {
        self.cache.clear().map_err(|e| {
            PaketYoneticisiHata::OnbellekHatasi(format!(
                "Önbellek temizlenemedi: {}",
                e
            ))
        })?;
        Ok(())
    }
}

// Eklenen PaketYoneticisiHata enum tanımı (örnek olarak, gerçek tanımınız farklı olabilir)
// Bu enum'ı paket_yoneticisi_hata.rs dosyasına veya ilgili modüle eklemeniz gerekir.
#[derive(thiserror::Error, Debug)]
pub enum PaketYoneticisiHata {
    #[error("Dosya sistemi hatası: {0}")]
    DosyaSistemiHatasi(#[from] std::io::Error),

    #[error("HTTP isteği hatası: {0}")]
    HttpIstegiHatasi(#[from] reqwest::Error),

    #[error("JSON hatası: {0}")]
    JsonHatasi(#[from] serde_json::Error),

    #[error("ZIP hatası: {0}")]
    ZipHatasi(#[from] zip::result::ZipError),

    #[error("Paket bulunamadı: {0}")]
    PaketBulunamadi(String),

    #[error("Paket kurulum hatası: {0}")]
    PaketKurulumHatasi(String),

    #[error("Önbellek hatası: {0}")] // Yeni: Önbellek hataları için varyant
    OnbellekHatasi(String),

    #[error("Bilinmeyen hata: {0}")]
    BilinmeyenHata(String),
}
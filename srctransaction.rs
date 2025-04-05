use std::io;
use std::io::ErrorKind;
use std::path::Path;
use crate::paket_yoneticisi_hata::PaketYoneticisiHata; // Özel hata enum'ımızı içe aktar
use log::{info, warn, error, debug}; // log kütüphanesini içe aktar

// Assuming 'Sahne64' modules are in the same crate
use crate::fs;
use crate::SahneError;

pub struct IslemYoneticisi {
    islem_gunlugu: String,
}

impl IslemYoneticisi {
    // Yeni bir IslemYoneticisi örneği oluşturur.
    pub fn yeni(islem_gunlugu: &str) -> Self {
        IslemYoneticisi {
            islem_gunlugu: islem_gunlugu.to_string(),
        }
    }

    // İşlem günlüğüne "ISLEM BASLADI" kaydını yazar ve işlemi başlatır.
    pub fn baslat_islem(&self) -> Result<(), PaketYoneticisiHata> {
        info!("İşlem başlatılıyor. Günlük dosyası: {}", self.islem_gunlugu);
        let günlük_dosyasi = &self.islem_gunlugu;
        let fd = fs::open(günlük_dosyasi, fs::O_CREAT | fs::O_WRONLY | fs::O_APPEND)
            .map_err(|e| PaketYoneticisiHata::IslemYoneticisiHatasi(format!("İşlem günlüğü açılırken hata oluştu: {}", e)))?;
        let mesaj = "ISLEM BASLADI\n";
        fs::write(fd, mesaj.as_bytes())
            .map_err(|e| PaketYoneticisiHata::IslemYoneticisiHatasi(format!("İşlem günlüğüne 'ISLEM BASLADI' yazılırken hata oluştu: {}", e)))?;
        fs::close(fd).unwrap_or_default();
        Ok(())
    }

    // İşlem günlüğüne bir işlem adımı kaydeder.
    pub fn islem_adimi(&self, adim: &str) -> Result<(), PaketYoneticisiHata> {
        debug!("İşlem adımı kaydediliyor: '{}'. Günlük dosyası: {}", adim, self.islem_gunlugu);
        let günlük_dosyasi = &self.islem_gunlugu;
        let fd = fs::open(günlük_dosyasi, fs::O_CREAT | fs::O_WRONLY | fs::O_APPEND)
            .map_err(|e| PaketYoneticisiHata::IslemYoneticisiHatasi(format!("İşlem günlüğü açılırken hata oluştu: {}", e)))?;
        let mesaj = format!("{}\n", adim);
        fs::write(fd, mesaj.as_bytes())
            .map_err(|e| PaketYoneticisiHata::IslemYoneticisiHatasi(format!("İşlem günlüğüne adım yazılırken hata oluştu ('{}'): {}", adim, e)))?;
        fs::close(fd).unwrap_or_default();
        Ok(())
    }

    // İşlem günlüğüne "ISLEM TAMAMLANDI" kaydını yazar ve işlemi tamamlar.
    pub fn tamamla_islem(&self) -> Result<(), PaketYoneticisiHata> {
        info!("İşlem tamamlanıyor. Günlük dosyası: {}", self.islem_gunlugu);
        let günlük_dosyasi = &self.islem_gunlugu;
        let fd = fs::open(günlük_dosyasi, fs::O_CREAT | fs::O_WRONLY | fs::O_APPEND)
            .map_err(|e| PaketYoneticisiHata::IslemYoneticisiHatasi(format!("İşlem günlüğü açılırken hata oluştu: {}", e)))?;
        let mesaj = "ISLEM TAMAMLANDI\n";
        fs::write(fd, mesaj.as_bytes())
            .map_err(|e| PaketYoneticisiHata::IslemYoneticisiHatasi(format!("İşlem günlüğüne 'ISLEM TAMAMLANDI' yazılırken hata oluştu: {}", e)))?;
        fs::close(fd).unwrap_or_default();
        Ok(())
    }

    // İşlemi geri alır. İşlem günlüğünü okur ve adımları tersine çevirmeye çalışır.
    pub fn geri_al_islem(&self) -> Result<(), PaketYoneticisiHata> {
        info!("İşlem geri alma başlatılıyor. Günlük dosyası: {}", self.islem_gunlugu);
        let günlük_dosyasi = &self.islem_gunlugu;
        let fd = fs::open(günlük_dosyasi, fs::O_RDONLY)
            .map_err(|e| PaketYoneticisiHata::IslemYoneticisiHatasi(format!("İşlem günlüğü açılırken hata oluştu: {}", e)))?;
        let mut islem_gunlugu_icerigi = String::new();
        let mut read_buffer = [0u8; 4096];
        loop {
            match fs::read(fd, &mut read_buffer) {
                Ok(0) => break,
                Ok(bytes_read) => {
                    islem_gunlugu_icerigi.push_str(String::from_utf8_lossy(&read_buffer[..bytes_read]).as_ref());
                }
                Err(e) => {
                    fs::close(fd).unwrap_or_default();
                    return Err(e.into());
                }
            }
        }
        fs::close(fd).unwrap_or_default();

        let islem_adimlari: Vec<&str> = islem_gunlugu_icerigi.lines().collect();

        // İşlem zaten tamamlanmışsa geri almayı reddet
        if islem_adimlari.last() == Some(&"ISLEM TAMAMLANDI") {
            let hata_mesaji = "İşlem geri alınamaz, zaten tamamlandı.".to_string();
            warn!("{}", hata_mesaji); // Uyarı mesajını logla
            return Err(PaketYoneticisiHata::IslemYoneticisiHatasi(hata_mesaji));
        }

        info!("İşlem geri alınıyor... Günlük dosyası temizleniyor: {}", self.islem_gunlugu);
        let fd_temizle = fs::open(günlük_dosyasi, fs::O_WRONLY | fs::O_TRUNC)
            .map_err(|e| PaketYoneticisiHata::IslemYoneticisiHatasi(format!("İşlem günlüğü açılırken hata oluştu (temizleme): {}", e)))?;
        fs::close(fd_temizle).unwrap_or_default();
        info!("İşlem geri alma tamamlandı. Günlük dosyası temizlendi: {}", self.islem_gunlugu);
        Ok(())
    }
}
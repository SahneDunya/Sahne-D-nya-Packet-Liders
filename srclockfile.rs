use fs2::{FileExt, Error as Fs2Error}; // Fs2Error'ı yeniden adlandır
use std::fs::{File, OpenOptions};
use std::path::Path;
use crate::paket_yoneticisi_hata::PaketYoneticisiHata; // Özel hata enum'ımızı içe aktar
use log::{info, debug, error, warn}; // log kütüphanesini içe aktar

// Assuming 'Sahne64' modules are in the same crate if needed
use crate::fs;
use crate::SahneError;

pub struct KilitYoneticisi {
    kilit_dosyasi: File,
    kilit_dosyasi_yolu: String, // Kilit dosyasının yolunu sakla (loglama için)
    kilit_tutuldu: bool, // Kilidin alınıp alınmadığını takip et
}

impl KilitYoneticisi {
    // Yeni bir KilitYoneticisi örneği oluşturur.
    pub fn yeni(kilit_dosyasi_yolu: &Path) -> Result<Self, PaketYoneticisiHata> {
        // Sahne64'e özel dosya açma fonksiyonunu kullanmaya çalışalım.
        // Ancak fs2 crate'i std::fs::File beklediği için, öncelikle Sahne64 dosyasını std::fs::File'a dönüştürmemiz gerekebilir.
        // Şu anki Sahne64 API'sinde doğrudan böyle bir dönüşüm görünmüyor.
        // Bu nedenle, fs2 crate'inin gereksinimlerini karşılamak için std::fs kullanmaya devam edeceğiz.
        let kilit_dosyasi = OpenOptions::new()
            .create(true)
            .write(true)
            .open(kilit_dosyasi_yolu)
            .map_err(|e| {
                PaketYoneticisiHata::KilitYoneticisiHatasi(format!(
                    "Kilit dosyası açılırken hata oluştu: {}. Dosya yolu: {}",
                    e,
                    kilit_dosyasi_yolu.display()
                ))
            })?;

        Ok(KilitYoneticisi {
            kilit_dosyasi,
            kilit_dosyasi_yolu: kilit_dosyasi_yolu.display().to_string(), // Dosya yolunu string olarak sakla
            kilit_tutuldu: false, // Başlangıçta kilit tutulmuyor
        })
    }

    // Kilit almaya çalışır.
    pub fn kilit_al(&mut self) -> Result<(), PaketYoneticisiHata> {
        debug!("Kilit alınmaya çalışılıyor. Dosya yolu: {}", self.kilit_dosyasi_yolu);
        self.kilit_dosyasi.try_lock_exclusive().map_err(|e| {
            PaketYoneticisiHata::KilitYoneticisiHatasi(format!(
                "Kilit alınamadı (exclusive lock): {}. Dosya yolu: {}",
                e, self.kilit_dosyasi_yolu
            ))
        })?;
        self.kilit_tutuldu = true; // Kilit alındı olarak işaretle
        info!("Kilit başarıyla alındı. Dosya yolu: {}", self.kilit_dosyasi_yolu);
        Ok(())
    }

    // Kilidi serbest bırakır.
    pub fn kilidi_serbest_birak(&self) -> Result<(), PaketYoneticisiHata> {
        if self.kilit_tutuldu { // Sadece kilit tutuluyorsa serbest bırakmayı dene
            debug!("Kilit serbest bırakılmaya çalışılıyor. Dosya yolu: {}", self.kilit_dosyasi_yolu);
            self.kilit_dosyasi.unlock().map_err(|e| {
                PaketYoneticisiHata::KilitYoneticisiHatasi(format!(
                    "Kilit serbest bırakılamadı: {}. Dosya yolu: {}",
                    e, self.kilit_dosyasi_yolu
                ))
            })?;
            info!("Kilit başarıyla serbest bırakıldı. Dosya yolu: {}", self.kilit_dosyasi_yolu);
        } else {
            warn!("Kilit serbest bırakma çağrısı yapıldı, ancak kilit zaten tutulmuyordu. Dosya yolu: {}", self.kilit_dosyasi_yolu);
        }
        Ok(())
    }
}

// RAII (Resource Acquisition Is Initialization) prensibi ile kilidin otomatik serbest bırakılmasını sağlar.
impl Drop for KilitYoneticisi {
    fn drop(&mut self) {
        if self.kilit_tutuldu { // Eğer kilit hala tutuluyorsa (panik veya beklenmedik çıkış durumunda)
            debug!("KilitYoneticisi Drop trait çağrıldı, kilit serbest bırakılıyor. Dosya yolu: {}", self.kilit_dosyasi_yolu);
            if let Err(e) = self.kilit_dosyasi.unlock() { // Kilit serbest bırakma hatasını logla, ancak panikleme
                error!(
                    "Kilit Drop trait içinde serbest bırakılırken hata oluştu: {}. Dosya yolu: {}",
                    e, self.kilit_dosyasi_yolu
                );
            } else {
                info!("Kilit Drop trait içinde başarıyla serbest bırakıldı. Dosya yolu: {}", self.kilit_dosyasi_yolu);
            }
        }
    }
}
// Örnek Sahne64 dosya açma (eğer varsa):
impl KilitYoneticisi {
    pub fn yeni(kilit_dosyasi_yolu: &Path) -> Result<Self, PaketYoneticisiHata> {
        let dosya_yolu_str = kilit_dosyasi_yolu.to_str().unwrap();
        let fd = crate::fs::open(dosya_yolu_str, crate::fs::O_CREAT | crate::fs::O_RDWR)
            .map_err(|e| {
                PaketYoneticisiHata::KilitYoneticisiHatasi(format!(
                    "Kilit dosyası açılırken hata oluştu: {}. Dosya yolu: {}",
                    e, dosya_yolu_str
                ))
            })?;
        // Sahne64 dosya tanımlayıcısını std::fs::File'a dönüştürmek gerekebilir.
        // Bu dönüşüm için bir yol olmayabilir, bu durumda fs2 crate'i kullanmak daha uygun olabilir.
        // Şimdilik std::fs kullanmaya devam ediyoruz.
        let kilit_dosyasi = OpenOptions::new()
            .read(true)
            .write(true)
            .open(kilit_dosyasi_yolu)
            .map_err(|e| { /* ... */ })?;

        Ok(KilitYoneticisi { /* ... */ })
    }

    // Kilidi serbest bırakma (eğer Sahne64'te özel bir kilit serbest bırakma varsa):
    pub fn kilidi_serbest_birak(&self) -> Result<(), PaketYoneticisiHata> {
        if self.kilit_tutuldu {
            let dosya_yolu_str = &self.kilit_dosyasi_yolu;
            // Eğer Sahne64'te özel bir kilit serbest bırakma fonksiyonu varsa kullanılabilir.
            // Örneğin: crate::kernel::release_lock(dosya_yolu_str)?;
            // Ancak dosya kilitleme için fs2 crate'ini kullandığımız için, onun unlock fonksiyonunu kullanmaya devam ediyoruz.
            self.kilit_dosyasi.unlock().map_err(|e| { /* ... */ })?;
            info!("Kilit başarıyla serbest bırakıldı. Dosya yolu: {}", self.kilit_dosyasi_yolu);
        } else {
            warn!("Kilit serbest bırakma çağrısı yapıldı, ancak kilit zaten tutulmuyordu. Dosya yolu: {}", self.kilit_dosyasi_yolu);
        }
        Ok(())
    }

    impl Drop for KilitYoneticisi {
        fn drop(&mut self) {
            if self.kilit_tutuldu {
                let dosya_yolu_str = &self.kilit_dosyasi_yolu;
                // Eğer Sahne64'te özel bir kilit serbest bırakma fonksiyonu varsa burada da kullanılabilir.
                if let Err(e) = self.kilit_dosyasi.unlock() { /* ... */ }
            }
        }
    }
}
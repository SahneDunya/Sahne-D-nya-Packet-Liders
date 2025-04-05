use std::path::{Path, PathBuf};
use zip::{ZipArchive, result::ZipError};
use crate::paket_yoneticisi_hata::PaketYoneticisiHata; // Özel hata enum'ımızı içe aktar

// Assuming 'Sahne64' modules are in the same crate
use crate::fs;
use crate::SahneError;

// Verilen ZIP arşivini belirtilen dizine açar.
pub fn zip_ac(arsiv_yolu: &str, cikartma_dizini: &str) -> Result<(), PaketYoneticisiHata> {
    let dosya = fs::open(arsiv_yolu, fs::O_RDONLY).map_err(|e| {
        PaketYoneticisiHata::ZipHatasi(ZipError::IoError(e.into())) // Dosya açma hatasını ZipError::IoError olarak PaketYoneticisiHata::ZipHatasi'na dönüştür
    })?;
    let mut arsiv = ZipArchive::new(dosya).map_err(PaketYoneticisiHata::ZipHatasi)?; // Zip arşivini aç, hata durumunda PaketYoneticisiHata::ZipHatasi'na dönüştür

    for i in 0..arsiv.len() {
        let mut arsiv_dosyasi = arsiv.by_index(i)?;
        let cikartma_yolu = PathBuf::from(cikartma_dizini).join(arsiv_dosyasi.name()); // PathBuf ile çıkarma yolunu oluştur
        let cikartma_yolu_str = cikartma_yolu.to_str().ok_or_else(|| PaketYoneticisiHata::ZipHatasi(ZipError::InvalidPath("Geçersiz UTF-8 yolu".into())))?;
        let cikartma_dizini_path = PathBuf::from(cikartma_dizini);
        let cikartma_dizini_str = cikartma_dizini_path.to_str().ok_or_else(|| PaketYoneticisiHata::ZipHatasi(ZipError::InvalidPath("Geçersiz UTF-8 çıkarma dizini yolu".into())))?;

        // Güvenlik kontrolü (temel): Çıkarma yolunun hedef dizin altında olduğundan emin ol
        // Daha gelişmiş path traversal saldırılarına karşı koruma için ek kontroller gerekebilir.
        if !cikartma_yolu.starts_with(cikartma_dizini_path) {
            return Err(PaketYoneticisiHata::ZipHatasi(ZipError::InvalidPath("Güvenlik sebebiyle geçersiz çıkarma yolu".into())));
        }

        if arsiv_dosyasi.name().ends_with('/') {
            // Klasör ise oluştur
            fs::create_dir_recursive(cikartma_yolu_str, 0o755).map_err(|e| {
                PaketYoneticisiHata::DosyaSistemiHatasi(e) // Dizin oluşturma hatasını PaketYoneticisiHata::DosyaSistemiHatasi'na dönüştür
            })?;
        } else {
            // Dosya ise çıkar
            if let Some(ebeveyn) = cikartma_yolu.parent() {
                let ebeveyn_str = ebeveyn.to_str().ok_or_else(|| PaketYoneticisiHata::ZipHatasi(ZipError::InvalidPath("Geçersiz UTF-8 ebeveyn yolu".into())))?;
                fs::create_dir_recursive(ebeveyn_str, 0o755).map_err(|e| {
                    PaketYoneticisiHata::DosyaSistemiHatasi(e) // Ebeveyn dizinleri oluşturma hatasını PaketYoneticisiHata::DosyaSistemiHatasi'na dönüştür
                })?;
            }
            let cikartma_dosyasi_fd = fs::open(cikartma_yolu_str, fs::O_WRONLY | fs::O_CREAT | fs::O_TRUNC).map_err(|e| {
                PaketYoneticisiHata::DosyaSistemiHatasi(e) // Dosya oluşturma hatasını PaketYoneticisiHata::DosyaSistemiHatasi'na dönüştür
            })?;
            let mut buffer = Vec::new();
            arsiv_dosyasi.read_to_end(&mut buffer)?;
            fs::write(cikartma_dosyasi_fd, &buffer).map_err(|e| {
                PaketYoneticisiHata::IoHatasi(e.into()) // Dosya kopyalama hatasını PaketYoneticisiHata::IoHatasi'na dönüştür
            })?;
            fs::close(cikartma_dosyasi_fd).unwrap_or_default();
        }
    }
    Ok(())
}

// Verilen ZIP arşivinin içeriğini listeleyen fonksiyon.
pub fn zip_icerik_listele(arsiv_yolu: &str) -> Result<Vec<String>, PaketYoneticisiHata> {
    let dosya = fs::open(arsiv_yolu, fs::O_RDONLY).map_err(|e| {
        PaketYoneticisiHata::ZipHatasi(ZipError::IoError(e.into())) // Dosya açma hatasını ZipError::IoError olarak PaketYoneticisiHata::ZipHatasi'na dönüştür
    })?;
    let mut arsiv = ZipArchive::new(dosya).map_err(PaketYoneticisiHata::ZipHatasi)?; // Zip arşivini aç, hata durumunda PaketYoneticisiHata::ZipHatasi'na dönüştür
    let mut icerikler = Vec::new();

    for i in 0..arsiv.len() {
        let arsiv_dosyasi = arsiv.by_index(i)?;
        icerikler.push(arsiv_dosyasi.name().to_string()); // Arşiv dosyasının adını listeye ekle
    }
    Ok(icerikler) // Arşiv içeriği listesini döndür
}
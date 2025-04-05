use md5::{Md5, Digest};
use std::io;
use crate::paket_yoneticisi_hata::PaketYoneticisiHata; // Özel hata enum'ımızı içe aktar

// Assuming 'Sahne64' modules are in the same crate
use crate::fs;
use crate::SahneError;

// Verilen dosyanın MD5 özetini hesaplar.
pub fn hesapla_md5(dosya_yolu: &str) -> Result<String, PaketYoneticisiHata> {
    let fd = fs::open(dosya_yolu, fs::O_RDONLY).map_err(|e| {
        PaketYoneticisiHata::ChecksumHatasi(format!(
            "Dosya açma hatası: {}: {}",
            dosya_yolu, e
        ))
    })?; // Dosyayı aç, hata durumunda PaketYoneticisiHata::ChecksumHatasi'na dönüştür

    let mut md5 = Md5::new(); // Yeni bir MD5 hesaplayıcısı oluştur
    let mut buffer = [0u8; 4096]; // Okuma için bir buffer oluştur
    loop {
        match fs::read(fd, &mut buffer) {
            Ok(0) => break, // Dosyanın sonuna gelindi
            Ok(bytes_read) => {
                md5.update(&buffer[..bytes_read]); // Okunan veriyi MD5 hesaplayıcısına ekle
            }
            Err(e) => {
                fs::close(fd).unwrap_or_default();
                return Err(PaketYoneticisiHata::ChecksumHatasi(format!(
                    "Dosya okuma hatası: {}: {}",
                    dosya_yolu, e
                )));
            }
        }
    }
    fs::close(fd).unwrap_or_default();

    let sonuc = md5.finalize(); // MD5 özetini hesapla
    Ok(format!("{:x}", sonuc)) // Hesaplanan MD5 özetini hex string olarak döndür
}

// Verilen dosyanın MD5 özetini hesaplar ve beklenen MD5 özeti ile karşılaştırır.
pub fn dogrula_md5(dosya_yolu: &str, beklenen_md5: &str) -> Result<bool, PaketYoneticisiHata> {
    let hesaplanan_md5 = hesapla_md5(dosya_yolu)?; // Dosyanın MD5 özetini hesapla

    if hesaplanan_md5 == beklenen_md5 {
        Ok(true) // Hesaplanan özet beklenenle eşleşiyorsa, true döndür
    } else {
        Ok(false) // Eşleşmiyorsa, false döndür
    }
}
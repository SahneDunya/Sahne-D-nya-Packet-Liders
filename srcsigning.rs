#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // String, Vec, format! için

// no_std ve alloc uyumlu kripto ve hex crate'leri
use sha2::{Sha256, Digest};
use hex;
// hex::EncodeError için From implementasyonu gerekebilir.

// Sahne64 API modülleri
use crate::resource; // Kaynak işlemleri (okuma)
use crate::SahneError; // Sahne64 hata türü
use crate::Handle; // Kaynak Handle'ları

// Özel hata enum'ımızı içe aktar (no_std uyumlu hali)
use crate::paket_yoneticisi_hatasi::PaketYoneticisiHatasi;
// SahneError, hex::EncodeError, SecurityError'dan dönüşüm From implementasyonları ile sağlanacak.
// SecurityError'dan SahneError ve hex::FromHexError handle ediliyordu.
use crate::srcsecurity::SecurityError; // İmza/Hash/Hex hatalarını SecurityError ile handle edelim

// log kütüphanesini içe aktar (no_std uyumlu backend varsayımıyla)
use log::{info, error, debug};

// String ve Vec from alloc
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format; // format! makrosu için

// no_std uyumlu print makroları (örnek çıktılar için)
use crate::print_macros::{println, eprintln};


// Helper fonksiyon: Sahne64 Kaynağından tüm içeriği Vec<u8> olarak oku.
// srcsecurity.rs veya utils modülünden yeniden kullanıldı.
// Note: Bu helper, utils gibi ortak bir modülde olmalıdır.
fn read_resource_to_vec(resource_id: &str) -> Result<Vec<u8>, SecurityError> { // Result türü SecurityError
    let handle = resource::acquire(resource_id, resource::MODE_READ)
        .map_err(|e| SecurityError::from(e))?; // SahneError -> SecurityError

    let mut buffer = Vec::new(); // alloc
    let mut temp_buffer = [0u8; 512]; // Stack buffer

    loop {
        match resource::read(handle, &mut temp_buffer) {
            Ok(0) => break,
            Ok(bytes_read) => {
                buffer.extend_from_slice(&temp_buffer[..bytes_read]); // alloc
            }
            Err(e) => {
                let _ = resource::release(handle);
                return Err(SecurityError::from(e)); // SahneError -> SecurityError
            }
        }
    }

    let release_result = resource::release(handle);
     if let Err(_e) = release_result {
          // Log error, but continue
     }

    Ok(buffer) // Vec<u8> (alloc)
}


// Basit bir örnek için, imzaları dosya içeriklerinin SHA256 karmalarının hex kodlanmış hali olarak hesaplar.
// package_resource_id: İmzalanacak paketin Kaynak ID'si.
// Dönüş değeri: Hesaplanmış imza (hex string) veya SecurityError.
// Not: Gerçek bir uygulamada, daha güvenli bir dijital imzalama yöntemi (asimetrik kriptografi) kullanılmalıdır.
pub fn sign_package(package_resource_id: &str) -> Result<String, SecurityError> { // Path yerine &str Kaynak ID, Result<String, SecurityError> olmalı
    debug!("Paket imzalanıyor: {}", package_resource_id); // no_std log

    // Paket Kaynağının içeriğini oku (Vec<u8> olarak)
    let package_data = read_resource_to_vec(package_resource_id)?; // Kendi helper'ımızı kullan (SecurityError döner)

    // Dosya içeriğinin SHA256 karmasını hesapla.
    let mut hasher = Sha256::new(); // Sha256::new() alloc gerektirir mi? Genellikle değil.
    hasher.update(&package_data); // update &[] alır no_std
    let digest = hasher.finalize(); // finalize GenericArray<u8, 32> no_std

    trace!("Paket özeti (SHA256) hesaplandı."); // no_std log

    // Karmayı hex string'e dönüştür.
    // hex::encode(&digest) -> String. alloc gerektirir, no_std.
    let signature = hex::encode(&digest); // hex::encode Result dönmüyor, hata fırlatabilir mi? Hayır, hep başarılı olmalı.

    debug!("Paket imzası (hex SHA256) hesaplandı: {}", signature); // no_std log
    Ok(signature) // İmza hex stringini döndür (alloc)
}

// Belirtilen paketin imzasını hesaplar ve beklenen imza (hex string) ile karşılaştırır.
// package_resource_id: Doğrulanacak paketin Kaynak ID'si.
// expected_signature: Beklenen imza (hex string).
// Dönüş değeri: İmza eşleşirse Ok(true), eşleşmezse Ok(false), veya hatalar durumunda SecurityError.
// Not: İmza eşleşmezse SecurityError::SignatureVerificationFailed hatası döndürmek yerine Ok(false) dönmek
// buradaki fonksiyonun sorumluluğuna daha uygun olabilir. İmza doğrulama başarısızlığını bir hata olarak
// ele almak çağıran kodun (srcsecurity.rs) sorumluluğu olabilir.
pub fn verify_package(package_resource_id: &str, expected_signature: &str) -> Result<bool, SecurityError> { // Path yerine &str Kaynak ID, Result<bool, SecurityError> olmalı
    debug!("Paket imzası doğrulanıyor. Paket: {}, Beklenen imza (ilk 8 char): {}", package_resource_id, &expected_signature.chars().take(8).collect::<String>()); // no_std log, take(8).collect alloc

    // Paketin imzasını hesapla.
    let calculated_signature = sign_package(package_resource_id)?; // sign_package çağrısı (SecurityError döner)

    // Hesaplanan imza beklenen imza ile eşleşiyor mu kontrol et.
    let is_valid = calculated_signature == expected_signature; // String == &str karşılaştırması no_std

    if is_valid {
        info!("İmza doğrulandı. Paket: {}", package_resource_id); // no_std log
    } else {
        warn!("İmza doğrulama başarısız! Hesaplanan: {}, Beklenen: {}. Paket: {}", calculated_signature, expected_signature, package_resource_id); // no_std log
    }

    Ok(is_valid) // Eşleşirse true, eşleşmezse false dön
}

// #[cfg(test)] bloğu std test runner'ı ve std bağımlılıkları gerektirir.
// Testler için mock resource veya Sahne64 simülasyonu gereklidir.

#[cfg(test)]
mod tests {
    // std::path, std::io, std::fs, tempfile, sha2, hex kullandığı için no_std'de doğrudan çalışmaz.
    // Mock resource::acquire/read/release ve test dosyası oluşturma/okuma helper'ları gerektirir.
}

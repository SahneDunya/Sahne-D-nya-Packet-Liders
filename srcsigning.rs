use std::path::Path;

// Assuming 'Sahne64' modules are in the same crate
use crate::fs;
use crate::SahneError; // Assuming SahneError is accessible here
use std::io;

// Basit bir örnek için, imzaları dosya içeriklerinin SHA256 karmaları olarak saklayalım.
// Gerçek bir uygulamada, daha güvenli bir imzalama yöntemi kullanılmalıdır.
pub fn sign_package(package_path: &Path) -> Result<String, SahneError> {
    let package_path_str = package_path.to_str().unwrap();
    let fd = fs::open(package_path_str, fs::O_RDONLY)?;
    let mut buffer = Vec::new();
    let mut read_buffer = [0u8; 4096];
    loop {
        match fs::read(fd, &mut read_buffer) {
            Ok(0) => break,
            Ok(bytes_read) => buffer.extend_from_slice(&read_buffer[..bytes_read]),
            Err(e) => {
                fs::close(fd).unwrap_or_default();
                return Err(e.into()); // Convert SahneError to io::Error if needed
            }
        }
    }
    fs::close(fd).unwrap_or_default();

    // Dosya içeriğinin SHA256 karmasını hesapla.
    let signature = sha256::digest(buffer.as_slice());
    Ok(signature)
}

pub fn verify_package(package_path: &Path, expected_signature: &str) -> Result<bool, SahneError> {
    // Paketin imzasını hesapla.
    let signature = sign_package(package_path)?;
    // Hesaplanan imza beklenen imza ile eşleşiyor mu kontrol et.
    Ok(signature == expected_signature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::{self as sahne_fs, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY};
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_sign_and_verify() {
        // Geçici bir dizin oluştur.
        let temp_dir = tempdir().unwrap();
        // Paket dosyasının yolunu oluştur.
        let package_path = temp_dir.path().join("test_package.txt");
        let package_path_str = package_path.to_str().unwrap();
        // Geçici paket dosyasını oluştur.
        let fd = sahne_fs::open(package_path_str, O_CREAT | O_WRONLY | O_TRUNC).unwrap();
        // Pakete içerik yaz.
        sahne_fs::write(fd, b"Test package content").unwrap();
        sahne_fs::close(fd).unwrap();

        // Paketi imzala.
        let signature = sign_package(&package_path).unwrap();
        // İmzayı doğrula, geçerli olmalı.
        let is_valid = verify_package(&package_path, &signature).unwrap();
        assert!(is_valid);

        // Geçersiz bir imza ile doğrulamayı dene, geçersiz olmalı.
        let is_invalid = verify_package(&package_path, "invalid_signature").unwrap();
        assert!(!is_invalid);

        // Geçici paket dosyasını sil.
        sahne_fs::remove_file(package_path_str).unwrap();
        // Geçici dizini kapat.
        temp_dir.close().unwrap();
    }
}
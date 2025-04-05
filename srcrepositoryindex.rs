use std::collections::HashMap;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

// Assuming 'Sahne64' modules are in the same crate
use crate::fs;
use crate::SahneError; // Assuming SahneError is accessible here

use serde::{Deserialize, Serialize};
use thiserror::Error;

// Tanımlanan özel hata türü
#[derive(Debug, Error)]
enum IndexError {
    #[error("IO hatası: {0}")]
    IoError(#[from] io::Error),
    #[error("JSON hatası: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Sahne64 File System Error: {0}")]
    Sahne64FileSystemError(#[from] crate::SahneError),
}

type IndexResult<T> = Result<T, IndexError>;

// Paket deposu indeksini temsil eden yapı
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct PackageIndex {
    packages: HashMap<String, Vec<String>>, // Paket adı -> Sürümler
}

impl PackageIndex {
    // Yeni bir boş PaketIndeksi oluşturur.
    fn new() -> Self {
        PackageIndex {
            packages: HashMap::new(),
        }
    }

    // Paketi indekse ekler.
    // Eğer paket zaten varsa, verilen sürüm listesine eklenir.
    fn add_package(&mut self, package_name: &str, version: &str) {
        self.packages
            .entry(package_name.to_string())
            .or_default()
            .push(version.to_string());
    }

    // Paketin indekste olup olmadığını kontrol eder.
    fn has_package(&self, package_name: &str) -> bool {
        self.packages.contains_key(package_name)
    }

    // Paketin sürümlerini döndürür.
    // Eğer paket bulunamazsa `None` döndürür.
    fn get_versions(&self, package_name: &str) -> Option<&Vec<String>> {
        self.packages.get(package_name)
    }

    // İndeksi dosyaya kaydeder.
    // JSON formatında dosyaya yazma işlemini gerçekleştirir.
    fn save_to_file(&self, file_path: &Path) -> IndexResult<()> {
        let path_str = file_path.to_str().ok_or(io::Error::new(io::ErrorKind::Other, "Geçersiz dosya yolu"))?;
        let file = fs::open(path_str, fs::O_CREAT | fs::O_WRONLY)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self)?;
        Ok(())
    }

    // İndeksi dosyadan yükler.
    // JSON formatındaki dosyadan okuma ve deserializasyon işlemini yapar.
    fn load_from_file(file_path: &Path) -> IndexResult<Self> {
        let path_str = file_path.to_str().ok_or(io::Error::new(io::ErrorKind::Other, "Geçersiz dosya yolu"))?;
        let file = fs::open(path_str, fs::O_RDONLY)?;
        let reader = BufReader::new(file);
        let index = serde_json::from_reader(reader)?;
        Ok(index)
    }
}

// İndeks dosyasının yolunu oluşturur.
// Repo yolunu temel alarak `index.json` dosyasının yolunu birleştirir.
fn get_index_path(repo_path: &Path) -> PathBuf {
    repo_path.join("index.json")
}

// İndeksi oluşturur veya yükler.
// Eğer indeks dosyası varsa yükler, yoksa yeni bir indeks oluşturur.
fn get_or_create_index(repo_path: &Path) -> IndexResult<PackageIndex> {
    let index_path = get_index_path(repo_path);
    if fs::exists(index_path.to_str().unwrap()) {
        PackageIndex::load_from_file(&index_path)
    } else {
        Ok(PackageIndex::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs as std_fs; // std::fs'yi farklı bir isimle import et
    use tempfile::tempdir;

    #[test]
    fn test_package_index() {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path().join("repo");
        std_fs::create_dir(&repo_path).unwrap();

        let mut index = get_or_create_index(&repo_path).unwrap();
        index.add_package("test_package", "1.0.0");
        index.add_package("test_package", "2.0.0");
        index.save_to_file(&get_index_path(&repo_path)).unwrap();

        let loaded_index = get_or_create_index(&repo_path).unwrap();
        assert_eq!(index, loaded_index);

        assert!(loaded_index.has_package("test_package"));
        assert_eq!(
            loaded_index.get_versions("test_package").unwrap(),
            &vec!["1.0.0".to_string(), "2.0.0".to_string()]
        );

        temp_dir.close().unwrap();
    }

    #[test]
    fn test_load_non_existent_index() {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path().join("repo");
        std_fs::create_dir(&repo_path).unwrap();

        let index = get_or_create_index(&repo_path).unwrap(); // Olmayan indeks yüklenmeye çalışıldığında yeni indeks oluşturmalı
        assert_eq!(index.packages.len(), 0); // Yeni indeks boş olmalı

        temp_dir.close().unwrap();
    }

    #[test]
    fn test_save_load_multiple_packages() {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path().join("repo");
        std_fs::create_dir(&repo_path).unwrap();

        let mut index = get_or_create_index(&repo_path).unwrap();
        index.add_package("package_a", "1.0.0");
        index.add_package("package_a", "1.1.0");
        index.add_package("package_b", "2.0.0");
        index.save_to_file(&get_index_path(&repo_path)).unwrap();

        let loaded_index = get_or_create_index(&repo_path).unwrap();
        assert_eq!(index, loaded_index);
        assert!(loaded_index.has_package("package_a"));
        assert!(loaded_index.has_package("package_b"));
        assert_eq!(
            loaded_index.get_versions("package_a").unwrap(),
            &vec!["1.0.0".to_string(), "1.1.0".to_string()]
        );
        assert_eq!(
            loaded_index.get_versions("package_b").unwrap(),
            &vec!["2.0.0".to_string()]
        );

        temp_dir.close().unwrap();
    }

    #[test]
    fn test_add_duplicate_package_version() {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path().join("repo");
        std_fs::create_dir(&repo_path).unwrap();

        let mut index = get_or_create_index(&repo_path).unwrap();
        index.add_package("dup_package", "1.0.0");
        index.add_package("dup_package", "1.0.0"); // Aynı sürümü tekrar ekle
        index.save_to_file(&get_index_path(&repo_path)).unwrap();

        let loaded_index = get_or_create_index(&repo_path).unwrap();
        assert_eq!(
            loaded_index.get_versions("dup_package").unwrap(),
            &vec!["1.0.0".to_string(), "1.0.0".to_string()] // Duplikat sürümlerin listeye eklendiğini kontrol et
        );

        temp_dir.close().unwrap();
    }
}
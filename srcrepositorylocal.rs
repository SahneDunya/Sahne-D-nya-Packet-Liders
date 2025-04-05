use std::path::{Path, PathBuf};

// Assuming 'Sahne64' modules are in the same crate
use crate::fs;
use crate::SahneError; // Assuming SahneError is accessible here
use std::io;

// Yerel depo yolu
pub struct LocalRepository {
    path: PathBuf,
}

impl LocalRepository {
    pub fn new(path: PathBuf) -> Self {
        LocalRepository { path }
    }

    // Yerel depoda bir paketin olup olmadığını kontrol eder
    pub fn has_package(&self, package_name: &str, version: &str) -> bool {
        let package_path = self.path.join(package_name).join(version);
        let package_path_str = package_path.to_str().unwrap();
        fs::metadata(package_path_str).is_ok()
    }

    // Yerel depodan bir paketi alır
    pub fn get_package(&self, package_name: &str, version: &str) -> Option<PathBuf> {
        let package_path = self.path.join(package_name).join(version);
        let package_path_str = package_path.to_str().unwrap();
        if fs::metadata(package_path_str).is_ok() {
            Some(package_path)
        } else {
            None
        }
    }

    // Yerel depoya bir paket ekler
    pub fn add_package(&self, package_path: &Path, version: &str) -> Result<(), SahneError> {
        let package_name = package_path.file_stem().unwrap().to_str().unwrap();
        let destination_path = self.path.join(package_name).join(version);
        let destination_path_str = destination_path.to_str().unwrap();
        let source_path_str = package_path.to_str().unwrap();
        let destination_file_path = destination_path.join(package_path.file_name().unwrap());
        let destination_file_path_str = destination_file_path.to_str().unwrap();

        fs::create_dir_all(destination_path_str, 0o755)?; // Klasörleri oluştur (izinlerle birlikte)

        let mut source_file = fs::open(source_path_str, fs::O_RDONLY)?;
        let mut destination_file = fs::open(destination_file_path_str, fs::O_CREAT | fs::O_WRONLY | fs::O_TRUNC)?;

        let mut buffer = [0u8; 4096];
        loop {
            match fs::read(source_file, &mut buffer) {
                Ok(0) => break,
                Ok(bytes_read) => {
                    fs::write(destination_file, &buffer[..bytes_read])?;
                }
                Err(e) => return Err(e.into()), // Convert SahneError to LocalRepositoryError if needed
            }
        }

        fs::close(source_file).unwrap_or_default();
        fs::close(destination_file).unwrap_or_default();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::{self as sahne_fs, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY};
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::tempdir;

    // Yardımcı fonksiyon: std::fs yerine sahne_fs kullanılarak dosya oluşturur
    fn create_test_file(path: &PathBuf, content: &[u8]) -> Result<(), SahneError> {
        let path_str = path.to_str().unwrap();
        let fd = sahne_fs::open(path_str, O_CREAT | O_WRONLY | O_TRUNC)?;
        sahne_fs::write(fd, content)?;
        sahne_fs::close(fd).unwrap_or_default();
        Ok(())
    }

    // Yardımcı fonksiyon: sahne_fs kullanılarak dosya içeriğini okur
    fn read_test_file(path: &PathBuf) -> Result<Vec<u8>, SahneError> {
        let path_str = path.to_str().unwrap();
        let fd = sahne_fs::open(path_str, O_RDONLY)?;
        let mut buffer = Vec::new();
        let mut read_buffer = [0u8; 1024];
        loop {
            match sahne_fs::read(fd, &mut read_buffer) {
                Ok(0) => break,
                Ok(bytes_read) => buffer.extend_from_slice(&read_buffer[..bytes_read]),
                Err(e) => {
                    sahne_fs::close(fd).unwrap_or_default();
                    return Err(e);
                }
            }
        }
        sahne_fs::close(fd).unwrap_or_default();
        Ok(buffer)
    }

    #[test]
    fn test_local_repository() {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path().join("repo");
        let repo_path_str = repo_path.to_str().unwrap();
        sahne_fs::create_dir(repo_path_str, 0o755).unwrap();

        let repo = LocalRepository::new(repo_path.clone());

        let package_path = temp_dir.path().join("test_package.txt");
        let package_content = b"Test package content";
        create_test_file(&package_path, package_content).unwrap();

        let package_version = "2.0.0"; // Define a version for the package
        repo.add_package(&package_path, package_version).unwrap();

        assert!(repo.has_package("test_package", package_version));
        assert!(repo.get_package("test_package", package_version).is_some());

        let retrieved_package_path = repo.get_package("test_package", package_version).unwrap();
        let content = read_test_file(&retrieved_package_path).unwrap();
        assert_eq!(content, package_content);

        let package_dir = repo_path.join("test_package").join(package_version);
        let package_file = package_dir.join("test_package.txt");
        sahne_fs::remove_file(package_file.to_str().unwrap()).unwrap();
        sahne_fs::remove_dir(package_dir.to_str().unwrap()).unwrap();

        sahne_fs::remove_file(package_path.to_str().unwrap()).unwrap();
        sahne_fs::remove_dir(repo_path_str).unwrap();
        temp_dir.close().unwrap();
    }
}
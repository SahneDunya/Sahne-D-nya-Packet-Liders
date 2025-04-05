use std::collections::HashSet;

// Assuming 'Sahne64' modules are in the same crate
use crate::fs;
use crate::SahneError; // Assuming SahneError is accessible here
use std::io::Write; // For writing to the file

pub struct TrustManager {
    trusted_publishers: HashSet<String>,
    trusted_packages: HashSet<String>,
    publishers_file_path: &'static str,
    packages_file_path: &'static str,
}

impl TrustManager {
    pub fn new() -> Self {
        let publishers_file_path = "/etc/trusted_publishers.list";
        let packages_file_path = "/etc/trusted_packages.list";
        let mut manager = TrustManager {
            trusted_publishers: HashSet::new(),
            trusted_packages: HashSet::new(),
            publishers_file_path,
            packages_file_path,
        };
        if let Err(e) = manager.load_trusted_data() {
            eprintln!("Güvenilen veri yüklenirken hata oluştu: {:?}", e);
            // Hata durumunda boş bir yönetici ile devam et
        }
        manager
    }

    fn load_trusted_data(&mut self) -> Result<(), SahneError> {
        self.load_trusted_publishers()?;
        self.load_trusted_packages()?;
        Ok(())
    }

    fn load_trusted_publishers(&mut self) -> Result<(), SahneError> {
        match fs::open(self.publishers_file_path, fs::O_RDONLY) {
            Ok(fd) => {
                let mut buffer = Vec::new();
                let mut read_buffer = [0u8; 128];
                loop {
                    match fs::read(fd, &mut read_buffer) {
                        Ok(bytes_read) => {
                            if bytes_read == 0 {
                                break;
                            }
                            buffer.extend_from_slice(&read_buffer[..bytes_read]);
                        }
                        Err(e) => {
                            fs::close(fd).unwrap_or_default();
                            return Err(e);
                        }
                    }
                }
                fs::close(fd).unwrap_or_default();

                if let Ok(content) = String::from_utf8(buffer) {
                    for line in content.lines() {
                        let publisher = line.trim();
                        if !publisher.is_empty() {
                            self.trusted_publishers.insert(publisher.to_string());
                        }
                    }
                }
            }
            Err(e) => {
                // Dosya yoksa sorun değil, boş liste ile başla
                if e != SahneError::FileNotFound {
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    fn load_trusted_packages(&mut self) -> Result<(), SahneError> {
        match fs::open(self.packages_file_path, fs::O_RDONLY) {
            Ok(fd) => {
                let mut buffer = Vec::new();
                let mut read_buffer = [0u8; 128];
                loop {
                    match fs::read(fd, &mut read_buffer) {
                        Ok(bytes_read) => {
                            if bytes_read == 0 {
                                break;
                            }
                            buffer.extend_from_slice(&read_buffer[..bytes_read]);
                        }
                        Err(e) => {
                            fs::close(fd).unwrap_or_default();
                            return Err(e);
                        }
                    }
                }
                fs::close(fd).unwrap_or_default();

                if let Ok(content) = String::from_utf8(buffer) {
                    for line in content.lines() {
                        let package = line.trim();
                        if !package.is_empty() {
                            self.trusted_packages.insert(package.to_string());
                        }
                    }
                }
            }
            Err(e) => {
                // Dosya yoksa sorun değil, boş liste ile başla
                if e != SahneError::FileNotFound {
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    pub fn add_trusted_publisher(&mut self, publisher_name: &str) {
        if self.trusted_publishers.insert(publisher_name.to_string()) {
            if let Err(e) = self.persist_trusted_publisher(publisher_name) {
                eprintln!("Güvenilen yayıncı kaydedilirken hata oluştu: {:?}", e);
            }
        }
    }

    fn persist_trusted_publisher(&self, publisher_name: &str) -> Result<(), SahneError> {
        let mut file = match fs::open(self.publishers_file_path, fs::O_CREAT | fs::O_WRONLY | fs::O_APPEND) {
            Ok(fd) => fd,
            Err(e) => return Err(e),
        };

        let line = format!("{}\n", publisher_name);
        let buffer = line.as_bytes();
        let mut written = 0;
        while written < buffer.len() {
            match fs::write(file, &buffer[written..]) {
                Ok(bytes) => written += bytes,
                Err(e) => {
                    fs::close(file).unwrap_or_default();
                    return Err(e);
                }
            }
        }
        fs::close(file).unwrap_or_default();
        Ok(())
    }

    pub fn remove_trusted_publisher(&mut self, publisher_name: &str) -> bool {
        let removed = self.trusted_publishers.remove(publisher_name);
        if removed {
            if let Err(e) = self.persist_trusted_publishers() {
                eprintln!("Güvenilen yayıncılar güncellenirken hata oluştu: {:?}", e);
                // Geri alma mekanizması burada düşünülebilir.
            }
        }
        removed
    }

    // Tüm güvenilen yayıncıları dosyaya yeniden yazar (basit ama potansiyel olarak verimsiz)
    fn persist_trusted_publishers(&self) -> Result<(), SahneError> {
        let mut file = match fs::open(self.publishers_file_path, fs::O_CREAT | fs::O_WRONLY | fs::O_TRUNC) {
            Ok(fd) => fd,
            Err(e) => return Err(e),
        };

        for publisher in &self.trusted_publishers {
            let line = format!("{}\n", publisher);
            let buffer = line.as_bytes();
            let mut written = 0;
            while written < buffer.len() {
                match fs::write(file, &buffer[written..]) {
                    Ok(bytes) => written += bytes,
                    Err(e) => {
                        fs::close(file).unwrap_or_default();
                        return Err(e);
                    }
                }
            }
        }
        fs::close(file).unwrap_or_default();
        Ok(())
    }

    pub fn is_trusted_publisher(&self, publisher_name: &str) -> bool {
        self.trusted_publishers.contains(publisher_name)
    }

    pub fn add_trusted_package(&mut self, package_name: &str) {
        if self.trusted_packages.insert(package_name.to_string()) {
            if let Err(e) = self.persist_trusted_package(package_name) {
                eprintln!("Güvenilen paket kaydedilirken hata oluştu: {:?}", e);
            }
        }
    }

    fn persist_trusted_package(&self, package_name: &str) -> Result<(), SahneError> {
        let mut file = match fs::open(self.packages_file_path, fs::O_CREAT | fs::O_WRONLY | fs::O_APPEND) {
            Ok(fd) => fd,
            Err(e) => return Err(e),
        };

        let line = format!("{}\n", package_name);
        let buffer = line.as_bytes();
        let mut written = 0;
        while written < buffer.len() {
            match fs::write(file, &buffer[written..]) {
                Ok(bytes) => written += bytes,
                Err(e) => {
                    fs::close(file).unwrap_or_default();
                    return Err(e);
                }
            }
        }
        fs::close(file).unwrap_or_default();
        Ok(())
    }

    pub fn remove_trusted_package(&mut self, package_name: &str) -> bool {
        let removed = self.trusted_packages.remove(package_name);
        if removed {
            if let Err(e) = self.persist_trusted_packages() {
                eprintln!("Güvenilen paketler güncellenirken hata oluştu: {:?}", e);
                // Geri alma mekanizması burada düşünülebilir.
            }
        }
        removed
    }

    // Tüm güvenilen paketleri dosyaya yeniden yazar (basit ama potansiyel olarak verimsiz)
    fn persist_trusted_packages(&self) -> Result<(), SahneError> {
        let mut file = match fs::open(self.packages_file_path, fs::O_CREAT | fs::O_WRONLY | fs::O_TRUNC) {
            Ok(fd) => fd,
            Err(e) => return Err(e),
        };

        for package in &self.trusted_packages {
            let line = format!("{}\n", package);
            let buffer = line.as_bytes();
            let mut written = 0;
            while written < buffer.len() {
                match fs::write(file, &buffer[written..]) {
                    Ok(bytes) => written += bytes,
                    Err(e) => {
                        fs::close(file).unwrap_or_default();
                        return Err(e);
                    }
                }
            }
        }
        fs::close(file).unwrap_or_default();
        Ok(())
    }

    pub fn is_trusted_package(&self, package_name: &str) -> bool {
        self.trusted_packages.contains(package_name)
    }
}
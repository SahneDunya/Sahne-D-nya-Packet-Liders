#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // HashMap, HashSet, String, Vec için

use alloc::collections::{BTreeMap, BTreeSet}; // no_std için yaygın olarak BTreeMap/BTreeSet kullanılır, Hash versiyonları da alloc ile mümkündür. HashMap/HashSet kullanalım şimdilik, alloc gerektirir.
use alloc::collections::{HashMap, HashSet};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use alloc::borrow::ToOwned; // &str'dan String'e çevirmek için
use crate::resource;
use crate::SahneError;
use crate::Handle;

// Özel hata enum'ımızı içe aktar (no_std uyumlu ve SahneError'ı içeren haliyle)
use crate::paket_yoneticisi_hata::PaketYoneticisiHatasi;
// SahneError, ParsingError, ConflictError vb. hatalardan dönüşüm From implementasyonları ile sağlanacak

// Basit bir paket tanımı
#[derive(Debug, Clone, PartialEq, Eq, Hash)] // Hash, Eq, PartialEq, Clone derive'ları alloc ile no_std'de çalışır
pub struct Package { // pub yapıldı ki dışarıdan kullanılabilsin
    pub name: String,
    pub version: String,
}

// Bağımlılıkları temsil eden bir yapı (Ana Paket -> Bağımlı Paket Listesi)
pub type Dependencies = HashMap<Package, Vec<Package>>; // pub yapıldı

// -- Helper fonksiyon: Kaynaktan tüm içeriği String olarak oku --
// Çeşitli modüllerde kullanışlı olabilir.
fn read_resource_to_string(resource_id: &str) -> Result<String, PaketYoneticisiHatasi> {
    let handle = resource::acquire(resource_id, resource::MODE_READ)
        .map_err(|e| {
             eprintln!("Kaynak acquire hatası ({}): {:?}", resource_id, e);
             PaketYoneticisiHatasi::from(e) // SahneError -> PaketYoneticisiHatasi
        })?;

    let mut buffer = Vec::new(); // alloc::vec::Vec kullanılıyor
    let mut temp_buffer = [0u8; 512]; // Okuma tamponu (stack'te)

    // Kaynağın tüm içeriğini oku (parça parça okuma döngüsü)
    loop {
        match resource::read(handle, &mut temp_buffer) {
            Ok(0) => break, // Kaynak sonu
            Ok(bytes_read) => {
                buffer.extend_from_slice(&temp_buffer[..bytes_read]);
            }
            Err(e) => {
                // Okuma hatası durumunda handle'ı serbest bırakıp hata dön
                let _ = resource::release(handle);
                 eprintln!("Kaynak okuma hatası ({}): {:?}", resource_id, e);
                return Err(PaketYoneticisiHatasi::from(e)); // SahneError -> PaketYoneticisiHatasi
            }
        }
    }

    // Handle'ı serbest bırak
    let release_result = resource::release(handle);
     if let Err(e) = release_result {
          eprintln!("Kaynak release hatası ({}): {:?}", resource_id, e);
          // Release hatası kritik olmayabilir, loglayıp devam edebiliriz.
     }

    // Tampondaki binary veriyi String'e çevir (UTF-8 varsayımıyla)
    // core::str::from_utf8 Result<&str, Utf8Error> döner.
    core::str::from_utf8(&buffer)
        .map(|s| s.to_owned()) // &str -> String (alloc gerektirir)
        .map_err(|_| {
            eprintln!("Kaynak içeriği geçerli UTF-8 değil ({})", resource_id);
            // UTF-8 hatası için özel bir PaketYoneticisiHatasi varyantı eklenebilir (e.g., ParsingError)
            PaketYoneticisiHatasi::GecersizParametre(format!("Geçersiz UTF-8 Kaynak içeriği: {}", resource_id)) // String kullanmak yerine hataya detay eklenebilir
        })
}


// Bağımlılıkları bir Sahne64 Kaynağından alır (örneğin bir yapılandırma dosyası gibi).
// dependencies_resource_id: Bağımlılık verilerini içeren Kaynağın ID'si (örn. "sahne://config/dependencies.list")
pub fn get_dependencies(dependencies_resource_id: &str) -> Result<Dependencies, PaketYoneticisiHatasi> { // Result türü PaketYoneticisiHatasi olmalı
    let mut deps = HashMap::new();

    // Bağımlılık verilerini Kaynaktan String olarak oku
    let file_content = match read_resource_to_string(dependencies_resource_id) {
        Ok(content) => content,
        Err(PaketYoneticisiHatasi::SahneApiHatasi(SahneError::ResourceNotFound)) => {
             // Bağımlılık dosyası bulunamadıysa, boş bağımlılık listesi ile devam et.
             // Bu, PaketYoneticisiHatasi::from(e) dönüşümünün SahneError::ResourceNotFound'ı özel olarak
             // PaketYoneticisiHatasi::PaketBulunamadi (veya YapilandirmaDosyasiBulunamadi gibi)
             // bir hataya maplediği durumda daha anlamlı olabilir.
             // Şu anki from implementasyonu genel SahneApiHatasi dönüyor.
             // Eğer buradan özel bir davranış istiyorsak match burada yapılmalı.
             println!("Bağımlılık Kaynağı bulunamadı ({}). Boş bağımlılık listesi ile devam ediliyor.", dependencies_resource_id);
             return Ok(HashMap::new()); // Boş bağımlılık listesi döndür
        }
        Err(e) => {
             // Diğer okuma hataları (resource hataları, UTF-8 hatası vb.)
             eprintln!("Bağımlılık Kaynağı okuma hatası ({}): {:?}", dependencies_resource_id, e);
             return Err(e); // Hata zaten doğru türde
        }
    };

    // Okunan String içeriği satır satır ve parça parça ayrıştır.
    // std::io::BufRead::lines yerine elle satırları ayırma.
    for line in file_content.lines() {
        let line = line.trim(); // Baştaki ve sondaki boşlukları sil (no_std uyumlu &str metodu)
        if line.is_empty() || line.starts_with('#') { // Boş satırları veya yorum satırlarını atla
            continue;
        }

        let parts: Vec<&str> = line.split(" -> ").collect(); // alloc::vec::Vec ve &str::split kullanılıyor
        if parts.len() == 2 {
            let package_str = parts[0];
            let dependencies_str = parts[1];

            let package_parts: Vec<&str> = package_str.split('@').collect();
            if package_parts.len() == 2 {
                let package_name = package_parts[0].to_string(); // &str -> String (alloc gerektirir)
                let package_version = package_parts[1].to_string(); // &str -> String (alloc gerektirir)

                let package = Package {
                    name: package_name,
                    version: package_version,
                };

                let mut dependency_list = Vec::new();
                for dep_str in dependencies_str.split(',') {
                    let dep_parts: Vec<&str> = dep_str.trim().split('@').collect();
                    if dep_parts.len() == 2 {
                         let dep_name = dep_parts[0].to_string(); // &str -> String
                         let dep_version = dep_parts[1].to_string(); // &str -> String
                        dependency_list.push(Package {
                            name: dep_name,
                            version: dep_version,
                        });
                    } else {
                        eprintln!("Geçersiz bağımlılık formatı: {}", dep_str);
                        // Geçersiz format hatası durumunda hata dönebiliriz.
                         return Err(PaketYoneticisiHatasi::ParsingError(format!("Geçersiz bağımlılık formatı: {}", dep_str)));
                        // Şimdilik sadece loglayıp devam edelim, ama gerçek PM'de hata durdurucu olabilir.
                    }
                }
                deps.insert(package, dependency_list); // HashMap insert (alloc gerektirir)
            } else {
                eprintln!("Geçersiz paket formatı: {}", package_str);
                 // Geçersiz format hatası durumunda hata dönebiliriz.
                 // return Err(PaketYoneticisiHatasi::ParsingError(format!("Geçersiz paket formatı: {}", package_str)));
            }
        } else if !line.trim().is_empty() {
            eprintln!("Geçersiz satır formatı: {}", line);
             // Geçersiz format hatası durumunda hata dönebiliriz.
              return Err(PaketYoneticisiHatasi::ParsingError(format!("Geçersiz satır formatı: {}", line)));
        }
    }

    Ok(deps)
}

// Çakışmaları tespit eden fonksiyon
// Bağımlılık Haritasını alır ve çakışan paket çiftlerini (aynı isimde farklı versiyonlar) döndürür.
pub fn detect_conflicts(dependencies: &Dependencies) -> HashSet<(Package, Package)> { // pub yapıldı
    let mut conflicts = HashSet::new(); // alloc::collections::HashSet
    let mut required_packages: HashMap<String, HashSet<Package>> = HashMap::new(); // alloc::collections::HashMap<String, HashSet<Package>>

    // Rekürsif bağımlılık toplama fonksiyonu
    // package: Mevcut işlenen paket
    // dependencies_map: Tüm bağımlılıkları içeren harita
    // collected_packages: Toplanan paketleri isimlerine göre gruplayan harita (isim -> Paket versiyonları kümesi)
    // visited: Döngüleri önlemek için ziyaret edilen paketler kümesi
    fn collect_dependencies_recursive(
        package: &Package,
        dependencies_map: &Dependencies,
        collected_packages: &mut HashMap<String, HashSet<Package>>,
        visited: &mut HashSet<Package>,
    ) {
        if visited.contains(package) {
            return; // Zaten ziyaret edildi, döngüyü kır
        }
        visited.insert(package.clone()); // Paketi ziyaret edildi olarak işaretle (Clone, Hash, Eq gerektirir)

        // Paketi, adı altında toplanan versiyonlar kümesine ekle
        collected_packages
            .entry(package.name.clone()) // Adını String olarak al (Clone gerektirir)
            .or_default() // Eğer yoksa yeni boş HashSet oluştur (alloc gerektirir)
            .insert(package.clone()); // Paketi kümeye ekle (Clone, Hash, Eq gerektirir)

        // Mevcut paketin bağımlılıklarını rekürsif olarak topla
        if let Some(deps) = dependencies_map.get(package) { // HashMap get (&Package, Package için Hash ve Eq kullanır)
            for dep in deps { // &Vec<Package> üzerinde iterasyon
                collect_dependencies_recursive(dep, dependencies_map, collected_packages, visited);
            }
        }
    }

    // Her bir kök paket için (yani bağımlılık haritasının anahtarları) bağımlılık ağacını gez
    for package in dependencies.keys() { // HashMap keys() iteratörü
        let mut collected_for_root: HashMap<String, HashSet<Package>> = HashMap::new();
        let mut visited = HashSet::new();
        collect_dependencies_recursive(package, dependencies, &mut collected_for_root, &mut visited);

        // Toplanan paketler içinde aynı isme sahip birden fazla versiyon varsa çakışma var demektir
        for (pkg_name, versions) in collected_for_root.iter() { // iterasyon
            if versions.len() > 1 { // Aynı isimde birden fazla versiyon var mı?
                // Çakışan versiyon çiftlerini bul ve conflicts kümesine ekle
                let versions_vec: Vec<_> = versions.iter().collect(); // HashSet'ten Vec'e dönüştür (alloc gerektirir)
                for i in 0..versions_vec.len() {
                    for j in i + 1..versions_vec.len() {
                        // Çakışan çifti (versiyon1, versiyon2) olarak conflicts kümesine ekle
                        // Sıralama önemli değil, HashSet aynı çifti farklı sırayla tekrar eklemez.
                        // Ancak burada emin olmak için canonical bir sıra belirleyebiliriz (örneğin isme ve versiyona göre sıralayıp çifti öyle eklemek)
                        // Şimdilik sadece olduğu gibi ekleyelim, HashSet'in içsel davranışı buna izin verir.
                        // elements() metodu stable sıralama sağlamaz, iter() de öyle.
                        // Güvenlik için çifti (min_versiyon, max_versiyon) şeklinde normalleştirebiliriz.
                        let p1 = versions_vec[i].clone(); // Paketi klonla
                        let p2 = versions_vec[j].clone(); // Paketi klonla

                        // Çifti (p1, p2) veya (p2, p1) olarak ekleyebiliriz. HashSet tekrarları kaldırır.
                        // Daha temiz bir yaklaşım: (min(p1, p2), max(p1, p2)) eklemek. Package PartialOrd implement etmeli bunun için.
                        // Package struct'ına PartialOrd implementasyonu ekleyelim.
                         let conflict_pair = if p1 < p2 { (p1, p2) } else { (p2, p1) };
                         conflicts.insert(conflict_pair); // Küme insert (alloc gerektirir)
                    }
                }
            }
        }
    }

    conflicts // Çakışma çiftleri kümesini döndür
}

// Package struct'ına PartialOrd ve Ord derive'larını ekleyelim ki çakışma çiftlerini normalleştirebilelim.
// Derivasyon sırası önemli olabilir (Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord).
impl PartialOrd for Package {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Package {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        // Önce isme göre sırala
        match self.name.cmp(&other.name) {
            core::cmp::Ordering::Equal => {
                // İsimler aynıysa versiyona göre sırala
                // Versiyon string'lerini doğrudan karşılaştırmak semantic versiyonlama için yeterli değil.
                // Gerçek bir PM'de versiyon string'leri ayrıştırılıp semantic olarak karşılaştırılmalıdır (semver crate'i gibi, ama no_std?).
                // Basitlik adına string karşılaştırması yapalım.
                self.version.cmp(&other.version)
            }
            ordering => ordering, // İsimler farklıysa o sıralamayı kullan
        }
    }
}


// Çakışmaları çözen fonksiyon.
// Basit örnek: Hiç çakışma yoksa orijinal bağımlılıkları döndürür, çakışma varsa hata döner.
// Gerçek bir PM'de bu fonksiyon, versiyon seçimi, kullanıcı onayı veya bağımlılık grafı manipülasyonu gibi
// karmaşık mantık içerebilir.
// dependencies: Tüm paketlerin bağımlılık haritası
// conflicts: detect_conflicts tarafından bulunan çakışma çiftleri kümesi
pub fn resolve_conflicts( // pub yapıldı
    dependencies: &Dependencies,
    conflicts: &HashSet<(Package, Package)>,
) -> Result<Dependencies, PaketYoneticisiHatasi> { // Result türü PaketYoneticisiHatasi olmalı
    if !conflicts.is_empty() {
        eprintln!("Çakışmalar tespit edildi, çözülemiyor:");
        for (p1, p2) in conflicts.iter() {
             eprintln!("- Çakışma: {:?} ve {:?}", p1, p2);
        }
        // Hata durumunda özel bir PaketYoneticisiHatasi varyantı dönelim.
        // return Err("Çakışmalar çözülemedi (basit örnek).".to_string()); // String yerine hata enum
        Err(PaketYoneticisiHatasi::PaketKurulumHatasi(String::from("Bağımlılık çakışmaları çözülemedi"))) // Kurulum hatası veya özel bir ConflictError
    } else {
        // Çakışma yoksa, orijinal bağımlılık haritasını döndür.
        Ok(dependencies.clone()) // HashMap clone (alloc gerektirir)
    }
}


// #[cfg(test)] bloğu std'ye bağımlı olduğu için kaldırıldı veya devre dışı bırakıldı.

#[cfg(test)]
mod tests {
    // Bu testler std::fs, tempfile vb. kullandığı için Sahne64'ün no_std ortamında çalışmaz.
    // Bunlar yerine no_std uyumlu in-memory testler veya entegrasyon testleri yazılmalıdır.
}

// --- PaketYoneticisiHatasi enum tanımının no_std uyumlu hale getirilmesi ---
// (paket_yoneticisi_hata.rs dosyasında veya ilgili modülde olmalı)

// paket_yoneticisi_hata.rs (Örnek - no_std uyumlu, Parsing/Conflict hatası eklenmiş)
#![no_std]
extern crate alloc;

use alloc::string::String;
use alloc::format;
use crate::SahneError;
// use postcard::Error as PostcardError; // Yapılandırma modülünden

// ... (ZipHatasi, OnbellekHatasi, ChecksumResourceError, GecersizParametre, PathTraversalHatasi, SahneApiHatasi) ...

#[derive(Debug)]
pub enum PaketYoneticisiHatasi {
    // ... (Çeşitli hatalar) ...

    // Kaynak içeriğini ayrıştırma (parsing) sırasında oluşan hatalar (örn. bağımlılık listesi formatı hatası, UTF-8 hatası)
    ParsingError(String), // Hata detayını string olarak tutmak alloc gerektirir. Alternatif: Hata detayını koda çevirme.

    // Bağımlılık çakışması tespit edildiğinde (çözülemeyen durumda)
    ConflictError(String), // Çakışma detayını string olarak tutmak alloc gerektirir. Belki çakışan paketleri döndüren bir yapı tutulur?

    // Genel paket kurulumu veya kaldırma hatası (belki ConflictError bunun altına girer)
    PaketKurulumHatasi(String), // Veya PaketYonetimiHatasi

    // Sahne64 API'sından gelen genel hatalar
    SahneApiHatasi(SahneError),

    // Serileştirme/Deserileştirme hataları (Yapılandırma modülünden)
    // SerializationError(PostcardError),
    // DeserializationError(PostcardError),

    GecersizParametre(String), // Fonksiyona geçersiz parametre geçilmesi

    // ... diğer hatalar ...
    BilinmeyenHata, // String'siz basit bir bilinmeyen hata
}

// SahneError'dan PaketYoneticisiHatasi'na dönüşüm
// Bu From implementasyonu genel SahneApiHatasi veya spesifik hatalar için kullanılabilir.
impl From<SahneError> for PaketYoneticisiHatasi {
    fn from(err: SahneError) -> Self {
        // Kaynakla ilgili hataları (özellikle bağımlılık dosyası okurken)
        // daha spesifik bir DependencyResourceError'a yönlendirebiliriz,
        // veya genel SahneApiHatasi içinde bırakabiliriz.
        match err {
             SahneError::ResourceNotFound => {
                 // Bağımlılık dosyası bulunamadı özel durumu get_dependencies içinde ele alınıyor.
                 // Burada ResourceNotFound başka bir yerden gelirse genel hata olarak ele alınır.
                 PaketYoneticisiHatasi::SahneApiHatasi(err)
             }
             SahneError::PermissionDenied |
             SahneError::InvalidHandle |
             SahneError::ResourceBusy |
             SahneError::InvalidOperation => {
                 // Resource ile ilgili diğer hatalar
                 PaketYoneticisiHatasi::SahneApiHatasi(err) // Veya özel bir ResourceError varyantı
             }
             _ => PaketYoneticisiHatasi::SahneApiHatasi(err),
        }
    }
}

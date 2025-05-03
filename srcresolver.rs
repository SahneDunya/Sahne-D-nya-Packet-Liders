#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // HashMap, HashSet, String, Vec, format! için

use alloc::collections::{HashMap, HashSet}; // std::collections::* yerine
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format; // format! makrosu için
use alloc::borrow::ToOwned; // &str -> String için

// Sahne64 API modülleri
use crate::resource; // Kaynak işlemleri
use crate::SahneError; // Sahne64 hata türü
use crate::Handle; // Kaynak Handle'ları

// Paket struct (basit bağımlılık grafı temsili için yerel kopya veya srcpackage'dan import)
// srcpackage.rs'deki Paket struct'ını kullanmak daha mantıklı, ancak burada bağımlılık grafı için
// basitleştirilmiş bir Package struct'ı tanımlayabiliriz veya srcpackage::Paket'i kullanırız.
// Bağımlılık grafı Package (name+version) struct'larını anahtar ve değer olarak kullandığı için
// Hash, Eq, PartialEq, Clone derive etmelidir. srcpackage::Paket de bunları ediyor.
// srcpackage::Paket'i kullanalım.
use crate::package::Paket; // Varsayım: Paket struct'ı srcpackage.rs'de tanımlı ve gerekli derive'lara sahip

// Özel hata enum'ımızı içe aktar (no_std uyumlu hali)
use crate::paket_yoneticisi_hatasi::PaketYoneticisiHatasi;
// SahneError ve yerel hatalardan dönüşüm From implementasyonları ile sağlanacak

// no_std uyumlu print makroları
use crate::print_macros::{println, eprintln};

// Bağımlılıkları temsil eden bir yapı (Paket -> Bağlı Olduğu Paketler)
// Bağımlılıklar genellikle sadece isim içerir Paket struct'ında.
// Buradaki Dependencies map'i çözümlenmiş (isim + versiyon) bağımlılıkları tutar.
// HashMap<Paket, Vec<Paket>> alloc gerektirir.
type Dependencies = HashMap<Paket, Vec<Paket>>;


// Bir Sahne64 Kaynağından (dosya gibi) bağımlılık verisini okur ve ayrıştırır.
// Örnek dosya formatı: `package_name@version -> dep1_name@dep1_version, dep2_name@dep2_version`
// resource_id: Bağımlılık verisini içeren Kaynağın ID'si.
// Dönüş değeri: Ayrıştırılmış bağımlılık map'i veya PaketYoneticisiHatasi.
// Not: Bu fonksiyon, Paket struct'ının bagimliliklar alanını kullanarak bağımlılıkları
// elde etmenin daha doğru yoluna bir geçiş adımı olarak ele alınabilir.
fn read_dependencies_from_resource(resource_id: &str) -> Result<Dependencies, PaketYoneticisiHatasi> { // SahneError yerine PaketYoneticisiHatasi
    let mut deps = HashMap::new(); // alloc gerektirir
    println!("Bağımlılık verisi okunuyor: {}", resource_id); // no_std print

    // Kaynağı okuma izniyle acquire et.
    match resource::acquire(resource_id, resource::MODE_READ) {
        Ok(handle) => {
            let mut buffer = Vec::new(); // Okunan baytları tutacak buffer (alloc::vec::Vec)
            let mut temp_buffer = [0u8; 512]; // Okuma tamponu (stack'te)
            let mut current_line = Vec::new(); // Mevcut satırı tutacak tampon

            // Kaynağı satır satır oku ve ayrıştır.
            // resource::read blocking okuduğunu ve satır sonu olmadığını varsayarak.
            loop {
                match resource::read(handle, &mut temp_buffer) {
                    Ok(0) => {
                        // EOF. Eğer son satırda '\n' yoksa, kalan buffer'ı işle.
                        if !current_line.is_empty() {
                             process_line(&current_line, &mut deps)?; // Kalan kısmı işle
                        }
                        break; // Döngüden çık
                    }
                    Ok(bytes_read) => {
                        // Okunan baytları işle, satır sonlarını (\n) bul ve satırları ayrıştır.
                        for byte in &temp_buffer[..bytes_read] {
                            if *byte == b'\n' {
                                 // Satır sonu bulundu, mevcut satırı işle ve tamponu temizle.
                                 process_line(&current_line, &mut deps)?; // current_line Vec<u8>
                                current_line.clear(); // Clear alloc
                            } else {
                                current_line.push(*byte); // Push alloc
                            }
                        }
                    }
                    Err(e) => {
                        // Okuma hatası durumunda handle'ı serbest bırakıp hata dön.
                        let _ = resource::release(handle);
                         eprintln!("Bağımlılık Kaynağı okuma hatası ({}): {:?}", resource_id, e); // no_std print
                        return Err(PaketYoneticisiHatasi::from(e)); // SahneError -> PaketYoneticisiHatasi
                    }
                }
            }

            // Handle'ı serbest bırak
            let release_result = resource::release(handle);
             if let Err(_e) = release_result {
                  eprintln!("Bağımlılık Kaynağı release hatası ({}): {:?}", resource_id, _e); // no_std print
             }

            Ok(deps) // Okunan ve ayrıştırılan bağımlılık map'ini döndür
        }
        Err(SahneError::ResourceNotFound) => {
            // Kaynak bulunamadıysa, boş bağımlılık listesi ile devam et.
            println!("Bağımlılık Kaynağı bulunamadı ({}). Boş bağımlılık listesi ile devam ediliyor.", resource_id); // no_std print
            Ok(HashMap::new()) // HashMap::new() alloc gerektirir
        }
        Err(e) => {
            // Diğer Sahne64 hataları.
             eprintln!("Bağımlılık Kaynağı acquire hatası ({}): {:?}", resource_id, e); // no_std print
            Err(PaketYoneticisiHatasi::from(e)) // SahneError -> PaketYoneticisiHatasi
        }
    }
}

// Helper fonksiyon: Bir satır (Vec<u8>) ayrıştırır ve bağımlılık map'ine ekler.
fn process_line(line_bytes: &[u8], deps: &mut Dependencies) -> Result<(), PaketYoneticisiHatasi> { // Result eklendi, parsing hatası dönebilir
    // Byte dilimini UTF-8 String'e çevir.
    let line = core::str::from_utf8(line_bytes)
        .map_err(|e| {
            eprintln!("Bağımlılık satırı UTF-8 değil: {:?}", e); // no_std print
            PaketYoneticisiHatasi::ParsingError(String::from("Geçersiz UTF-8 karakteri")) // ParsingError alloc
        })?; // Hata durumunda ? ile yay

    let line = line.trim(); // trim() &str metodu no_std
    if line.is_empty() {
        return Ok(()); // Boş satırları atla
    }

    let parts: Vec<&str> = line.split(" -> ").collect(); // split, collect Vec<&str> no_std
    if parts.len() == 2 {
        let package_str = parts[0];
        let dependencies_str = parts[1];

        let package = parse_package_id(package_str)?; // package_str -> Paket struct (parsing hatası olabilir)

        let mut dependency_list = Vec::new(); // alloc gerektirir
        for dep_str in dependencies_str.split(',') {
            let dep = parse_package_id(dep_str.trim())?; // dep_str -> Paket struct (parsing hatası olabilir)
            dependency_list.push(dep); // push alloc
        }
        deps.insert(package, dependency_list); // insert alloc

    } else {
         eprintln!("Geçersiz bağımlılık satır formatı: {}", line); // no_std print
        return Err(PaketYoneticisiHatasi::ParsingError(format!("Geçersiz satır formatı: {}", line))); // ParsingError alloc
    }

    Ok(()) // Satır başarıyla işlendi
}

// Helper fonksiyon: "name@version" formatındaki stringi Paket struct'ına ayrıştırır.
fn parse_package_id(package_id_str: &str) -> Result<Paket, PaketYoneticisiHatasi> { // Result eklendi, parsing hatası dönebilir
    let parts: Vec<&str> = package_id_str.split('@').collect(); // split, collect Vec<&str> no_std
    if parts.len() == 2 {
        Ok(Paket {
            ad: parts[0].to_owned(), // to_owned() alloc
            surum: parts[1].to_owned(), // to_owned() alloc
            // Paket struct'ındaki diğer alanları varsayılan/boş değerlerle doldur
            bagimliliklar: Vec::new(), // alloc
            aciklama: None,
            dosya_adi: None,
            checksums: HashMap::new(), // alloc
            dosyalar: Vec::new(), // alloc
            kurulum_scripti: None,
            kaldirma_scripti: None,
        })
    } else {
         eprintln!("Geçersiz paket ID formatı: {}", package_id_str); // no_std print
        Err(PaketYoneticisiHatasi::ParsingError(format!("Geçersiz paket ID formatı: {}", package_id_str))) // ParsingError alloc
    }
}


// Döngü içeren bağımlılıkları elde eder (read_dependencies_from_resource üzerine inşa edilir)
// resource_id: Bağımlılık verisini içeren Kaynağın ID'si.
// Dönüş değeri: Ayrıştırılmış bağımlılık map'i (döngü eklenmiş hali) veya PaketYoneticisiHatasi.
fn get_dependencies_with_cycle(resource_id: &str) -> Result<Dependencies, PaketYoneticisiHatasi> { // SahneError yerine PaketYoneticisiHatasi
    let mut deps = read_dependencies_from_resource(resource_id)?; // Bağımlılıkları oku

    // Döngüyü manuel olarak ekleyelim (örnek amaçlı)
    let package_c = Paket {
        ad: "C".to_owned(), // alloc
        surum: "3.0.0".to_owned(), // alloc
        bagimliliklar: Vec::new(), aciklama: None, dosya_adi: None,
        checksums: HashMap::new(), dosyalar: Vec::new(),
        kurulum_scripti: None, kaldirma_scripti: None,
    };
    let package_a = Paket {
        ad: "A".to_owned(), // alloc
        surum: "1.0.0".to_owned(), // alloc
        bagimliliklar: Vec::new(), aciklama: None, dosya_adi: None,
        checksums: HashMap::new(), dosyalar: Vec::new(),
        kurulum_scripti: None, kaldirma_scripti: None,
    };

    // Eğer "C@3.0.0" varsa, ona "A@1.0.0" bağımlılığını ekleyelim.
    if deps.contains_key(&package_c) {
        if let Some(deps_for_c) = deps.get_mut(&package_c) {
            deps_for_c.push(package_a); // push alloc
        }
    } else {
        // "C@3.0.0" yoksa, onu oluşturup bağımlılığını ekle
        deps.insert(package_c, vec![package_a]); // insert, vec! alloc
    }
    Ok(deps) // Güncellenmiş map'i döndür
}

// Özel hata türü (no_std uyumlu)
#[derive(Debug, Clone, PartialEq, Eq)] // Debug, Clone, PartialEq, Eq derive'ları no_std'de çalışır
enum DependencyResolverError { // İsim DependencyError yerine DependencyResolverError olarak değiştirildi çakışmayı önlemek için
    CycleDetected(String), // Döngü tespit edildi. Döngüdeki paketleri detay olarak ekleyebiliriz. String alloc gerektirir.
    // Diğer çözümleme hataları eklenebilir (örn. bulunamayan paket).
    PackageNotFound(String), // Çözümlenemeyen paket. String alloc gerektirir.
}

// core::fmt::Display implementasyonu
impl core::fmt::Display for DependencyResolverError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DependencyResolverError::CycleDetected(s) => write!(f, "Bağımlılık döngüsü tespit edildi: {}", s),
            DependencyResolverError::PackageNotFound(s) => write!(f, "Bağımlılık çözümlenemedi, paket bulunamadı: {}", s),
        }
    }
}

// Basit bir bağımlılık çözücü (döngü tespiti ile)
// dependencies: Çözümlenmiş bağımlılık grafı (HashMap<Paket, Vec<Paket>>).
// root_package: Çözümlemeye başlanacak kök paket.
// Dönüş değeri: Kurulması gereken paketlerin kümesi (HashSet<Paket>) veya DependencyResolverError.
fn resolve_dependencies(
    dependencies: &Dependencies,
    root_package: &Paket, // Kök paket de Paket struct'ı olmalı
) -> Result<HashSet<Paket>, DependencyResolverError> { // Result türü DependencyResolverError olmalı
    let mut resolved = HashSet::new(); // alloc gerektirir
    let mut to_resolve = vec![root_package.clone()]; // Kök paketi stack'e ekle (clone, vec! alloc)
    let mut resolving_stack: HashSet<Paket> = HashSet::new(); // Döngü tespiti için yığın (alloc gerektirir)

    // root_package'ın bağımlılık grafında olup olmadığını kontrol etmeliyiz.
    // Eğer yoksa, resolve edilmeye çalışılamaz (veya kendi kendine bağımlılığı yoksa boş döner).
    // Kök paket grafın bir parçası değilse hata dönmek mantıklı olabilir.
     if !dependencies.contains_key(root_package) {
         return Err(DependencyResolverError::PackageNotFound(format!("Kök paket bağımlılık grafında bulunamadı: {}@{}", root_package.ad, root_package.surum))); // format! alloc
     }


    while let Some(package) = to_resolve.pop() { // pop() Option<Paket>
        // Eğer paket zaten çözümlenmişse veya şu an çözülmekte olan yığındaysa atla.
        if resolved.contains(&package) { // contains (&Paket)
            continue;
        }

        if resolving_stack.contains(&package) { // contains (&Paket)
             // Döngü tespit edildi. Hata olarak dön.
             eprintln!("Bağımlılık döngüsü tespit edildi: {}@{}", package.ad, package.surum); // no_std print
            return Err(DependencyResolverError::CycleDetected(format!("{}@{}", package.ad, package.surum))); // format! alloc
        }

        // Paketi işleme yığınına ekle.
        resolving_stack.insert(package.clone()); // insert, clone alloc

        // Paketin bağımlılıklarını al ve çözülecekler listesine ekle.
        if let Some(deps) = dependencies.get(&package) { // get (&Paket)
            for dep in deps {
                 // Bağımlılık grafında olmayan bir bağımlılık varsa hata dön (bulunamayan paket).
                 // Her bağımlılığın kendisi de grafın bir anahtarı olmalıdır (veya yaprak düğümse bağımlılığı yoktur).
                 // Basitlik adına, burada sadece bağımlılığı to_resolve'a ekliyoruz.
                 // Gerçek bir çözümleyicide, burada paketin depoda/veritabanında olup olmadığını kontrol etmek gerekir.
                  if !dependencies.contains_key(dep) && !dep.bagimliliklar.is_empty() { // Eğer grafın bir parçası değilse ve kendisinin bağımlılığı varsa... karmaşık.
                 //     // Basit hata: Eğer bağımlılık grafın anahtarlarında yoksa ve zaten çözülmüş/çözülüyor değilse, bulunamadı hatası verebiliriz.
                  }

                 to_resolve.push(dep.clone()); // push, clone alloc
            }
        } else {
             // Eğer paketin bağımlılığı yoksa veya graf içinde yoksa, bu bir yaprak düğümdür.
             // Bu normal olabilir.
        }

        // Paket çözümlendi olarak işaretle.
        resolved.insert(package.clone()); // insert, clone alloc

        // Paketi işleme yığınından çıkar.
        resolving_stack.remove(&package); // remove (&Paket)
    }

    Ok(resolved) // Çözümlenmiş paketlerin kümesini döndür
}


// #[cfg(test)] bloğu std test runner'ı ve dosya sistemi/resource mock'ları gerektirir.
// Bu blok no_std ortamında derlenmeyecektir eğer std feature aktif değilse ve test ortamı yoksa.

#[cfg(test)]
mod tests {
    // std::collections, std::io, std::path, std::fs, tempfile kullandığı için no_std'de doğrudan çalışmaz.
    // Bağımlılık dosyasını okuma helper'ları ve test senaryoları mock resource veya Sahne64 simülasyonu gerektirir.
}

// --- Paket Struct Tanımı ---
// srcpackage.rs modülünde tanımlanmıştır ve no_std uyumludur.
// Gerekli derive'lara (Debug, Clone, PartialEq, Eq, Hash) sahip olduğu varsayılır.


// --- PaketYoneticisiHatasi Enum Tanımı ---
// srcerror.rs modülünde tanımlanmıştır ve no_std uyumludur.
// SahneApiHatasi, ParsingError varyantları kullanılmaktadır.
// SahneError'dan PaketYoneticisiHatasi'na dönüşüm için From implementasyonu gereklidir.
// DependencyResolverError'dan PaketYoneticisiHatasi'na dönüşüm için From implementasyonu eklenmelidir.

// srcerror.rs (Güncellenmiş - DependencyResolverError'dan dönüşüm eklenmiş)
#![no_std]
extern crate alloc;

// ... diğer importlar ...

use crate::srcresolver::DependencyResolverError; // DependencyResolverError'ı içe aktar

#[derive(Debug)]
pub enum PaketYoneticisiHatasi {
    // ... diğer hatalar ...

    // Bağımlılık çözümleme sırasında oluşan hatalar
    DependencyResolutionError(DependencyResolverError), // DependencyResolverError'ı sarmalar

    // Parsing hataları (bağımlılık dosyasından okuma sırasında)
    ParsingError(String), // String alloc gerektirir

    // ... diğer hatalar ...
}

// DependencyResolverError'dan PaketYoneticisiHatasi'na dönüşüm
impl From<DependencyResolverError> for PaketYoneticisiHatasi {
    fn from(err: DependencyResolverError) -> Self {
        PaketYoneticisiHatasi::DependencyResolutionError(err)
    }
}

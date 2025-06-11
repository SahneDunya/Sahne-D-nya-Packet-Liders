#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // Bellek ayırma için alloc crate'i

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::boxed::Box; // ZipArchive::new için Box gerekebilir

use zip::{ZipArchive, result::ZipError};
// Eğer zip crate'i no_std modunda Read/Write trait'leri sağlıyorsa, onları import edin.
// Aksi halde kendi Read benzeri trait'imizi ve implementasyonumuzu yazmalıyız.
// Şimdilik zip crate'inin core/alloc uyumlu Read beklediğini varsayalım.
// Gerçek implementasyon Sahne64'ün resource::read üzerine kurulmalı.
 use zip::read::ZipFile; // Belki ZipFile için de core::io::Read implementasyonu gerekir.

// Sahne64 API modüllerini içe aktarın
use crate::resource;
use crate::memory; // Global allocator için gerekebilir
use crate::SahneError; // Sahne64'ün hata türü
use crate::Handle; // Sahne64'ün Handle türü

// Özel hata enum'ımızı içe aktar (güncellenmiş haliyle)
use crate::paket_yoneticisi_hata::PaketYoneticisiHata;

// Sahne64 resource::read üzerine kurulu basit bir Read implementasyonu
// zip crate'inin tam olarak ne beklediğine göre bu struct ve trait değişebilir.
// Varsayım: zip crate'i Read trait'inin core::io veya benzeri bir no_std versiyonunu kullanıyor.
struct SahneResourceReader {
    handle: Handle,
    // Okuma pozisyonu takip edilebilir, ancak resource::read offset argümanı almıyorsa
    // bu Reader her zaman baştan okur veya Kaynak seek edilebilir olmalıdır.
    // Sahne64'ün resource::read'i offset almıyordu, bu bir sınırlama olabilir.
    // Belki resource::control ile seek yapmak mümkündür?
    // Şimdilik basitlik adına, okuma işleminin kaynağın başından başladığını varsayalım.
}

impl SahneResourceReader {
    fn new(handle: Handle) -> Self {
        Self { handle }
    }
}

// Rust'ın henüz deneysel olan core::io::Read trait'ini kullanmak yerine,
// zip crate'inin kendi no_std okuma trait'i varsa onu kullanmalıyız.
// Veya geçici olarak std::io::Read'e benzer bir trait tanımlayıp onu implement etmeliyiz.
// zip crate'i genellikle std::io::Read bekler. No_std zip crate'leri farklı olabilir.
// Farzedelim ki zip crate'i şöyle bir trait bekliyor:
// trait NoStdRead { fn read(&mut self, buf: &mut [u8]) -> Result<usize, YourIoError>; }
// Veya zip crate'i, ZipArchive::new için std::io::Read yerine doğrudan RawHandle veya benzeri bir şey alabilir.
// zip crate'inin no_std desteğini ve gereksinimlerini kontrol etmek kritik.

// **Önemli Not:** zip crate'inin no_std desteği genellikle `std::io::Read` ve `std::io::Write`'ın
// bir `no_std` ortamında yeniden implemente edilmiş hallerine dayanır. Bu oldukça karmaşık olabilir.
// Alternatif olarak, daha basit bir `no_std` uyumlu deflate/zip dekompresyon kütüphanesi aramak
// veya yazmak gerekebilir.
// Bu taslakta, SahneResourceReader'ın bir şekilde zip crate'inin beklediği okuma arayüzünü
// sağladığını VARSAYIYORUZ.

// Dizin oluşturma işlevi için placeholder. Sahne64'te nasıl yapılır?
// Varsayım: Belirli bir resource ID formatı ("sahne://install/paket_adi/dosya/")
// Acquire/Write sırasında kernel/service tarafından parent resource'lar otomatik oluşturulur VEYA
// resource::control ile özel bir 'mkdir' benzeri komut gönderilebilir.
// Şimdilik, dosya yazmadan önce parent resource'u 'acquire' ile açmayı deneyerek
// kernelin CREATE flag'i ile dizin benzeri resource'ları oluşturmasını umalım.
// Bu varsayım gerçek Sahne64 tasarımına bağlıdır. Daha sağlam bir yaklaşım gerekiyorsa
// burası çekirdek API'sında yeni bir syscall veya resource::control kullanımı gerektirir.
fn sahne_create_resource_recursive(resource_id: &str) -> Result<(), PaketYoneticisiHata> {
    // resource_id "sahne://install/my_package/path/to/file" ise,
    // "sahne://install/my_package/", "sahne://install/my_package/path/", vb.
    // resource'larını oluşturmayı denemeliyiz.
    // Sahne64'ün resource ID formatı ve CREATE flag'inin davranışı net olmalı.
    // Şimdilik basit string manipülasyonu ile parent yolları bulup acquire etmeyi deneyelim.

    let mut current_path = String::new();
    let parts = resource_id.split('/');

    for part in parts {
        if part.is_empty() {
            continue; // Başlangıçtaki 'sahne://' veya '/'-lerin arasındaki boşluklar
        }
        current_path.push_str(part);
        // Eğer bu son parça değilse VEYA son parça bir dizin gibi düşünülüyorsa (örn. sonu '/ ile bitiyorsa)
        // bu kısmı bir "dizin" resource'u olarak acquire etmeyi deneyelim.
        // Ancak zip arşivindeki isimler '/ ile bitebilir veya bitmeyebilir.
        // Gerçek Sahne64 modeline göre bu ayrım netleşmeli.

        // Varsayım: Sadece dosya yazarken parent path'leri oluşturmaya çalışmak yeterli.
        // Veya zip entry adı '/' ile bitiyorsa bunu bir "dizin" olarak işlemek.
        // zip_ac fonksiyonundaki döngüde bu ayrım zaten yapılıyor.
        // Buradaki fonksiyon sadece 'dizin' resource'ları için çağrılırsa anlamlı.
        // zip_ac içinde, dosya yazmadan önce parent dizin resource'unu acquire etmeye çalışalım.
    }
    // Eğer fonksiyon sadece dizinler için çağrılıyorsa, son yolu acquire et:
     match resource::acquire(resource_id, resource::MODE_CREATE) {
         Ok(handle) => { let _ = resource::release(handle); Ok(()) }, // Handle'ı hemen bırakabiliriz? Veya tutmalı mıyız? Dizin resource'larının ömrü nasıl yönetilir?
         Err(e) => Err(PaketYoneticisiHata::from_sahne_error(e)),
     }

    // Zip çıkarma bağlamında, dosya yazmadan önce parent dizini sağlamak daha yaygın.
    // Bu fonksiyonu bu bağlama uyarlayalım.
    // Parametre olarak tam yolu alsın, parent dizinleri oluşturmaya çalışsın.

    let parent_path_opt = resource_id.rfind('/').map(|idx| &resource_id[..idx]);

    if let Some(parent_path) = parent_path_opt {
        if parent_path.is_empty() {
            // Kök dizin, oluşturmaya gerek yok?
            return Ok(());
        }
        // Parent yolu, örneğin "sahne://install/my_package/path/to"
        // Bu yolu acquire etmeye çalışalım. Varsayım: acquire(MODE_CREATE) ile
        // eksik parent'lar otomatik oluşur veya bu resource bir klasör gibi davranır.
        // Veya Sahne64'te 'klasör' resource'ları için özel bir acquire flag'i vardır.
        // resource::MODE_CREATE | resource::MODE_CONTAINER ? (Örnek bir flag)

        match resource::acquire(parent_path, resource::MODE_CREATE /* | resource::MODE_CONTAINER */) {
             Ok(handle) => {
                 // Başarıyla acquire ettik, bu Handle'ı bırakabiliriz eğer sadece oluşturmaksa amaç.
                 // Eğer dizin resource'larının ömrü Handle'a bağlıysa, bu Handle'ın yönetimi gerekir.
                 // Kurulum süresince açık tutulup sonra mı bırakılmalı? Karmaşık bir konu.
                 // Şimdilik oluşturduktan sonra bırakalım, kernelin kalıcılığı sağladığını varsayarak.
                 let _ = resource::release(handle);
                 Ok(())
             },
             Err(SahneError::NamingError) => {
                  // Kaynak isimlendirme hatası, belki yol geçersiz veya kısıtlı bir alan?
                  Err(PaketYoneticisiHata::from_sahne_error(SahneError::NamingError))
             }
             Err(e) => {
                 // Diğer Sahne64 hataları
                 Err(PaketYoneticisiHata::from_sahne_error(e))
             }
         }
    } else {
        // Eğer yol '/' içermiyorsa (örn. "dosya_adi"), parent yok, bir şey oluşturmaya gerek yok.
        Ok(())
    }
}


// Verilen ZIP arşivini belirtilen Kaynak ID'si altına açar.
// arsiv_resource_id: Açılacak ZIP arşivinin Sahne64 Kaynak ID'si (örn. "sahne://downloads/paket.zip")
// cikartma_base_resource_id: Paket içeriğinin çıkarılacağı ana dizin gibi davranan Sahne64 Kaynak ID'si (örn. "sahne://installed_packages/my_package/")
pub fn zip_ac(arsiv_resource_id: &str, cikartma_base_resource_id: &str) -> Result<(), PaketYoneticisiHata> {
    // 1. ZIP Arşiv Kaynağını Aç
    let arsiv_handle = resource::acquire(arsiv_resource_id, resource::MODE_READ)
        .map_err(|e| PaketYoneticisiHata::from_sahne_error(e))?; // SahneError'ı kendi hatamıza çevir

    // 2. ZIP Arşivini Okumak için SahneResourceReader kullanma (Varsayımsal)
    // zip crate'inin ZipArchive::new fonksiyonu std::io::Read bekler.
    // No_std ortamında bu ya zip crate'inin no_std Read trait'i ile,
    // ya da kendi Read adaptörümüzle çözülmeli.
    // Geçici olarak zip crate'inin Raw handle alabildiğini veya bizim Reader'ımızı kabul ettiğini varsayalım.
    // Gerçek zip crate kullanımı muhtemelen bir SahneResourceReader instance'ı yaratıp bunu zip'e vermeyi içerir.

    // ZipArchive::new std::io::Read beklediği için, SahneResourceReader'ın std::io::Read implement etmesi lazım.
    // No_std'de std::io implementasyonu olamaz. Ya zip crate'inin no_std::io'su var, ya da bu yaklaşım baştan hatalı.
    // Alternatif: ZIP dosyasının tamamını belleğe oku ve oradan aç (büyük dosyalar için uygun değil).
    // En olası çözüm: zip crate'inin no_std adaptörünü kullanmak veya farklı kütüphane.

    // VEYA: ZIP dosyasını RAM'e oku (büyük bellek gerektirir!)
    // let mut arsiv_data = Vec::new();
    // // Sahne64 resource::read offset almadığı için, tüm dosyayı tek seferde okumak zor olabilir
    // // veya kaynağın Seek edilebilir olması (resource::control ile?) gerekir.
    // // En kötü ihtimalle parçalar halinde okuma döngüsü.
    // // Basitlik adına tüm dosyayı okuduğumuzu varsayalım (Seek mümkünse veya dosya küçükse).
    // // Bu kısım ciddi bir Sahne64-spesifik IO implementasyonu gerektirir.
    // // Şimdilik sadece placeholder bırakalım.
    //
    // // OKUMA İMPLEMENTASYONU GEREKİYOR: zip crate'inin beklediği arayüze göre resource::read'i kullanma
    //
    // // Örnek bir Okuma Döngüsü (resource::read offset almazsa)
     let mut temp_buffer = [0u8; 4096]; // Küçük okuma tamponu
     loop {
         match resource::read(arsiv_handle, &mut temp_buffer) {
             Ok(0) => break, // EOF
             Ok(n) => arsiv_data.extend_from_slice(&temp_buffer[..n]),
             Err(e) => {
                 let _ = resource::release(arsiv_handle);
                 return Err(PaketYoneticisiHata::from_sahne_error(e));
             }
         }
     }
     let arsiv_kullanilabilir_veri = std::io::Cursor::new(arsiv_data); // std::io::Cursor kullanamayız no_std'de

    // **GERÇEK ÇÖZÜM:** zip crate'inin no_std desteği varsa, o desteğe uygun bir Reader implementasyonu yazmak.
    // zip crate'i `by_index` metodu ile doğrudan bir `ZipFile` struct'ı döner, bu struct'ın da
    // Read implementasyonu olmalıdır. Bu implementasyon da yine alttan Sahne64 resource::read'i kullanmalıdır.

    // Varsayım: zip crate'inin no_std uyumlu ZipArchive::new fonksiyonu var ve SahneResourceReader gibi bir şeyi alabiliyor.
    let arsiv_reader = SahneResourceReader::new(arsiv_handle);
    let mut arsiv = ZipArchive::new(arsiv_reader).map_err(PaketYoneticisiHata::ZipHatasi)?;

    // 3. Dosyaları Çıkar
    let cikartma_base_path = String::from(cikartma_base_resource_id); // String olarak tutalım

    for i in 0..arsiv.len() {
        let mut arsiv_dosyasi = arsiv.by_index(i)?; // Bu ZipFile struct'ı Read implement etmeli

        let dosya_adi = arsiv_dosyasi.name();
        // Zip entry isimleri bazen mutlak yol veya '..' içerebilir, temizlemek gerekir.
        // Bu basit temizlik örneği, daha kapsamlı bir path sanitization gerekebilir.
        let temizlenmis_dosya_adi = dosya_adi.replace("..", "_").replace("//", "/"); // Basit sanitization

        // Hedef kaynak ID'sini oluştur
        let cikartma_resource_id = if cikartma_base_path.ends_with('/') || temizlenmis_dosya_adi.starts_with('/') {
            // Eğer base path '/' ile bitiyorsa veya dosya adı '/' ile başlıyorsa doğrudan birleştir
            // (Başlangıç '/' zip formatında nadir olsa da ihtimale karşı)
             let mut path = String::from(&cikartma_base_path);
             if temizlenmis_dosya_adi.starts_with('/') {
                 path.push_str(&temizlenmis_dosya_adi[1..]); // Başlangıç '/' karakterini atla
             } else {
                 path.push_str(&temizlenmis_dosya_adi);
             }
             path
        } else {
            // Base path '/' ile bitmiyorsa ve dosya adı '/' ile başlamıyorsa araya '/' ekle
            alloc::format!("{}/{}", cikartma_base_path, temizlenmis_dosya_adi)
        };


        // Güvenlik kontrolü (Sahne64 Kaynak ID'leri için):
        // Çıkarma yolunun, belirtilen temel resource ID'si altında olduğundan emin ol.
        if !cikartma_resource_id.starts_with(&cikartma_base_path) {
             // Bu kontrol string manipülasyonu ile yapılıyor,
             // ancak Sahne64'ün kendi path/naming kurallarına tam uyumlu olmayabilir.
             // Sahne64 çekirdeği de acquire sırasında yetki/güvenlik kontrolü yapmalıdır.
             eprintln!("Güvenlik hatası: Geçersiz çıkarma yolu denemesi: {}", cikartma_resource_id);
             // Mevcut handle'ı bırakıp hata dönebiliriz.
             let _ = resource::release(arsiv_handle);
             return Err(PaketYoneticisiHata::ZipHatasi(ZipError::InvalidPath(String::from("Güvenlik sebebiyle geçersiz çıkarma yolu"))));
        }


        if temizlenmis_dosya_adi.ends_with('/') {
            // Klasör Kaynağı ise (varsayım) oluşturmayı deneyelim.
            // Sahne64'te 'klasör' resource'u nasıl temsil edilir ve oluşturulur?
            // Varsayım 1: resource::acquire(path_ends_with('/'), MODE_CREATE) işe yarar.
            // Varsayım 2: Sahne64 resource modelinde 'klasör' diye bir şey yok, sadece dosya gibi resource'lar var.
            // Varsayım 3: resource::control(parent_handle, MKDIR_COMMAND, ...) gibi bir mekanizma var.
            // Varsayım 4: Dosya yazarken parent resource'lar otomatik oluşturulur.

            // Eğer Varsayım 1 veya 3 geçerliyse, burada o çağrı yapılmalı.
            // Şimdilik Varsayım 4'ü temel alıp, dosya yazmadan önce parent'ı 'acquire' etmeyi deneyelim
            // (Bu da Varsayım 1'e benzer bir etki yaratır).

            // `sahne_create_resource_recursive` fonksiyonu artık burada tam yolu alıp parent'ları deneyecek
             if temizlenmis_dosya_adi.len() > 1 { // Sadece kök '/' değilse işlem yap
                 let dir_resource_id = &cikartma_resource_id; // Zaten '/' ile bitiyor (muhtemelen zip formatına göre)
                 // Sadece varlığını sağlamak için acquire ve hemen release.
                 // resource::MODE_CREATE yeterli olmalı, klasöre özel flag gerekmeyebilir.
                 match resource::acquire(dir_resource_id, resource::MODE_CREATE) {
                      Ok(dir_handle) => {
                          let _ = resource::release(dir_handle); // Handle'ı hemen bırak
                      }
                      Err(e) => {
                           eprintln!("Dizin Kaynağı oluşturma hatası ({}): {:?}", dir_resource_id, e);
                           let _ = resource::release(arsiv_handle); // Arşiv handle'ını temizle
                           return Err(PaketYoneticisiHata::from_sahne_error(e));
                      }
                 }
             }


        } else {
            // Dosya Kaynağı ise çıkar
            if let Some(ebeveyn_path) = cikartma_resource_id.rfind('/').map(|idx| &cikartma_resource_id[..idx]) {
                 if !ebeveyn_path.is_empty() {
                     // Ebeveyn dizin/kaynak yolunu sağlamaya çalış.
                     // Varsayım: resource::acquire(parent_path, MODE_CREATE) parent resource'u oluşturur.
                     match resource::acquire(ebeveyn_path, resource::MODE_CREATE) {
                          Ok(parent_handle) => {
                              let _ = resource::release(parent_handle); // Handle'ı hemen bırak
                          }
                          Err(e) => {
                               eprintln!("Ebeveyn Kaynağı oluşturma hatası ({}): {:?}", ebeveyn_path, e);
                               let _ = resource::release(arsiv_handle); // Arşiv handle'ını temizle
                               return Err(PaketYoneticisiHata::from_sahne_error(e));
                          }
                     }
                 }
            }


            // Dosya Kaynağını oluştur ve aç
            let cikartma_dosyasi_handle = resource::acquire(
                &cikartma_resource_id,
                resource::MODE_WRITE | resource::MODE_CREATE | resource::MODE_TRUNCATE // Yazma, Oluştur, Varsa içeriği sil
            ).map_err(|e| {
                 eprintln!("Çıkarma Dosya Kaynağı acquire hatası ({}): {:?}", cikartma_resource_id, e);
                 let _ = resource::release(arsiv_handle); // Arşiv handle'ını temizle
                 PaketYoneticisiHata::from_sahne_error(e)
            })?;

            // Arşiv dosyasının içeriğini tampona oku (bu da zip crate'inin Read implementasyonunu kullanır)
            let mut buffer = Vec::new(); // alloc::vec::Vec kullanılıyor
            // arsiv_dosyasi, zip::read::ZipFile olmalı ve Read implement etmeli.
            // Bu read çağrısı alttan SahneResourceReader'ı kullanmalı (varsayım).
            match arsiv_dosyasi.read_to_end(&mut buffer) {
                 Ok(_) => {
                      // Tampondaki veriyi Sahne64 Kaynağına yaz
                      match resource::write(cikartma_dosyasi_handle, &buffer) {
                           Ok(_) => {
                                // Yazma başarılı
                           }
                           Err(e) => {
                                eprintln!("Çıkarma Kaynağına yazma hatası ({}): {:?}", cikartma_resource_id, e);
                                let _ = resource::release(cikartma_dosyasi_handle); // Dosya handle'ını temizle
                                let _ = resource::release(arsiv_handle); // Arşiv handle'ını temizle
                                return Err(PaketYoneticisiHatasi::from_sahne_error(e));
                           }
                      }
                 }
                 Err(e) => {
                      eprintln!("Zip dosyasından okuma hatası ({}): {:?}", dosya_adi, e);
                      let _ = resource::release(cikartma_dosyasi_handle); // Dosya handle'ını temizle
                      let _ = resource::release(arsiv_handle); // Arşiv handle'ını temizle
                      // ZipError'ı PaketYoneticisiHata'ya map etmeliyiz.
                      return Err(PaketYoneticisiHatasi::ZipHatasi(e)); // ZipError::IoError SahneError'dan gelebilir
                 }
            }

            // Çıkarma Dosya Kaynağı Handle'ını serbest bırak
            match resource::release(cikartma_dosyasi_handle) {
                 Ok(_) => {}, // Başarılı
                 Err(e) => {
                      eprintln!("Çıkarma Dosya Kaynağı release hatası ({}): {:?}", cikartma_resource_id, e);
                      // Hata olsa bile devam etmeye çalışabiliriz, ama handle'ı bırakamamak sorun.
                      // Ciddi bir hata olarak dönebiliriz.
                      let _ = resource::release(arsiv_handle); // Arşiv handle'ını temizle
                      return Err(PaketYoneticisiHatasi::from_sahne_error(e));
                 }
            }
        }
    }

    // 4. ZIP Arşiv Handle'ını Serbest Bırak
    match resource::release(arsiv_handle) {
        Ok(_) => Ok(()), // Başarılı, tüm işlemler bitti
        Err(e) => {
            eprintln!("Arşiv Kaynağı release hatası ({}): {:?}", arsiv_resource_id, e);
            Err(PaketYoneticisiHatasi::from_sahne_error(e)) // Hata döndür
        }
    }
}


// Verilen ZIP arşivinin içeriğini listeleyen fonksiyon.
// arsiv_resource_id: Listelenecek ZIP arşivinin Sahne64 Kaynak ID'si.
pub fn zip_icerik_listele(arsiv_resource_id: &str) -> Result<Vec<String>, PaketYoneticisiHatasi> {
    // 1. ZIP Arşiv Kaynağını Aç
    let arsiv_handle = resource::acquire(arsiv_resource_id, resource::MODE_READ)
        .map_err(|e| PaketYoneticisiHatasi::from_sahne_error(e))?; // SahneError'ı kendi hatamıza çevir

    // 2. ZIP Arşivini Okumak için SahneResourceReader kullanma (Varsayımsal)
    let arsiv_reader = SahneResourceReader::new(arsiv_handle);
    let mut arsiv = ZipArchive::new(arsiv_reader).map_err(PaketYoneticisiHatasi::ZipHatasi)?;

    // 3. İçerikleri Listele
    let mut icerikler = Vec::new(); // alloc::vec::Vec kullanılıyor

    for i in 0..arsiv.len() {
        // zip::by_index metodu başarısız olursa ZipError döner, bu zaten ? ile PaketYoneticisiHatasi::ZipHatasi'na döner.
        let arsiv_dosyasi = arsiv.by_index(i)?;
        // name() metodu Result<&str, PathBuf> döner, PathBuf kısmı no_std'de sorun olabilir.
        // Zip formatında entry isimleri raw bytes'tır, UTF-8 olmayabilir.
        // zip crate'inin no_std modunda name() davranışı kontrol edilmeli.
        // Geçici olarak name()'in &str döndürdüğünü varsayalım veya to_string() hatasını yakalayalım.

        let entry_name = arsiv_dosyasi.name(); // Varsayım: &str dönüyor

        // Eğer zip crate'inin no_std'deki name() metodu PathBuf veya Result<_, PathBuf> dönüyorsa
        // PathBuf kısmı no_std'de çalışmaz. Bu durumda zip formatının raw bytes ismini alıp
        // kendimiz String'e çevirmemiz (UTF-8 varsayımıyla veya farklı bir kodlama ile) gerekir.
        // Ya da zip crate'inin no_std'ye özel name() metodunu kullanmalıyız.

        icerikler.push(entry_name.to_string()); // String'e çevirip listeye ekle
    }

    // 4. ZIP Arşiv Handle'ını Serbest Bırak
    match resource::release(arsiv_handle) {
        Ok(_) => Ok(icerikler), // Başarılı, listeyi döndür
        Err(e) => {
            eprintln!("Arşiv Kaynağı release hatası ({}): {:?}", arsiv_resource_id, e);
            Err(PaketYoneticisiHatasi::from_sahne_error(e)) // Hata döndür
        }
    }
}

// --- PaketYoneticisiHata enum'ının SahneError'ı içerecek şekilde güncellenmesi ---
// (Bu enum tanımı muhtemelen başka bir dosyadadır, ancak burada nasıl görüneceğine dair bir taslak)

// paket_yoneticisi_hata.rs (Örnek)
#[derive(Debug)]
pub enum PaketYoneticisiHatasi {
    ZipHatasi(zip::result::ZipError), // Zip crate hataları
    DosyaSistemiHatasi(SahneError), // Sahne64 kaynak/FS hataları için
    SahneApiHatasi(SahneError), // Genel Sahne64 API hataları için
    GecersizParametre(String), // Fonksiyona geçersiz parametre geçilmesi
    PathTraversalHatasi(String), // Güvenlik: Path traversal denemesi
    // ... diğer paket yöneticisi özel hataları ...
}

impl From<zip::result::ZipError> for PaketYoneticisiHatasi {
    fn from(err: zip::result::ZipError) -> Self {
        // ZipError içindeki IoError'ı da ayrıca map etmek gerekebilir
        match err {
            zip::result::ZipError::IoError(io_err) => {
                 // Burada std::io::Error'ı SahneError'a veya PaketYoneticisiHatasi'na maplemek lazım.
                 // std::io::Error'ın kendisi no_std'de yok.
                 // Bu durumda zip crate'inin no_std'de ne tür bir IO hatası döndürdüğünü anlamak gerek.
                 // Varsayım: zip crate'i no_std modunda ZipError::IoError içinde kendi no_std hata tipini veya () döndürür.
                 // Veya PaketYoneticisiHatasi::from_sahne_error'ı kullanır.
                 // En basit: ZipError'ı olduğu gibi sakla ve gerektiğinde detaylandır.
                 PaketYoneticisiHatasi::ZipHatasi(err)
            }
            _ => PaketYoneticisiHatasi::ZipHatasi(err),
        }
    }
}

// SahneError'dan PaketYoneticisiHatasi'na dönüşüm
impl PaketYoneticisiHatasi {
    pub fn from_sahne_error(err: SahneError) -> Self {
        match err {
            // SahneError'daki spesifik hataları daha özel paket yöneticisi hatalarına mapleyebiliriz
            SahneError::ResourceNotFound => PaketYoneticisiHatasi::DosyaSistemiHatasi(err), // Veya PaketNotFound?
            SahneError::PermissionDenied => PaketYoneticisiHatasi::DosyaSistemiHatasi(err), // Yetki Hatası
            SahneError::InvalidHandle => PaketYoneticisiHatasi::SahneApiHatasi(err), // Geçersiz Handle
            // ... diğer SahneError varyantları ...
            _ => PaketYoneticisiHatasi::SahneApiHatasi(err), // Kalanları genel API hatası
        }
    }
}

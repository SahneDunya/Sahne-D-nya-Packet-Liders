#![no_std]
extern crate alloc; // String, Vec, format! için

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use alloc::borrow::ToOwned; // &str'dan String'e çevirmek için

// Sahne64 kütüphanemizi içeri aktaralım
// Güncel API tanımımıza göre modülleri ve tipleri güncelleyelim
use crate::resource;
use crate::task; // process yerine
use crate::memory; // Zaten global allocator aracılığıyla kullanılıyor olabilir
use crate::kernel; // CLI doğrudan kernel bilgisi almayabilir, pkg_manager alabilir
use crate::messaging; // ipc yerine
use crate::{SahneError, Handle, TaskId};

// Paket yönetimi ile ilgili fonksiyonlarımızı içeren modül
// Bu fonksiyonlar artık no_std ortamında ve Sahne64 API'sını kullanarak çalışacak
mod pkg_manager {
    use super::*; // Üst modüldeki öğelere erişim (resource, task, SahneError vb.)
    use crate::paket_yoneticisi_hata::PaketYoneticisiHatasi; // Özel hata enum'ımız
    // SahneError'dan PaketYoneticisiHatasi'na dönüşüm From implementasyonu ile sağlanacak

    // Kurulu paket listesinin saklandığı Kaynak ID'si (varsayımsal)
    const INSTALLED_PACKAGES_LIST_RESOURCE: &str = "sahne://config/installed_packages.list";
    // Kurulum/kaldırma scriptlerinin bulunabileceği temel Kaynak Yolu (varsayımsal)
    const PACKAGE_SCRIPT_BASE_RESOURCE: &str = "sahne://system/package_scripts/";
    // Kurulu paketlerin dosyalarının saklandığı temel Kaynak Yolu (varsayımsal)
     const INSTALLED_FILES_BASE_RESOURCE: &str = "sahne://installed_packages/"; // Önceki srccache/srcarchive'dan hatırlatma

    // Kurulu paketleri listeler.
    pub fn list_packages() -> Result<(), PaketYoneticisiHatasi> { // Result türü PaketYoneticisiHatasi olmalı
        println!("Kurulu paketler listeleniyor...");

        // Kurulu paketler listesini içeren Kaynağı oku
        match resource::acquire(INSTALLED_PACKAGES_LIST_RESOURCE, resource::MODE_READ) {
            Ok(handle) => {
                let mut buffer = Vec::new(); // alloc::vec::Vec kullanılıyor
                let mut temp_buffer = [0u8; 512]; // Okuma tamponu (stack'te)

                loop {
                    // Kaynaktan parça parça oku
                    match resource::read(handle, &mut temp_buffer) {
                        Ok(0) => break, // Kaynak sonu
                        Ok(bytes_read) => {
                            buffer.extend_from_slice(&temp_buffer[..bytes_read]);
                        }
                        Err(e) => {
                            // Okuma hatası durumunda handle'ı serbest bırakıp hata dön
                            let _ = resource::release(handle);
                            eprintln!("Paket listesi okuma hatası (Kaynak: {}): {:?}", INSTALLED_PACKAGES_LIST_RESOURCE, e);
                            return Err(PaketYoneticisiHatasi::from(e)); // SahneError'ı PaketYoneticisiHatasi'na çevir
                        }
                    }
                }

                // Handle'ı serbest bırak
                let release_result = resource::release(handle);
                 if let Err(e) = release_result {
                      eprintln!("Paket listesi Kaynağı release hatası ({}): {:?}", INSTALLED_PACKAGES_LIST_RESOURCE, e);
                      // Release hatası kritik olmayabilir, loglayıp devam edebiliriz.
                 }


                // Okunan içeriği String'e çevir ve yazdır
                // Kaynak içeriğinin UTF-8 olduğunu varsayalım.
                if let Ok(contents) = core::str::from_utf8(&buffer) {
                    // Paket listesini satır satır işlemek isteyebiliriz.
                    // Basitlik adına tüm içeriği yazdıralım.
                    println!("{}", contents);
                } else {
                    eprintln!("Paket listesi içeriği geçersiz UTF-8.");
                    // UTF-8 hatası için özel bir PaketYoneticisiHatasi varyantı eklenebilir.
                    return Err(PaketYoneticisiHatasi::GecersizParametre(String::from("Paket listesi içeriği UTF-8 değil")));
                }

                Ok(())
            }
            Err(SahneError::ResourceNotFound) => {
                // Kaynak bulunamadıysa, henüz kurulu paket yok demektir.
                println!("Henüz kurulu paket yok.");
                Ok(())
            }
            Err(e) => {
                // Diğer Sahne64 hataları
                eprintln!("Paket listesi Kaynağı acquire hatası ({}): {:?}", INSTALLED_PACKAGES_LIST_RESOURCE, e);
                Err(PaketYoneticisiHatasi::from(e)) // SahneError'ı PaketYoneticisiHatasi'na çevir
            }
        }
    }

    // Yeni bir paket ekler (Kurulumun bir parçası olarak düşünülebilir).
    // Gerçek ekleme mantığı (dosyaları kopyalama, veritabanını güncelleme) burada veya başka bir modülde olur.
    // Burada sadece örnekteki gibi bir "kurulum scripti" çalıştırmayı taklit edelim.
    // package_name: Eklenecek paketin adı.
    pub fn add_package(package_name: &str) -> Result<(), PaketYoneticisiHatasi> { // Result türü PaketYoneticisiHatasi olmalı
        println!("{} paketi ekleniyor...", package_name);

        // Kurulum scripti Kaynak ID'sini oluştur (varsayımsal)
        let install_script_resource_id = format!("{}{}/install.sh", PACKAGE_SCRIPT_BASE_RESOURCE, package_name);

        // Kurulum scripti Kaynağına erişim Handle'ı edin (çalıştırma izniyle?)
        // Sahne64 API'sında 'çalıştırma izni' Mode flag'i olmayabilir.
        // Task::spawn bir kod Kaynağı Handle'ı bekler. Bu Kaynak çalıştırılabilir kodu içermelidir.
        // Varsayım: install.sh Kaynağı, task::spawn'ın anlayacağı bir formatta (örneğin, bir ara dil veya native kod).
        // Veya paket yöneticisi, scripti okuyup kendisi yorumlar/çalıştırır (daha karmaşık).
        // Basitlik adına, install.sh Kaynağının task::spawn tarafından doğrudan çalıştırılabildiğini varsayalım.

        match resource::acquire(&install_script_resource_id, resource::MODE_READ) { // MODE_READ yeterli mi? Çalıştırma için özel flag mi lazım?
             Ok(script_handle) => {
                 println!("Kurulum scripti Kaynağı edinildi: {}", install_script_resource_id);

                 // Yeni bir görev (task) olarak scripti başlat
                 // task::spawn function signature: spawn(code_handle: Handle, args: &[u8])
                 let task_args: &[u8] = b""; // Kurulum scriptine argüman geçilebilir. Şimdilik boş.

                 match task::spawn(script_handle, task_args) {
                      Ok(new_tid) => {
                          println!("Paket kurulum görevi başlatıldı, TaskId: {:?}", new_tid);
                          // Script Handle'ı görev başlatıldıktan sonra serbest bırakılabilir mi?
                          // Çekirdeğin code_handle'ın bir kopyasını alıp almadığına bağlı.
                          // Güvenli olması için görev bitene kadar Handle'ı açık tutmak gerekebilir.
                          // Veya kernel handle'ı consume ediyordur. Varsayım: Kernel kopyalar, user space bırakabilir.
                          let _ = resource::release(script_handle); // Handle'ı serbest bırak

                          // Kurulum görevinin tamamlanmasını beklemek isteyebiliriz (process::wait benzeri).
                          // Sahne64 API'sında task::wait syscall'u tanımlı değildi.
                          // Bu bir eksiklik. Paket yöneticisi bu görevin bitmesini nasıl bekleyecek?
                          // Messaging ile görev bitiş mesajı gönderebilir mi?
                          // Şimdilik beklemeyelim.

                          Ok(())
                      }
                      Err(e) => {
                          let _ = resource::release(script_handle); // Hata durumunda handle'ı temizle
                          eprintln!("Paket kurulum görevi başlatılamadı (Kaynak: {}): {:?}", install_script_resource_id, e);
                          Err(PaketYoneticisiHatasi::from(e)) // SahneError'ı PaketYoneticisiHatasi'na çevir
                      }
                 }
             }
             Err(e) => {
                 eprintln!("Kurulum scripti Kaynağı acquire hatası ({}): {:?}", install_script_resource_id, e);
                 Err(PaketYoneticisiHatasi::from(e)) // SahneError'ı PaketYoneticisiHatasi'na çevir
             }
        }
    }

    // Bir paketi kaldırır.
    // package_name: Kaldırılacak paketin adı.
    pub fn remove_package(package_name: &str) -> Result<(), PaketYoneticisiHatasi> { // Result türü PaketYoneticisiHatasi olmalı
        println!("{} paketi kaldırılıyor...", package_name);

        // Kaldırma scripti Kaynak ID'sini oluştur (varsayımsal)
        let uninstall_script_resource_id = format!("{}{}/uninstall.sh", PACKAGE_SCRIPT_BASE_RESOURCE, package_name);

        // Kaldırma scripti Kaynağına erişim Handle'ı edin
        match resource::acquire(&uninstall_script_resource_id, resource::MODE_READ) { // Yine çalıştırma için Handle edinme
             Ok(script_handle) => {
                 println!("Kaldırma scripti Kaynağı edinildi: {}", uninstall_script_resource_id);

                 // Yeni bir görev (task) olarak scripti başlat
                 let task_args: &[u8] = b""; // Kaldırma scriptine argüman geçilebilir.

                 match task::spawn(script_handle, task_args) {
                      Ok(new_tid) => {
                          println!("Paket kaldırma görevi başlatıldı, TaskId: {:?}", new_tid);
                           let _ = resource::release(script_handle); // Handle'ı serbest bırak
                          // Görevin bitmesini beklemek gerekebilir (task::wait eksikliği).
                          Ok(())
                      }
                      Err(e) => {
                          let _ = resource::release(script_handle); // Hata durumunda handle'ı temizle
                          eprintln!("Paket kaldırma görevi başlatılamadı (Kaynak: {}): {:?}", uninstall_script_resource_id, e);
                          Err(PaketYoneticisiHatasi::from(e)) // SahneError'ı PaketYoneticisiHatasi'na çevir
                      }
                 }
             }
             Err(SahneError::ResourceNotFound) => {
                  // Kaldırma scripti yoksa paket belki de düzgün kurulmamıştır veya script gerektirmiyordur.
                  // Veya bu bir hata olabilir. Şimdilik loglayıp devam edelim veya özel hata dönelim.
                  eprintln!("Kaldırma scripti bulunamadı ({}). Paket dosyaları manuel silinmeli?", uninstall_script_resource_id);
                  // Paketin dosyalarının ve veritabanı kaydının silinmesi burada ele alınmalı.
                  // Bu, resource::delete veya resource::control kullanımı gerektirir (eksik fonksiyonellik).
                  // Varsayım: Paket yöneticisi, kurulu dosyaların listesini tutar ve silme işlemi bu listeye göre yapılır.
                  // Bu da resource::delete'in varlığını gerektirir.
                  // Şimdilik sadece script çalıştırmayı taklit ettiğimiz için, script yoksa hata dönmek yerine uyarı verelim.
                  Ok(()) // Script yoksa hata değil, uyarı verip devam et
             }
             Err(e) => {
                 eprintln!("Kaldırma scripti Kaynağı acquire hatası ({}): {:?}", uninstall_script_resource_id, e);
                 Err(PaketYoneticisiHatasi::from(e)) // SahneError'ı PaketYoneticisiHatasi'na çevir
             }
        }
    }

    // Bir paketi arar.
    // package_name: Aranacak paketin adı.
    pub fn search_package(package_name: &str) -> Result<(), PaketYoneticisiHatasi> { // Result türü PaketYoneticisiHatasi olmalı
        println!("{} paketi aranıyor...", package_name);
        // Gerçek arama, bir uzak depodan (ağ kaynağı üzerinden) veya yerel bir index kaynağından yapılır.
        // Bu, messaging veya resource modüllerini (network resource gibi) kullanmayı gerektirir.
        // Şimdilik statik çıktı mantığını koruyalım.

        println!("'{}' için sonuçlar (şimdilik statik):", package_name);
        println!(" - {} (açıklama)", package_name);
        Ok(())
    }

    // Bir paketi kurar.
    // package_name: Kurulacak paketin adı.
    // Kurulum genellikle: İndir -> Sağlamasını Kontrol Et -> Çıkar -> Ekle (script çalıştır + DB kaydı) adımlarını içerir.
    pub fn install_package(package_name: &str) -> Result<(), PaketYoneticisiHatasi> { // Result türü PaketYoneticisiHatasi olmalı
        println!("{} paketi kuruluyor...", package_name);
        // Burada indirme (ağ kaynağı resource'u?), checksum doğrulama (srcchecksum),
        // arşivden çıkarma (srcarchive) adımları çağrılmalı.
        // Ardından add_package fonksiyonu (veya onun içeriği) çalıştırılmalı.

        // Örnek Akış (Placeholder):
        // 1. Paketi indir (örn. "sahne://remoterepo/packages/my_package.zip" Kaynağından "sahne://cache/packages/my_package.zip" Kaynağına)
            let download_source = format!("sahne://remoterepo/packages/{}.zip", package_name);
            let download_dest = format!("sahne://cache/packages/{}.zip", package_name);
        //    // İndirme fonksiyonu burada çağrılır (ağ kaynakları için özel resource type?).
        //    // İndirme işlemi resource::read/write ile olabilir, veya özel bir download syscall'u?
        //    // Geçici olarak indirme başarılı oldu varsayalım.
            println!("{} paketi indirildi (varsayımsal olarak).", package_name);


        // 2. İndirilen dosyanın checksum'ını doğrula
            let expected_md5 = "..."; // Paket meta verisinden alınmalı
            match crate::srcchecksum::dogrula_md5(&download_dest, expected_md5) {
                 Ok(true) => println!("Checksum doğrulama başarılı."),
                 Ok(false) => {
                     eprintln!("Checksum doğrulama BAŞARISIZ.");
                     // İndirilen dosyayı temizle?
                      resource::delete(&download_dest); // Eksik fonksiyonellik
                     return Err(PaketYoneticisiHatasi::PaketKurulumHatasi(String::from("Checksum doğrulaması başarısız")));
                 }
                 Err(e) => {
                      eprintln!("Checksum hesaplama/doğrulama hatası: {:?}", e);
                      return Err(e); // Hata zaten doğru türde
                 }
            }

        // 3. Paketi önbellekten/indirilen yerden kurulu alana çıkar (srcarchive)
            let archive_resource_id = download_dest;
            let extract_dest_resource_id = format!("sahne://installed_packages/{}/", package_name);
            match crate::srcarchive::zip_ac(&archive_resource_id, &extract_dest_resource_id) {
                 Ok(_) => println!("Paket dosyaları çıkarıldı."),
                 Err(e) => {
                     eprintln!("Paket dosyaları çıkarma hatası: {:?}", e);
        //             // Çıkarılan dosyaları temizle?
                      resource::delete(&extract_dest_resource_id); // Eksik fonksiyonellik
                     return Err(e); // Hata zaten doğru türde
                 }
            }

        // 4. Paketi sisteme ekle (script çalıştır, veritabanı kaydı oluştur, vb.)
        //    Bu genellikle add_package fonksiyonunun içeriği olur.
            add_package(package_name)?; // Eğer add_package sadece script çalıştırıyorsa

        // Eğer kurulum süreci add_package'in ötesinde adımlar içeriyorsa, onlar burada olur.
        // Örn: Veritabanına paket bilgilerini kaydetme.
        // Bu, kurulu paketler listesi Kaynağına (INSTALLED_PACKAGES_LIST_RESOURCE) yazmayı gerektirir.
        // Bu kaynağın LOCK edilmesi, içeriğinin okunup güncellenmesi ve tekrar yazılması lazım.
        // Bu işlemler için sync::lock_* ve resource::* kullanılır.

        // Şimdilik sadece add_package çağrısı yapalım (örnekteki gibi script çalıştırmayı taklit eden).
        add_package(package_name)?; // Script çalıştırma adımı

        println!("{} paketi başarıyla kuruldu (varsayımsal).", package_name);

        Ok(())
    }
}


// Sahne64 için basit bir no_std komut satırı argümanı ayrıştırıcı.
// Argümanların çekirdek tarafından bir *const *const u8 veya benzeri bir yapıda
// main fonksiyonuna iletildiği varsayılır.
// Bu örnekte, argümanları elle tanımlanmış bir &[&str] dilimi olarak ele alıyoruz.
// Gerçek Sahne64 ortamında argümanlar farklı şekilde sağlanabilir.
struct Arguments<'a> {
    args: &'a [&'a str],
}

impl<'a> Arguments<'a> {
    // Çekirdekten alınan argümanları işlemek için kurucu
    // Gerçek implementasyon çekirdeğin argümanları nasıl verdiğine bağlıdır.
    // fn from_raw_args(argc: usize, argv: *const *const u8) -> Self { ... }
    // Şimdilik basit bir dilimden oluşturalım.
    fn new(args: &'a [&'a str]) -> Self {
        Arguments { args }
    }

    // Program adını (ilk argüman) atlayarak komutları ve argümanları döndürür.
    fn iter(&self) -> impl Iterator<Item = &'a str> {
        self.args.iter().copied().skip(1) // İlk argümanı (program adı) atla
    }
}

// main fonksiyonu (no_std ortamı için)
// Sahne64 çekirdeği bu fonksiyona argümanları iletecektir.
// Şimdilik argümanları sabit bir dilim olarak tanımlayalım.
#[no_mangle] // Çekirdek tarafından çağrılabilmesi için ismin bozulmasını engelle
pub extern "C" fn main(argc: usize, argv: *const *const u8) -> i32 { // C ABI'si ile çekirdekten çağrılacak
    // argc: argüman sayısı
    // argv: C string'lerinin işaretçilerinin dizisi (**char)

    // unsafe blok içinde raw işaretçileri güvenli dilimlere çevir
    let args_slice = unsafe {
        // argv null olmamalı ve ilk argc elemanı geçerli *const u8 olmalı
        if argv.is_null() {
            // Argüman yoksa veya geçersizse boş bir dilim kullan
            &[]
        } else {
            core::slice::from_raw_parts(argv, argc)
        }
    };

    // Her bir *const u8'i &str'ye çevir (UTF-8 varsayımıyla)
    // Geçersiz UTF-8 olursa bu kısım hata verebilir.
    // Güvenlik için daha dikkatli bir çevrim yapılmalı.
    let args: Vec<&str> = args_slice.iter().filter_map(|&ptr| {
         if ptr.is_null() { return None; }
         // C string null-terminated olmalıdır.
         let c_str = unsafe { core::ffi::CStr::from_ptr(ptr as *const core::ffi::c_char) };
         c_str.to_str().ok() // UTF-8'e çevir, hata olursa None dön
    }).collect();


    let arguments = Arguments::new(&args);
    let mut arg_iter = arguments.iter();

    // Basit argüman ayrıştırma (clap yerine manuel yaklaşım)
    let command = arg_iter.next(); // İlk argüman komut olmalı (listele, kur, kaldir vb.)

    let result = match command {
        Some("listele") => {
            // listele komutu argüman almaz (şimdilik)
            if arg_iter.next().is_none() {
                pkg_manager::list_packages()
            } else {
                 eprintln!("'listele' komutu fazladan argüman alamaz.");
                 Err(PaketYoneticisiHatasi::GecersizParametre(String::from("fazladan argüman")))
            }
        }
        Some("ekle") => {
            // ekle komutu 1 argüman alır (paket adı)
            if let Some(package_name) = arg_iter.next() {
                if arg_iter.next().is_none() {
                    pkg_manager::add_package(package_name)
                } else {
                    eprintln!("'ekle' komutu fazladan argüman alamaz.");
                    Err(PaketYoneticisiHatasi::GecersizParametre(String::from("fazladan argüman")))
                }
            } else {
                eprintln!("'ekle' komutu paket adı gerektirir.");
                Err(PaketYoneticisiHatasi::GecersizParametre(String::from("paket adı eksik")))
            }
        }
        Some("kaldir") => {
            // kaldir komutu 1 argüman alır (paket adı)
             if let Some(package_name) = arg_iter.next() {
                if arg_iter.next().is_none() {
                    pkg_manager::remove_package(package_name)
                } else {
                    eprintln!("'kaldir' komutu fazladan argüman alamaz.");
                    Err(PaketYoneticisiHatasi::GecersizParametre(String::from("fazladan argüman")))
                }
            } else {
                eprintln!("'kaldir' komutu paket adı gerektirir.");
                Err(PaketYoneticisiHatasi::GecersizParametre(String::from("paket adı eksik")))
            }
        }
         Some("ara") => {
            // ara komutu 1 argüman alır (paket adı)
             if let Some(package_name) = arg_iter.next() {
                if arg_iter.next().is_none() {
                    pkg_manager::search_package(package_name)
                } else {
                    eprintln!("'ara' komutu fazladan argüman alamaz.");
                    Err(PaketYoneticisiHatasi::GecersizParametre(String::from("fazladan argüman")))
                }
            } else {
                eprintln!("'ara' komutu paket adı gerektirir.");
                Err(PaketYoneticisiHatasi::GecersizParametre(String::from("paket adı eksik")))
            }
        }
         Some("kur") => {
            // kur komutu 1 argüman alır (paket adı)
             if let Some(package_name) = arg_iter.next() {
                if arg_iter.next().is_none() {
                    pkg_manager::install_package(package_name)
                } else {
                    eprintln!("'kur' komutu fazladan argüman alamaz.");
                    Err(PaketYoneticisiHatasi::GecersizParametre(String::from("fazladan argüman")))
                }
            } else {
                eprintln!("'kur' komutu paket adı gerektirir.");
                Err(PaketYoneticisiHatasi::GecersizParametre(String::from("paket adı eksik")))
            }
        }
        Some(cmd) => {
            eprintln!("Bilinmeyen komut: '{}'. Bilinen komutlar: listele, ekle, kaldir, ara, kur", cmd);
             Err(PaketYoneticisiHatasi::GecersizParametre(format!("bilinmeyen komut: {}", cmd)))
        }
        None => {
            // Hiç argüman yoksa (sadece program adı) kullanım bilgisini göster
            println!("Paket Yöneticisi (Sahne64)");
            println!("Kullanım: paket_yoneticisi <komut> [argümanlar]");
            println!("Komutlar: listele, ekle, kaldir, ara, kur");
             Ok(()) // Bilgi mesajı başarı sayılır
        }
    };

    // İşlem sonucuna göre çıkış kodu döndür
    match result {
        Ok(_) => 0, // Başarı
        Err(e) => {
             eprintln!("Hata: {:?}", e); // Hata mesajını yazdır
             -1 // Hata kodu
        }
    }
}

// Standart kütüphane yoksa panic handler (önceki koddan alınmıştır)
#[cfg(not(test))] // Testler std kullanabilir
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // Gerçek bir sistemde burada hata mesajı bir porta yazılır,
    // sistem yeniden başlatılır veya sadece sonsuz döngüye girilir.
     print!("PANIC: {}", info); // print makrosu kullanılabilir (no_std implementasyonu varsa)
    loop {
        core::hint::spin_loop(); // İşlemciyi meşgul etmeden bekle
    }
}

// no_std ortamı için print!/println! makroları (önceki koddan alınmıştır)
// Bu makrolar gerçek çıktı mekanizmasına (örn. resource::write) bağlanmalıdır.
#[macro_use] // Makroları crate köküne taşıyarak her yerden erişilmesini sağlar
mod print_macros {
    use core::fmt;
    use core::fmt::Write;

    // TODO: Gerçek çıktı için Sahne64 Kaynağı Handle'ını kullanın.
    // Bu struct Sahne64'ün standart çıktı kaynağını temsil etmeli.
    struct StdoutResourceWriter;

    impl fmt::Write for StdoutResourceWriter {
        fn write_str(&mut self, s: &str) -> fmt::Result {
             // Burada gerçek Sahne64 Kaynak API'sı çağrılmalıdır.
             // Varsayım: Stdout Kaynağına Handle başlangıçta göreve verilir.
             // Bu Handle'a global/static bir yerden erişilmeli veya her print çağrısı için alınmalı?
             // Global static kullanmak no_std + thread safety için zor olabilir.
             // Basitlik adına, şimdilik çıktıyı çekirdeğe ileten bir syscall (varsa)
             // veya sabit bir debug portuna yazma gibi bir işlem olabilir.
             // Aşağıdaki satır sadece placeholder'dır.
              let stdout_handle = get_stdout_handle(); // Varsayımsal fonksiyon
              let _ = crate::resource::write(stdout_handle, s.as_bytes()); // Veya özel bir çıktı syscall'u
              Ok(()) // Yazma başarısız olsa bile fmt::Result::Ok() dönebiliriz basit örnekte
             let _ = s; // unused warning'i önlemek için
             Ok(()) // Gerçekte yazma yapılmıyor
        }
    }

     #[macro_export]
     macro_rules! print {
         ($($arg:tt)*) => ({
             let mut writer = $crate::print_macros::StdoutResourceWriter; // struct adı değişti
             let _ = core::fmt::write(&mut writer, core::format_args!($($arg)*));
         });
     }

     #[macro_export]
     macro_rules! println {
         () => ($crate::print!("\n"));
         ($($arg:tt)*) => ($crate::print!("{}\n", core::format_args!($($arg)*)));
     }

     #[macro_export]
     macro_rules! eprintln {
         () => ($crate::print!("\n")); // Şimdilik stderr yok, stdout'a yaz
         ($($arg:tt)*) => ($crate::print!("{}\n", core::format_args!($($arg)*)));
     }
}

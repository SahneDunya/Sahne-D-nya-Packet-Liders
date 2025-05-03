#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // String, Vec, Box için

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::boxed::Box; // Eğer Box kullanılıyorsa

// Sahne64 API modüllerini içe aktarın
use crate::resource;
use crate::task; // thread yerine task veya create_thread
use crate::messaging; // mpsc yerine
use crate::SahneError;
use crate::Handle; // Kaynak Handle'ları
use crate::TaskId; // Görev (Task) ID'leri

// Özel hata enum'ımızı içe aktar (no_std uyumlu hali)
use crate::paket_yoneticisi_hata::PaketYoneticisiHatasi;
// SahneError'dan dönüşüm From implementasyonu ile sağlanacak
// use crate::paket_yoneticisi_hata::from_sahne_error;

// no_std uyumlu print makroları
use crate::print_macros::{println, eprintln};

// GUI (Frontend) ile iletişim için kullanılacak mesaj formatları (örnek)
// Bunlar bincode/postcard gibi no_std uyumlu bir kütüphane ile serileştirilebilir.
// Basitlik adına şimdilik String veya Vec<String> mesajları varsayalım.

// İstek mesajları (GUI -> Backend)
#[derive(Debug, PartialEq)]
pub enum GuiRequest {
    RefreshPackageList,
    InstallPackage(String),
    RemovePackage(String),
    SearchPackage(String),
    // ... diğer GUI istekleri
}

// Güncelleme/Yanıt mesajları (Backend -> GUI)
#[derive(Debug, PartialEq)]
pub enum GuiUpdate {
    PackageList(Vec<String>), // Paket listesi güncellemesi
    InstallationStatus(String), // Kurulum durumu mesajı
    RemovalStatus(String), // Kaldırma durumu mesajı
    SearchResult(Vec<String>), // Arama sonuçları
    Error(PaketYoneticisiHatasi), // Backend'den hata bilgisi
    // ... diğer GUI yanıtları
}


// Paket listesini Sahne64 Kaynakları kullanarak fetch etme fonksiyonu.
// srccli.rs'deki fetch_package_list_sahne64 fonksiyonunun refaktoringi.
// package_list_resource_id: Paket listesini içeren Kaynağın ID'si.
fn fetch_package_list_sahne64(package_list_resource_id: &str) -> Result<Vec<String>, PaketYoneticisiHatasi> {
    // Kaynağı oku (sadece okuma izniyle)
    let handle = resource::acquire(package_list_resource_id, resource::MODE_READ)
        .map_err(|e| {
             eprintln!("Paket listesi Kaynağı acquire hatası ({}): {:?}", package_list_resource_id, e);
             PaketYoneticisiHatasi::from(e) // SahneError -> PaketYoneticisiHatasi
        })?;

    let mut buffer = Vec::new(); // Kaynak içeriğini tutacak tampon (alloc::vec::Vec)
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
                 eprintln!("Paket listesi Kaynağı okuma hatası ({}): {:?}", package_list_resource_id, e);
                return Err(PaketYoneticisiHatasi::from(e)); // SahneError -> PaketYoneticisiHatasi
            }
        }
    }

    // Handle'ı serbest bırak
    let release_result = resource::release(handle);
     if let Err(e) = release_result {
          eprintln!("Paket listesi Kaynağı release hatası ({}): {:?}", package_list_resource_id, e);
          // Release hatası kritik olmayabilir, loglayıp devam edebiliriz.
     }

    // Tampondaki binary veriyi String'e çevir (UTF-8 varsayımıyla)
    // core::str::from_utf8 Result<&str, Utf8Error> döner.
    core::str::from_utf8(&buffer)
        .map_err(|_| {
            eprintln!("Paket listesi içeriği geçerli UTF-8 değil ({})", package_list_resource_id);
            // UTF-8 hatası için PaketYoneticisiHatasi::ParsingError kullanılabilir.
            PaketYoneticisiHatasi::ParsingError(format!("Geçersiz UTF-8 Kaynak içeriği: {}", package_list_resource_id)) // String kullanmak yerine hataya detay eklenebilir
        })
        .map(|content| {
            // String içeriği satır satır ayır ve paket isimlerini topla
             content.lines()
                   .map(|line| line.trim().to_string()) // Her satırı String yap (alloc)
                   .filter(|line| !line.is_empty()) // Boş satırları atla
                   .collect::<Vec<String>>() // Vec<String> olarak topla (alloc)
        })
}

// GUI Backend görevi/iş parçacığı için ana döngü.
// GUI'den gelen istekleri dinler ve yanıtları geri gönderir.
// request_rx_handle: GUI'den istek almak için mesajlaşma Handle'ı.
// update_tx_task_id: GUI'ye güncelleme göndermek için hedef görev (Task) ID'si.
// package_list_resource_id: Paket listesi kaynağının ID'si (fetch_package_list_sahne64 için).
pub fn gui_backend_loop(
    request_rx_handle: Handle,
    update_tx_task_id: TaskId,
    package_list_resource_id: &str, // Yapılandırmadan gelecek
    // Diğer gerekli bağımlılıklar (örn. Package Database Handle, Config, FeatureFlags vb.)
) {
    println!("GUI Backend döngüsü başlatıldı.");

    // İletişim için tampon
    let mut message_buffer = [0u8; 1024]; // Mesajları almak için buffer

    loop {
        // GUI'den gelen mesajı al (bloklayıcı)
        // messaging::receive Handle almaz, Task ID'ye mesaj gelir.
        // Eğer mesajlaşma handle üzerinden yapılıyorsa (messaging::receive_from gibi bir syscall varsa)
        // request_rx_handle kullanılır. API'da TaskId üzerinden messaging vardı.
        // Varsayım: GUI'den gelen mesajlar doğrudan bu backend görevinin Task ID'sine gelir.
        // Veya GUI ve Backend aynı görev içinde farklı thread'lerde çalışıyorsa, mesajlaşma thread'ler arası bir Queue ile yapılır (std::sync::mpsc yerine no_std Queue).

        // Sahne64 API'sına göre Task ID üzerinden receive:
        let recv_result = messaging::receive(&mut message_buffer); // Kendi Task ID'mize gelen mesajı al

        match recv_result {
            Ok(bytes_received) => {
                if bytes_received == 0 {
                    // Bağlantı kapandı veya boş mesaj?
                    println!("GUI bağlantısı kapatıldı?");
                    break; // Döngüyü sonlandır
                }

                // Gelen bayt dizisini GuiRequest enum'ına deserialize et (örneğin postcard ile)
                // Varsayım: Gelen mesaj GuiRequest'in postcard serileştirilmiş hali.
                match postcard::from_bytes_copy::<GuiRequest>(&message_buffer[..bytes_received]) {
                     Ok(request) => {
                         println!("GUI'den istek alındı: {:?}", request);

                         let mut response: GuiUpdate; // Yanıt mesajı

                         // İsteğe göre işlem yap
                         match request {
                             GuiRequest::RefreshPackageList => {
                                 match fetch_package_list_sahne64(package_list_resource_id) {
                                     Ok(package_list) => {
                                         response = GuiUpdate::PackageList(package_list);
                                     }
                                     Err(e) => {
                                         response = GuiUpdate::Error(e);
                                     }
                                 }
                             }
                             GuiRequest::InstallPackage(package_name) => {
                                 // Kurulum mantığını çağır (başka modülden)
                                  let install_result = crate::pkg_manager::install_package(&package_name);
                                  response = match install_result { Ok(_) => GuiUpdate::InstallationStatus(format!("{} başarıyla kuruldu.", package_name)), Err(e) => GuiUpdate::Error(e) };
                                 // Placeholder:
                                 println!("Paket kurma isteği: {}", package_name);
                                 response = GuiUpdate::InstallationStatus(format!("{} kurulumu başlatıldı...", package_name));
                             }
                             // ... diğer istekleri işle (Remove, Search vb.)
                             _ => {
                                  eprintln!("Bilinmeyen GUI isteği: {:?}", request);
                                  response = GuiUpdate::Error(PaketYoneticisiHatasi::InvalidParameter(format!("Bilinmeyen GUI isteği")));
                             }
                         }

                         // Yanıt mesajını serialize et (örneğin postcard ile)
                         match postcard::to_postcard(&response) {
                             Ok(response_data) => {
                                 // Yanıtı GUI görevine gönder
                                 // messaging::send(target_task: TaskId, message: &[u8])
                                 match messaging::send(update_tx_task_id, &response_data) { // response_data Vec<u8> ise & ile dilime çevrilir
                                     Ok(_) => {
                                         //println!("Yanıt GUI'ye gönderildi.");
                                     }
                                     Err(e) => {
                                         eprintln!("Yanıt GUI görevine gönderilemedi: {:?}", e);
                                         // Hata durumunda ne yapılmalı? Logla, veya GUI'ye hata göndermeye çalış (bu da başarısız olabilir).
                                     }
                                 }
                             }
                             Err(e) => {
                                 eprintln!("Yanıt mesajı serileştirilemedi: {:?}", e);
                                 // Serileştirme hatasını GUI'ye bildirmek zor olabilir. Loglamak yeterli olabilir.
                             }
                         }

                     }
                     Err(e) => {
                          eprintln!("Gelen mesaj deserialize edilemedi: {:?}", e);
                          // Deserialize hatasını GUI'ye bildirmek zor olabilir. Loglamak yeterli olabilir.
                          // Belki format hatası olduğunu belirten özel bir hata mesajı gönderilebilir (eğer iletişim hala açıksa).
                     }
                }
            }
            Err(e) => {
                eprintln!("Mesaj alma hatası (GUI Backend): {:?}", e);
                // Hata durumunda döngüyü sonlandırabilir veya kurtarma mekanizması çalıştırabilir.
                // Örneğin, GUI görevi sonlandıysa COMM_ERROR gelebilir.
                break; // Hata durumunda döngüden çık
            }
        }
    }

    println!("GUI Backend döngüsü sonlandı.");
    // Görev/thread sonlandırılır (task::exit veya exit_thread)
    task::exit(0); // Normal çıkış
}

// GUI (Frontend) görev/iş parçacığı için mesaj alma döngüsü (örnek).
// GUI uygulamasının ana döngüsünde (plinkleme/renderlama) bu döngü çalıştırılmalı.
// update_rx_handle: Backend'den güncelleme almak için mesajlaşma Handle'ı.
// request_tx_task_id: Backend'e istek göndermek için hedef görev (Task) ID'si.
// GUI_app_struct: GUI'nin kendi state'i ve renderlama logic'ini tutan yapı.
// Bu fonksiyon, GUI framework'ünün (eframe/egui'nin Sahne64 uyumlu versiyonu varsa)
// event döngüsü içinde veya ayrı bir thread'de çalışabilir.
// Şu an eframe/egui'yi kullanamadığımız için sadece mesaj alma döngüsünü simüle ediyoruz.
pub fn gui_frontend_message_loop(
    update_rx_handle: Handle, // Backend'den güncelleme almak için Handle
    request_tx_task_id: TaskId, // Backend'e istek göndermek için TaskId
    gui_app_struct: &mut Gui, // GUI struct'ına referans
) {
     let mut message_buffer = [0u8; 1024]; // Mesaj almak için buffer

     // Bu döngü GUI'nin render döngüsüyle senkronize çalışmalı veya GUI event queue'una mesajları iletmeli.
     // Basitlik adına bloklamayan receive kullanmayı deneyelim veya ayrı bir thread/task'te bu döngüyü çalıştıralım.
     // messaging::receive varsayılan olarak bloklayıcıdır. Non-blocking receive için özel bir flag gerekebilir.
     // messaging::receive syscall'ının handle almadığını varsayarak (önceki API tanımı),
     // bu döngü GUI görevine gelen mesajları dinler.

     // Eğer bu ayrı bir thread/task'te çalışıyorsa ve GUI'nin ana döngüsüne mesaj iletmesi gerekiyorsa
     // aralarında başka bir IPC mekanizması (no_std Queue) olmalıdır.
      std::time::Duration::from_millis(10) // Uyuma süresi (poll etmek için)
      task::sleep(10).unwrap(); // Sahne64 uyku syscall'u

     // UI event döngüsünde çalışan non-blocking receive varsayımı:
      loop { // GUI'nin render veya event döngüsü içinde
         match messaging::receive_non_blocking(&mut message_buffer) { // Varsayımsal non-blocking receive
             Ok(bytes_received) => {
                  if bytes_received > 0 {
                       // Gelen mesajı deserialize et (postcard)
                       match postcard::from_bytes_copy::<GuiUpdate>(&message_buffer[..bytes_received]) {
                            Ok(update) => {
                                println!("GUI Güncelleme alındı: {:?}", update);
                                // GUI state'ini güncelle (örn. gui_app_struct.update_package_list)
                                 match update {
                                     GuiUpdate::PackageList(list) => gui_app_struct.update_package_list(list),
                                     GuiUpdate::InstallationStatus(msg) => { /* Durum çubuğunu güncelle */ println!("Durum: {}", msg); },
                                     GuiUpdate::Error(e) => { /* Hata mesajını göster */ eprintln!("GUI Hata: {:?}", e); },
     //                                // ... diğer güncellemeleri işle
                                 }
                                // GUI'nin yeniden çizilmesini tetikle (eframe/egui metodu, Sahne64 uyumlu GUI gerek)
                                 ctx.request_repaint();
                            }
                            Err(e) => { eprintln!("Gelen güncelleme deserialize edilemedi: {:?}", e); }
                       }
                  }
             }
             Err(SahneError::NoMessage) => {
                 // Mesaj yok, bekleme (non-blocking receive'de normal)
             }
             Err(e) => {
                 eprintln!("Mesaj alma hatası (GUI Frontend): {:?}", e);
     //            // Hata durumunda döngüden çıkış veya kurtarma
                 break;
             }
         }
         // UI'ı çiz ve event'leri işle (eframe/egui'nin işi)
          ui_framework.process_events_and_render();
      }
}

// GUI iletişim kanallarını başlatan fonksiyon (Backend tarafı için kullanılır).
// Bu fonksiyon, GUI görevini başlatmaz, sadece iletişim altyapısını kurar
// ve Backend döngüsünün kullanacağı Handle'ları/TaskID'leri döndürür.
// package_list_resource_id: GUI Backend'in kullanacağı paket listesi kaynağı ID'si.
// Bu fonksiyon muhtemelen paket yöneticisinin ana başlatma mantığında (srccli.rs veya lib.rs) çağrılacak.
pub fn start_gui_communication_backend(
    package_list_resource_id: &str,
    // Diğer backend bağımlılıkları
) -> Result<(), PaketYoneticisiHatasi> { // Bu fonksiyon GUI Backend task'ı kendisi spawn edebilir
    println!("GUI iletişim backend başlatılıyor...");

    // GUI (Frontend) görevinin Task ID'si bilinmeli.
    // Ya GUI görevi zaten çalışıyor ve ID'si biliniyor, ya da burada spawn ediliyor.
    // Varsayım: GUI görevi başka bir yerden başlatılıyor ve Task ID'si biliniyor VEYA
    // Backend, GUI task'ı başlatıyor.
    // Eğer backend başlatıyorsa:
     let gui_code_handle = resource::acquire("sahne://bin/package_manager_gui", resource::MODE_READ)?; // GUI kod kaynağı
     let gui_task_args = b""; // GUI'ye argümanlar
     let gui_task_id = task::spawn(gui_code_handle, gui_task_args)?;
     let _ = resource::release(gui_code_handle); // Handle'ı bırak

    // Basitlik adına, GUI görev ID'sinin yapılandırmadan geldiğini veya
    // bir isimlendirme hizmeti ile bulunabildiğini varsayalım.
     let gui_task_id = get_gui_task_id()?; // Varsayımsal fonksiyon

    // Veya daha olası senaryo: Paket yöneticisi tek görevdir, GUI ayrı bir thread'dir.
    // Bu durumda mpsc yerine thread-safe no_std Queue kullanılır.
    // Ancak orijinal kod 2 ayrı thread kullanıyordu, bu 2 ayrı Task'ı simule etmeye daha yakın.

    // Eğer 2 ayrı Task ise: Backend Task kendi receive Handle'ına sahip olmalı.
    // Messaging API'sında receive Task ID'ye göre yapılıyor.
    // Send metodu Task ID alıyor. Bu durumda Handshake gerekli olabilir:
    // GUI -> Backend mesajı gönderirken Backend'in Task ID'sini bilmeli.
    // Backend -> GUI mesajı gönderirken GUI'nin Task ID'sini bilmeli.

    // Handshake Senaryosu:
    // 1. Backend başlatılır, kendi Task ID'sini alır.
    // 2. Backend bir 'BackendReady' mesajı gönderir (nereye? Bilinen bir iletişim kaynağına?). Bu mesaj kendi Task ID'sini içerir.
    // 3. GUI başlatılır, 'BackendReady' mesajını bekler, Backend'in Task ID'sini öğrenir.
    // 4. GUI kendi Task ID'sini bir 'GuiReady' mesajı ile Backend'e gönderir.
    // 5. Artık ikisi de birbirinin Task ID'sini bilir.

    // Bu karmaşık Handshake yerine, iletişim için isimlendirilmiş kaynaklar (Messaging Kanalları) kullanmak daha kolay olur.
    // Sahne64 API'sında messaging kanalları TODO olarak işaretlenmişti.
    // Eğer messaging kanalları olsaydı:
    // Backend tarafında:
     let request_channel_handle = resource::acquire("sahne://ipc/pkgmgr/requests", resource::MODE_RECEIVE)?; // İstek kanalı handle'ı
     let update_channel_handle = resource::acquire("sahne://ipc/pkgmgr/updates", resource::MODE_SEND)?; // Güncelleme kanalı handle'ı
    // Sonra gui_backend_loop(request_channel_handle, update_channel_handle, ...); çağrılır.

    // GUI tarafında (ayrı bir task'te):
     let request_channel_handle = resource::acquire("sahne://ipc/pkgmgr/requests", resource::MODE_SEND)?; // İstek kanalı handle'ı
     let update_channel_handle = resource::acquire("sahne://ipc/pkgmgr/updates", resource::MODE_RECEIVE)?; // Güncelleme kanalı handle'ı
    // Sonra GUI'nin kendi mesaj işleme döngüsü bu handle'ları kullanır.

    // Mevcut Sahne64 Messaging API'sı (Task ID üzerinden send/receive) ile:
    // Backend başlatıldığında kendi Task ID'sini bilir.
    // GUI başlatıldığında kendi Task ID'sini bilir.
    // Birbirlerinin ID'sini bilmeleri gerekir. Ya yapılandırmada sabittir (riskli) ya da isimlendirme hizmeti gerekir (Sahne64 API'da yok).

    // Varsayım: GUI Task ID'si ve Backend Task ID'si bir şekilde biliniyor veya bulunabiliyor.
    // let backend_task_id = task::current_id()?; // Backend kendi ID'sini bilir
     let gui_task_id = get_gui_task_id(); // Varsayımsal olarak GUI ID'si bulundu

    // Bu fonksiyon sadece Backend döngüsünü başlatacak.
    // gui_backend_loop(backend'in receive Handle'ı?, gui_task_id, package_list_resource_id, ...);
    // Messaging receive Task ID üzerinden olduğu için receive Handle'ı diye bir şey API'da yoktu.

    // Gerekli olan sadece GUI'nin Task ID'si (update göndermek için) ve backend'in kendi receive mekanizması.
     gui_backend_loop(gui_task_id, package_list_resource_id, ...);

    // Bu fonksiyonun amacı, Backend task/thread'ini başlatmak ve ona gereken bilgileri sağlamak.
    // Eğer backend ayrı bir Task ise, bu fonksiyon o Task'ı spawn etmeli.
     let backend_code_handle = resource::acquire("sahne://bin/package_manager_backend", resource::MODE_READ)?;
     let backend_task_args = b""; // Argümanlar: GUI Task ID'si? Config Kaynak ID'si?
     let backend_task_id = task::spawn(backend_code_handle, backend_task_args)?;
     let _ = resource::release(backend_code_handle);

    // Veya Backend aynı Task içinde bir Thread ise:
     let package_list_res_id_string = package_list_resource_id.to_owned();
     task::create_thread(
         gui_backend_loop as u64, // Fonksiyon işaretçisi
         4096, // Stack boyutu (varsayımsal)
    //     // Argümanlar: GUI Task ID'si? Package List Resource ID?
    //     // Çoklu argüman geçmek için struct'ı Box içinde heap'e koyup işaretçisini geçmek gerekebilir.
          Box::into_raw(Box::new(BackendArgs { gui_task_id, package_list_res_id_string })) as u64
         0 // Şimdilik dummy arg
     )?;

    // En basit: Bu fonksiyon sadece Backend döngüsünü başlatan *kod* olarak tasarlanır.
    // Başka bir yer (örn. srccli.rs'nin main'i) bu kodu çalıştıracak Task/Thread'i spawn eder.
    // Bu fonksiyonun kendisi sadece backend'in işlevini tanımlar.
    // Fonksiyon imzası: gui_backend_loop(BackendArgs { gui_task_id, package_list_resource_id, ... });

    // Eğer bu fonksiyon Backend Task'ın ana entry point'iyse, C ABI'si ile çağrılır:
     #[no_mangle] pub extern "C" fn backend_entry(argc: usize, argv: *const *const u8) -> i32 { ... argümanları ayrıştır ... gui_backend_loop(...) ... }

    // Önceki start_gui fonksiyonunun mantığı:
    // 1. İletişim kanallarını kur (mpsc -> messaging)
    // 2. GUI thread/task'ini başlat (eframe -> hypothetical Sahne64 GUI task)
    // 3. Backend thread/task'ini başlat (logic -> gui_backend_loop)
    // 4. Request receiver'ı döndür.

    // start_gui refaktoringi:
    // messaging channel Handshake/isimlendirme varsayımı ile:
     pub fn start_gui_system(
         gui_code_resource_id: &str,
         backend_code_resource_id: &str,
         package_list_resource_id: &str,
         // Diğer gerekli kaynak ID'leri/argümanlar
     ) -> Result<TaskId, PaketYoneticisiHatasi> // GUI Task ID'sini döndür
     {
    //     // 1. Messaging Kanallarını Kur/Edin (Sahne64 isimlendirilmiş kanal varsayımı)
         let request_channel_handle_for_gui = resource::acquire("sahne://ipc/pkgmgr/requests", resource::MODE_SEND)?;
         let update_channel_handle_for_gui = resource::acquire("sahne://ipc/pkgmgr/updates", resource::MODE_RECEIVE)?;
         let request_channel_handle_for_backend = resource::acquire("sahne://ipc/pkgmgr/requests", resource::MODE_RECEIVE)?;
         let update_channel_handle_for_backend = resource::acquire("sahne://ipc/pkgmgr/updates", resource::MODE_SEND)?;
    //
    //     // 2. GUI Task'ını başlat
         let gui_code_handle = resource::acquire(gui_code_resource_id, resource::MODE_READ)?;
    //     // Argümanlar: update_channel_handle (receive), request_channel_handle (send)
    //     // Argümanları serileştirme veya Box içinde geçirme.
          let gui_args = ...;
         let gui_task_id = task::spawn(gui_code_handle, b"")?; // Dummy arg
         let _ = resource::release(gui_code_handle);
    //
    //     // 3. Backend Task'ını başlat
         let backend_code_handle = resource::acquire(backend_code_resource_id, resource::MODE_READ)?;
    //     // Argümanlar: request_channel_handle (receive), update_channel_handle (send), package_list_resource_id
          let backend_args = ...;
         let backend_task_id = task::spawn(backend_code_handle, b"")?; // Dummy arg
         let _ = resource::release(backend_code_handle);
    //
    //     // Başlatılan task'ların handle'larını tutmak gerekebilir mi? task::wait eksikliği.
    //     // Şimdilik sadece GUI Task ID'sini döndürelim.
         Ok(gui_task_id)
     }

    // Eğer mesajlaşma Task ID üzerinden ise (mevcut API):
    // Handshake olmadan Task ID'leri bilmek zor. Ya yapılandırmadan gelir ya da isimlendirme hizmeti.
    // Varsayım: GUI Task ID'si ve Backend Task ID'si birbirine yapılandırma veya argüman olarak geçiliyor.

    // Bu dosya sadece GUI ve Backend arasındaki İLETİŞİM mantığını tanımlayan kodları içersin.
    // GUI Task'ın kendisi (eframe/egui kullanan kısım) bu no_std repo'da olamaz.
    // Backend Task'ın ana döngüsü olan `gui_backend_loop` burada kalsın.
    // `Workspace_package_list_sahne64` burada kalsın.
    // GUI Request/Update enum'ları burada kalsın.

    // `start_gui` fonksiyonu GUI Task'ı başlatma mantığını içeriyordu.
    // Bu Sahne64'e refaktore edilirse, GUI Task'ının *Sahne64 uyumlu* kodunu spawn etmelidir.
    // O Sahne64 uyumlu GUI kodu başka bir repo'da veya bu repo'nun std özellikli test/simülasyon kısmında olabilir.

    // Bu dosyanın odağı: Backend'in iletişim döngüsü ve veri çekme fonksiyonu.
    // start_gui fonksiyonunu kaldırıp, yerine gui_backend_loop'u public yapalım.
    // Başka bir modül (srccli veya lib) backend task'ı spawn edip gui_backend_loop'u çalıştırır.

    // Original struct Gui ve impl App tamamen kaldırılmalı.
    // Sadece iletişim mesajları ve backend loop kalmalı.
}

// Original struct Gui, impl App for Gui tamamen kaldırıldı.

// Original start_gui fonksiyonu kaldırıldı.
// Yerine gui_backend_loop public yapıldı.

// Original main fonksiyonu kaldırıldı.
// Panic handler ve print makroları diğer modüllerde zaten var.


// --- GuiRequest ve GuiUpdate enum'ları kaldı ---
// --- fetch_package_list_sahne64 fonksiyonu kaldı ---
// --- gui_backend_loop fonksiyonu public yapıldı ---
// --- PaketYoneticisiHatasi ve SahneError importları kaldı ---
// --- alloc, resource, task, messaging importları kaldı ---
// --- print_macros importu kaldı ---

// GUI backend'in ana döngüsü, başka bir Task tarafından spawn edilip çağrılacak.
// İşaretçi olarak geçilebilmesi için argümanları Box içinde struct olarak almak daha uygun.
 #[derive(Debug)] // Debug derive'ı no_std'de çalışır.
 pub struct BackendArgs {
     pub update_tx_task_id: TaskId,
     pub package_list_resource_id: String, // Ownership alınmalı
//     // Diğer bağımlılıklar...
 }

// gui_backend_loop fonksiyon imzası güncellendi:
 pub fn gui_backend_loop(args_ptr: *mut c_void) -> ! { // task::create_thread argüman imzasına uygun
     // args_ptr'den BackendArgs struct'ını geri al.
     let args = unsafe { Box::from_raw(args_ptr as *mut BackendArgs) };
//     // ... döngü mantığı ...
     task::exit(0); // Çıkış
 }

#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // String, Vec için

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format; // Hata mesajları için

// Sahne64 API modüllerini içe aktarın
use crate::resource;
use crate::task; // sleep için
// messaging, task::create_thread gibi modüller de GUI iletişimi için gerekir, ama bu dosya sadece veri çekme odaklı olsun.
use crate::SahneError;
use crate::Handle; // Kaynak Handle'ları

// Özel hata enum'ımızı içe aktar (no_std uyumlu hali)
use crate::paket_yoneticisi_hata::PaketYoneticisiHatasi;
// SahneError'dan dönüşüm From implementasyonu ile sağlanacak


// no_std uyumlu print makroları (loglama için)
use crate::print_macros::{println, eprintln};


// Paket listesini Sahne64 Kaynakları kullanarak fetch etme fonksiyonu.
// srccli.rs ve GUI backend'i tarafından çağrılabilir.
// package_list_resource_id: Paket listesini içeren Kaynağın ID'si.
// Dönüş değeri: Paket isimlerinin listesi veya hata.
pub fn fetch_package_list_sahne64(package_list_resource_id: &str) -> Result<Vec<String>, PaketYoneticisiHatasi> {
    // Kaynağı oku (sadece okuma izniyle)
    let handle = resource::acquire(package_list_resource_id, resource::MODE_READ)
        .map_err(|e| {
             eprintln!("fetch_package_list: Kaynak acquire hatası ({}): {:?}", package_list_resource_id, e);
             PaketYoneticisiHatasi::from(e) // SahneError -> PaketYoneticisiHatasi
        })?;

    let mut buffer = Vec::new(); // Kaynak içeriğini tutacak tampon (alloc::vec::Vec)
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
                 eprintln!("fetch_package_list: Kaynak okuma hatası ({}): {:?}", package_list_resource_id, e);
                return Err(PaketYoneticisiHatasi::from(e)); // SahneError -> PaketYoneticisiHatasi
            }
        }
    }

    // Handle'ı serbest bırak
    let release_result = resource::release(handle);
     if let Err(e) = release_result {
          eprintln!("fetch_package_list: Kaynak release hatası ({}): {:?}", package_list_resource_id, e);
          // Release hatası kritik olmayabilir, loglayıp devam edebiliriz.
     }

    // Tampondaki binary veriyi String'e çevir (UTF-8 varsayımıyla)
    // core::str::from_utf8 Result<&str, Utf8Error> döner.
    core::str::from_utf8(&buffer)
        .map_err(|_| {
            eprintln!("fetch_package_list: Kaynak içeriği geçerli UTF-8 değil ({})", package_list_resource_id);
            // UTF-8 hatası için PaketYoneticisiHatasi::ParsingError kullanılabilir.
            PaketYoneticisiHatasi::ParsingError(format!("Geçersiz UTF-8 Kaynak içeriği: {}", package_list_resource_id)) // String kullanmak yerine hataya detay eklenebilir
        })
        .map(|content| {
            // String içeriği satır satır ayır ve paket isimlerini topla
             content.lines()
                   .map(|line| line.trim().to_string()) // Her satırı String yap (alloc)
                   .filter(|line| !line.is_empty()) // Boş satırları atla
                   .collect::<Vec<String>>() // Vec<String> olarak topla (alloc)
        })
}

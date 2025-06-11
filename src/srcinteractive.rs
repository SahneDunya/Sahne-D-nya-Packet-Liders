#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // HashMap, String, Vec, Box, format! için

use alloc::collections::HashMap; // std::collections::HashMap yerine
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::boxed::Box; // dyn CommandHandler için
use alloc::format; // format! makrosu için
use alloc::borrow::ToOwned; // &str -> String için
use core::ffi::c_void; // Raw pointer type for task arguments

// Sahne64 API modülleri
use crate::resource; // Giriş/Çıkış için
use crate::task; // Görev yönetimi (spawn, exit)
use crate::SahneError; // Sahne64 hata türü
use crate::Handle; // Kaynak Handle'ları

// Özel hata enum'ımızı içe aktar (no_std uyumlu hali)
use crate::paket_yoneticisi_hata::PaketYoneticisiHatasi;
// SahneError'dan dönüşüm From implementasyonu ile sağlanacak

// no_std uyumlu print makroları (console Kaynağına yazacak)
use crate::print_macros::{print, println, eprintln};


// Konsol Girdi Kaynağı Handle'ı (main'den geçilmeli veya global/static olmalı)
// Global/static handle yönetimi no_std'de dikkatli yapılmalıdır (Raw pointer, MaybeUninit, unsafe).
// Basitlik adına, get_input fonksiyonunun console input handle'ını argüman olarak aldığını varsayalım.

// Kullanıcıdan girdi alır
// console_input_handle: Konsol girdi kaynağına ait Handle.
// prompt: Kullanıcıya gösterilecek prompt stringi.
// Dönüş değeri: Okunan satır (String) veya hata.
pub fn get_input(console_input_handle: Handle, prompt: &str) -> Result<String, PaketYoneticisiHatasi> { // Result türü PaketYoneticisiHatasi olmalı
    // Prompt'u yazdır
    print!("{}", prompt);
    // print! makrosu zaten alttan resource::write kullanmalı ve muhtemelen flush da yapar.
    // Eğer yapmıyorsa, console kaynağı için resource::control ile bir flush komutu gerekebilir.
    // Şimdilik print!'in yeterli olduğunu varsayalım.

    let mut buffer = Vec::new(); // Okunan baytları tutacak buffer (alloc::vec::Vec)
    let mut temp_buffer = [0u8; 64]; // Küçük okuma tamponu (stack'te)

    // Satır sonu karakterini (\n) bulana kadar Kaynaktan oku.
    // resource::read varsayılan olarak blocking okur. Non-blocking veya line-buffered okuma
    // console Kaynağının özelliğine veya resource::control komutlarına bağlıdır.
    // Blocking, satır sonuna kadar okuduğunu varsayalım.
    loop {
        match resource::read(console_input_handle, &mut temp_buffer) {
            Ok(0) => {
                // EOF (Girdi kaynağı kapandı). Eğer hiç bayt okunmadıysa boş String, yoksa kısmi satır.
                break; // Döngüden çık
            }
            Ok(bytes_read) => {
                buffer.extend_from_slice(&temp_buffer[..bytes_read]);
                // Okunan baytlar arasında satır sonu var mı kontrol et.
                if temp_buffer[..bytes_read].contains(&b'\n') {
                    break; // Satır sonu bulundu, okumayı durdur.
                }
            }
            Err(e) => {
                // Okuma hatası durumunda hata dön.
                 eprintln!("Girdi Kaynağı okuma hatası: {:?}", e);
                return Err(PaketYoneticisiHatasi::from(e)); // SahneError -> PaketYoneticisiHatasi
            }
        }
    }

    // Tampondaki baytları String'e çevir. UTF-8 varsayımı.
    // Satır sonu dahil olabilir, trim() ile temizlenecek.
    core::str::from_utf8(&buffer)
        .map_err(|_| {
             eprintln!("Girdi geçerli UTF-8 değil.");
            // UTF-8 hatası için PaketYoneticisiHatasi::ParsingError kullanılabilir.
            PaketYoneticisiHatasi::ParsingError(String::from("Geçersiz UTF-8 girdi")) // String alloc gerektirir
        })
        .map(|s| s.trim().to_owned()) // &str -> String, baştaki/sondaki boşlukları ve \n'yi temizle (alloc)
}


// Komutları işlemesi için bir trait tanımla.
// execute metodu PaketYoneticisiHatasi dönecek.
trait CommandHandler {
    fn execute(&self, args: &[&str], console_input_handle: Handle, console_output_handle: Handle) -> Result<(), PaketYoneticisiHatasi>;
    fn description(&self) -> String; // String alloc gerektirir.
}

// "hello" komutu
struct HelloCommand;
impl CommandHandler for HelloCommand {
    fn execute(&self, _args: &[&str], _console_input_handle: Handle, _console_output_handle: Handle) -> Result<(), PaketYoneticisiHatasi> {
        println!("Merhaba!"); // no_std print makrosu
        Ok(())
    }
    fn description(&self) -> String {
        String::from("Basit bir merhaba mesajı gösterir.") // String alloc gerektirir.
    }
}

// "date" komutu (Placeholder)
struct DateCommand;
impl CommandHandler for DateCommand {
    // Bu komut başka bir Sahne64 görevini (task) başlatmalıdır (örneğin "sahne://bin/date").
    // Ancak, Sahne64 API'sında task::wait ve task çıktısını yakalama mekanizmaları eksiktir.
    // Bu nedenle bu implementasyon sadece görevi başlatır ve çıktıyı göremeyebilir.
    fn execute(&self, _args: &[&str], _console_input_handle: Handle, _console_output_handle: Handle) -> Result<(), PaketYoneticisiHatasi> {
        println!("'date' komutu çalıştırılıyor (task başlatıldı)..."); // no_std print makrosu

        // "date" komutunun çalıştırılabilir Kaynağına Handle edin (varsayımsal)
        let date_command_resource_id = "sahne://bin/date"; // Varsayımsal Kaynak ID'si
        match resource::acquire(date_command_resource_id, resource::MODE_READ) { // Çalıştırma izni için MODE_READ yeterli mi?
            Ok(command_handle) => {
                // Yeni bir görev (task) olarak komutu başlat
                // task::spawn function signature: spawn(code_handle: Handle, args: &[u8])
                let task_args: &[u8] = b""; // date komutuna argüman geçilebilir. Şimdilik boş.

                match task::spawn(command_handle, task_args) {
                    Ok(new_tid) => {
                        println!("'date' görevi başlatıldı, TaskId: {:?}", new_tid); // no_std print makrosu
                        let _ = resource::release(command_handle); // Handle'ı bırak
                        // Görevin tamamlanmasını beklemek ve çıktısını yakalamak API'da eksik.
                        // Bu görev çıktıyı doğrudan konsola yazıyorsa görünebilir.
                        Ok(())
                    }
                    Err(e) => {
                        let _ = resource::release(command_handle); // Hata durumunda handle'ı temizle
                        eprintln!("'date' görevi başlatılamadı (Kaynak: {}): {:?}", date_command_resource_id, e); // no_std print makrosu
                        Err(PaketYoneticisiHatasi::from(e)) // SahneError -> PaketYoneticisiHatasi
                    }
                }
            }
            Err(e) => {
                 eprintln!("'date' komut Kaynağı acquire hatası ({}): {:?}", date_command_resource_id, e); // no_std print makrosu
                Err(PaketYoneticisiHatasi::from(e)) // SahneError -> PaketYoneticisiHatasi
            }
        }
    }
    fn description(&self) -> String {
        String::from("Sistem tarihini gösterir (yeni görev olarak başlatır).") // String alloc gerektirir.
    }
}

// "exit" komutu
struct ExitCommand;
impl CommandHandler for ExitCommand {
    fn execute(&self, _args: &[&str], _console_input_handle: Handle, _console_output_handle: Handle) -> Result<(), PaketYoneticisiHatasi> {
        println!("Çıkılıyor..."); // no_std print makrosu
        task::exit(0); // Sahne64 görevini sonlandır. Bu fonksiyon geri dönmez.
         return Ok(()) // unreachable
    }
    fn description(&self) -> String {
        String::from("Programdan çıkar (görevi sonlandırır).") // String alloc gerektirir.
    }
}

// "echo" komutu - argüman alabilen bir komut
struct EchoCommand; // Struct artık argümanları kendisi tutmuyor, execute metodu alıyor.

impl CommandHandler for EchoCommand {
    // args: Komutun argümanları dilimi (&[&str]).
    fn execute(&self, args: &[&str], _console_input_handle: Handle, _console_output_handle: Handle) -> Result<(), PaketYoneticisiHatasi> {
        // Argümanları boşlukla birleştirip yazdır. join alloc gerektirir.
        println!("{}", args.join(" ")); // no_std print makrosu
        Ok(())
    }
    fn description(&self) -> String {
        String::from("Verilen argümanları ekrana yazdırır. Kullanım: echo [mesaj]") // String alloc gerektirir.
    }
}

// "ls" komutu (Placeholder)
struct LsCommand;
impl CommandHandler for LsCommand {
     // "date" komutu gibi, başka bir Sahne64 görevini başlatmalıdır.
     // task::wait ve çıktı yakalama eksikliği burada da geçerlidir.
    fn execute(&self, _args: &[&str], _console_input_handle: Handle, _console_output_handle: Handle) -> Result<(), PaketYoneticisiHatasi> {
        println!("'ls' komutu çalıştırılıyor (task başlatıldı)..."); // no_std print makrosu

        let ls_command_resource_id = "sahne://bin/ls"; // Varsayımsal Kaynak ID'si
        match resource::acquire(ls_command_resource_id, resource::MODE_READ) {
            Ok(command_handle) => {
                let task_args: &[u8] = b""; // ls komutuna argümanlar (örn. b"-l /sahne://config/")
                match task::spawn(command_handle, task_args) {
                    Ok(new_tid) => {
                        println!("'ls' görevi başlatıldı, TaskId: {:?}", new_tid); // no_std print makrosu
                        let _ = resource::release(command_handle);
                        // Görevin tamamlanmasını beklemek ve çıktısını yakalamak API'da eksik.
                        Ok(())
                    }
                    Err(e) => {
                        let _ = resource::release(command_handle);
                        eprintln!("'ls' görevi başlatılamadı (Kaynak: {}): {:?}", ls_command_resource_id, e); // no_std print makrosu
                        Err(PaketYoneticisiHatasi::from(e)) // SahneError -> PaketYoneticisiHatasi
                    }
                }
            }
            Err(e) => {
                eprintln!("'ls' komut Kaynağı acquire hatası ({}): {:?}", ls_command_resource_id, e); // no_std print makrosu
                Err(PaketYoneticisiHatasi::from(e)) // SahneError -> PaketYoneticisiHatasi
            }
        }
    }
    fn description(&self) -> String {
        String::from("Dizin içeriğini listeler (yeni görev olarak başlatır).") // String alloc gerektirir.
    }
}

// "clear" komutu (Placeholder)
struct ClearCommand;
impl CommandHandler for ClearCommand {
     // "date" ve "ls" gibi, başka bir Sahne64 görevini başlatmalıdır.
     // task::wait eksikliği burada da geçerlidir. Çıktı yerine ekranı temizleme komutu gönderir.
    fn execute(&self, _args: &[&str], _console_input_handle: Handle, _console_output_handle: Handle) -> Result<(), PaketYoneticisiHatasi> {
        println!("'clear' komutu çalıştırılıyor (task başlatıldı)..."); // no_std print makrosu

        let clear_command_resource_id = "sahne://bin/clear"; // Varsayımsal Kaynak ID'si
        match resource::acquire(clear_command_resource_id, resource::MODE_READ) {
            Ok(command_handle) => {
                 // task::spawn'a argüman olarak konsol Handle'ını geçmek gerekebilir ki clear komutu nereyi temizleyeceğini bilsin.
                 // Veya konsol kaynağına özel bir control komutu gönderilir.
                 // task::spawn için argümanlar &[u8]. Handle'ı byte olarak nasıl geçeriz? Pointer olarak?
                 // task::spawn(command_handle, &console_output_handle as *const Handle as *const u8 as &[u8])? Güvenli değil.
                 // En iyisi clear komutunun kendisi konsol kaynağını isimle bulsun veya bir standart handle'dan alsın.
                 // Şimdilik argümansız başlatalım.
                let task_args: &[u8] = b"";
                match task::spawn(command_handle, task_args) {
                    Ok(new_tid) => {
                        println!("'clear' görevi başlatıldı, TaskId: {:?}", new_tid); // no_std print makrosu
                        let _ = resource::release(command_handle);
                        // Görevin tamamlanmasını beklemek API'da eksik.
                        Ok(())
                    }
                    Err(e) => {
                        let _ = resource::release(command_handle);
                        eprintln!("'clear' görevi başlatılamadı (Kaynak: {}): {:?}", clear_command_resource_id, e); // no_std print makrosu
                        Err(PaketYoneticisiHatasi::from(e)) // SahneError -> PaketYoneticisiHatasi
                    }
                }
            }
            Err(e) => {
                 eprintln!("'clear' komut Kaynağı acquire hatası ({}): {:?}", clear_command_resource_id, e); // no_std print makrosu
                Err(PaketYoneticisiHatasi::from(e)) // SahneError -> PaketYoneticisiHatasi
            }
        }
    }
    fn description(&self) -> String {
        String::from("Ekranı temizler (yeni görev olarak başlatır).") // String alloc gerektirir.
    }
}


// "help" komutu
struct HelpCommand {
    // Command map'in bir klonunu tutmak yerine, referansını tutmak daha iyi olabilir
    // ancak bu trait object'in lifetime'ını yönetmeyi zorlaştırır.
    // Box<dyn CommandHandler> Clone etmez default olarak. Eğer CommandHandler trait'ine Clone
    // supertrait olarak eklenirse klonlanabilir.
    // Clone + Box<dyn Trait + Clone> deseni gerektirir.
    // Veya sadece komut isimlerinin listesini ve açıklamalarını tutarız.
    // Mevcut kod HashMap<String, Box<dyn CommandHandler>> klonluyor, bu da Box içindeki objenin Clone olmasını gerektirir.
    // Tüm CommandHandler implementasyonları Clone derive ediyor mu? EchoCommand etmiyor.
    // Hello, Date, Exit, Ls, Clear struct'ları Copy derive ediyor. EchoCommand Clone derive etmeli.
    // Trait objelerinin Clone edilmesi karmaşıktır. Basitlik adına, HelpCommand'ın sadece komut isimlerini ve açıklamalarını
    // tuttuğunu varsayalım veya command_map'in referansını alıp lifetime'ı yönetelim.
    // Orijinal kodun klonladığını varsayarak devam edelim, ancak bu klonlama CommandHandler trait'ini Clone etmeyi zorlar.
    Box<dyn CommandHandler> + Clone
    command_info: HashMap<String, String>, // Komut adı -> Açıklama
}

// CommandHandler trait'ini Clone etmesini sağlayalım (eğer tüm implementasyonlar Clone ediyorsa)
 trait CommandHandler : Debug + Send + Sync + 'static { ... }
 trait CommandHandler : Debug + Clone + Send + Sync + 'static { ... } // Böyle olmalı

// EchoCommand Clone derive etmeli
#[derive(Clone)] // EchoCommand için Clone derive'ı eklendi.
struct EchoCommandProps { args: Vec<String> } // EchoCommand'ın state'i
struct EchoCommand; // EchoCommand handler'ı state tutmaz

impl HelpCommand {
    // command_map'in referansını alıp açıklama map'ini oluşturur.
    // Lifetime 'a, command_map'in lifetime'ından gelmelidir.
    // Veya main fonksiyonunda oluşturulan map'in ownership'ini alır.
    // Main'de map oluşturulup start_interactive_loop'a geçiriliyor. HelpCommand'a referans geçerken
    // HelpCommand struct'ı map'in kendisini değil, sadece bilgiyi saklayabilir.
     pub fn new(command_map: &HashMap<String, Box<dyn CommandHandler>>) -> Self {
         let mut command_info = HashMap::new();
         for (name, handler) in command_map.iter() { // Referansları al, klonlama yapma (map'in kendisini)
              command_info.insert(name.clone(), handler.description()); // String klonlama (alloc)
         }
         HelpCommand { command_info } // Sadece isim ve açıklama map'ini tutar
     }
}
impl CommandHandler for HelpCommand {
    fn execute(&self, _args: &[&str], _console_input_handle: Handle, _console_output_handle: Handle) -> Result<(), PaketYoneticisiHatasi> {
        println!("Kullanılabilir komutlar:"); // no_std print makrosu
        for (name, description) in &self.command_info { // command_info map'ini kullan
            println!("- {}: {}", name, description); // no_std print makrosu
        }
        Ok(())
    }
    fn description(&self) -> String {
        String::from("Kullanılabilir komutları ve açıklamalarını listeler.") // String alloc gerektirir.
    }
}


// Komutları işle.
// command_str: Kullanıcının girdiği komut satırı stringi.
// command_map: Komut isimlerini CommandHandler trait objelerine eşleyen harita.
// console_input_handle: Konsol girdi kaynağı handle.
// console_output_handle: Konsol çıktı kaynağı handle.
// Dönüş değeri: Başarı veya PaketYoneticisiHatasi.
pub fn handle_command(
    command_str: &str,
    command_map: &HashMap<String, Box<dyn CommandHandler>>,
    console_input_handle: Handle,
    console_output_handle: Handle,
) -> Result<(), PaketYoneticisiHatasi> { // Result türü PaketYoneticisiHatasi olmalı
    // Komut satırını kelimelere ayır. split_whitespace() &str metodu, no_std uyumlu.
    let parts: Vec<&str> = command_str.split_whitespace().collect(); // collect Vec<String> değil Vec<&str>
    if parts.is_empty() {
        return Ok(()); // Boş girdi, başarı
    }

    let command_name = parts[0]; // İlk kelime komut adı (&str)
    // Geri kalan kelimeler argümanlar (&[&str] dilimi)
    let args: &[&str] = &parts[1..];


    // Komut map'inde komut adını ara.
    if let Some(handler) = command_map.get(command_name) { // HashMap get (&str)
        // Komut işleyicisini bulduk, execute metodu çağır.
        // EchoCommand özel durumu kaldırıldı, execute metodu argümanları zaten alıyor.
        handler.execute(args, console_input_handle, console_output_handle)?; // execute çağrısı ve ? operatörü

    } else {
        // Komut bulunamadı.
        println!("Geçersiz komut. 'help' komutunu kullanarak kullanılabilir komutları görebilirsiniz."); // no_std print makrosu
        // Geçersiz komut hata olarak döndürülebilir:
         return Err(PaketYoneticisiHatasi::InvalidParameter(format!("Bilinmeyen komut: {}", command_name))); // format! alloc
    }
    Ok(()) // Komut bulundu ve işlendi (başarılı veya kendi hatasını bastı/döndü)
}

// Etkileşimli döngüyü başlatır.
// command_map: Komut map'i.
// console_input_handle: Konsol girdi handle.
// console_output_handle: Konsol çıktı handle.
// Dönüş değeri: Döngüden çıkış durumunda (exit komutu dışında) hata veya başarı.
// Exit komutu task::exit çağırdığı için normalde bu fonksiyon geri dönmez.
// Eğer exit dışı bir nedenle döngü biterse Result dönebilir.
pub fn start_interactive_loop(
    command_map: HashMap<String, Box<dyn CommandHandler>>, // HashMap ownership'i alınır
    console_input_handle: Handle,
    console_output_handle: Handle,
) -> Result<(), PaketYoneticisiHatasi> { // io::Result<()> yerine PaketYoneticisiHatasi
    loop {
        // Kullanıcıdan girdi al
        let input = get_input(console_input_handle, "> ")?; // ? operatörü PaketYoneticisiHatasi'nı yayar

        // Gelen girdiyi işle (komut ve argümanları ayrıştır, handler çağır)
        match handle_command(&input, &command_map, console_input_handle, console_output_handle) {
            Ok(_) => {
                // Komut başarıyla işlendi veya hata bastırıldı. Döngü devam eder.
            }
            Err(e) => {
                 // Komut işleme sırasında bir hata oluştu. Hatayı yazdır.
                eprintln!("Komut işlenirken hata oluştu: {:?}", e); // no_std print makrosu
                // Döngü devam eder, kullanıcıya tekrar prompt gösterilir.
                // Ciddi bir hata durumunda döngüden çıkılabilir.
                 return Err(e); // Eğer hatada döngüden çıkılacaksa
            }
        }

        // Exit komutu task::exit çağırdığı için buraya ulaşılmayacaktır eğer kullanıcı "exit" yazarsa.
    }
     Ok(()) // Döngü normalde bitmez, bu satıra unreachable etiketi eklenebilir.
}


// main fonksiyonu (Sahne64 Task Entry Point)
// Bu fonksiyon Sahne64 çekirdeği tarafından C ABI'si ile çağrılacaktır.
// argc: argüman sayısı
// argv: C string'lerinin işaretçilerinin dizisi (**char)
// console_input_handle: Konsol girdi kaynağı handle (çekirdek tarafından sağlanmalı?)
// console_output_handle: Konsol çıktı kaynağı handle (çekirdek tarafından sağlanmalı?)
// Handle'lar main'e argüman olarak geçilirse daha temiz olur.
// Varsayım: Sahne64 çekirdeği standart girdi/çıktı handle'larını task'ın main fonksiyonuna argüman olarak geçirir.
// Şimdilik argüman listesine ekleyelim.
#[no_mangle] // Çekirdek tarafından çağrılabilmesi için ismin bozulmasını engelle
pub extern "C" fn main(
    argc: usize,
    argv: *const *const u8,
    console_input_handle: Handle, // Varsayımsal
    console_output_handle: Handle, // Varsayımsal
) -> i32 { // Çıkış kodu (task::exit kullanılır ama imza böyle)

    // Argümanları ayrıştır (srccli.rs'deki gibi)
    // Bu Interactive shell argüman alabilir (örn. interaktif moda geçmek için flag).
    // Ancak komut işleme döngüsü kendi input/output handle'larını kullanır.
    // Main fonksiyonu başlangıç argümanlarını işleyebilir.
    // Şimdilik başlangıç argümanlarını yoksayalım.

    let mut command_map: HashMap<String, Box<dyn CommandHandler>> = HashMap::new(); // alloc::collections::HashMap

    // Komut işleyicilerini HashMap'e ekle
    // Box::new alloc gerektirir.
    command_map.insert("hello".to_owned(), Box::new(HelloCommand)); // to_owned() alloc
    command_map.insert("date".to_owned(), Box::new(DateCommand)); // to_owned() alloc
    command_map.insert("exit".to_owned(), Box::new(ExitCommand)); // to_owned() alloc
    command_map.insert("ls".to_owned(), Box::new(LsCommand)); // to_owned() alloc
    command_map.insert("echo".to_owned(), Box::new(EchoCommand)); // to_owned() alloc

    // Help komutu, command_map'teki diğer komutların açıklamalarına ihtiyaç duyar.
    // HelpCommand::new(command_map) HelpCommand içinde command_map'in klonunu veya referansını tutar.
    // HelpCommand'ın new metodu güncellendi (sadece isim/açıklama map'ini oluşturur).
     let help_command_handler = Box::new(HelpCommand::new(&command_map)); // HelpCommand struct'ı ve Box
    command_map.insert("help".to_owned(), help_command_handler); // to_owned() alloc


    println!("Sahne64 İnteraktif Kabuk Başlatıldı!"); // no_std print makrosu
    println!("Çıkmak için 'exit' yazın."); // no_std print makrosu

    // Etkileşimli döngüyü başlat
    let result = start_interactive_loop(command_map, console_input_handle, console_output_handle);

    // Döngü normalde geri dönmez (exit komutu task::exit çağırır).
    // Eğer döngü başka bir hatadan dönerse, hata kodunu işle.
    match result {
        Ok(_) => {
            // Bu koda sadece exit komutu dışında bir nedenle döngü sonlanırsa ulaşılır.
             task::exit(0) // Normal çıkış
            0 // Başarı (eğer return Ok(()) yapıldıysa)
        }
        Err(e) => {
             eprintln!("İnteraktif kabuk beklenmedik şekilde sonlandı: {:?}", e); // no_std print makrosu
             task::exit(-1) // Hata kodu ile çıkış
            -1 // Hata
        }
    }

    // Çekirdek bu main fonksiyonundan döndüğünde görevi sonlandırabilir.
    // task::exit kullanmak her zaman en güvenli yöntemdir.
}

#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // String, Vec, format! için

// Sahne64 API modülleri
use crate::resource; // Kaynak işlemleri (konsol çıktısı/girdisi)
use crate::task; // Belki ayrı görevler için
use crate::SahneError; // Sahne64 hata türü
use crate::Handle; // Kaynak Handle'ları

// String and Vec from alloc
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format; // format! makrosu için

// Özel hata enum'ımızı içe aktar (no_std uyumlu hali)
use crate::paket_yoneticisi_hatasi::PaketYoneticisiHatasi;
// TuiError'dan dönüşüm eklenecek.
use crate::srcerror::PaketYoneticisiHatasi as ErrorMapping; // From impl için yeniden adlandır

// no_std uyumlu log makroları (dahili hatalar için)
use log::{info, warn, error, debug};

// no_std uyumlu print makroları (doğrudan konsola yazmak için değil, hata/log çıktısı için)
 use crate::print_macros::{println, eprintln};


// TUI (Text User Interface) hatalarını temsil eden enum (no_std uyumlu).
// Debug ve Display manuel implementasyonları.
// srcstui.rs'de tanımlanan TuiError ile tutarlı olmalıdır.
#[derive(Debug)] // Debug derive'ı no_std'de çalışır
pub enum TuiError {
    // Sahne64 Kaynak (Konsol Çıktısı/Girdisi) Hatası
    Sahne64ResourceError(SahneError), // SahneError'ı sarmalar

    // Terminal kontrol hatası (imleç, renk, temizleme gibi API'lar eksik veya hata verdiğinde)
    TerminalControlError(String), // String alloc gerektirir

    // Terminal girdi hatası (klavye olayı okuma gibi API'lar eksik veya hata verdiğinde)
    TerminalInputError(String), // String alloc gerektirir

    // Diğer TUI ile ilgili hatalar
    // ...
}

// core::fmt::Display implementasyonu (kullanıcı dostu mesajlar için)
impl core::fmt::Display for TuiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TuiError::Sahne64ResourceError(e) => write!(f, "Sahne64 Kaynak hatası: {:?}", e),
            TuiError::TerminalControlError(s) => write!(f, "Terminal kontrol hatası: {}", s),
            TuiError::TerminalInputError(s) => write!(f, "Terminal girdi hatası: {}", s),
        }
    }
}

// From implementasyonu
impl From<SahneError> for TuiError {
    fn from(err: SahneError) -> Self {
        TuiError::Sahne64ResourceError(err)
    }
}


// UI durumunu ve etkileşimlerini yönetecek ana yapı.
// std'deki crossterm gibi bir terminal backend'ine ve olay döngüsüne bağlıdır.
// Sahne64'e uyarlanırken, terminal Kaynak Handle'ları ve Sahne64 olay/kontrol API'ları kullanılır.
pub struct PackageManagerUI {
    // Konsol çıktı Kaynağının Handle'ı
    console_output_handle: Handle,
    // Konsol girdi Kaynağının Handle'ı
    console_input_handle: Handle, // Klavye girdisi için
    // UI durumu (örn. seçili menü öğesi, mevcut ekran)
    selected_menu: u8,
    // ... diğer UI durumu değişkenleri ...
}

impl PackageManagerUI {
    // Yeni bir PackageManagerUI örneği oluşturur.
    // Gerekli konsol Kaynaklarını acquire eder.
    // console_output_resource_id: Konsol çıktı Kaynağının ID'si (örn. "sahne://console/out").
    // console_input_resource_id: Konsol girdi Kaynağının ID'si (örn. "sahne://console/in").
    // Dönüş değeri: Başarılı UI yapısı veya TuiError.
    pub fn new(console_output_resource_id: &str, console_input_resource_id: &str) -> Result<Self, TuiError> { // Result<Self, TuiError> olmalı
        debug!("UI başlatılıyor. Çıktı Kaynağı: {}, Girdi Kaynağı: {}", console_output_resource_id, console_input_resource_id); // no_std log

        // Konsol çıktı Kaynağını acquire et.
        let console_output_handle = resource::acquire(console_output_resource_id, resource::MODE_WRITE)
            .map_err(|e| {
                 error!("Konsol çıktı Kaynağı acquire hatası ({}): {:?}", console_output_resource_id, e); // no_std log
                TuiError::from(e) // SahneError -> TuiError
            })?;

        // Konsol girdi Kaynağını acquire et.
        let console_input_handle = resource::acquire(console_input_resource_id, resource::MODE_READ) // Okuma izni
            .map_err(|e| {
                 error!("Konsol girdi Kaynağı acquire hatası ({}): {:?}", console_input_resource_id, e); // no_std log
                 let _ = resource::release(console_output_handle); // Diğer handle'ı temizle
                TuiError::from(e) // SahneError -> TuiError
            })?;

        // Sahne64 API Eksikliği: Terminali ham moda geçirme (enable_raw_mode) burada yapılmalıdır.
        // Varsayımsal resource::control komutu veya resource özelliği gerekebilir.
          match resource::control(console_input_handle, resource::CMD_ENABLE_RAW_MODE, &[]) { // Varsayımsal komut
              Ok(_) => info!("Terminal ham moda geçirildi."),
              Err(e) => {
                   warn!("Terminal ham moda geçirilemedi: {:?}", e); // Uyarı olarak logla ama devam et.
                   // Ham mod olmadan tuş girdilerini yorumlamak zor olacaktır.
              }
          }
         info!("UYARI: Sahne64 API'sında terminal ham mod kontrolü eksik."); // no_std log


        Ok(PackageManagerUI {
            console_output_handle,
            console_input_handle,
            selected_menu: 1, // Başlangıç durumu
            // ... diğer başlangıç durumu atamaları ...
        })
    }

    // Ana arayüz döngüsünü başlatır.
    // Klavye girdisini okur, UI durumunu günceller ve ekranı yeniden çizer.
    // Dönüş değeri: UI döngüsü tamamlandığında Başarı veya TuiError.
    pub fn run(&mut self) -> Result<(), PaketYoneticisiHatasi> { // Result<(), PaketYoneticisiHatasi> olmalı
        info!("UI ana döngüsü başlatılıyor."); // no_std log

        // Sahne64 API Eksikliği: Terminal çıktısını temizleme ve imleç kontrolü (crossterm::execute!)
        // Varsayımsal resource::control komutları gereklidir.
        // Varsayımsal komutlar: CMD_CLEAR_SCREEN, CMD_MOVE_CURSOR { x: u16, y: u16 }
         match resource::control(self.console_output_handle, resource::CMD_CLEAR_SCREEN, &[]) { /* ... */ }
         match resource::control(self.console_output_handle, resource::CMD_MOVE_CURSOR, &pos_bytes) { /* ... */ }
         info!("UYARI: Sahne64 API'sında ekran temizleme ve imleç kontrolü eksik."); // no_std log


        loop { // Ana UI döngüsü
            // Ekranı mevcut duruma göre çiz.
             if let Err(e) = self.draw_screen() { // draw_screen TrustError dönebilir
                  error!("Ekran çizme hatası: {:?}", e); // no_std log
                 // Ekran çizme hatası durumunda döngüden çıkalım ve hata dönelim.
                 return Err(PaketYoneticisiHatasi::from(e)); // TuiError -> PaketYoneticisiHatasi
             }

             // Sahne64 API Eksikliği: Terminal olaylarını (klavye tuşları) okuma ve bekleme (crossterm::event::poll/read)
             // Varsayımsal resource::read veya resource::poll ve girdi baytlarını yorumlama mantığı gereklidir.
             // Varsayımsal API: resource::read(handle, buffer) non-blocking veya blocking olabilir. poll(handle, timeout) ile bekleme.
             info!("UYARI: Sahne64 API'sında terminal girdi okuma (olay döngüsü) eksik."); // no_std log

            // Simülasyon: Varsayımsal olarak bir girdi olayı bekleyelim.
            // Gerçek implementasyonda resource::read veya poll kullanılacaktır.
             let input_event: Option<SimulatedKeyEvent> = None; // Simulate reading input

            // Eğer bir girdi olayı varsa işle
             if let Some(key_event) = input_event {
                // Olay türünü ve tuşu işle
                 match key_event.code {
                     SimulatedKeyCode::Up => {
                          self.selected_menu = if self.selected_menu > 1 { self.selected_menu - 1 } else { 4 };
                     }
                     SimulatedKeyCode::Down => {
                          self.selected_menu = if self.selected_menu < 4 { self.selected_menu + 1 } else { 1 };
                     }
                     SimulatedKeyCode::Enter => {
                          // Seçime göre ilgili ekranı çalıştır.
                           match self.selected_menu {
                               1 => { if let Err(e) = self.list_packages_screen() { error!("Paket listeleme ekranı hatası: {:?}", e); } }
                               2 => { if let Err(e) = self.add_package_screen() { error!("Paket ekleme ekranı hatası: {:?}", e); } }
                               3 => { if let Err(e) = self.remove_package_screen() { error!("Paket kaldırma ekranı hatası: {:?}", e); } }
                               4 => { info!("UI'dan çıkış seçildi."); break; } // Çıkış
                               _ => {} // Bilinmeyen seçim
                           }
                     }
                     SimulatedKeyCode::Char(c) => {
                         // Sayı tuşları veya 'q' ile menü seçimi/çıkış.
                          match c {
                              '1' => self.selected_menu = 1,
                              '2' => self.selected_menu = 2,
                              '3' => self.selected_menu = 3,
                              '4' => self.selected_menu = 4,
                              'q' | 'Q' => { info!("UI'dan 'q' ile çıkış seçildi."); break; } // Çıkış
                              _ => {} // Bilinmeyen tuş
                          }
                     }
                     SimulatedKeyCode::Esc => { info!("UI'dan Esc ile çıkış seçildi."); break; } // Çıkış
                     _ => {} // Diğer tuşlar
                 }
            }

            // Girdi işlendikten sonra kısa bir bekleme gerekebilir (event::poll simülasyonu).
            // Sahne64 API'sında görev bekletme syscall'ı (`task::sleep`?) gerekebilir.
             task::sleep(Duration::from_millis(10)); // Varsayımsal sleep API'sı
        }

        // Sahne64 API Eksikliği: Terminali normal moda döndürme (disable_raw_mode)
        // Varsayımsal resource::control komutu gerekebilir.
          match resource::control(self.console_input_handle, resource::CMD_DISABLE_RAW_MODE, &[]) { /* ... */ }
         info!("UYARI: Sahne64 API'sında terminal normal mod kontrolü eksik."); // no_std log


        Ok(()) // Başarı
    }

    // Ekranı mevcut duruma göre çizme fonksiyonu (Placeholder).
    // İçeride Sahne64 terminal kontrol API'larını kullanmalıdır.
    fn draw_screen(&self) -> Result<(), TuiError> { // Result<(), TuiError> olmalı
         // Sahne64 API Eksikliği: Ekranı temizleme, imleç taşıma, renk ayarlama
         // resource::control komutları veya benzeri API'lar gerekli.

         info!("Ekran çiziliyor (Placeholder). Seçili menü: {}", self.selected_menu); // no_std log

         // Basit metin yazma ile ekran çizimini simüle edelim (önceki srctui.rs'deki gibi).
         // Temizleme ve imleç taşıma yapılamadığı için çıktı birikerek gidecektir.

          let items_to_display = match self.selected_menu {
              1 => vec!["Paketleri Listele".to_owned(), "Paket Ekle".to_owned(), "Paket Kaldır".to_owned(), "Çıkış".to_owned()],
              // Diğer menü seçenekleri için farklı öğeler dönebilir veya boş kalabilir.
              _ => vec![],
          };

          let mut lines_to_draw = Vec::new(); // alloc
          lines_to_draw.push(format!("Paket Yöneticisi Arayüzü (Menü {})\n", self.selected_menu)); // alloc
          lines_to_draw.push("--------------------------\n".to_owned()); // alloc

          for (i, item) in items_to_display.iter().enumerate() { // iter, enumerate no_std
              let prefix = if (i + 1) as u8 == self.selected_menu { "> " } else { "  " }; // no_std logic
              lines_to_draw.push(format!("{}{}\n", prefix, item)); // alloc
          }
           lines_to_draw.push("\nTuşlar: ↑↓ Enter, Sayılar 1-4, q/Q/Esc\n".to_owned()); // alloc


         // Konsol Kaynağına yaz (buffer kullanarak).
         // write_string_to_resource helper'ını kullanamayız çünkü truncate etmemeli.
         // Manuel yazma döngüsü gerekli.
         let mut output_buffer = Vec::new(); // alloc
         for line in lines_to_draw {
             output_buffer.extend_from_slice(line.as_bytes()); // extend_from_slice alloc
         }

         let buffer_to_write = output_buffer.as_slice(); // Vec<u8> -> &[u8]
         let mut written = 0;
         while written < buffer_to_write.len() {
             match resource::write(self.console_output_handle, &buffer_to_write[written..]) {
                  Ok(bytes_written) => {
                       if bytes_written == 0 {
                            error!("draw_screen: Kaynak yazma hatası: Kaynak yazmayı durdurdu."); // no_std log
                           return Err(TuiError::Sahne64ResourceError(SahneError::InvalidOperation));
                       }
                       written += bytes_written;
                  }
                  Err(e) => {
                       error!("draw_screen: Kaynak yazma hatası: {:?}", e); // no_std log
                      return Err(TuiError::from(e)); // SahneError -> TuiError
                  }
             }
         }

        Ok(()) // Başarı
    }

    // Paket listeleme ekranı (Placeholder).
    // Sahne64 terminal kontrol ve veri gösterme API'larını kullanmalıdır.
    fn list_packages_screen(&self) -> Result<(), TuiError> { // Result<(), TuiError> olmalı
         // Sahne64 API Eksikliği: Ekranı temizleme, başlık çizme, veri gösterme
         // resource::control komutları ve belki paket veritabanı modülü erişimi gerekli.

         info!("Paket listeleme ekranı (Placeholder)."); // no_std log

         // Ekranı temizle ve başlık çiz (placeholder simülasyonu)
          self.clear_and_draw_title("Paketleri Listele")?; // Bu helper da placeholder


         // Paket listesini al (varsayımsal olarak paket veritabanından veya depodan)
          let packages = crate::database::get_installed_packages()?; // Varsayımsal database çağrısı
          let packages_from_repo = crate::repository::get_available_packages()?; // Varsayımsal repository çağrısı

         let items_to_display = vec!["Paket A@1.0".to_owned(), "Paket B@2.1".to_owned()]; // Simüle edilmiş paket listesi (alloc)

         // Paketleri ekrana yaz.
          let mut output_buffer = Vec::new(); // alloc
          output_buffer.extend_from_slice(b"Kurulu Paketler:\n"); // alloc
          for item in items_to_display {
               output_buffer.extend_from_slice(format!("- {}\n", item).as_bytes()); // alloc
          }


         let buffer_to_write = output_buffer.as_slice(); // Vec<u8> -> &[u8]
         let mut written = 0;
         while written < buffer_to_write.len() {
             match resource::write(self.console_output_handle, &buffer_to_write[written..]) {
                  Ok(bytes_written) => {
                       if bytes_written == 0 {
                            error!("list_packages_screen: Kaynak yazma hatası: Kaynak yazmayı durdurdu."); // no_std log
                           return Err(TuiError::Sahne64ResourceError(SahneError::InvalidOperation));
                       }
                       written += bytes_written;
                  }
                  Err(e) => {
                       error!("list_packages_screen: Kaynak yazma hatası: {:?}", e); // no_std log
                      return Err(TuiError::from(e)); // SahneError -> TuiError
                  }
             }
         }


         // Kullanıcıdan devam etmek için tuşa basmasını bekle (Placeholder).
         // Sahne64 API Eksikliği: Terminal girdi okuma (blocking read veya poll).
          self.wait_for_key_press()?; // Bu helper da placeholder

        Ok(()) // Başarı
    }

    // Paket ekleme ekranı (Placeholder).
    fn add_package_screen(&self) -> Result<(), TuiError> { // Result<(), TuiError> olmalı
        info!("Paket ekleme ekranı (Placeholder)."); // no_std log
         self.clear_and_draw_title("Paket Ekle")?; // Placeholder helper

         // Kullanıcıdan paket adı/URL'si girmesini bekle (Placeholder).
         // Sahne64 API Eksikliği: Terminal girdi okuma (satır okuma).
          let input_package_name_or_url = "ornek_paket@1.0.0"; // Simüle edilmiş girdi stringi

         // Paket ekleme mantığı (Placeholder).
          crate::installer::install_package(input_package_name_or_url)?; // Varsayımsal installer çağrısı


         // Başarı/hata mesajını göster (Placeholder).
          let message = format!("Simüle edilmiş paket ekleme: {}", input_package_name_or_url); // alloc
          let buffer_to_write = format!("{}\n", message).as_bytes(); // alloc

          let mut written = 0;
          while written < buffer_to_write.len() {
              match resource::write(self.console_output_handle, &buffer_to_write[written..]) {
                   Ok(bytes_written) => {
                        if bytes_written == 0 {
                             error!("add_package_screen: Kaynak yazma hatası: Kaynak yazmayı durdurdu."); // no_std log
                            return Err(TuiError::Sahne64ResourceError(SahneError::InvalidOperation));
                        }
                        written += bytes_written;
                   }
                   Err(e) => {
                        error!("add_package_screen: Kaynak yazma hatası: {:?}", e); // no_std log
                       return Err(TuiError::from(e)); // SahneError -> TuiError
                   }
              }
          }

         self.wait_for_key_press()?; // Placeholder helper

        Ok(()) // Başarı
    }

    // Paket kaldırma ekranı (Placeholder).
    fn remove_package_screen(&self) -> Result<(), TuiError> { // Result<(), TuiError> olmalı
        info!("Paket kaldırma ekranı (Placeholder)."); // no_std log
         self.clear_and_draw_title("Paket Kaldır")?; // Placeholder helper

         // Kullanıcıdan kaldırılacak paket adı girmesini bekle (Placeholder).
         // Sahne64 API Eksikliği: Terminal girdi okuma (satır okuma).
          let input_package_name = "ornek_paket"; // Simüle edilmiş girdi stringi

         // Paket kaldırma mantığı (Placeholder).
          crate::installer::remove_package(input_package_name)?; // Varsayımsal installer çağrısı

         // Başarı/hata mesajını göster (Placeholder).
          let message = format!("Simüle edilmiş paket kaldırma: {}", input_package_name); // alloc
          let buffer_to_write = format!("{}\n", message).as_bytes(); // alloc

          let mut written = 0;
          while written < buffer_to_write.len() {
              match resource::write(self.console_output_handle, &buffer_to_write[written..]) {
                   Ok(bytes_written) => {
                        if bytes_written == 0 {
                             error!("remove_package_screen: Kaynak yazma hatası: Kaynak yazmayı durdurdu."); // no_std log
                            return Err(TuiError::Sahne64ResourceError(SahneError::InvalidOperation));
                        }
                        written += bytes_written;
                   }
                   Err(e) => {
                        error!("remove_package_screen: Kaynak yazma hatası: {:?}", e); // no_std log
                       return Err(TuiError::from(e)); // SahneError -> TuiError
                   }
              }
          }

         self.wait_for_key_press()?; // Placeholder helper

        Ok(()) // Başarı
    }

    // Ekranı temizleme ve başlık çizme yardımcı fonksiyonu (Placeholder).
    // Sahne64 terminal kontrol API'larını kullanmalıdır.
    fn clear_and_draw_title(&self, title: &str) -> Result<(), TuiError> { // Result<(), TuiError> olmalı
         // Sahne64 API Eksikliği: Ekranı temizleme, imleç taşıma, renk ayarlama
         // resource::control komutları gerekli.
         info!("Ekran temizleniyor ve başlık çiziliyor (Placeholder): {}", title); // no_std log

         // Basit metin çıktısı ile simüle edelim (temizleme ve imleç taşıma yapılamayacaktır).
          let output = format!("--- {} ---\n\n", title); // alloc
          let buffer_to_write = output.as_bytes();

          let mut written = 0;
          while written < buffer_to_write.len() {
              match resource::write(self.console_output_handle, &buffer_to_write[written..]) {
                   Ok(bytes_written) => {
                        if bytes_written == 0 {
                            error!("clear_and_draw_title: Kaynak yazma hatası: Kaynak yazmayı durdurdu."); // no_std log
                           return Err(TuiError::Sahne64ResourceError(SahneError::InvalidOperation));
                       }
                       written += bytes_written;
                   }
                   Err(e) => {
                        error!("clear_and_draw_title: Kaynak yazma hatası: {:?}", e); // no_std log
                       return Err(TuiError::from(e)); // SahneError -> TuiError
                   }
              }
          }

         Ok(()) // Başarı (Simülasyon)
    }


    // Kullanıcıdan bir tuşa basmasını bekleyen yardımcı fonksiyon (Placeholder).
    // Sahne64 terminal girdi API'larını kullanmalıdır.
    fn wait_for_key_press(&self) -> Result<(), TuiError> { // Result<(), TuiError> olmalı
         // Sahne64 API Eksikliği: Terminal girdi okuma (blocking read veya poll).
         // resource::read(self.console_input_handle, &mut buffer) Blocking okuma
         // resource::poll(self.console_input_handle, timeout) ile bekleme

         info!("Bir tuşa basılması bekleniyor (Placeholder)."); // no_std log

         // Basit metin çıktısı ile kullanıcıya bilgi verelim.
          let message = "\nDevam etmek için bir tuşa basın...\n"; // &str no_std
          let buffer_to_write = message.as_bytes();

          let mut written = 0;
          while written < buffer_to_write.len() {
              match resource::write(self.console_output_handle, &buffer_to_write[written..]) {
                   Ok(bytes_written) => {
                        if bytes_written == 0 {
                           error!("wait_for_key_press: Kaynak yazma hatası: Kaynak yazmayı durdurdu."); // no_std log
                           return Err(TuiError::Sahne64ResourceError(SahneError::InvalidOperation));
                       }
                       written += bytes_written;
                   }
                   Err(e) => {
                        error!("wait_for_key_press: Kaynak yazma hatası: {:?}", e); // no_std log
                       return Err(TuiError::from(e)); // SahneError -> TuiError
                   }
              }
          }


         // Varsayımsal blocking read çağrısı
           let mut input_buffer = [0u8; 1]; // Sadece 1 bayt oku (tuş)
           match resource::read(self.console_input_handle, &mut input_buffer) {
               Ok(_) => { info!("Tuş basıldı, devam ediliyor."); Ok(()) }
               Err(e) => {
                    error!("Tuş girdisi okuma hatası: {:?}", e);
                    Err(TuiError::TerminalInputError(format!("Girdi okuma hatası: {:?}", e))) // alloc
               }
           }
         // Şu an için sadece başarı dönelim, çünkü okuma implemente değil.
         warn!("UYARI: Sahne64 API'sında terminal girdi okuma eksik. Tuş girdisi gerçekte beklenmiyor."); // no_std log
         Ok(()) // Simülasyon olarak başarı dön
    }

    // UI sona erdiğinde kaynakları serbest bırak.
    // Drop implementasyonu veya explicit close/shutdown fonksiyonu kullanılabilir.
    // Basitlik adına Drop implementasyonu ekleyelim.
}

impl Drop for PackageManagerUI {
    fn drop(&mut self) {
        info!("UI kaynakları serbest bırakılıyor."); // no_std log
        // Handle'ları serbest bırak. Hata durumunda paniklemeyelim.
        let release_output_result = resource::release(self.console_output_handle);
         if let Err(e) = release_output_result {
              error!("Konsol çıktı Handle release hatası: {:?}", e); // no_std log
         }

        let release_input_result = resource::release(self.console_input_handle);
         if let Err(e) = release_input_result {
              error!("Konsol girdi Handle release hatası: {:?}", e); // no_std log
         }

        // Sahne64 API Eksikliği: Terminali normal moda döndürme (disable_raw_mode)
        // Bu da burada yapılmalıdır.
         // match resource::control(self.console_input_handle, resource::CMD_DISABLE_RAW_MODE, &[]) { /* ... */ }
         info!("UYARI: Sahne64 API'sında terminal normal mod kontrolü eksik (Drop)."); // no_std log
    }
}

// --- Simüle Edilmiş Klavye Girdi Türleri (Sahne64 API'sında olması gerekenlere örnek) ---
// Sahne64 terminal girdi Kaynağı bayt akışı yerine yapısal olaylar döndürüyorsa kullanılabilir.
// Veya raw baytları bu türlere çevirme mantığı gereklidir.
#[derive(Debug)] // Debug derive'ı no_std'de çalışır
struct SimulatedKeyEvent {
    code: SimulatedKeyCode,
    // modifiers: SimulatedKeyModifiers, // Shift, Ctrl, Alt gibi modifiyeler
    // ... diğer olay detayları (mouse konumu vb.) ...
}

#[derive(Debug, PartialEq, Eq)] // Debug, PartialEq, Eq derive'ları no_std'de çalışır
enum SimulatedKeyCode {
    Up,
    Down,
    Enter,
    Esc,
    Char(char), // char Copy derive no_std
     F(u8), // F1-F12 gibi fonksiyon tuşları
    // Home, End, PageUp, PageDown, Delete, Insert ...
    // ... diğer özel tuşlar ...
}

 #[derive(Debug, PartialEq, Eq)] // Debug, PartialEq, Eq derive'ları no_std'de çalışır
 struct SimulatedKeyModifiers {
     shift: bool, // bool Copy derive no_std
     ctrl: bool, // bool Copy derive no_std
      alt: bool, // bool Copy derive no_std
//     // ...
 }


// #[cfg(test)] bloğu std test runner'ı ve std bağımlılıkları gerektirir.
// Testler için mock resource ve task veya Sahne64 simülasyonu gereklidir.

#[cfg(test)]
mod tests {
    // std::io, std::time, crossterm, std::string, std::vec kullandığı için no_std'de doğrudan çalışmaz.
    // Mock resource (console output/input), task (spawn, sleep) ve SimulatedKeyEvent üretme/işleme altyapısı gerektirir.
}

// --- TuiError enum tanımı ---
// Bu dosyanın başında tanımlanmıştır ve no_std uyumludur.


// --- PaketYoneticisiHatasi enum tanımı ---
// srcerror.rs modülünde tanımlanmıştır ve no_std uyumludur.
// TuiError'dan PaketYoneticisiHatasi'na dönüşüm için From implementasyonu eklenmelidir.

// srcerror.rs (Örnek - no_std uyumlu, TuiError'dan dönüşüm eklenmiş)
#![no_std]
extern crate alloc;

// ... diğer importlar ...

use crate::srcui::TuiError; // TuiError'ı içe aktar

#[derive(Debug)]
pub enum PaketYoneticisiHatasi {
    // ... diğer hatalar ...

    // UI işlemleri sırasında oluşan hatalar
    TuiError(TuiError), // TuiError'ı sarmalar

    // ... diğer hatalar ...
}

// TuiError'dan PaketYoneticisiHatasi'na dönüşüm
impl From<TuiError> for PaketYoneticisiHatasi {
    fn from(err: TuiError) -> Self {
        PaketYoneticisiHatasi::TuiError(err)
    }
}

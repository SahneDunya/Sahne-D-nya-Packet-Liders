#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // String, Vec, format! için

use alloc::string::{String, ToString}; // std::string::String yerine
use alloc::vec::Vec; // std::vec::Vec yerine
use alloc::format; // format! makrosu için
use alloc::borrow::ToOwned; // to_string().repeat() yerine

// Sahne64 API modülleri
use crate::resource; // Çıktı için
use crate::task; // Zaman bilgisi için
use crate::SahneError; // Sahne64 hata türü
use crate::Handle; // Kaynak Handle'ları

// Özel hata enum'ımızı içe aktar (no_std uyumlu hali)
use crate::paket_yoneticisi_hata::PaketYoneticisiHatasi;
// SahneError'dan dönüşüm From implementasyonu ile sağlanacak

pub struct ProgressBar {
    total: usize,
    current: usize,
    width: usize,
    // std::time::Instant yerine Sahne64'ün sağladığı zaman türü (örn. mikrosaniye cinsinden u64)
    start_time_us: u64, // Başlangıç zamanı mikrosaniye olarak (task::current_time_us varsayımıyla)
    message: String, // İsteğe bağlı mesaj (String alloc gerektirir)
    filled_char: char, // Dolu kısım için karakter
    empty_char: char, // Boş kısım için karakter
    show_percentage: bool, // Yüzdeyi gösterip göstermeme
    show_time: bool, // Geçen süreyi gösterip göstermeme
}

impl ProgressBar {
    // Yeni bir varsayılan ilerleme çubuğu örneği oluşturur.
    pub fn new(total: usize, width: usize) -> Result<Self, PaketYoneticisiHatasi> { // Sahne64 zamanını alırken hata dönebilir
        // Başlangıç zamanını Sahne64 API'sından al
        let start_time_us = task::current_time_us().map_err(|e| {
             eprintln!("ProgressBar::new: Zaman bilgisi alınamadı: {:?}", e);
            PaketYoneticisiHatasi::SahneApiHatasi(e) // SahneError -> PaketYoneticisiHatasi
        })?;

        Ok(ProgressBar {
            total,
            current: 0,
            width,
            start_time_us,
            message: String::new(), // alloc gerektirir
            filled_char: '#',
            empty_char: ' ',
            show_percentage: true,
            show_time: false,
        })
    }

    // Özel karakterler ve mesaj ayarları ile yeni ilerleme çubuğu oluşturur.
    pub fn with_config(
        total: usize,
        width: usize,
        filled_char: char,
        empty_char: char,
        show_percentage: bool,
        show_time: bool,
    ) -> Result<Self, PaketYoneticisiHatasi> { // Sahne64 zamanını alırken hata dönebilir
        let start_time_us = task::current_time_us().map_err(|e| {
             eprintln!("ProgressBar::with_config: Zaman bilgisi alınamadı: {:?}", e);
             PaketYoneticisiHatasi::SahneApiHatasi(e) // SahneError -> PaketYoneticisiHatasi
        })?;

        Ok(ProgressBar {
            total,
            current: 0,
            width,
            start_time_us,
            message: String::new(), // alloc gerektirir
            filled_char,
            empty_char,
            show_percentage,
            show_time,
        })
    }

    // İlerlemeyi artırır ve çubuğu belirtilen konsol Handle'ına çizer.
    // console_output_handle: Çıktı yazılacak konsol Kaynağının Handle'ı.
    pub fn update(&mut self, increment: usize, console_output_handle: Handle) -> Result<(), PaketYoneticisiHatasi> { // SahneError yerine PaketYoneticisiHatasi
        self.current += increment;
        // Toplamı aşmaması için kontrol eklenebilir
        if self.current > self.total {
             self.current = self.total;
        }
        self.draw(console_output_handle)?; // console Handle'ını draw'a geçir
        Ok(())
    }

    // Mevcut ilerlemeyi doğrudan ayarlar ve belirtilen konsol Handle'ına çizer.
    // console_output_handle: Çıktı yazılacak konsol Kaynağının Handle'ı.
    pub fn set_current(&mut self, current: usize, console_output_handle: Handle) -> Result<(), PaketYoneticisiHatasi> { // SahneError yerine PaketYoneticisiHatasi
        self.current = current;
         // Toplamı aşmaması için kontrol
         if self.current > self.total {
             self.current = self.total;
         }
        self.draw(console_output_handle)?; // console Handle'ını draw'a geçir
        Ok(())
    }

    // Mesajı ayarlar veya günceller.
    pub fn set_message(&mut self, message: &str) {
        self.message = message.to_string(); // to_string() alloc gerektirir
    }

    // İlerleme çubuğunu belirtilen konsol Handle'ına çizer.
    // console_output_handle: Çıktı yazılacak konsol Kaynağının Handle'ı.
    fn draw(&self, console_output_handle: Handle) -> Result<(), PaketYoneticisiHatasi> { // SahneError yerine PaketYoneticisiHatasi
        // Yüzde hesapla (total 0 olabilir)
        let percent = if self.total > 0 { (self.current as f64 / self.total as f64) * 100.0 } else { 0.0 };
        // Dolu ve boş karakter sayısını hesapla
        let filled = (self.width as f64 * (percent / 100.0)).round() as usize; // Yuvarlama eklendi
        let empty = self.width.saturating_sub(filled); // Negatif olmaması için saturating_sub

        // Geçen süreyi hesapla (Sahne64 zamanını kullanarak)
        let current_time_us = task::current_time_us().map_err(|e| {
             eprintln!("ProgressBar::draw: Zaman bilgisi alınamadı: {:?}", e);
             PaketYoneticisiHatasi::SahneApiHatasi(e) // SahneError -> PaketYoneticisiHatasi
        })?;
        let elapsed_us = current_time_us.saturating_sub(self.start_time_us); // Negatif olmaması için saturating_sub
        let elapsed_s = elapsed_us as f64 / 1_000_000.0; // Mikrosaniye -> saniye

        // Süre stringi oluştur
        let time_str = if self.show_time {
            format!(" ({:.2}s)", elapsed_s) // format! alloc gerektirir. f64 formatlama desteği var mı?
        } else {
            String::new() // alloc gerektirir
        };

        // Yüzde stringi oluştur
        let percentage_str = if self.show_percentage {
            format!(" {}%", percent as usize) // format! alloc gerektirir
        } else {
            String::new() // alloc gerektirir
        };

        // Mesaj prefixi oluştur
        let message_prefix = if !self.message.is_empty() {
            format!("{}: ", self.message) // format! alloc gerektirir
        } else {
            String::new() // alloc gerektirir
        };

        // İlerleme çubuğu stringini oluştur
        let output = format!(
            "\r{}{}[{}{}]{}{}", // \r satır başına döner (console Kaynağı destekliyorsa)
            message_prefix, // String (alloc)
            "", // Sabit açılış parantezi yerine boş bıraktık
            self.filled_char.to_string().repeat(filled), // String (alloc), repeat alloc
            self.empty_char.to_string().repeat(empty), // String (alloc), repeat alloc
            "", // Sabit kapanış parantezi yerine boş bıraktık
            percentage_str, // String (alloc)
            time_str, // String (alloc)
        );

        // Çıktı Kaynağına yaz
        // console_output_handle Handle'ını kullan
        let buffer = output.as_bytes(); // String -> &[u8]
        let write_result = resource::write(console_output_handle, buffer); // Yazma

        // resource::write tek seferde tamamlamayabilir, loop gerekebilir (interactive.rs get_input'taki gibi)
        // veya resource::write'ın tamamını yazdığını varsayalım.
         loop { ... resource::write(handle, &buffer[written..]) ... }

        write_result.map_err(|e| {
             eprintln!("ProgressBar::draw: Kaynak yazma hatası: {:?}", e); // no_std print
            PaketYoneticisiHatasi::from(e) // SahneError -> PaketYoneticisiHatasi
        })?; // Hata durumunda ? ile yay


        // flush gerekli mi? resource::write flush yapmıyor olabilir.
        // Eğer console Kaynağı flush komutunu destekliyorsa:
         match resource::control(console_output_handle, RESOURCE_CONTROL_CMD_FLUSH, &[]) { // Varsayımsal flush komutu
             Ok(_) => {}
             Err(e) => { eprintln!("ProgressBar::draw: Kaynak flush hatası: {:?}", e); } // Logla
         }

        Ok(()) // Başarı
    }

    // İlerleme tamamlandığında çağrılır. Yeni satıra geçer.
    // console_output_handle: Çıktı yazılacak konsol Kaynağının Handle'ı.
    pub fn finish(&self, console_output_handle: Handle) -> Result<(), PaketYoneticisiHatasi> { // SahneError yerine PaketYoneticisiHatasi
        // İlerleme çubuğunu %100 olarak son kez çiz (eğer henüz %100 değilse)
         this.draw(console_output_handle)?; // Current total'a eşitlenmemiş olabilir

        // Yeni satıra geçmek için sadece bir newline karakteri yaz.
        let newline_buffer = b"\n"; // byte slice

        let write_result = resource::write(console_output_handle, newline_buffer); // Yazma

        write_result.map_err(|e| {
             eprintln!("ProgressBar::finish: Kaynak yazma hatası: {:?}", e); // no_std print
             PaketYoneticisiHatasi::from(e) // SahneError -> PaketYoneticisiHatasi
        })?; // Hata durumunda ? ile yay

        // Flush gerekli mi?
         match resource::control(console_output_handle, RESOURCE_CONTROL_CMD_FLUSH, &[]) { // Varsayımsal flush komutu
             Ok(_) => {}
             Err(e) => { eprintln!("ProgressBar::finish: Kaynak flush hatası: {:?}", e); } // Logla
         }

        Ok(()) // Başarı
    }
}

// #[cfg(test)] bloğu std test runner'ı gerektirir.
// No_std ortamında testler için özel bir test runner veya std feature flag'i gerekir.
// Bu blok no_std ortamında derlenmeyecektir eğer std feature aktif değilse.

#[cfg(test)]
mod tests {
    // std::thread, std::time::Duration kullandığı için no_std'de çalışmaz.
    // Testler, Sahne64 task::sleep ve zaman fonksiyonlarını kullanacak şekilde yeniden yazılmalı.
    // Ayrıca çıktı yakalama veya mock resource kullanma gerektirir.
}

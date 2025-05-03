#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // String, Vec, format! için

// Sahne64 API modülleri
use crate::resource; // Kaynak işlemleri (yazma)
use crate::SahneError; // Sahne64 hata türü
use crate::Handle; // Kaynak Handle'ları

// String and Vec from alloc
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format; // format! makrosu için

// log kütüphanesini içe aktar (no_std uyumlu backend varsayımıyla)
 use log::{info, warn, error, debug}; // TUI modülü genellikle kendi çıktısını üretir, loglama dahili hatalar için olabilir.

// no_std uyumlu print makroları (dahili hataları loglamak için kullanılabilir)
use crate::print_macros::{println, eprintln};


// TUI (Text User Interface) hatalarını temsil eden enum (no_std uyumlu).
// Debug ve Display manuel implementasyonları.
#[derive(Debug)] // Debug derive'ı no_std'de çalışır
pub enum TuiError {
    // Sahne64 Kaynak (Konsol Çıktısı) Hatası
    Sahne64ResourceError(SahneError), // SahneError'ı sarmalar

    // Diğer TUI ile ilgili hatalar (örn. terminal boyutu alma hatası, renk hatası - eğer implement edilirse)
     TerminalControlError(String), // String alloc gerektirir
}

// core::fmt::Display implementasyonu (kullanıcı dostu mesajlar için)
impl core::fmt::Display for TuiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TuiError::Sahne64ResourceError(e) => write!(f, "Sahne64 Kaynak hatası: {:?}", e),
             TuiError::TerminalControlError(s) => write!(f, "Terminal kontrol hatası: {}", s),
        }
    }
}

// From implementasyonu
impl From<SahneError> for TuiError {
    fn from(err: SahneError) -> Self {
        TuiError::Sahne64ResourceError(err)
    }
}


// TUI yapısı std'de bulunan tui crate'i gibi karmaşık bir arayüzü temsil etmez.
// Bu yapı, Sahne64'ün resource mekanizmasını kullanarak konsola basit metin çıktısı
// veren işlevselliği sağlar. Daha gelişmiş TUI için Sahne64'e özel bir terminal backend'i
// geliştirilmesi gerekir.


// Öğeleri Sahne64'ün konsol çıktı Kaynağına basitçe yazar.
// items: Konsola yazılacak string dilimi.
// console_output_handle: Çıktı yazılacak konsol Kaynağının Handle'ı.
// Dönüş değeri: Başarı veya TuiError (Kaynak yazma hatası).
pub fn draw_sahne64_tui(items: &[String], console_output_handle: Handle) -> Result<(), TuiError> { // console_output_handle: Handle eklendi
    // Stdout dosya tanımlayıcısı (fd 1) yerine doğrudan Kaynak Handle'ını kullanıyoruz.
     let stdout_fd = 1; // Bu artık kullanılmıyor.

    for item in items { // items: &[String] no_std+alloc uyumlu
        let line = format!("{}\n", item); // format! alloc
        let buffer = line.as_bytes(); // String -> &[u8]

        // Kaynak Handle'ına yaz. resource::write tek seferde yazmayabilir, loop gerekebilir.
        let mut written = 0;
        while written < buffer.len() {
            match resource::write(console_output_handle, &buffer[written..]) {
                Ok(bytes_written) => {
                    if bytes_written == 0 {
                        // Hiçbir şey yazılamadı, Kaynak hatası olabilir
                         eprintln!("TUI Kaynağı yazma hatası: Kaynak yazmayı durdurdu."); // no_std print
                         return Err(TuiError::Sahne64ResourceError(SahneError::InvalidOperation)); // Veya daha uygun bir hata
                    }
                    written += bytes_written;
                }
                Err(e) => {
                     // Yazma hatası
                     eprintln!("TUI Kaynağı yazma hatası: {:?}", e); // no_std print
                    return Err(TuiError::from(e)); // SahneError -> TuiError
                }
            }
        }
    }
    Ok(()) // Başarı
}

// #[cfg(test)] bloğu std test runner'ı ve std bağımlılıkları gerektirir.
// Testler için mock resource veya Sahne64 simülasyonu gereklidir.

#[cfg(test)]
mod tests {
    // std::io, std::string, std::vec kullandığı için no_std'de doğrudan çalışmaz.
    // Mock resource::write ve çıktı yakalama/kontrol mekanizması gerektirir.
}

// --- PaketYoneticisiHatasi enum tanımı ---
// srcerror.rs modülünde tanımlanmıştır ve no_std uyumludur.
// TuiError'dan PaketYoneticisiHatasi'na dönüşüm için From implementasyonu eklenmelidir.

// srcerror.rs (Örnek - no_std uyumlu, TuiError'dan dönüşüm eklenmiş)
#![no_std]
extern crate alloc;

// ... diğer importlar ...

use crate::srctui::TuiError; // TuiError'ı içe aktar

#[derive(Debug)]
pub enum PaketYoneticisiHatasi {
    // ... diğer hatalar ...

    // TUI işlemleri sırasında oluşan hatalar
    TuiError(TuiError), // TuiError'ı sarmalar

    // ... diğer hatalar ...
}

// TuiError'dan PaketYoneticisiHatasi'na dönüşüm
impl From<TuiError> for PaketYoneticisiHatasi {
    fn from(err: TuiError) -> Self {
        PaketYoneticisiHatasi::TuiError(err)
    }
}

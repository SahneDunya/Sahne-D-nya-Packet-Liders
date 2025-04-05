use std::io::{self, Write};
use std::time::Instant;

// Assuming 'Sahne64' modules are in the same crate
use crate::fs;
use crate::SahneError; // Assuming SahneError is accessible here

// Geliştirilmiş bir ilerleme çubuğu yapısı
pub struct ProgressBar {
    total: usize,
    current: usize,
    width: usize,
    start_time: Instant, // Başlangıç zamanını kaydet
    message: String,        // İsteğe bağlı mesaj
    filled_char: char,        // Dolu kısım için karakter
    empty_char: char,         // Boş kısım için karakter
    show_percentage: bool, // Yüzdeyi gösterip göstermeme
    show_time: bool,          // Geçen süreyi gösterip göstermeme
}

impl ProgressBar {
    pub fn new(total: usize, width: usize) -> Self {
        ProgressBar {
            total,
            current: 0,
            width,
            start_time: Instant::now(), // Başlangıç zamanını ayarla
            message: String::new(),         // Başlangıçta boş mesaj
            filled_char: '#',                 // Varsayılan dolu karakter
            empty_char: ' ',                  // Varsayılan boş karakter
            show_percentage: true,          // Yüzde varsayılan olarak gösterilir
            show_time: false,                 // Süre varsayılan olarak gösterilmez
        }
    }

    // Özel karakterler ve mesaj ile yeni ilerleme çubuğu
    pub fn with_config(
        total: usize,
        width: usize,
        filled_char: char,
        empty_char: char,
        show_percentage: bool,
        show_time: bool,
    ) -> Self {
        ProgressBar {
            total,
            current: 0,
            width,
            start_time: Instant::now(),
            message: String::new(),
            filled_char,
            empty_char,
            show_percentage,
            show_time,
        }
    }

    // İlerlemeyi artırır ve çubuğu günceller
    pub fn update(&mut self, increment: usize) -> Result<(), SahneError> {
        self.current += increment;
        self.draw()?;
        Ok(())
    }

    // Mevcut ilerlemeyi doğrudan ayarlar
    pub fn set_current(&mut self, current: usize) -> Result<(), SahneError> {
        self.current = current;
        self.draw()?;
        Ok(())
    }

    // Mesajı ayarlar veya günceller
    pub fn set_message(&mut self, message: &str) {
        self.message = message.to_string();
    }

    // İlerleme çubuğunu çizer
    fn draw(&self) -> Result<(), SahneError> {
        let percent = if self.total > 0 { (self.current as f64 / self.total as f64) * 100.0 } else { 0.0 };
        let filled = (self.width as f64 * (percent / 100.0)) as usize;
        let empty = self.width - filled;

        let elapsed_time = self.start_time.elapsed();
        let time_str = if self.show_time {
            format!(" ({:.2}s)", elapsed_time.as_secs_f64())
        } else {
            String::new()
        };

        let percentage_str = if self.show_percentage {
            format!(" {}%", percent as usize)
        } else {
            String::new()
        };

        let message_prefix = if !self.message.is_empty() {
            format!("{}: ", self.message)
        } else {
            String::new()
        };

        let output = format!(
            "\r{}{}[{}{}]{}{}",
            message_prefix,
            "[", // Sabit açılış parantezi
            self.filled_char.to_string().repeat(filled),
            self.empty_char.to_string().repeat(empty),
            "]", // Sabit kapanış parantezi
            percentage_str,
            time_str,
        );

        let stdout_fd = 1; // Assuming file descriptor 1 is standard output
        match fs::open(stdout_fd, fs::O_WRONLY) {
            Ok(fd) => {
                let buffer = output.as_bytes();
                let mut written = 0;
                while written < buffer.len() {
                    match fs::write(fd, &buffer[written..]) {
                        Ok(bytes) => written += bytes,
                        Err(e) => {
                            fs::close(fd).unwrap_or_default();
                            return Err(SahneError::from(e)); // Convert std::io::Error to SahneError
                        }
                    }
                }
                fs::close(fd).unwrap_or_default();
                Ok(())
            }
            Err(e) => Err(SahneError::from(e)), // Convert std::io::Error to SahneError
        }
    }

    // İlerleme tamamlandığında çağrılır
    pub fn finish(&self) -> Result<(), SahneError> {
        let newline = "\n";
        let stdout_fd = 1;
        match fs::open(stdout_fd, fs::O_WRONLY | fs::O_APPEND) {
            Ok(fd) => {
                let buffer = newline.as_bytes();
                fs::write(fd, buffer).map_err(SahneError::from)?;
                fs::close(fd).unwrap_or_default();
                Ok(())
            }
            Err(e) => Err(SahneError::from(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_progress_bar() {
        let mut progress_bar = ProgressBar::new(100, 50);
        progress_bar.set_message("İşlem Devam Ediyor"); // Mesaj ayarla
        for i in 0..100 {
            progress_bar.update(1).unwrap();
            thread::sleep(Duration::from_millis(50)); // Daha iyi görselleştirme için yavaşlat
            if i == 50 {
                progress_bar.set_message("Yarısına Gelindi"); // Mesajı güncelle
            }
        }
        progress_bar.finish().unwrap();
    }

    #[test]
    fn test_custom_progress_bar() {
        let mut progress_bar = ProgressBar::with_config(
            200,
            30,
            '=', // Dolu karakteri değiştir
            '.', // Boş karakteri değiştir
            true,  // Yüzdeyi göster
            true,  // Süreyi göster
        );
        progress_bar.set_message("Özel Çubuk Testi");
        for _ in 0..200 {
            progress_bar.update(1).unwrap();
            thread::sleep(Duration::from_millis(25));
        }
        progress_bar.finish().unwrap();
    }

    #[test]
    fn test_set_current_progress_bar() {
        let mut progress_bar = ProgressBar::new(100, 40);
        progress_bar.set_message("Doğrudan Ayarlama Testi");
        progress_bar.set_current(30).unwrap(); // İlerlemeyi doğrudan ayarla
        thread::sleep(Duration::from_secs(1));
        progress_bar.set_current(70).unwrap(); // İlerlemeyi tekrar ayarla
        thread::sleep(Duration::from_secs(1));
        progress_bar.set_current(100).unwrap(); // Tamamla
        progress_bar.finish().unwrap();
    }

    #[test]
    fn test_no_percentage_progress_bar() {
        let mut progress_bar = ProgressBar::with_config(
            100,
            30,
            '*',
            '-',
            false, // Yüzdeyi gösterme
            true,  // Süreyi göster
        );
        progress_bar.set_message("Yüzdesiz Çubuk");
        for _ in 0..100 {
            progress_bar.update(1).unwrap();
            thread::sleep(Duration::from_millis(30));
        }
        progress_bar.finish().unwrap();
    }

    #[test]
    fn test_zero_total_progress_bar() {
        let mut progress_bar = ProgressBar::new(0, 30); // Toplam 0
        progress_bar.set_message("Sıfır Toplam Testi");
        for _ in 0..50 { // Yine de güncellemeye çalış
            progress_bar.update(1).unwrap();
            thread::sleep(Duration::from_millis(20));
        }
        progress_bar.finish().unwrap(); // Tamamlama sorunsuz olmalı
    }
}
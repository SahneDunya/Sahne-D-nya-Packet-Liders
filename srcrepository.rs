#![no_std] // Eğer bu dosya tek başına derleniyorsa bu satırı ekleyebilirsiniz

use crate::package::Paket;
use crate::fs; // Sahne64 dosya sistemi modülü

pub struct DepoYoneticisi {
    pub depo_url: String,
    pub yerel_depo_yolu: String,
    paket_listesi_cache: Option<Vec<Paket>>, // Paket listesini önbellekte saklamak için
}

impl DepoYoneticisi {
    pub fn yeni(depo_url: String, yerel_depo_yolu: String) -> Self {
        DepoYoneticisi {
            depo_url,
            yerel_depo_yolu,
            paket_listesi_cache: None, // Başlangıçta önbellek boş
        }
    }

    fn _paket_listesini_indir_dahili(&mut self) -> Result<Vec<Paket>, super::SahneError> {
        let depo_url = format!("{}/paketler.json", self.depo_url);
        println!("Paket listesi indirme işlemi şu anda desteklenmiyor. Yerel depodan okunacak: {}", depo_url);
        // Gerçek bir uygulamada ağ modülü ve indirme mekanizması gerekecektir.
        // Şimdilik boş bir Vec dönüyoruz.
        Ok(Vec::new())
    }


    pub fn paket_listesini_indir(&mut self) -> Result<Vec<Paket>, super::SahneError> {
        if let Some(ref paketler) = self.paket_listesi_cache {
            // Önbellekte paket listesi varsa onu döndür
            return Ok(paketler.clone());
        }

        // Önbellekte yoksa indir (şimdilik yerelden oku)
        // let paketler = self._paket_listesini_indir_dahili()?;
        let paketler = self._yerel_paket_listesini_oku()?;
        self.paket_listesi_cache = Some(paketler.clone()); // İndirilen listeyi önbelleğe al
        Ok(paketler)
    }

    fn _yerel_paket_listesini_oku(&self) -> Result<Vec<Paket>, super::SahneError> {
        let yerel_depo_dosyasi = format!("{}/paketler.json", self.yerel_depo_yolu);
        println!("Yerel paket listesi okunuyor: {}", yerel_depo_dosyasi);
        match fs::open(&yerel_depo_dosyasi, fs::O_RDONLY) {
            Ok(fd) => {
                let mut icerik = Vec::new();
                let mut buffer = [0u8; 128];
                loop {
                    match fs::read(fd, &mut buffer) {
                        Ok(bytes_read) => {
                            if bytes_read == 0 {
                                break;
                            }
                            icerik.extend_from_slice(&buffer[..bytes_read]);
                        }
                        Err(e) => {
                            fs::close(fd)?;
                            return Err(e);
                        }
                    }
                }
                fs::close(fd)?;

                // Şu anda basit bir şekilde içeriği UTF-8 stringine dönüştürüyoruz ve
                // elle ayrıştırma yapmamız gerekecek. Gerçek bir JSON ayrıştırıcıya ihtiyacımız var.
                match String::from_utf8(icerik) {
                    Ok(json_str) => {
                        // Basit bir örnek ayrıştırma (gerçekte çok daha karmaşık olabilir)
                        let mut paketler = Vec::new();
                        // Burada JSON ayrıştırma mantığı olmalı.
                        // Şimdilik sadece bir hata mesajı ve boş bir liste dönüyoruz.
                        eprintln!("Uyarı: JSON ayrıştırma henüz tam olarak desteklenmiyor. Elle ayrıştırma gerekebilir: {}", json_str);
                        // Örnek bir paket oluşturma (gerçekte JSON'dan okunmalı)
                        // paketler.push(Paket { ad: "ornek_paket".to_string(), surum: "1.0".to_string(), dosya_adi: Some("ornek_paket.zip".to_string()) });
                        Ok(paketler)
                    }
                    Err(_) => Err(super::SahneError::InvalidParameter), // Geçersiz UTF-8
                }
            }
            Err(e) => {
                eprintln!("Yerel paket listesi dosyası açılamadı: {:?}", e);
                Err(e)
            }
        }
    }

    pub fn yerel_depoyu_guncelle(&mut self) -> Result<(), super::SahneError> {
        let paketler = self.paket_listesini_indir()?;

        let yerel_depo_dosyasi = format!("{}/paketler.json", self.yerel_depo_yolu);
        // Şu anda sadece bellekteki paket listesini yerel dosyaya yazıyoruz.
        // Serileştirme için bir JSON kütüphanesine ihtiyacımız olacak.
        let dosya_icerigi = format!("/* Paket listesi (JSON serileştirme henüz desteklenmiyor) */\n{:?}", paketler);

        match fs::open(&yerel_depo_dosyasi, fs::O_WRONLY | fs::O_CREAT) {
            Ok(fd) => {
                fs::write(fd, dosya_icerigi.as_bytes())?;
                fs::close(fd)?;
                self.paket_listesi_cache = Some(paketler.clone()); // Yerel depoyu güncellediğimizde önbelleği de güncelle
                println!("Yerel depo güncellendi: {}", yerel_depo_dosyasi); // Kullanıcıya bilgi ver
                Ok(())
            }
            Err(e) => {
                eprintln!("Yerel depo dosyası yazılamadı: {:?}", e);
                Err(e)
            }
        }
    }

    pub fn paket_ara(&mut self, paket_adi: &str) -> Result<Option<Paket>, super::SahneError> {
        let paketler = self.paket_listesini_indir()?;
        let bulunan_paket = paketler.into_iter().find(|paket| paket.ad == paket_adi);
        Ok(bulunan_paket)
    }
}
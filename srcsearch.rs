use regex::Regex;
use crate::package::Paket;
use crate::paket_yoneticisi_hata::PaketYoneticisiHata; // Özel hata enum'ımızı içe aktar
use log::{debug, error, trace}; // log kütüphanesini içe aktar

// Assuming 'Sahne64' modules are in the same crate if needed
use crate::fs;
use crate::SahneError;

pub struct AramaYoneticisi {}

impl AramaYoneticisi {
    // Yeni bir AramaYoneticisi örneği oluşturur.
    pub fn yeni() -> Self {
        AramaYoneticisi {}
    }

    // Verilen paket listesinde, arama desenine göre paket arar.
    pub fn paket_ara<'a>(paketler: &'a Vec<Paket>, arama_deseni: &str) -> Result<Vec<&'a Paket>, PaketYoneticisiHata> {
        debug!("Paket araması başlatılıyor. Arama deseni: '{}'", arama_deseni);

        // Arama desenini kullanarak bir Regex nesnesi oluşturmaya çalış
        let regex = Regex::new(arama_deseni).map_err(|e| {
            let hata_mesaji = format!("Geçersiz arama deseni: {}. Hata: {}", arama_deseni, e);
            error!("{}", hata_mesaji); // Hata mesajını logla
            PaketYoneticisiHata::AramaYoneticisiHatasi(hata_mesaji) // Hata durumunda AramaYoneticisiHatasi döndür
        })?;

        trace!("Regex deseni derlendi: '{}'", arama_deseni);

        // Paket listesini filtrele ve arama desenine uyan paketleri topla
        let sonuclar: Vec<&Paket> = paketler
            .iter()
            .filter(|paket| {
                let eslesme_bulundu = regex.is_match(&paket.ad) || paket.aciklama.as_ref().map_or(false, |aciklama| regex.is_match(aciklama));
                trace!("Paket '{}' için arama yapılıyor. Eşleşme bulundu: {}", paket.ad, eslesme_bulundu);
                eslesme_bulundu
            })
            .collect();

        debug!("Arama tamamlandı. {} paket bulundu. Arama deseni: '{}'", sonuclar.len(), arama_deseni);
        Ok(sonuclar) // Arama sonuçlarını döndür
    }

    // **Sahne64 Özel Fonksiyonları Entegrasyonu (Olası Senaryolar)**

    // Eğer paket listesi bir dosyadan okunuyorsa, Sahne64'e özel dosya okuma fonksiyonları kullanılabilir.
    
    pub fn paketleri_yukle() -> Result<Vec<Paket>, PaketYoneticisiHata> {
        let dosya_yolu = "/etc/sahne64/paketler.list";
        match fs::open(dosya_yolu, fs::O_RDONLY) {
            Ok(fd) => {
                let mut paketler = Vec::new();
                let reader = io::BufReader::new(fd);
                for line_result in reader.lines() {
                    match line_result {
                        Ok(line) => {
                            // Paketi satırdan ayrıştırma mantığı
                            let parcalar: Vec<&str> = line.split(',').collect();
                            if parcalar.len() == 2 {
                                let ad = parcalar[0].trim().to_string();
                                let aciklama = Some(parcalar[1].trim().to_string());
                                paketler.push(Paket { ad, aciklama });
                            }
                        }
                        Err(e) => return Err(PaketYoneticisiHata::DosyaOkumaHatasi(e.into())),
                    }
                }
                fs::close(fd).unwrap_or_default();
                Ok(paketler)
            }
            Err(e) => Err(PaketYoneticisiHata::DosyaAcmaHatasi(e.into())),
        }
    }
    */
    // Eğer loglama Sahne64'e özel bir mekanizma kullanıyorsa, log kütüphanesi yerine o kullanılabilir.

    pub fn sahne64_log(seviye: &str, mesaj: &str) {
        match seviye {
            "debug" => crate::kernel::debug_log(mesaj),
            "error" => crate::kernel::error_log(mesaj),
            "trace" => crate::kernel::trace_log(mesaj),
            _ => {}
        }
    }

}
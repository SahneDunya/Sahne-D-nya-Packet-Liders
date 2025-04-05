use serde::{Deserialize, Serialize};
use serde_json;
use reqwest;
use zip::ZipArchive;
use std::{fs, io};
use std::path::{Path, PathBuf};

// Paket Yapısı (Paket Verilerini Temsil Eder)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Paket {
    pub ad: String,
    pub surum: String,
    pub bagimliliklar: Vec<String>,
    pub aciklama: Option<String>,
    pub dosya_adi: Option<String>,
}

impl Paket {
    pub fn yeni(ad: String, surum: String, bagimliliklar: Vec<String>) -> Self {
        Paket {
            ad,
            surum,
            bagimliliklar,
            aciklama: None,
            dosya_adi: None,
        }
    }
}

// Depo Yöneticisi Yapısı (Paket Deposunu Yönetir)
pub struct DepoYoneticisi {
    pub depo_url: String,
    pub yerel_depo_yolu: String,
    paket_listesi_cache: Option<Vec<Paket>>, // Paket listesi önbelleği
}

impl DepoYoneticisi {
    pub fn yeni(depo_url: String, yerel_depo_yolu: String) -> Self {
        DepoYoneticisi {
            depo_url,
            yerel_depo_yolu,
            paket_listesi_cache: None, // Başlangıçta önbellek boş
        }
    }

    // Paket Listesini İndirme (Önbellek Mekanizması ile)
    pub fn paket_listesini_indir(&mut self) -> Result<Vec<Paket>, Box<dyn std::error::Error>> {
        if let Some(ref paketler) = self.paket_listesi_cache {
            // Önbellekte varsa listeyi döndür
            println!("Önbellekten paket listesi kullanılıyor.");
            return Ok(paketler.clone());
        }

        // Önbellekte yoksa indir
        println!("Paket listesi indiriliyor: {}", self.depo_url);
        let depo_paket_listesi_url = format!("{}/paketler.json", self.depo_url);
        let response = reqwest::blocking::get(&depo_paket_listesi_url)?;

        if !response.status().is_success() {
            return Err(format!("Paket listesi indirme hatası, HTTP Durumu: {}", response.status()).into());
        }

        let paketler: Vec<Paket> = response.json()?;
        self.paket_listesi_cache = Some(paketler.clone()); // İndirilen listeyi önbelleğe kaydet
        Ok(paketler)
    }

    // Yerel Depoyu Güncelleme
    pub fn yerel_depoyu_guncelle(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Yerel depo güncelleniyor: {}", self.yerel_depo_yolu);
        let paketler = self.paket_listesini_indir()?; // Paket listesini indir veya önbellekten al
        let yerel_depo_dosyasi_path = PathBuf::from(&self.yerel_depo_yolu).join("paketler.json");

        let dosya_icerigi = serde_json::to_string_pretty(&paketler)?;
        fs::write(&yerel_depo_dosyasi_path, dosya_icerigi)?;

        self.paket_listesi_cache = Some(paketler.clone()); // Yerel depo güncellendiğinde önbelleği de güncelle
        println!("Yerel depo başarıyla güncellendi: {:?}", yerel_depo_dosyasi_path);
        Ok(())
    }

    // Paket Arama (Paket Adına Göre)
    pub fn paket_ara(&mut self, paket_adi: &str) -> Result<Option<Paket>, Box<dyn std::error::Error>> {
        let paketler = self.paket_listesini_indir()?; // Paket listesini indir veya önbellekten al

        let bulunan_paket = paketler.into_iter().find(|paket| paket.ad == paket_adi);
        Ok(bulunan_paket)
    }
}

// Kurulum Yöneticisi Yapısı (Paket Kurulumunu Yönetir)
pub struct KurulumYoneticisi {
    pub paket_deposu_yolu: String, // Paket dosyalarının bulunduğu depo yolu (yerel veya uzak)
    pub kurulum_dizini: String,     // Paketlerin kurulacağı dizin
}

impl KurulumYoneticisi {
    pub fn yeni(paket_deposu_yolu: String, kurulum_dizini: String) -> Self {
        KurulumYoneticisi {
            paket_deposu_yolu,
            kurulum_dizini,
        }
    }

    // Paketi İndirme Fonksiyonu
    pub fn paketi_indir(&self, paket: &Paket) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(dosya_adi) = &paket.dosya_adi {
            let paket_url = format!("{}/{}", self.paket_deposu_yolu, dosya_adi);
            let paket_yolu = PathBuf::from(&self.paket_deposu_yolu).join(dosya_adi);

            // Paket dosyası zaten varsa indirmeyi atla
            if paket_yolu.exists() {
                println!("Paket dosyası zaten mevcut: {:?}", paket_yolu);
                return Ok(());
            }

            println!("Paket indiriliyor: {} -> {:?}", paket.ad, paket_yolu);
            let response = reqwest::blocking::get(&paket_url)?;

            if !response.status().is_success() {
                return Err(format!("Paket indirme başarısız oldu, HTTP Durumu: {}", response.status()).into());
            }

            let mut dosya = fs::File::create(&paket_yolu)?;
            let mut kaynak = response.bytes()?;
            io::copy(&mut kaynak.as_ref(), &mut dosya)?; // Akış ile dosyaya kopyalama

            println!("Paket indirildi: {:?}", paket_yolu);
            Ok(())
        } else {
            println!("Paket için dosya adı belirtilmemiş: {}", paket.ad);
            Ok(()) // Dosya adı yoksa hata yerine bilgilendirme mesajı ve OK dön
        }
    }

    // Paketi Kurulum Diznine Çıkarma Fonksiyonu (Zip Arşivinden)
    pub fn paketi_kur(&self, paket: &Paket) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(dosya_adi) = &paket.dosya_adi {
            let paket_yolu = PathBuf::from(&self.paket_deposu_yolu).join(dosya_adi);
            let dosya = fs::File::open(&paket_yolu)?;
            let mut arsiv = ZipArchive::new(dosya)?;

            println!("Paket kurulumuna başlanıyor: {}", paket.ad);

            for i in 0..arsiv.len() {
                let mut arsiv_dosyasi = arsiv.by_index(i)?;
                let cikartma_yolu_str = format!("{}/{}", self.kurulum_dizini, arsiv_dosyasi.name());
                let cikartma_yolu = PathBuf::from(&cikartma_yolu_str);

                if arsiv_dosyasi.name().ends_with('/') {
                    // Klasör ise oluştur
                    fs::create_dir_all(&cikartma_yolu)?;
                } else {
                    // Dosya ise çıkar
                    if let Some(ebeveyn_dizin) = cikartma_yolu.parent() {
                        fs::create_dir_all(ebeveyn_dizin)?; // Ebeveyn dizinleri oluştur
                    }
                    let mut cikartma_dosyasi = fs::File::create(&cikartma_yolu)?;
                    io::copy(&mut arsiv_dosyasi, &mut cikartma_dosyasi)?;
                }
            }
            println!("Paket başarıyla kuruldu: {}", paket.ad);
            Ok(())
        } else {
            println!("Paket için dosya adı belirtilmemiş: {}", paket.ad);
            Ok(()) // Dosya adı yoksa hata yerine bilgilendirme mesajı ve OK dön
        }
    }
}
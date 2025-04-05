#![no_std] // Eğer bu dosya tek başına derleniyorsa bu satırı ekleyebilirsiniz

use crate::package::Paket;
use crate::fs; // Sahne64 dosya sistemi modülü

pub struct KurulumYoneticisi {
    pub paket_deposu_yolu: String,
    pub kurulum_dizini: String,
}

impl KurulumYoneticisi {
    pub fn yeni(paket_deposu_yolu: String, kurulum_dizini: String) -> Self {
        KurulumYoneticisi {
            paket_deposu_yolu,
            kurulum_dizini,
        }
    }

    pub fn paketi_indir(&self, paket: &Paket) -> Result<(), super::SahneError> {
        if let Some(dosya_adi) = &paket.dosya_adi {
            let paket_url = format!("{}/{}", self.paket_deposu_yolu, dosya_adi);
            let paket_yolu = format!("{}/{}", self.paket_deposu_yolu, dosya_adi);

            // Şu anda indirme işlevini basitleştiriyoruz.
            // Gerçek bir uygulamada ağ modülü ve indirme mekanizması gerekecektir.
            println!("İndirme işlemi şu anda desteklenmiyor. Paketin {} adresinde olduğunu varsayıyoruz.", paket_yolu);
            Ok(())
        } else {
            println!("Paket için dosya adı belirtilmemiş.");
            Ok(())
        }
    }

    pub fn paketi_kur(&self, paket: &Paket) -> Result<(), super::SahneError> {
        if let Some(dosya_adi) = &paket.dosya_adi {
            let paket_yolu = format!("{}/{}", self.paket_deposu_yolu, dosya_adi);

            println!("Paket kurulumuna başlanıyor: {:?}", paket.ad);
            println!("Paket dosya yolu: {:?}", paket_yolu);

            // Şu anda sadece dosyanın varlığını kontrol edip basit bir işlem yapıyoruz.
            // Gerçek bir uygulamada paket formatına (örneğin, ZIP) göre açma ve çıkarma işlemleri yapılmalıdır.

            match fs::open(&paket_yolu, fs::O_RDONLY) {
                Ok(fd) => {
                    println!("Paket dosyası bulundu ve açıldı (fd: {}).", fd);
                    fs::close(fd)?; // Dosyayı hemen kapatıyoruz, gerçekte içeriğini okuyacağız.

                    // Basit bir örnek olarak, paketin içeriğini kurulum dizinine kopyalayabiliriz.
                    // Bu kısım gerçek paket formatına göre çok daha karmaşık olacaktır.
                    let hedef_yol = format!("{}/{}", self.kurulum_dizini, dosya_adi);
                    let mut girdi_buffer = [0u8; 1024]; // Örnek bir buffer boyutu
                    let mut cikti_fd = fs::open(&hedef_yol, fs::O_WRONLY | fs::O_CREAT)?;

                    let mut girdi_fd = fs::open(&paket_yolu, fs::O_RDONLY)?;
                    loop {
                        match fs::read(girdi_fd, &mut girdi_buffer) {
                            Ok(bytes_read) => {
                                if bytes_read == 0 {
                                    break;
                                }
                                fs::write(cikti_fd, &girdi_buffer[..bytes_read])?;
                            }
                            Err(e) => {
                                fs::close(girdi_fd)?;
                                fs::close(cikti_fd)?;
                                return Err(e);
                            }
                        }
                    }
                    fs::close(girdi_fd)?;
                    fs::close(cikti_fd)?;

                    println!("Paket içeriği kopyalandı: {:?}", paket.ad);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Paket dosyası açılamadı: {:?}", e);
                    Err(e)
                }
            }
        } else {
            println!("Paket için dosya adı belirtilmemiş.");
            Ok(())
        }
    }
}
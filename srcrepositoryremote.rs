#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // String, Vec, format! için

use alloc::string::{String, ToString}; // std::string::String yerine
use alloc::vec::Vec; // std::vec::Vec yerine
use alloc::format; // format! makrosu için
use alloc::borrow::ToOwned; // to_string() için

// Sahne64 API modülleri
use crate::resource; // Ağ ve dosya sistemi benzeri işlemler için
use crate::SahneError; // Sahne64 hata türü
use crate::Handle; // Kaynak Handle'ları

// Özel hata enum'ımızı içe aktar (no_std uyumlu hali)
use crate::paket_yoneticisi_hatasi::PaketYoneticisiHatasi;
// SahneError'dan dönüşüm From implementasyonu ile sağlanacak

// no_std uyumlu print makroları
use crate::print_macros::{println, eprintln};


// Sahne64 Kaynak tabanlı ağ API'sı kullanılarak uzak paket deposunu yönetir.
// Bir URL, belirli bir şemaya sahip bir Kaynak ID'si olarak kabul edilir.
pub struct RemoteRepository {
    // Uzak deponun temel Kaynak ID'si (URL formatında olabilir, örn. "http://example.com/packages/")
    pub url: String, // String alloc gerektirir.
}

impl RemoteRepository {
    // Yeni bir RemoteRepository örneği oluşturur.
    // url: Uzak deponun temel Kaynak ID'si (URL).
    pub fn new(url: String) -> Self {
        RemoteRepository { url } // String ownership'i alınır.
    }

    // Uzak depoda belirli bir paketin arşiv dosyasının Kaynak ID'sini (URL) oluşturur.
    // package_name: Paketin adı.
    // version: Paketin sürümü.
    // package_file_name: Paketin arşiv dosyasının adı (Paket struct'ından gelmeli).
    // Dönüş değeri: Paketin Kaynak ID'si (URL) String olarak (alloc gerektirir).
    pub fn get_package_resource_id(
        &self,
        package_name: &str,
        version: &str,
        package_file_name: &str, // Dosya adını da bilmek gerekir
    ) -> String { // String alloc gerektirir.
        // URL formatını birleştirerek Kaynak ID'sini oluştur.
        // Varsayım: Uzak depoda yapı `base_url/paket_adi/surum/dosya_adi` şeklindedir.
        format!("{}/{}/{}/{}", self.url, package_name, version, package_file_name) // format! alloc
    }

    // Uzak depodan belirli bir paketi indirir ve hedef yerel Kaynağa kaydeder.
    // package_name: İndirilecek paketin adı.
    // version: İndirilecek paketin sürümü.
    // package_file_name: İndirilecek paketin arşiv dosyasının adı (Paket struct'ından gelir).
    // destination_resource_id: İndirilen dosyanın kaydedileceği yerel Kaynağın ID'si.
    // Dönüş değeri: Başarı veya PaketYoneticisiHatasi.
    pub fn download_package(
        &self,
        package_name: &str,
        version: &str,
        package_file_name: &str, // Dosya adını da bilmek gerekir
        destination_resource_id: &str, // PathBuf yerine &str Kaynak ID
    ) -> Result<(), PaketYoneticisiHatasi> { // SahneError yerine PaketYoneticisiHatasi
        // İndirilecek paketin uzak Kaynak ID'sini (URL) oluştur.
        let source_url = self.get_package_resource_id(package_name, version, package_file_name); // String (alloc)

        println!("Paket indiriliyor: {} -> {}", source_url, destination_resource_id); // no_std print

        // Uzak (ağ) Kaynağı okuma izniyle acquire et. Bu HTTP GET isteğini başlatır.
        let source_handle = resource::acquire(&source_url, resource::MODE_READ) // URL Kaynak ID'si olarak kullanılıyor
            .map_err(|e| {
                 eprintln!("Uzak Kaynak acquire hatası ({}): {:?}", source_url, e); // no_std print
                PaketYoneticisiHatasi::from(e) // SahneError -> PaketYoneticisiHatasi (Ağ hatası olabilir)
            })?;

        // Hedef (yerel) Kaynağı yazma izniyle aç/oluştur/sil.
        // MODE_CREATE varsa Kaynak yoksa oluşturur, MODE_TRUNCATE varsa içeriği siler.
        // hedef_kaynak_id'nin parent resource'larının oluşturulması srcinstaller.rs/srcrepositorylocal.rs'deki helper'ı gerektirebilir.
          let _ = crate::srcrepositorylocal::sahne_create_resource_recursive(parent_of_destination_resource_id)?; // Eğer helper public ve kullanılabilirse

        let destination_handle = resource::acquire(
            destination_resource_id,
            resource::MODE_WRITE | resource::MODE_CREATE | resource::MODE_TRUNCATE
        ).map_err(|e| {
             eprintln!("Hedef Kaynak acquire hatası ({}): {:?}", destination_resource_id, e); // no_std print
            PaketYoneticisiHatasi::from(e) // SahneError -> PaketYoneticisiHatasi
        })?;

        // HTTP Status Code kontrolü (Sahne64 API desteği gerekiyor)
        // resource::control ile status code alınabildiği varsayımıyla:
         const RESOURCE_CONTROL_CMD_GET_HTTP_STATUS: u33 = 3; // Varsayımsal komut
         match resource::control(source_handle, RESOURCE_CONTROL_CMD_GET_HTTP_STATUS, &[]) {
              Ok(status_bytes) => {
        //          // status_bytes'ı u16/u32'ye çevir.
                  if status_code != 200 {
                      let _ = resource::release(source_handle); // Hata durumunda handle'ları kapat
                      let _ = resource::release(destination_handle);
                      return Err(PaketYoneticisiHatasi::NetworkError(format!("HTTP hatası: {}, URL: {}", status_code, source_url)));
                  }
              }
              Err(e) => {
                   // Status code alınamadı hatası (kritik olabilir veya olmayabilir)
                   eprintln!("HTTP status code alınamadı (Kaynak: {}): {:?}", source_url, e);
                   // Hata durumunda devam mı, dur mu? Şimdilik devam edelim.
              }
         }
        // API desteği yoksa, sadece resource::read/write hatalarına güvenilir.

        // Kaynaktan oku ve hedefe yaz döngüsü (indirme ve kaydetme).
        let mut buffer = [0u8; 4096]; // Okuma/yazma buffer'ı (stack'te)
        loop {
            // Kaynak Kaynağından oku (ağ üzerinden response body'yi alır)
            match resource::read(source_handle, &mut buffer) {
                Ok(0) => break, // Kaynak sonu (indirme tamamlandı)
                Ok(bytes_read) => {
                    // Hedef Kaynağa yaz (yerel depolama)
                    match resource::write(destination_handle, &buffer[..bytes_read]) {
                        Ok(_) => {
                            // Başarılı yazma
                        }
                        Err(e) => {
                             eprintln!("İndirme yazma hatası ({} -> {}): {:?}", source_url, destination_resource_id, e); // no_std print
                            let _ = resource::release(source_handle); // Handle'ları temizle
                            let _ = resource::release(destination_handle);
                            return Err(PaketYoneticisiHatasi::from(e)); // SahneError -> PaketYoneticisiHatasi
                        }
                    }
                }
                Err(e) => {
                    // Okuma hatası (örn. ağ kesintisi, sunucu hatası)
                     eprintln!("İndirme okuma hatası ({}): {:?}", source_url, e); // no_std print
                    let _ = resource::release(source_handle); // Handle'ları temizle
                    let _ = resource::release(destination_handle);
                    return Err(PaketYoneticisiHatasi::from(e)); // SahneError -> PaketYoneticisiHatasi
                }
            }
        }

        // Handle'ları serbest bırak
        let release_source_result = resource::release(source_handle);
         if let Err(_e) = release_source_result {
              eprintln!("Uzak Kaynak Handle release hatası ({}): {:?}", source_url, _e); // no_std print
         }

        let release_destination_result = resource::release(destination_handle);
         if let Err(_e) = release_destination_result {
             eprintln!("Hedef Kaynak Handle release hatası ({}): {:?}", destination_resource_id, _e); // no_std print
         }


        println!("Paket başarıyla indirildi ve kaydedildi: {}", destination_resource_id); // no_std print
        Ok(()) // Başarı
    }
}

// Hypothetical Sahne64 network module removed, as network access is modeled via resources in srcnetwork.rs.

// #[cfg(test)] bloğu std test runner'ı gerektirir ve Sahne64 resource/network mock'ları gerektirir.
// Bu blok no_std ortamında derlenmeyecektir eğer std feature aktif değilse ve test ortamı yoksa.

#[cfg(test)]
mod tests {
    // std::path, tempfile, std::fs kullandığı için no_std'de doğrudan çalışmaz.
    // Mock resource/network veya Sahne64 simülasyonu gerektirir.
}

#![no_std]
extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;

// disk_cache crate'i std::io ve std::fs'e bağımlıdır, bu nedenle no_std'de kullanılamaz.
// Bunun yerine kendi önbellek mantığımızı Sahne64 kaynakları üzerine kuracağız.
use disk_cache::{Cache, Error as DiskCacheError};

// Sahne64 API modüllerini içe aktarın
use crate::resource;
use crate::SahneError;
use crate::Handle;

// Özel hata enum'ımızı içe aktar (no_std uyumlu ve SahneError'ı içeren haliyle)
use crate::paket_yoneticisi_hata::PaketYoneticisiHatasi;
// SahneError'dan dönüşüm helper'ı (paket_yoneticisi_hata.rs'de tanımlı olduğunu varsayalım)
use crate::paket_yoneticisi_hata::from_sahne_error;

// -- Dizin Oluşturma Helper Fonksiyonu (srcarchive.rs'den alınmıştır, burada da gerekli olabilir) --
// Sahne64'te 'dizin' kaynaklarının nasıl oluşturulduğuna dair varsayımlar içerir.
// Bu fonksiyonun varlığını ve çalışma prensibini srcarchive.rs dosyasındaki açıklamalar belirliyor.
// Gerçek Sahne64 çekirdeği bu işlevselliği sağlamalıdır.
fn sahne_create_resource_recursive(resource_id: &str) -> Result<(), PaketYoneticisiHatasi> {
     // resource_id'nin parent yollarını bularak resource::acquire(path, MODE_CREATE) ile
     // onları oluşturmaya çalışır. '/' ile biten yollar için ayrı bir acquire çağrısı.

     let parent_path_opt = resource_id.rfind('/').map(|idx| &resource_id[..idx]);

     if let Some(parent_path) = parent_path_opt {
          if parent_path.is_empty() || parent_path == "/" {
               // Kök dizin veya Sahne64 root kaynağı, oluşturmaya gerek yok.
               return Ok(());
          }
          // Parent yolu, örneğin "sahne://cache/packages/my_package" -> parent "sahne://cache/packages"
          // Veya "sahne://cache/packages/" -> parent "sahne://cache/"
          // Bu yolu acquire etmeye çalışalım. Varsayım: acquire(MODE_CREATE) ile
          // eksik parent'lar otomatik oluşur veya bu resource bir klasör gibi davranır.
          // Veya Sahne64'te 'klasör' resource'ları için özel bir acquire flag'i vardır.

          match resource::acquire(parent_path, resource::MODE_CREATE /* | resource::MODE_CONTAINER */) {
               Ok(handle) => {
                    // Başarıyla acquire ettik, bu Handle'ı bırakabiliriz eğer sadece oluşturmaksa amaç.
                    let _ = resource::release(handle);
                    Ok(())
               },
               Err(e) => {
                    // Hata durumunda, bu SahneError'ı PaketYoneticisiHatasi'na çevir
                    eprintln!("Ebeveyn Kaynağı oluşturma hatası ({}): {:?}", parent_path, e);
                    Err(PaketYoneticisiHatasi::from_sahne_error(e))
               }
           }
     } else {
          // Eğer yol '/' içermiyorsa (örn. "dosya_adi"), parent yok, bir şey oluşturmaya gerek yok.
          Ok(())
     }
}


// Sahne64 kaynaklarını kullanarak paket önbelleğini yöneten yapı.
// disk_cache yerine Sahne64'ün 'resource' modülünü kullanır.
pub struct PaketOnbellek {
    // Önbellek verilerinin saklanacağı temel Kaynak ID'si (örn. "sahne://cache/packages/")
    base_resource_id: String,
}

impl PaketOnbellek {
    // onbellek_base_resource_id: Önbelleğin ana Kaynak ID'si (örn. "sahne://cache/packages/")
    pub fn yeni(onbellek_base_resource_id: &str) -> Result<PaketOnbellek, PaketYoneticisiHatasi> {
        // Önbellek temel dizin/kaynak yolunun varlığını sağlamak.
        // Sahne64 resource modeline göre dizin oluşturma/sağlama mekanizması burada kullanılmalı.
        // Örn: resource::acquire(onbellek_base_resource_id, resource::MODE_CREATE | resource::MODE_DIRECTORY_OR_CONTAINER)
        // Veya recursive helper fonksiyonumuzu kullanma.

        // Varsayım: onbellek_base_resource_id '/ ile bitiyor ve bir 'dizin' kaynağı gibi davranacak.
        // Bu Kaynağı acquire ederek varlığını sağlamaya çalışalım.
        match resource::acquire(onbellek_base_resource_id, resource::MODE_CREATE) {
             Ok(handle) => {
                  let _ = resource::release(handle); // Handle'ı hemen bırak
                  Ok(PaketOnbellek { base_resource_id: onbellek_base_resource_id.to_string() })
             }
             Err(e) => {
                 eprintln!("Önbellek temel Kaynağı oluşturulamadı/edinilemedi ({}): {:?}", onbellek_base_resource_id, e);
                 Err(PaketYoneticisiHatasi::from_sahne_error(e))
             }
        }
    }

    // Bir önbellek anahtarı için tam Kaynak ID'sini oluşturur.
    // Örn: base_resource_id = "sahne://cache/packages/", anahtar = "my_package_v1.0" -> "sahne://cache/packages/my_package_v1.0"
    fn get_item_resource_id(&self, anahtar: &str) -> String {
        if self.base_resource_id.ends_with('/') {
            // Base resource ID zaten '/' ile bitiyorsa
            format!("{}{}", self.base_resource_id, anahtar)
        } else {
            // Araya '/' ekleyerek birleştir
            format!("{}/{}", self.base_resource_id, anahtar)
        }
    }

    // Önbellekten veri alır.
    pub fn paket_verisini_al(&self, anahtar: &str) -> Result<Option<Vec<u8>>, PaketYoneticisiHatasi> {
        let item_resource_id = self.get_item_resource_id(anahtar);

        // Kaynağı oku (sadece okuma izniyle)
        match resource::acquire(&item_resource_id, resource::MODE_READ) {
            Ok(handle) => {
                let mut buffer = Vec::new(); // Veriyi okuyacağımız tampon

                // resource::read fonksiyonu offset almaz. Tüm içeriği okumak için
                // ya kaynağın boyutu bilinmeli (resource::control ile?),
                // ya da okuma döngüsü yapılmalı.
                // Basitlik adına, tek seferde okunabildiğini varsayalım (resource::read dosyanın sonuna kadar okuyabilir).
                // Veya boyut bilgisini alıp tam o boyutta buffer ayırmalı.

                // Eğer resource::read tek çağrıda tüm kaynağı okuyamıyorsa, döngü gerekir:
                 let mut temp_buffer = [0u8; 4096]; // Küçük okuma tamponu
                 loop {
                     match resource::read(handle, &mut temp_buffer) {
                         Ok(0) => break, // EOF
                         Ok(n) => buffer.extend_from_slice(&temp_buffer[..n]),
                         Err(e) => {
                             let _ = resource::release(handle);
                             return Err(PaketYoneticisiHatasi::from_sahne_error(e));
                         }
                     }
                 }

                // Handle'ı serbest bırak
                let release_result = resource::release(handle);
                if let Err(e) = release_result {
                     eprintln!("Önbellek öğesi Kaynağı release hatası ({}): {:?}", item_resource_id, e);
                     // Release hatası kritik olmayabilir, loglayıp devam edebiliriz veya dönebiliriz.
                     // Şimdilik loglayıp devam edelim.
                }

                Ok(Some(buffer))
            }
            Err(SahneError::ResourceNotFound) => {
                // Kaynak bulunamadıysa, önbellekte yok demektir.
                Ok(None)
            }
            Err(e) => {
                // Diğer Sahne64 hataları
                eprintln!("Önbellek öğesi Kaynağı acquire hatası ({}): {:?}", item_resource_id, e);
                Err(PaketYoneticisiHatasi::from_sahne_error(e))
            }
        }
    }

    // Önbelleğe veri kaydeder.
    pub fn paket_verisini_kaydet(&self, anahtar: &str, veri: &[u8]) -> Result<(), PaketYoneticisiHatasi> {
        let item_resource_id = self.get_item_resource_id(anahtar);

        // Kaynağın parent dizinlerini oluştur (Sahne64 modeline göre)
        // Bu adım, Kaynak ID'si "sahne://cache/packages/my_package/data" ise,
        // "sahne://cache/packages/" ve "sahne://cache/packages/my_package/"
        // resource'larının varlığını sağlamayı içerir.
        sahne_create_resource_recursive(&item_resource_id)?;


        // Kaynağı yazma izniyle oluştur/aç (varsa sil)
        match resource::acquire(
            &item_resource_id,
            resource::MODE_WRITE | resource::MODE_CREATE | resource::MODE_TRUNCATE
        ) {
            Ok(handle) => {
                // Veriyi yaz
                let write_result = resource::write(handle, veri);

                // Handle'ı serbest bırak
                let release_result = resource::release(handle);

                // Yazma veya release hatalarını kontrol et
                if let Err(e) = write_result {
                     eprintln!("Önbellek öğesi Kaynağı yazma hatası ({}): {:?}", item_resource_id, e);
                     // Yazma başarısızsa release'in sonucunu yoksayabiliriz, ana hata yazma hatasıdır.
                     return Err(PaketYoneticisiHatasi::from_sahne_error(e));
                }
                 if let Err(e) = release_result {
                      eprintln!("Önbellek öğesi Kaynağı release hatası ({}): {:?}", item_resource_id, e);
                      // Release hatası loglanmalı ama write başarılıysa Ok dönebiliriz?
                      // Hata yönetim stratejisine bağlı. Şimdilik loglayalım.
                 }

                Ok(()) // Hem yazma hem release başarılıysa
            }
            Err(e) => {
                eprintln!("Önbellek öğesi Kaynağı acquire hatası ({}): {:?}", item_resource_id, e);
                Err(PaketYoneticisiHatasi::from_sahne_error(e))
            }
        }
    }

    // Önbellekten veri siler.
    // **ÖNEMLİ:** Sahne64 API'sında resource silme syscall'u/mekanizması yok.
    // Bu fonksiyonun implementasyonu için çekirdek API'nın güncellenmesi gerekir (örn. resource::delete veya resource::control ile bir DELETE komutu).
    pub fn paket_verisini_sil(&self, anahtar: &str) -> Result<(), PaketYoneticisiHatasi> {
        let item_resource_id = self.get_item_resource_id(anahtar);

        eprintln!("UYARI: Sahne64 API'sında doğrudan resource silme mekanizması bulunmamaktadır. 'paket_verisini_sil' fonksiyonu tamamlanmamıştır.");
        eprintln!("Silinmek istenen Kaynak ID: {}", item_resource_id);

        // ÖRNEK placeholder implementasyon (GERÇEKTE ÇALIŞMAYACAK!)
        // Varsayım: resource::control ile DELETE komutu gönderilebilir.
         const RESOURCE_CONTROL_DELETE_COMMAND: u64 = 1; // Sahne64 çekirdeğinde tanımlı olmalı

         match resource::acquire(&item_resource_id, 0) { // Handle almak için sadece varlığını kontrol et
             Ok(handle) => {
        //          // Kaynağı silmek için kontrol komutu gönder
                  match resource::control(handle, RESOURCE_CONTROL_DELETE_COMMAND, 0) {
                      Ok(_) => {
                          // Başarılı, şimdi handle'ı bırakalım (kernel zaten silmiş olabilir)
                          let _ = resource::release(handle); // Bu çağrı hata verebilir eğer kaynak silindiyse
                          Ok(())
                      }
                      Err(e) => {
                          let _ = resource::release(handle);
                          eprintln!("Kaynak silme control komutu hatası ({}): {:?}", item_resource_id, e);
                          Err(PaketYoneticisiHatasi::from_sahne_error(e))
                      }
                  }
             }
             Err(SahneError::ResourceNotFound) => {
        //         // Kaynak zaten yoksa silinmiş sayılır.
                 Ok(())
             }
             Err(e) => {
                 // Diğer acquire hataları
                 eprintln!("Kaynak silme acquire hatası ({}): {:?}", item_resource_id, e);
                 Err(PaketYoneticisiHatasi::from_sahne_error(e))
             }
         }

         // Şu anki API ile silme mümkün değil. Hata dönelim veya unimplemented! yapalım.
         Err(PaketYoneticisiHatasi::SahneApiHatasi(SahneError::NotSupported)) // Veya özel bir hata
    }

    // Önbelleği temizler (tüm öğeleri siler).
    // **ÖNEMLİ:** Sahne64 API'sında belirli bir yol altındaki tüm resource'ları listeleme mekanizması yok.
    // Bu fonksiyonun implementasyonu için çekirdek API'nın güncellenmesi gerekir (örn. resource::list veya bir iterator kaynağı).
    pub fn onbellegi_temizle(&self) -> Result<(), PaketYoneticisiHatasi> {
        eprintln!("UYARI: Sahne64 API'sında resource listeleme mekanizması bulunmamaktadır. 'onbellegi_temizle' fonksiyonu tamamlanmamıştır.");
        eprintln!("Temizlenmek istenen temel Kaynak ID: {}", self.base_resource_id);

        // Önbellek altındaki tüm öğeleri listelemek için bir mekanizma gerekli.
        // Varsayım: resource::acquire ile özel bir 'listeleyici' kaynağı alınabilir.
        // Örn: match resource::acquire(&self.base_resource_id, resource::MODE_LIST) { ... }
        // Bu 'listeleyici' kaynaktan okuma yaparak altındaki item ID'lerini alabiliriz.
        // Veya resource::control ile bir LIST komutu gönderilir ve sonuç alınır.

        // Şu anki API ile listeleyip silemeyiz. Hata dönelim veya unimplemented! yapalım.
        Err(PaketYoneticisiHatasi::SahneApiHatasi(SahneError::NotSupported)) // Veya özel bir hata
    }
}

// --- PaketYoneticisiHata enum tanımının no_std uyumlu hale getirilmesi ---
// (Bu enum tanımı muhtemelen 'paket_yoneticisi_hata.rs' dosyasındadır)
#![no_std]
extern crate alloc;

use alloc::string::String;
use alloc::format; // format! makrosu için
use crate::SahneError; // Sahne64'ün hata türü

// Eğer zip crate'inin no_std hatası varsa onu da buraya ekleyin
// Eğer reqwest, serde_json gibi crate'lerin no_std alternatifleri veya hata türleri varsa onları kullanın.
// Varsayım: ZipError no_std'de de kullanılabilir veya kendi no_std ZipError'ımız var.

#[derive(Debug)] // thiserror::Error yerine Debug kullanıyoruz çünkü thiserror std gerektirebilir.
                 // Alternatif olarak thiserror'ın no_std versiyonu varsa o kullanılabilir.
pub enum PaketYoneticisiHatasi {
    
     DosyaSistemiHatasi(SahneError), // Belki sadece SahneApiHatasi yeterlidir

    // reqwest ve serde_json no_std'de doğrudan kullanılamaz.
    // HTTP ve JSON işlemlerini kendiniz veya no_std uyumlu kütüphanelerle yapmalısınız.
     HttpIstegiHatasi(...), // no_std uyumlu hata türü
     JsonHatasi(...), // no_std uyumlu hata türü

    // Zip hatası (zip crate'inin no_std hatası varsa onu kullanın)
    ZipHatasi(zip::result::ZipError), // zip crate'inin no_std ZipError'ını varsayıyoruz

    PaketBulunamadi(String),
    PaketKurulumHatasi(String),

    // Önbellek hatası (detaylı string yerine özel hata varyantları veya SahneError'ı kullanabiliriz)
    // String kullanmak alloc gerektirir.
    OnbellekHatasi(String), // Genel önbellek operasyon hatası

    // Sahne64 API'sından gelen hatalar için net bir varyant
    SahneApiHatasi(SahneError),

    GecersizParametre(String), // Fonksiyona geçersiz parametre geçilmesi
    PathTraversalHatasi(String), // Güvenlik: Path traversal denemesi

    // ... diğer paket yöneticisi özel, no_std uyumlu hatalar ...
    BilinmeyenHata, // String'siz basit bir bilinmeyen hata (alloc kullanmamak için)
}

// SahneError'dan PaketYoneticisiHatasi::SahneApiHatasi'na dönüşüm helper'ı
impl From<SahneError> for PaketYoneticisiHatasi {
    fn from(err: SahneError) -> Self {
        // SahneError'daki bazı spesifik hataları PaketYoneticisiHatasi'nda
        // daha spesifik varyantlara maplemek isterseniz burada yapın.
        // Örn: match err { SahneError::ResourceNotFound => PaketYoneticisiHatasi::PaketBulunamadi(...) }
        PaketYoneticisiHatasi::SahneApiHatasi(err)
    }
}

// ZipError'dan dönüşüm (zip crate'inin no_std hatasını varsayarak)
impl From<zip::result::ZipError> for PaketYoneticisiHatasi {
    fn from(err: zip::result::ZipError) -> Self {
        // ZipError içindeki IO hatalarını (ki bunlar no_std'de farklı olacaktır)
        // SahneApiHatasi'na maplemek gerekebilir.
        match err {
            zip::result::ZipError::IoError(_) => {
                 // Zip'ten gelen IO hatasını SahneError'a çevirmek mümkün değil
                 // çünkü zip'in IO hatası Sahne64 API hatası değil.
                 // Bu durumda ya ZipHatasi içinde spesifik no_std IO hatasını taşımalı,
                 // ya da bu hatayı genel bir Onbellek/Zip hatasına sarmalıyız.
                 PaketYoneticisiHatasi::OnbellekHatasi(format!("Zip IO Hatası: {:?}", err)) // String kullanmak yerine
                 // PaketYoneticisiHatasi::ZipIoHatasi(...) gibi yeni varyant eklenebilir.
            }
            _ => PaketYoneticisiHatasi::ZipHatasi(err),
        }
    }
}

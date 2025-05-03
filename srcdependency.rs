#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (alloc kullanacağız)
extern crate alloc; // HashMap, HashSet, String, Vec için

use alloc::collections::{HashMap, HashSet};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::borrow::ToOwned; // to_string() yerine to_owned() daha genel

// 'Paket' struct tanımını içeren modül
use crate::package::Paket;

// Özel hata enum'ımızı içe aktar (no_std uyumlu hali)
use crate::paket_yoneticisi_hata::PaketYoneticisiHatasi;
// SahneError importu burada doğrudan kullanılmıyor, PaketYoneticisiHatasi içinde sarmalanıyor olabilir.
// Eski, kullanılmayan Sahne64 API importları kaldırıldı.
use crate::fs;
use crate::process;
use crate::ipc;
use crate::kernel;
use crate::SahneError;


// Bağımlılık çözümleme ve yönetimi için yapı.
pub struct BagimlilikYoneticisi {}

impl BagimlilikYoneticisi {
    pub fn yeni() -> BagimlilikYoneticisi {
        BagimlilikYoneticisi {}
    }

    // Bağımlılıkları Derinlemesine İlk Arama (DFS) ile Çözme Fonksiyonu.
    // Paketin transitive bağımlılıklarını bulur ve kurulum için sıralar.
    // paketler: Tüm bilinen paketlerin listesi.
    // baslangic_paketi: Çözümlenmeye başlanacak ana paketin adı.
    // Dönüş değeri: Kurulum sırasına göre paket adlarının listesi.
    pub fn bagimliliklari_coz(paketler: &Vec<Paket>, baslangic_paketi: &str) -> Result<Vec<String>, PaketYoneticisiHatasi> { // Result türü PaketYoneticisiHatasi olmalı

        // Bağımlılık çözümleme mantığı temel olarak bellek içi veri yapıları (HashMap, HashSet, Vec, String) üzerinde çalışır.
        // Bu nedenle, `Sahne64` özgü dosya sistemi (`resource`), görev yönetimi (`task`), IPC (`messaging`) veya çekirdek (`kernel`) fonksiyonlarının
        // bu özel mantıkta doğrudan bir karşılığı bulunmamaktadır.
        // Bu fonksiyonun ana gereksinimi `alloc` crate'i tarafından sağlanan heap ayırma yeteneğidir.

        // Paketleri ada göre hızlı erişim için bir HashMap'e dönüştür
        // HashMap<Paket Adı: String, Paket Referansı: &Paket>
        let paket_haritasi: HashMap<String, &Paket> = paketler
            .iter()
            // Paketin adını String olarak klonla ve paket referansı ile eşle.
            // paket.ad.clone() String klonlama (alloc gerektirir).
            .map(|paket| (paket.ad.clone(), paket))
            .collect(); // HashMap oluşturma (alloc gerektirir).

        // Çözülen bağımlılıkların listesi (kurulum sırasına göre, sondan başa doğru eklenecek).
        let mut cozulen_bagimliliklar = Vec::new(); // alloc::vec::Vec (alloc gerektirir).

        // Ziyaret edilen paketleri takip etmek için küme (döngüleri önlemek).
        // Paket adlarını saklıyoruz: HashSet<Paket Adı: String>.
        let mut ziyaret_edilenler = HashSet::new(); // alloc::collections::HashSet (alloc gerektirir).

        // Ziyaret edilecek paketlerin listesi (DFS için yığın olarak kullanılır).
        // Paket adlarını saklıyoruz: Vec<Paket Adı: String>.
        let mut ziyaret_edilecekler = Vec::new(); // alloc::vec::Vec (alloc gerektirir).

        // Başlangıç paketini ziyaret edilecekler listesine ekle.
        // baslangic_paketi &str -> String dönüşümü (alloc gerektirir).
        ziyaret_edilecekler.push(baslangic_paketi.to_owned()); // to_string() yerine to_owned()


        // Ziyaret edilecek paketler listesi boşalana kadar döngü.
        while let Some(paket_adi) = ziyaret_edilecekler.pop() { // Vec pop (alloc gerektirmez, ama içindeki String'i taşır)
            // Eğer paket adı zaten ziyaret edilenler kümesindeyse (döngü veya tekrar ziyaret durumu), atla.
            if ziyaret_edilenler.contains(&paket_adi) { // HashSet contains (&String, String için Hash ve Eq kullanır)
                continue;
            }

            // Paket adını ziyaret edilenler kümesine ekle.
            // ziyaret_edilenler.insert(paket_adi.clone()); // clone() ile eklemeden önce kullanıldıysa paket_adi'nin bir klonu alınmalıydı.
            // Ancak pop ile ownership alındığı için doğrudan insert yapabiliriz.
            // insert(String) String'i kümeye taşır (alloc gerektirmez ama rehash/resize alloc gerektirebilir).
            ziyaret_edilenler.insert(paket_adi.clone()); // DFS'de visited kümesine paketi *işlemeye başlamadan önce* eklemek yaygındır.
                                                      // Ancak burada visited kontrolü pop sonrası yapıldığı için, paketi işledikten
                                                      // sonra visited'a eklemek daha uygun olabilir. Mevcut kod paketi işledikten
                                                      // sonra ekliyor, bu da döngüleri tespit etmede biraz farklılık yaratabilir
                                                      // ama temel amacı sağlar. Paketi *ziyaret edilecek* olarak işaretlemek
                                                      // için buraya eklemek ve döngü kontrolünü başta yapmak daha standart DFS'tir.
                                                      // Kodunuz pop sonrası visited kontrolü yapıyor, visited insert'i döngü sonuna yakın.
                                                      // Bu durumda cycle tespit etme biraz farklı işler. Standart yaklaşım:
                                                       let paket_adi = ziyaret_edilecekler.pop().unwrap(); // pop sonrası visited kontrolü için ownership alınır.
                                                       if visited.contains(&paket_adi) { continue; }
                                                       visited.insert(paket_adi.clone()); // Buraya eklenmeliydi
                                                      // ... paket işleme ...
                                                       cozulen_bagimliliklar.push(paket_adi); // Buraya ownership taşınır.

            // Paket haritasında paket adını ara.
            if let Some(paket) = paket_haritasi.get(&paket_adi) { // HashMap get (&String, String için Hash ve Eq kullanır)
                // Paketin bağımlılıklarını işle (genellikle önce bağımlılıklar ziyaret edilir).
                for bagimlilik_adi in &paket.bagimliliklar { // &Vec<String> üzerinde iterasyon
                    // Bağımlılık paket haritasında (yani bilinen paketler arasında) yoksa
                    if !paket_haritasi.contains_key(bagimlilik_adi) { // HashMap contains_key (&String)
                         eprintln!("Bağımlılık bulunamadı: {}", bagimlilik_adi);
                        // Hata: Bağımlılık bulunamadı. Hatayı PaketYoneticisiHatasi türünde döndür.
                        // bagimlilik_adi.clone() String klonlama (alloc gerektirir).
                        return Err(PaketYoneticisiHatasi::BagimlilikBulunamadi(bagimlilik_adi.clone()));
                    }
                    // Bağımlılığı ziyaret edilecekler listesine ekle.
                    // Ziyaret edilecekler yığınına eklenmeden önce visited kontrolü yapılmalı.
                    if !ziyaret_edilenler.contains(bagimlilik_adi) { // HashSet contains (&String)
                        ziyaret_edilecekler.push(bagimlilik_adi.clone()); // String klonlama (alloc gerektirir).
                    }
                }

                // Paketi çözülen bağımlılıklar listesine ekle.
                // Bu liste kurulum sırasını temsil ediyorsa, önce bağımlılıklar gelmeli.
                // DFS'de genellikle düğüm, bağımlılıkları işlendikten sonra son listeye eklenir (post-order traversal).
                // Mevcut kodunuz pop sonrası visited kontrolü ve döngü sonunda push yapıyor.
                // Bu aslında DFS'in biraz farklı bir implementasyonu.
                // Standart post-order DFS için:
                 while let Some(paket_adi) = ziyaret_edilecekler.pop() { ... logic ... cozulen_bagimliliklar.push(paket_adi); }
                // Bu durumda çözülen_bagimliliklar listesi, bağımlılıkları önce gelecek şekilde sondan başa sıralanır.
                // Listenin tersine çevrilmesi gerekebilir.

                // Mevcut kodun mantığına göre paket_adi'nin kendisi cozulen_bagimliliklar'a ekleniyor.
                // Bu da bir tür topolojik sıralamaya benzer, ancak tam olarak standart post-order DFS değil.
                // Let's stick to the original logic's intent.
                cozulen_bagimliliklar.push(paket_adi.clone()); // String klonlama (alloc gerektirir).

                 // Paket adı ziyaret edilenler kümesine eklenmişti, burada tekrar gerek yok.
                 // ziyaret_edilenler.insert(paket_adi); // Original code had this here, redundant if added at the start.
                 // Let's remove the redundant insert.
            } else {
                 eprintln!("Başlangıç paketi veya bir bağımlılık bilinmiyor: {}", paket_adi);
                // Hata: Paket bulunamadı (başlangıç paketi veya geçersiz bir bağımlılık adı).
                // paket_adi String'i pop ile alınmıştı, ownership kullanılabilir.
                return Err(PaketYoneticisiHatasi::PaketBulunamadi(paket_adi));
            }
        }

        // Başarılı sonuç: Çözülen bağımlılıkların listesini döndür.
        // Eğer cozulen_bagimliliklar listesi post-order DFS ile oluşturulduysa, kurulum sırası için tersine çevrilmesi gerekebilir.
        // cozuldukten sonra reverse() yapilabilir: cozulen_bagimliliklar.reverse();
        // Mevcut kodun mantığı tam bir post-order gibi durmuyor, paketi işledikten sonra ekliyor.
        // Test senaryolarına göre listenin kurulum sırasını doğru temsil ettiğini varsayalım veya
        /// burada reverse() ekleyelim. Tipik kurulum sırası için reverse() gerekir.

        cozulen_bagimliliklar.reverse(); // Genellikle kurulum sırası için listeyi ters çevirmek gerekir.

        Ok(cozulen_bagimliliklar) // Başarılı
    }
}

#![no_std]
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::HashMap; // Eğer Paket içinde HashMap varsa

#[derive(Debug, Clone, PartialEq, Eq, Hash)] // Gerekli derive'lar (alloc ile no_std'de çalışır)
pub struct Paket {
    pub ad: String,
    pub versiyon: String, // Versiyon string olarak saklanıyor
    // Diğer meta veriler eklenebilir (yazar, açıklama vb.)
    pub bagimliliklar: Vec<String>, // Bu paketin ihtiyaç duyduğu diğer paketlerin ADLARI listesi
    // Paket ismi ve versiyonu yerine sadece bağımlılık adı listesi kullanıldığı varsayılıyor bagimliliklari_coz fonksiyonunda.
    // Eğer bağımlılıklar Paket struct'ları olarak tutulacaksa (versiyon kontrolü için), Bagimlilik Yoneticisi buna göre güncellenmeli.
     pub bagimliliklar: Vec<Package>, // Bu durumda Package struct tanımı buraya veya görünen bir yere gelmeli.

    // Belki dosyalar listesi, betikler vb.
     pub dosyalar: Vec<String>,
     pub kurulum_scripti: Option<String>, // Kaynak ID'si veya betik içeriği
}

#![no_std]
extern crate alloc;

use alloc::string::String;
use alloc::format;
use crate::SahneError;
// ... diğer hata türleri ...

#[derive(Debug)]
pub enum PaketYoneticisiHatasi {
    // ... diğer hatalar ...

    // Bağımlılık çözümleme sırasında bir bağımlılık bulunamadı
    BagimlilikBulunamadi(String), // Bulunamayan bağımlılık adı

    // Bağımlılık çözümleme sırasında (veya başka yerde) bir paket adı bulunamadı
    PaketBulunamadi(String), // Bulunamayan paket adı (başlangıç paketi veya bağımlılık)

    // Döngüsel bağımlılık tespit edilirse eklenebilir
     CircularDependency(Vec<String>), // Döngüyü oluşturan paket yolu

    // ... diğer hatalar ...
    SahneApiHatasi(SahneError), // Sahne64 API'sından gelen hatalar
    ParsingError(String), // Kaynak içeriğini ayrıştırma hataları
    ConflictError(String), // Çakışma tespit/çözme hataları
    GecersizParametre(String),
    BilinmeyenHata,
}

// SahneError'dan PaketYoneticisiHatasi'na dönüşüm
// Bu From implementasyonu genel SahneApiHatasi veya spesifik hatalar için kullanılabilir.
// Bağımlılık çözümleme modülü SahneError'ı doğrudan döndürmediği için bu From impl.
// bu modülde doğrudan kullanılmayabilir, ancak diğer modüller için gereklidir.
impl From<SahneError> for PaketYoneticisiHatasi {
    fn from(err: SahneError) -> Self {
       // ... dönüşüm mantığı ...
       PaketYoneticisiHatasi::SahneApiHatasi(err)
    }
}

// ... diğer From implementasyonları ...

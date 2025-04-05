use std::collections::{HashMap, HashSet};
use crate::package::Paket;
use crate::paket_yoneticisi_hata::PaketYoneticisiHata; // Özel hata enum'ımızı içe aktar

// Assuming 'Sahne64' modules are in the same crate (though likely not directly applicable here)
use crate::fs;
use crate::process;
use crate::ipc;
use crate::kernel;
use crate::SahneError;

pub struct BagimlilikYoneticisi {}

impl BagimlilikYoneticisi {
    pub fn yeni() -> BagimlilikYoneticisi {
        BagimlilikYoneticisi {}
    }

    // Bağımlılıkları Derinlemesine İlk Arama (DFS) ile Çözme Fonksiyonu
    pub fn bagimliliklari_coz(paketler: &Vec<Paket>, baslangic_paketi: &str) -> Result<Vec<String>, PaketYoneticisiHata> {
        // Bağımlılık çözümleme mantığı temel olarak bellek içi veri yapıları (HashMap, HashSet, Vec) üzerinde çalışır.
        // Bu nedenle, `Sahne64` özgü dosya sistemi, süreç yönetimi, IPC veya çekirdek fonksiyonlarının
        // bu özel mantıkta doğrudan bir karşılığı bulunmamaktadır.

        // Paketleri ada göre hızlı erişim için bir HashMap'e dönüştür
        let paket_haritasi: HashMap<String, &Paket> = paketler
            .iter()
            .map(|paket| (paket.ad.clone(), paket))
            .collect();

        let mut cozulen_bagimliliklar = Vec::new(); // Çözülen bağımlılıkların listesi (kurulum sırasına göre)
        let mut ziyaret_edilenler = HashSet::new(); // Ziyaret edilen paketleri takip etmek için (döngüleri önlemek)
        let mut ziyaret_edilecekler = Vec::new(); // Ziyaret edilecek paketlerin listesi (DFS için yığın)

        ziyaret_edilecekler.push(baslangic_paketi.to_string()); // Başlangıç paketini ziyaret edilecekler listesine ekle

        while let Some(paket_adi) = ziyaret_edilecekler.pop() { // Ziyaret edilecek paketler bitene kadar döngü
            if ziyaret_edilenler.contains(&paket_adi) {
                continue; // Eğer paket zaten ziyaret edildiyse (döngü durumunda), atla
            }

            if let Some(paket) = paket_haritasi.get(&paket_adi) { // Paket haritasında paket adını ara
                for bagimlilik in &paket.bagimliliklar { // Paketin bağımlılıklarını işle
                    if !paket_haritasi.contains_key(bagimlilik) { // Bağımlılık paket haritasında yoksa
                        return Err(PaketYoneticisiHata::BagimlilikBulunamadi(bagimlilik.clone())); // Hata: Bağımlılık bulunamadı
                    }
                    ziyaret_edilecekler.push(bagimlilik.clone()); // Bağımlılığı ziyaret edilecekler listesine ekle
                }
                cozulen_bagimliliklar.push(paket_adi.clone()); // Paketi çözülen bağımlılıklar listesine ekle
                ziyaret_edilenler.insert(paket_adi); // Paketi ziyaret edilenler kümesine ekle
            } else {
                return Err(PaketYoneticisiHata::PaketBulunamadi(paket_adi)); // Hata: Paket bulunamadı
            }
        }

        Ok(cozulen_bagimliliklar) // Başarılı sonuç: Çözülen bağımlılıkların listesini döndür
    }
}
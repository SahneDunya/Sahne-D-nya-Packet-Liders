use thiserror::Error;
use std::io;
use reqwest;
use serde_json;
use zip;

// Assuming 'Sahne64' modules are in the same crate (though errors are defined here, not directly using those modules)
use crate::fs;
use crate::process;
use crate::ipc;
use crate::kernel;
use crate::SahneError;

#[derive(Error, Debug)]
pub enum PaketYoneticisiHata {
    #[error("Dosya sistemi hatası: {0}")]
    DosyaSistemiHatasi(#[from] io::Error),

    #[error("HTTP isteği sırasında hata oluştu: {0}")]
    HttpIstegiHatasi(#[from] reqwest::Error),

    #[error("JSON serileştirme veya seriden çıkarma hatası: {0}")]
    JsonHatasi(#[from] serde_json::Error),

    #[error("ZIP arşivi işlenirken hata oluştu: {0}")]
    ZipHatasi(#[from] zip::result::ZipError),

    #[error("Paket bulunamadı: {0}")]
    PaketBulunamadi(String),

    #[error("Paket kurulumunda hata: {0}")]
    PaketKurulumHatasi(String),

    #[error("Bağımlılık sorunu: {0}")]
    BagimlilikSorunuHatasi(String), // Yeni: Bağımlılık sorunları için

    #[error("Checksum doğrulama hatası: Paket bütünlüğü doğrulanamadı.")]
    ChecksumDogrulamaHatasi, // Yeni: Checksum doğrulama hatası için (ek bilgiye gerek yoksa)

    #[error("Yetki hatası: İşlem için gerekli yetki bulunmuyor.")]
    YetkiHatasi, // Yeni: Yetki hataları için (ek bilgiye gerek yoksa)

    #[error("Paket çakışması: {0}")]
    PaketCakismaHatasi(String), // Yeni: Paket çakışma hataları için

    #[error("Beklenmedik bir hata oluştu: {0}")]
    BeklenmedikHata(String), // Önceden BilinmeyenHata, daha açıklayıcı bir isimle

    #[error("Betik çalıştırma hatası: {0}")]
    BetikCalistirmaHatasi(String), // Yeni: Betik çalıştırma hataları için

    #[error("Önbellek hatası: {0}")]
    OnbellekHatasi(String), // Yeni: Önbellek hataları için

    #[error("Bağımlılık bulunamadı: {0}")]
    BagimlilikBulunamadi(String), // Yeni: Belirli bir bağımlılık bulunamadığında
}
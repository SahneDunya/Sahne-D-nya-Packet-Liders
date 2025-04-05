use std::io;
use crate::paket_yoneticisi_hata::PaketYoneticisiHata; // Özel hata enum'ımızı içe aktar
use log::{info, error}; // log kütüphanesini içe aktar

// Assuming 'Sahne64' modules are in the same crate
use crate::process;
use crate::SahneError;

// Verilen betik dosyasını çalıştırır.
pub fn betik_calistir(betik_yolu: &str) -> Result<(), PaketYoneticisiHata> {
    info!("Betik çalıştırılıyor: {}", betik_yolu);

    // İşletim sistemine göre komutu yapılandır
    let (program, args) = if cfg!(target_os = "windows") {
        ("cmd".to_string(), vec!["/C".to_string(), betik_yolu.to_string()])
    } else {
        ("sh".to_string(), vec!["-c".to_string(), betik_yolu.to_string()])
    };

    // Sahne64 özel fonksiyonlarını kullanarak betiği çalıştır
    match process::create_process(&program, &args) {
        Ok(pid) => {
            match process::execute_process(pid) {
                Ok(exit_code) => {
                    match process::wait_process(pid) {
                        Ok(_) => {
                            match process::get_process_output(pid) {
                                Ok((stdout, stderr)) => {
                                    if exit_code == 0 {
                                        info!("Betik başarıyla çalıştı: {}", betik_yolu);
                                        Ok(())
                                    } else {
                                        let hata_mesaji = format!(
                                            "Betik hatayla sonlandı. Çıkış kodu: {}. Betik yolu: {}. Hata içeriği: {}",
                                            exit_code, betik_yolu, stderr.trim()
                                        );
                                        error!("{}", hata_mesaji);
                                        Err(PaketYoneticisiHata::BetikCalistirmaHatasi(hata_mesaji))
                                    }
                                }
                                Err(e) => {
                                    let hata_mesaji = format!(
                                        "Betik çıktısı alınamadı: {}. Betik yolu: {}",
                                        e, betik_yolu
                                    );
                                    error!("{}", hata_mesaji);
                                    Err(PaketYoneticisiHata::BetikCalistirmaHatasi(hata_mesaji))
                                }
                            }
                        }
                        Err(e) => {
                            let hata_mesaji = format!(
                                "Betik çalışırken hata oluştu (wait hatası): {}. Betik yolu: {}",
                                e, betik_yolu
                            );
                            error!("{}", hata_mesaji);
                            Err(PaketYoneticisiHata::BetikCalistirmaHatasi(hata_mesaji))
                        }
                    }
                }
                Err(e) => {
                    let hata_mesaji = format!(
                        "Betik yürütülemedi: {}. Betik yolu: {}",
                        e, betik_yolu
                    );
                    error!("{}", hata_mesaji);
                    Err(PaketYoneticisiHata::BetikCalistirmaHatasi(hata_mesaji))
                }
            }
        }
        Err(e) => {
            let hata_mesaji = format!(
                "Betik başlatılamadı: {}. Betik yolu: {}",
                e, betik_yolu
            );
            error!("{}", hata_mesaji);
            Err(PaketYoneticisiHata::BetikCalistirmaHatasi(hata_mesaji))
        }
    }
}
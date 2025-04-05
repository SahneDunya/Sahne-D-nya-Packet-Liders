use crossterm::{
    cursor,
    execute,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers}, // event modülünü içe aktar
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
    Result, // Result tipini açıkça içe aktar
};
use std::io::{stdout, Write};
use std::time::Duration; // Duration tipini içe aktar

// Assuming 'Sahne64' modules are in the same crate (though likely not directly applicable here)
use crate::fs;
use crate::process;
use crate::ipc;
use crate::kernel;
use crate::SahneError;

pub fn baslat_arayuz() -> Result<()> {
    // `crossterm` crate'i, terminal etkileşimleri için işletim sistemine özgü API'leri kullanır.
    // Bu nedenle, `Sahne64` özgü fonksiyonların burada doğrudan bir karşılığı bulunmamaktadır.
    // `crossterm`'ün sağladığı fonksiyonlar, terminalin ham modunu etkinleştirme,
    // ekranı temizleme, imleci hareket ettirme, renkleri ayarlama ve klavye olaylarını okuma gibi
    // düşük seviyeli terminal işlemlerini gerçekleştirir.

    enable_raw_mode()?; // Ham moda geçiş

    let mut stdout = stdout();

    let mut secili_menu = 1; // Başlangıçta seçili menü 1 olsun

    loop { // Ana döngü
        ekrani_ciz(&mut stdout, secili_menu)?; // Ekranı çiz
        stdout.flush()?;

        if event::poll(Duration::from_millis(100))? { // 100ms bekleme ile olayları kontrol et
            if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? { // Klavye olayını oku
                match code {
                    KeyCode::Up => {
                        secili_menu = if secili_menu > 1 { secili_menu - 1 } else { 4 }; // Yukarı ok tuşu: Seçili menüyü yukarı kaydır
                    }
                    KeyCode::Down => {
                        secili_menu = if secili_menu < 4 { secili_menu + 1 } else { 1 }; // Aşağı ok tuşu: Seçili menüyü aşağı kaydır
                    }
                    KeyCode::Enter => {
                        // Enter tuşu ile seçim yapıldığında yapılacak işlemler
                        match secili_menu {
                            1 => paketleri_listele_ekrani(&mut stdout)?, // 1: Paketleri Listele
                            2 => paket_ekle_ekrani(&mut stdout)?,     // 2: Paket Ekle
                            3 => paket_kaldir_ekrani(&mut stdout)?,  // 3: Paket Kaldır
                            4 => break, // 4: Çıkış (döngüden çık)
                            _ => {}
                        }
                    }
                    KeyCode::Char(c) => {
                        if modifiers.is_empty() { // Modifier tuşları (Ctrl, Shift vb.) basılmamışsa
                            match c {
                                '1' => secili_menu = 1,
                                '2' => secili_menu = 2,
                                '3' => secili_menu = 3,
                                '4' => secili_menu = 4,
                                'q' | 'Q' => break, // 'q' veya 'Q' ile çıkış
                                _ => {}
                            }
                        }
                    }
                    KeyCode::Esc => break, // Esc tuşu ile çıkış
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?; // Ham moddan çıkış
    Ok(())
}

// Arayüz ekranını çizme fonksiyonu
fn ekrani_ciz(stdout: &mut impl Write, secili_menu: u8) -> Result<()> {
    execute!(
        stdout,
        Clear(ClearType::All), // Ekranı temizle
        cursor::MoveTo(0, 0), // İmleci (0, 0) konumuna taşı
        SetForegroundColor(Color::Green),
        SetBackgroundColor(Color::Black),
        Print("Paket Yöneticisi Arayüzü"),
        ResetColor,
        cursor::MoveTo(0, 2),
    )?;

    // Menü seçeneklerini çiz
    menu_secenegini_ciz(stdout, 1, "Paketleri Listele", secili_menu == 1)?;
    menu_secenegini_ciz(stdout, 2, "Paket Ekle", secili_menu == 2)?;
    menu_secenegini_ciz(stdout, 3, "Paket Kaldır", secili_menu == 3)?;
    menu_secenegini_ciz(stdout, 4, "Çıkış", secili_menu == 4)?;

    Ok(())
}

// Tek bir menü seçeneğini çizme fonksiyonu
fn menu_secenegini_ciz(stdout: &mut impl Write, numara: u8, metin: &str, secili: bool) -> Result<()> {
    execute!(stdout, cursor::MoveTo(0, (numara + 1) as u16))?; // Menü öğesini uygun satıra taşı

    if secili {
        execute!(
            stdout,
            SetBackgroundColor(Color::DarkGreen), // Seçiliyse arka plan rengini değiştir
            SetForegroundColor(Color::White),     // Seçiliyse yazı rengini değiştir
            Print("> "), // Seçili olduğunu belirtmek için işaret
            Print(metin),
            ResetColor, // Renkleri sıfırla
        )?;
    } else {
        execute!(
            stdout,
            Print("  "), // Seçili değilse boşluk
            SetForegroundColor(Color::Grey), // Seçili değilse yazı rengini gri yap
            Print(metin),
            ResetColor, // Renkleri sıfırla
        )?;
    }
    Ok(())
}

// Menü seçeneklerine karşılık gelen ekran çizme fonksiyonları (şimdilik boş)
fn paketleri_listele_ekrani(stdout: &mut impl Write) -> Result<()> {
    ekran_temizle_ve_baslik_ciz(stdout, "Paketleri Listele")?;
    execute!(stdout, Print("Paket listesi yakında burada olacak...\n"))?;
    beklemek_icin_tiklayiniz(stdout)?; // Kullanıcıdan devam etmek için tuşa basmasını bekle
    Ok(())
}

fn paket_ekle_ekrani(stdout: &mut impl Write) -> Result<()> {
    ekran_temizle_ve_baslik_ciz(stdout, "Paket Ekle")?;
    execute!(stdout, Print("Paket ekleme ekranı yakında burada olacak...\n"))?;
    beklemek_icin_tiklayiniz(stdout)?; // Kullanıcıdan devam etmek için tuşa basmasını bekle
    Ok(())
}

fn paket_kaldir_ekrani(stdout: &mut impl Write) -> Result<()> {
    ekran_temizle_ve_baslik_ciz(stdout, "Paket Kaldır")?;
    execute!(stdout, Print("Paket kaldırma ekranı yakında burada olacak...\n"))?;
    beklemek_icin_tiklayiniz(stdout)?; // Kullanıcıdan devam etmek için tuşa basmasını bekle
    Ok(())
}

// Ekranı temizleme ve başlık çizme yardımcı fonksiyonu
fn ekran_temizle_ve_baslik_ciz(stdout: &mut impl Write, baslik: &str) -> Result<()> {
    execute!(
        stdout,
        Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        SetForegroundColor(Color::Cyan),
        Print(format!("--- {} ---\n\n", baslik)), // Başlığı çiz
        ResetColor,
    )?;
    Ok(())
}

// Kullanıcıdan bir tuşa basmasını bekleyen yardımcı fonksiyon
fn beklemek_icin_tiklayiniz(stdout: &mut impl Write) -> Result<()> {
    execute!(
        stdout,
        Print("\nDevam etmek için bir tuşa basın...") // Kullanıcıya bilgi mesajı
    )?;
    stdout.flush()?;
    event::read()?; // Tuş girdisini oku (ve hiçbir şey yapma)
    Ok(())
}
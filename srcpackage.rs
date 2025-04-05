#[derive(Debug, Clone)] 
pub struct Paket {
    pub ad: String,
    pub surum: String,
    pub bagimliliklar: Vec<String>,
    pub aciklama: Option<String>,
    pub dosya_adi: Option<String>,
}

impl Paket {
    pub fn yeni(ad: String, surum: String, bagimliliklar: Vec<String>) -> Paket {
        Paket {
            ad,
            surum,
            bagimliliklar,
            aciklama: None,
            dosya_adi: None,
        }
    }
}
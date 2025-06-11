use std::any::Any;
use std::collections::HashMap;
use thiserror::Error; // Gelişmiş hata yönetimi için thiserror kütüphanesini ekliyoruz

// Daha iyi hata yönetimi için özel bir hata türü tanımlıyoruz
#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Plugin çalışma hatası: {0}")]
    RunError(String),
}

// Plugin sonuçları için bir Result aliası tanımlıyoruz, kod okunabilirliğini artırır
pub type PluginResult<T> = Result<T, PluginError>;

pub trait Plugin {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    // Daha iyi hata yönetimi için Result dönüş türünü güncelliyoruz ve özel hata tipimizi kullanıyoruz
    fn run(&mut self, context: &mut PluginContext) -> PluginResult<()>;
}

pub struct PluginContext {
    data: HashMap<String, Box<dyn Any>>,
}

impl PluginContext {
    pub fn new() -> Self {
        PluginContext {
            data: HashMap::new(),
        }
    }

    pub fn get_data<T: 'static>(&self, key: &str) -> Option<&T> {
        self.data.get(key).and_then(|value| value.downcast_ref::<T>())
    }

    pub fn set_data<T: 'static>(&mut self, key: &str, value: T) {
        self.data.insert(key.to_string(), Box::new(value));
    }
}

// Örnek bir plugin uygulaması (isteğe bağlı, nasıl kullanılacağını göstermek için)
pub struct ExamplePlugin {
    name: String,
    version: String,
}

impl ExamplePlugin {
    pub fn new() -> Self {
        ExamplePlugin {
            name: "ExamplePlugin".to_string(),
            version: "1.0.0".to_string(),
        }
    }
}

impl Plugin for ExamplePlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn run(&mut self, context: &mut PluginContext) -> PluginResult<()> {
        println!("{} v{} çalışıyor", self.name(), self.version());

        // Context'ten veri almayı dene
        if let Some(message) = context.get_data::<String>("message") {
            println!("Context mesajı: {}", message);
        } else {
            println!("Context'te 'message' verisi bulunamadı.");
        }

        // Context'e veri ekle
        context.set_data("response", "Plugin çalışması başarılı!".to_string());

        Ok(()) // Başarılı dönüş
    }
}

fn main() {
    let mut context = PluginContext::new();
    context.set_data("message", "Merhaba, plugin dünyası!".to_string());

    let mut plugin = ExamplePlugin::new();
    println!("Plugin Adı: {}", plugin.name());
    println!("Plugin Versiyonu: {}", plugin.version());

    match plugin.run(&mut context) {
        Ok(_) => {
            println!("Plugin başarıyla çalıştı.");
            if let Some(response) = context.get_data::<String>("response") {
                println!("Context yanıtı: {}", response);
            }
        }
        Err(e) => {
            eprintln!("Plugin hatayla karşılaştı: {}", e);
        }
    }
}

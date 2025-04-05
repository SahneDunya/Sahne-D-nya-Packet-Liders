use std::io::{self, Write};
use std::collections::HashMap;

// Assuming 'Sahne64' modules are in the same crate
use crate::fs;
use crate::process; // Import process module
use crate::SahneError; // Assuming SahneError is accessible here

// Kullanıcıdan girdi alır
pub fn get_input(prompt: &str) -> io::Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_string())
}

// Komutları işlemesi için bir trait tanımla
trait CommandHandler {
    fn execute(&self) -> Result<(), SahneError>;
    fn description(&self) -> String;
}

// "hello" komutu
struct HelloCommand;
impl CommandHandler for HelloCommand {
    fn execute(&self) -> Result<(), SahneError> {
        println!("Merhaba!");
        Ok(())
    }
    fn description(&self) -> String {
        String::from("Basit bir merhaba mesajı gösterir.")
    }
}

// "date" komutu
struct DateCommand;
impl CommandHandler for DateCommand {
    fn execute(&self) -> Result<(), SahneError> {
        match process::spawn("date") {
            Ok(mut child) => {
                // Komutun tamamlanmasını bekle ve çıktısını al
                match child.wait_with_output() {
                    Ok(output) => {
                        if output.status == 0 {
                            println!("{}", String::from_utf8_lossy(&output.stdout));
                        } else {
                            eprintln!("Komut hatası (date): {}", String::from_utf8_lossy(&output.stderr));
                        }
                        Ok(())
                    }
                    Err(e) => Err(SahneError::ProcessError(format!("Komut çıktısı alınırken hata: {:?}", e))),
                }
            }
            Err(e) => Err(SahneError::ProcessError(format!("'date' komutu başlatılamadı: {:?}", e))),
        }
    }
    fn description(&self) -> String {
        String::from("Sistem tarihini gösterir.")
    }
}

// "exit" komutu
struct ExitCommand;
impl CommandHandler for ExitCommand {
    fn execute(&self) -> Result<(), SahneError> {
        process::exit(0);
    }
    fn description(&self) -> String {
        String::from("Programdan çıkar.")
    }
}

// "echo" komutu - örnek olarak argüman alabilen bir komut
struct EchoCommand {
    args: Vec<String>,
}

impl EchoCommand {
    fn new(args: Vec<String>) -> Self {
        EchoCommand { args }
    }
}

impl CommandHandler for EchoCommand {
    fn execute(&self) -> Result<(), SahneError> {
        println!("{}", self.args.join(" "));
        Ok(())
    }
    fn description(&self) -> String {
        String::from("Verilen argümanları ekrana yazdırır. Kullanım: echo [mesaj]")
    }
}
// "ls" komutu - harici komut örneği ve hata yönetimi
struct LsCommand;

impl CommandHandler for LsCommand {
    fn execute(&self) -> Result<(), SahneError> {
        match process::spawn("ls") {
            Ok(mut child) => {
                match child.wait_with_output() {
                    Ok(output) => {
                        if output.status == 0 {
                            println!("{}", String::from_utf8_lossy(&output.stdout));
                        } else {
                            eprintln!("Komut hatası (ls): {}", String::from_utf8_lossy(&output.stderr));
                        }
                        Ok(())
                    }
                    Err(e) => Err(SahneError::ProcessError(format!("Komut çıktısı alınırken hata: {:?}", e))),
                }
            }
            Err(e) => Err(SahneError::ProcessError(format!("'ls' komutu başlatılamadı: {:?}", e))),
        }
    }
    fn description(&self) -> String {
        String::from("Bulunduğunuz dizindeki dosyaları listeler.")
    }
}
// "clear" komutu - ekranı temizleme
struct ClearCommand;

impl CommandHandler for ClearCommand {
    fn execute(&self) -> Result<(), SahneError> {
        // Sahne64'e özgü ekran temizleme komutu veya mekanizması
        match process::spawn("clear") {
            Ok(mut child) => {
                match child.wait() {
                    Ok(status) => {
                        if status == 0 {
                            Ok(())
                        } else {
                            Err(SahneError::ProcessError("Ekran temizleme komutu başarısız oldu.".to_string()))
                        }
                    }
                    Err(e) => Err(SahneError::ProcessError(format!("Ekran temizleme komutu beklenirken hata: {:?}", e))),
                }
            }
            Err(e) => Err(SahneError::ProcessError(format!("'clear' komutu başlatılamadı: {:?}", e))),
        }
    }
    fn description(&self) -> String {
        String::from("Ekranı temizler.")
    }
}


// "help" komutu
struct HelpCommand {
    command_map: HashMap<String, Box<dyn CommandHandler>>,
}

impl HelpCommand {
    fn new(command_map: &HashMap<String, Box<dyn CommandHandler>>) -> Self {
        HelpCommand { command_map: command_map.clone() } // Klonlayarak bağımsızlık
    }
}
impl CommandHandler for HelpCommand {
    fn execute(&self) -> Result<(), SahneError> {
        println!("Kullanılabilir komutlar:");
        for (name, handler) in &self.command_map {
            println!("- {}: {}", name, handler.description());
        }
        Ok(())
    }
    fn description(&self) -> String {
        String::from("Kullanılabilir komutları ve açıklamalarını listeler.")
    }
}


// Komutları işle
pub fn handle_command(command_str: &str, command_map: &HashMap<String, Box<dyn CommandHandler>>) -> Result<(), SahneError> {
    let parts: Vec<&str> = command_str.split_whitespace().collect();
    if parts.is_empty() {
        return Ok(()); // Boş girdi
    }

    let command_name = parts[0];
    let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();


    if let Some(handler) = command_map.get(command_name) {
        // Eğer komut argüman alıyorsa ve handler argüman bekliyorsa, argümanları işle
        if command_name == "echo" { // Örnek olarak "echo" için argümanları işle
            let echo_command = EchoCommand::new(args);
            echo_command.execute()?;
        } else {
            handler.execute()?;
        }
    } else {
        println!("Geçersiz komut. 'help' komutunu kullanarak kullanılabilir komutları görebilirsiniz.");
    }
    Ok(())
}

// Etkileşimli döngüyü başlatır
pub fn start_interactive_loop(command_map: HashMap<String, Box<dyn CommandHandler>>) -> io::Result<()> {
    loop {
        let input = get_input("> ")?;
        match handle_command(&input, &command_map) {
            Ok(_) => {}
            Err(e) => eprintln!("Komut işlenirken hata oluştu: {:?}", e),
        }
    }
}


fn main() -> io::Result<()> {
    let mut command_map: HashMap<String, Box<dyn CommandHandler>> = HashMap::new();
    command_map.insert("hello".to_string(), Box::new(HelloCommand));
    command_map.insert("date".to_string(), Box::new(DateCommand));
    command_map.insert("exit".to_string(), Box::new(ExitCommand));
    command_map.insert("ls".to_string(), Box::new(LsCommand));
    command_map.insert("help".to_string(), Box::new(HelpCommand::new(&command_map))); // Help komutunu ekle ve command_map'i geçir.
    command_map.insert("clear".to_string(), Box::new(ClearCommand)); // Clear komutunu ekle.

    start_interactive_loop(command_map)?;
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_input() {
        // Manuel test gerektirir.
        println!("Lütfen manuel olarak 'test input' yazın ve enter'a basın.");
        let result = get_input("Test Input Prompt: ").unwrap();
        assert_eq!(result, "test input");
    }

    #[test]
    fn test_handle_command_hello() {
        let mut command_map: HashMap<String, Box<dyn CommandHandler>> = HashMap::new();
        command_map.insert("hello".to_string(), Box::new(HelloCommand));
        let result = handle_command("hello", &command_map);
        assert!(result.is_ok());
        // Burada çıktıyı yakalamak ve doğrulamak için daha karmaşık testler yazılabilir.
    }

    #[test]
    fn test_handle_command_invalid() {
        let command_map: HashMap<String, Box<dyn CommandHandler>> = HashMap::new(); // Boş map ile test
        let result = handle_command("invalid_command", &command_map);
        assert!(result.is_ok()); // Geçersiz komut hatası basar, ancak hata döndürmez.
    }

    #[test]
    fn test_echo_command() {
        let mut command_map: HashMap<String, Box<dyn CommandHandler>> = HashMap::new();
        // Echo komutunu doğru şekilde ekleyin
        command_map.insert("echo".to_string(), Box::new(EchoCommand::new(vec![]))); // Başlangıçta boş args ile ekleyin.
        let result = handle_command("echo test message", &command_map);
        assert!(result.is_ok());
        // Çıktıyı yakalama ve doğrulama daha gelişmiş test teknikleri gerektirir.
    }
}
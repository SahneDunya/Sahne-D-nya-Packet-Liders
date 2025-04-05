use eframe::{egui, App, Frame};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

// Assuming 'Sahne64' modules are in the same crate
use crate::fs;
use crate::SahneError; // Assuming SahneError is accessible here

pub struct Gui {
    package_list: Vec<String>,
    request_tx: mpsc::Sender<Vec<String>>, // For sending requests from GUI
    update_rx: mpsc::Receiver<Vec<String>>, // For receiving updates to package list
}

impl Gui {
    pub fn new(request_tx: mpsc::Sender<Vec<String>>, update_rx: mpsc::Receiver<Vec<String>>) -> Self {
        Gui {
            package_list: Vec::new(),
            request_tx,
            update_rx,
        }
    }

    pub fn run(mut self) {
        // Start a separate thread to receive package list updates
        thread::spawn(move || {
            loop {
                if let Ok(packages) = self.update_rx.recv() {
                    // Update package list when new list is received
                    crate::srcgui::Gui::update_package_list(&mut self, packages);
                }
            }
        });

        let native_options = eframe::NativeOptions::default();
        eframe::run_native(
            "Paket Yöneticisi",
            native_options,
            Box::new(|cc| Box::new(self)),
        );
    }

    pub fn update_package_list(&mut self, packages: Vec<String>) {
        self.package_list = packages;
    }
}

impl App for Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Paket Listesi");
            for package in &self.package_list {
                ui.label(package);
            }
            if ui.button("Paketleri Yenile").clicked() {
                // Paketleri yenileme isteğini gönder
                self.request_tx.send(Vec::new()).unwrap();
            }
        });
    }
}

pub fn start_gui() -> mpsc::Receiver<Vec<String>> {
    let (request_tx, request_rx) = mpsc::channel(); // For requests from GUI
    let (update_tx, update_rx) = mpsc::channel();    // For updates to GUI

    let gui = Gui::new(request_tx, update_rx);
    thread::spawn(move || {
        gui.run();
    });

    // Start a separate thread to handle package refresh requests
    thread::spawn(move || {
        loop {
            if request_rx.recv().is_ok() {
                // Fetch package list using Sahne64 specific functions
                println!("Paket listesi yenileme isteği alındı...");
                match fetch_package_list_sahne64() {
                    Ok(updated_packages) => {
                        println!("Paket listesi güncellendi.");
                        // Send the updated list back to GUI
                        update_tx.send(updated_packages).unwrap();
                    }
                    Err(e) => eprintln!("Paket listesi alınırken hata oluştu: {:?}", e),
                }
            }
        }
    });

    request_rx // Return the receiver for requests (though not actively used in this example after setup)
}

// Fetch package list using Sahne64 file system
fn fetch_package_list_sahne64() -> Result<Vec<String>, SahneError> {
    let package_list_path = "/etc/packages.list"; // Assuming this is where the package list is stored
    match fs::open(package_list_path, fs::O_RDONLY) {
        Ok(fd) => {
            let mut buffer = Vec::new();
            let mut read_buffer = [0u8; 128]; // Read in chunks
            loop {
                match fs::read(fd, &mut read_buffer) {
                    Ok(bytes_read) => {
                        if bytes_read == 0 {
                            break;
                        }
                        buffer.extend_from_slice(&read_buffer[..bytes_read]);
                    }
                    Err(e) => {
                        fs::close(fd).unwrap_or_default(); // Attempt to close on error
                        return Err(e);
                    }
                }
            }
            fs::close(fd).unwrap_or_default();

            match String::from_utf8(buffer) {
                Ok(content) => {
                    let packages: Vec<String> = content
                        .lines()
                        .map(|line| line.trim().to_string())
                        .filter(|line| !line.is_empty())
                        .collect();
                    Ok(packages)
                }
                Err(_) => Err(SahneError::InvalidParameter), // Or a more specific error if needed
            }
        }
        Err(e) => Err(e),
    }
}

// Simulate fetching package list from a backend (removed)
// fn fetch_package_list() -> Vec<String> { ... }

fn main() {
    let _request_receiver = start_gui();
    // In a real application, you might use request_receiver to send initial requests or handle other logic
    // For this example, the GUI refresh button drives the updates.

    // Keep the main thread alive for the example to run, in a real application,
    // the main thread might have other tasks.
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}
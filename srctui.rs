use std::io;
use thiserror::Error;

// Assuming 'Sahne64' modules are in the same crate
use crate::fs;
use crate::SahneError; // Assuming SahneError is accessible here

#[derive(Error, Debug)]
pub enum TuiError {
    #[error("Failed to initialize terminal: {0}")]
    TerminalInitializationError(#[source] io::Error),
    #[error("Failed to draw to terminal: {0}")]
    TerminalDrawError(#[source] io::Error),
    #[error("Sahne64 File System Error: {0}")]
    Sahne64FileSystemError(#[from] crate::SahneError),
    #[error("IO Error: {0}")]
    IoError(#[from] io::Error),
}

// Removed the Tui struct as tui crate is not directly usable in no_std environment without a compatible backend

// Simplified function to display items using Sahne64's file system (assuming stdout is fd 1)
pub fn draw_sahne64_tui(items: &[String]) -> Result<(), TuiError> {
    let stdout_fd = 1; // Assuming file descriptor 1 is standard output

    for item in items {
        let line = format!("{}\n", item);
        let buffer = line.as_bytes();
        let mut written = 0;
        while written < buffer.len() {
            match fs::write(stdout_fd, &buffer[written..]) {
                Ok(bytes) => written += bytes,
                Err(e) => return Err(TuiError::Sahne64FileSystemError(e)),
            }
        }
    }
    Ok(())
}
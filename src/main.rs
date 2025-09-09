#![windows_subsystem = "windows"]

mod scanner;
mod gui;
mod filter;

use std::env;
use std::path::PathBuf;
use windows::core::Result;
use crossbeam_channel::unbounded;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    
    let target_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    if !target_path.exists() {
        eprintln!("Error: Path '{}' does not exist", target_path.display());
        return Ok(());
    }

    if !target_path.is_dir() {
        eprintln!("Error: Path '{}' is not a directory", target_path.display());
        return Ok(());
    }

    let (sender, receiver) = unbounded();
    
    let mut window = gui::FileListWindow::new()?;
    
    window.set_update_receiver(receiver);
    
    let scanner_path = target_path.clone();
    std::thread::spawn(move || {
        let scanner = scanner::Scanner::with_sender(sender);
        scanner.scan_directory(&scanner_path);
    });

    window.load_directory(&target_path);
    
    window.run_message_loop()?;

    Ok(())
}
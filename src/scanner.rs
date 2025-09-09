use std::path::{Path, PathBuf};
use std::time::{SystemTime, Instant};
use rayon::prelude::*;
use walkdir::{WalkDir, DirEntry};
use crossbeam_channel::Sender;

#[derive(Debug, Clone)]
pub enum ScanMessage {
    Started,
    Batch(Vec<FileInfo>),
    Completed { elapsed_ms: u128, file_count: usize },
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_shortcut: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub extension: Option<String>,
}

impl FileInfo {
    fn from_entry(entry: &DirEntry) -> Option<Self> {
        let path = entry.path().to_path_buf();
        
        // Use entry.file_type() instead of full metadata for speed
        let file_type = entry.file_type();
        let is_dir = file_type.is_dir();
        
        // Check if it's a shortcut
        let extension = if !is_dir {
            path.extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase())
        } else {
            None
        };
        
        let is_shortcut = extension.as_deref() == Some("lnk");
        
        // Only get full metadata for files (not directories) to save time
        let (size, modified) = if !is_dir {
            entry.metadata().ok()
                .map(|m| (m.len(), m.modified().ok()))
                .unwrap_or((0, None))
        } else {
            (0, None)
        };
        
        Some(FileInfo {
            name: entry.file_name().to_string_lossy().to_string(),
            is_dir,
            is_shortcut,
            size,
            modified,
            extension,
            path,
        })
    }

    pub fn size_formatted(&self) -> String {
        if self.is_dir {
            String::new()
        } else {
            format_size(self.size)
        }
    }

    pub fn get_icon(&self) -> &str {
        if self.is_dir {
            return "📁";
        }
        
        // Special icon for shortcuts
        if self.is_shortcut {
            return "🔗";
        }

        let ext = self.extension.as_ref().map(|s| s.to_lowercase());
        match ext.as_deref() {
            // Documents
            Some("pdf") => "📄",
            Some("doc") | Some("docx") => "📝",
            Some("txt") | Some("md") | Some("rtf") => "📃",
            Some("xls") | Some("xlsx") | Some("csv") => "📊",
            Some("ppt") | Some("pptx") => "📽️",
            
            // Images
            Some("jpg") | Some("jpeg") | Some("png") | Some("gif") | Some("bmp") | Some("svg") | Some("webp") => "🖼️",
            Some("ico") => "🎨",
            Some("psd") | Some("ai") => "🎨",
            
            // Video
            Some("mp4") | Some("avi") | Some("mkv") | Some("mov") | Some("wmv") | Some("flv") => "🎬",
            
            // Audio
            Some("mp3") | Some("wav") | Some("flac") | Some("aac") | Some("ogg") | Some("wma") => "🎵",
            
            // Code
            Some("rs") | Some("py") | Some("js") | Some("ts") | Some("cpp") | Some("c") | Some("h") => "💻",
            Some("html") | Some("css") | Some("scss") | Some("sass") => "🌐",
            Some("json") | Some("xml") | Some("yaml") | Some("toml") => "⚙️",
            Some("sql") | Some("db") => "🗄️",
            
            // Archives
            Some("zip") | Some("rar") | Some("7z") | Some("tar") | Some("gz") => "📦",
            
            // Executables
            Some("exe") | Some("msi") | Some("bat") | Some("cmd") => "⚡",
            Some("dll") | Some("sys") => "🔧",
            
            // Special
            Some("log") => "📋",
            Some("bak") | Some("tmp") => "💾",
            
            _ => "📄"  // Default file icon
        }
    }

    pub fn modified_formatted(&self) -> String {
        self.modified
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| {
                let datetime = chrono::DateTime::<chrono::Local>::from(
                    SystemTime::UNIX_EPOCH + d
                );
                datetime.format("%Y-%m-%d %H:%M").to_string()
            })
            .unwrap_or_default()
    }
}

fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

pub struct Scanner {
    sender: Option<Sender<ScanMessage>>,
}

impl Scanner {
    pub fn new() -> Self {
        Scanner { sender: None }
    }

    pub fn with_sender(sender: Sender<ScanMessage>) -> Self {
        Scanner { sender: Some(sender) }
    }

    pub fn scan_directory(&self, path: &Path) -> Vec<FileInfo> {
        let start_time = Instant::now();
        
        // Send start signal
        if let Some(ref sender) = self.sender {
            let _ = sender.send(ScanMessage::Started);
        }
        
        let batch_size = 100;
        let mut batch = Vec::with_capacity(batch_size);
        let mut all_files = Vec::new();

        let walker = WalkDir::new(path)
            .follow_links(false)
            .max_open(10)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path() != path); 

        for entry in walker {
            if let Some(file_info) = FileInfo::from_entry(&entry) {
                // Skip directories - only include files
                if file_info.is_dir {
                    continue;
                }
                
                batch.push(file_info);
                
                if batch.len() >= batch_size {
                    // Send batch without cloning
                    if let Some(ref sender) = self.sender {
                        let _ = sender.send(ScanMessage::Batch(batch.clone()));
                    }
                    
                    // Keep a local copy if needed
                    all_files.extend(batch.drain(..));
                    batch.reserve(batch_size);
                }
            }
        }

        if !batch.is_empty() {
            if let Some(ref sender) = self.sender {
                let _ = sender.send(ScanMessage::Batch(batch.clone()));
            }
            all_files.extend(batch);
        }
        
        // Send completion signal with timing info
        let elapsed = start_time.elapsed();
        if let Some(ref sender) = self.sender {
            let _ = sender.send(ScanMessage::Completed {
                elapsed_ms: elapsed.as_millis(),
                file_count: all_files.len(),
            });
        }

        all_files
    }

    pub fn scan_directory_parallel(&self, path: &Path) -> Vec<FileInfo> {
        // Skip only the most problematic folders that significantly slow scanning
        let skip_dirs = [
            "node_modules",           // Can have 100k+ files
            "$RECYCLE.BIN",          // System recycle bin
            "System Volume Information", // Windows system folder
            ".git\\objects",         // Git internal objects
            "AppData\\Local\\Temp",  // Temp files
            "AppData\\Roaming\\npm-cache", // NPM cache
        ];
        
        let entries: Vec<_> = WalkDir::new(path)
            .follow_links(false)
            .max_depth(10)  // Increased depth limit
            .max_open(50)  // Increase for better parallelism
            .into_iter()
            .filter_entry(|e| {
                // Only filter directories, not files
                if e.file_type().is_dir() && e.depth() > 0 {
                    let path_str = e.path().to_string_lossy();
                    let name = e.file_name().to_string_lossy();
                    
                    // Skip if directory name matches exactly
                    if skip_dirs.iter().any(|&skip| {
                        if skip.contains('\\') {
                            // For paths with backslash, check if path contains it
                            path_str.to_lowercase().contains(&skip.to_lowercase())
                        } else {
                            // For simple names, check exact match
                            name.eq_ignore_ascii_case(skip)
                        }
                    }) {
                        return false;
                    }
                }
                true // Include everything else
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.path() != path)
            .take(100000)  // Increased limit
            .collect();

        let files: Vec<_> = entries
            .par_iter()
            .filter_map(|entry| FileInfo::from_entry(entry))
            .filter(|f| !f.is_dir)  // Only include files, not directories
            .collect();

        files
    }
}
#![allow(unused_unsafe)]

use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::{
            LibraryLoader::*,
            DataExchange::*,
            Memory::*,
        },
        UI::{
            Controls::*,
            Shell::*,
            WindowsAndMessaging::*,
            Input::KeyboardAndMouse::*,
        },
    },
};
use std::sync::{Arc, Mutex};
use std::path::Path;
use std::time::Instant;
use crate::scanner::{FileInfo, ScanMessage};
use crate::filter::Filter;
use crossbeam_channel::{Receiver, unbounded, Sender};
use std::thread;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const ID_LISTVIEW: i32 = 1001;
const ID_SEARCHBOX: i32 = 1002;
const ID_STATUSBAR: i32 = 1003;
const ID_COPY_BUTTON: i32 = 1004;
const ID_OPEN_FOLDER_BUTTON: i32 = 1005;
const ID_CHK_TYPE: i32 = 1006;
const ID_CHK_SIZE: i32 = 1007;
const ID_CHK_MODIFIED: i32 = 1008;
const ID_CHK_PATH: i32 = 1009;
const ID_SIGNATURE: i32 = 1010;
const ID_HELP_BUTTON: i32 = 1011;
const ID_CTX_OPEN: i32 = 2001;
const ID_CTX_OPEN_FOLDER: i32 = 2002;
const ID_CTX_COPY_PATH: i32 = 2003;
const ID_CTX_COPY_NAME: i32 = 2004;
const WM_UPDATE_LIST: u32 = WM_USER + 1;
const WM_HOTKEY: u32 = 0x0312;
const WM_UPDATE_SEARCH: u32 = WM_USER + 2;
const WM_DROPFILES: u32 = 0x0233;
const LVN_KEYDOWN: u32 = 4294967141; // LVN_FIRST - 155
const NM_CUSTOMDRAW: u32 = 4294967284; // NM_FIRST - 12
const CF_UNICODETEXT: u32 = 13;
const EN_CHANGE: u32 = 0x0300;
const CDDS_PREPAINT: u32 = 1;
const CDDS_ITEMPREPAINT: u32 = 0x10001;
const CDRF_NOTIFYITEMDRAW: u32 = 0x20;
const CDRF_NEWFONT: u32 = 0x2;
const BS_PUSHBUTTON: u32 = 0x00000000;
const BS_AUTOCHECKBOX: u32 = 0x00000003;
const BM_GETCHECK: u32 = 0x00F0;
const BM_SETCHECK: u32 = 0x00F1;
const BST_CHECKED: u32 = 0x0001;
const SS_CENTER: u32 = 0x00000001;

#[repr(C)]
#[allow(non_snake_case)]
struct NMLVKEYDOWN {
    hdr: NMHDR,
    wVKey: u16,
    flags: u32,
}

#[repr(C)]
#[allow(non_snake_case)]
struct NMLISTVIEW {
    hdr: NMHDR,
    iItem: i32,
    iSubItem: i32,
    uNewState: u32,
    uOldState: u32,
    uChanged: u32,
    ptAction: POINT,
    lParam: isize,
}

#[repr(C)]
#[allow(non_snake_case)]
struct NMLVCUSTOMDRAW {
    nmcd: NMCUSTOMDRAW,
    clrText: u32,
    clrTextBk: u32,
    iSubItem: i32,
    dwItemType: u32,
    clrFace: u32,
    iIconEffect: i32,
    iIconPhase: i32,
    iPartId: i32,
    iStateId: i32,
    rcText: RECT,
    uAlign: u32,
}

#[repr(C)]
#[allow(non_snake_case)]
struct NMCUSTOMDRAW {
    hdr: NMHDR,
    dwDrawStage: u32,
    hdc: HDC,
    rc: RECT,
    dwItemSpec: usize,
    uItemState: u32,
    lItemlParam: isize,
}

pub struct FileListWindow {
    hwnd: HWND,
    list_view: HWND,
    search_box: HWND,
    status_bar: HWND,
    copy_button: HWND,
    open_folder_button: HWND,
    chk_type: HWND,
    chk_size: HWND,
    chk_modified: HWND,
    chk_path: HWND,
    signature_label: HWND,
    help_button: HWND,
    files: Arc<Mutex<Vec<FileInfo>>>,
    filtered_files: Arc<Mutex<Vec<FileInfo>>>,
    filter: Arc<Mutex<Filter>>,
    search_sender: Option<Sender<String>>,
    is_searching: Arc<AtomicBool>,
    scan_animation_frame: Arc<Mutex<usize>>,
    scan_start_time: Arc<Mutex<Option<Instant>>>,
    scan_elapsed_ms: Arc<AtomicUsize>,
    is_scanning: Arc<AtomicBool>,
    show_type: Arc<AtomicBool>,
    show_size: Arc<AtomicBool>,
    show_modified: Arc<AtomicBool>,
    show_path: Arc<AtomicBool>,
    sort_column: Arc<Mutex<i32>>,
    sort_ascending: Arc<AtomicBool>,
}

impl FileListWindow {
    pub fn new() -> Result<Box<Self>> {
        let mut window = Box::new(Self {
            hwnd: HWND::default(),
            list_view: HWND::default(),
            search_box: HWND::default(),
            status_bar: HWND::default(),
            copy_button: HWND::default(),
            open_folder_button: HWND::default(),
            chk_type: HWND::default(),
            chk_size: HWND::default(),
            chk_modified: HWND::default(),
            chk_path: HWND::default(),
            signature_label: HWND::default(),
            help_button: HWND::default(),
            files: Arc::new(Mutex::new(Vec::new())),
            filtered_files: Arc::new(Mutex::new(Vec::new())),
            filter: Arc::new(Mutex::new(Filter::new())),
            search_sender: None,
            is_searching: Arc::new(AtomicBool::new(false)),
            scan_animation_frame: Arc::new(Mutex::new(0)),
            scan_start_time: Arc::new(Mutex::new(None)),
            scan_elapsed_ms: Arc::new(AtomicUsize::new(0)),
            is_scanning: Arc::new(AtomicBool::new(false)),
            show_type: Arc::new(AtomicBool::new(true)),
            show_size: Arc::new(AtomicBool::new(true)),
            show_modified: Arc::new(AtomicBool::new(true)),
            show_path: Arc::new(AtomicBool::new(true)),
            sort_column: Arc::new(Mutex::new(-1)),
            sort_ascending: Arc::new(AtomicBool::new(true)),
        });

        window.create_window()?;
        window.setup_search_thread();
        Ok(window)
    }

    pub fn set_update_receiver(&mut self, receiver: Receiver<ScanMessage>) {
        let files = Arc::clone(&self.files);
        let filtered_files = Arc::clone(&self.filtered_files);
        let filter = Arc::clone(&self.filter);
        let scan_start_time = Arc::clone(&self.scan_start_time);
        let scan_elapsed_ms = Arc::clone(&self.scan_elapsed_ms);
        let is_scanning = Arc::clone(&self.is_scanning);
        let hwnd = self.hwnd.0 as isize;
        
        thread::spawn(move || {
            while let Ok(msg) = receiver.recv() {
                match msg {
                    ScanMessage::Started => {
                        // Record start time
                        *scan_start_time.lock().unwrap() = Some(Instant::now());
                        is_scanning.store(true, Ordering::SeqCst);
                        scan_elapsed_ms.store(0, Ordering::SeqCst);
                        
                        unsafe {
                            let hwnd = HWND(hwnd as *mut _);
                            PostMessageW(hwnd, WM_UPDATE_LIST, WPARAM(0), LPARAM(0)).ok();
                        }
                    },
                    ScanMessage::Batch(batch) => {
                        // Append batch to master list
                        {
                            let mut all = files.lock().unwrap();
                            all.extend(batch.iter().cloned());
                        }
                        
                        // Append only matches to filtered list (so search results grow live)
                        {
                            let flt = filter.lock().unwrap();
                            let mut ff = filtered_files.lock().unwrap();
                            for item in &batch {
                                if flt.matches(item) {
                                    ff.push(item.clone());
                                }
                            }
                        }
                        
                        // Update elapsed time during scan
                        if let Some(start) = *scan_start_time.lock().unwrap() {
                            let elapsed = start.elapsed().as_millis() as usize;
                            scan_elapsed_ms.store(elapsed, Ordering::SeqCst);
                        }
                        
                        unsafe {
                            let hwnd = HWND(hwnd as *mut _);
                            PostMessageW(hwnd, WM_UPDATE_LIST, WPARAM(0), LPARAM(0)).ok();
                        }
                        
                        // Small delay to batch UI updates  
                        thread::sleep(std::time::Duration::from_millis(30));
                    },
                    ScanMessage::Completed { elapsed_ms, file_count: _ } => {
                        // Mark scan as complete
                        is_scanning.store(false, Ordering::SeqCst);
                        scan_elapsed_ms.store(elapsed_ms as usize, Ordering::SeqCst);
                        
                        unsafe {
                            let hwnd = HWND(hwnd as *mut _);
                            PostMessageW(hwnd, WM_UPDATE_LIST, WPARAM(0), LPARAM(0)).ok();
                        }
                    }
                }
            }
        });
    }

    fn create_window(&mut self) -> Result<()> {
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let window_class = w!("FileListGenerator");

            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(window_proc),
                hInstance: instance.into(),
                hIcon: LoadIconW(None, IDI_APPLICATION)?,
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as _),
                lpszClassName: window_class,
                ..Default::default()
            };

            RegisterClassExW(&wc);

            self.hwnd = CreateWindowExW(
                WS_EX_ACCEPTFILES, // Accept dropped files
                window_class,
                w!("File List Generator"),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                1200,
                800,
                None,
                None,
                instance,
                Some(self as *mut _ as _),
            )?;

            self.create_controls()?;
            
            // Register hotkeys (removed Ctrl+C and Ctrl+V to avoid hijacking)
            let _ = RegisterHotKey(self.hwnd, 1, HOT_KEY_MODIFIERS(0x0002), 0x46); // Ctrl+F (MOD_CONTROL = 0x0002)
            let _ = RegisterHotKey(self.hwnd, 3, HOT_KEY_MODIFIERS(0x0002), 0x4F); // Ctrl+O
        }
        
        Ok(())
    }

    fn create_controls(&mut self) -> Result<()> {
        unsafe {
            let instance = GetModuleHandleW(None)?;

            self.search_box = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("EDIT"),
                w!(""),
                WS_CHILD | WS_VISIBLE | WS_BORDER | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
                10, 10, 300, 25,
                self.hwnd,
                HMENU(ID_SEARCHBOX as _),
                instance,
                None,
            )?;

            SendMessageW(
                self.search_box,
                EM_SETCUEBANNER,
                WPARAM(1),
                LPARAM(w!("Search: filename, .ext, >10mb, <1kb...").as_ptr() as _),
            );

            // Add Copy to Clipboard button
            self.copy_button = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                w!("Copy List"),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_PUSHBUTTON as u32),
                320, 10, 80, 25,
                self.hwnd,
                HMENU(ID_COPY_BUTTON as _),
                instance,
                None,
            )?;

            // Add Open Folder button
            self.open_folder_button = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                w!("Open Folder"),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_PUSHBUTTON as u32),
                410, 10, 90, 25,
                self.hwnd,
                HMENU(ID_OPEN_FOLDER_BUTTON as _),
                instance,
                None,
            )?;

            // Add checkboxes for column visibility
            self.chk_type = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                w!("Type"),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
                510, 10, 60, 25,
                self.hwnd,
                HMENU(ID_CHK_TYPE as _),
                instance,
                None,
            )?;
            SendMessageW(self.chk_type, BM_SETCHECK, WPARAM(BST_CHECKED as usize), LPARAM(0));

            self.chk_size = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                w!("Size"),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
                580, 10, 60, 25,
                self.hwnd,
                HMENU(ID_CHK_SIZE as _),
                instance,
                None,
            )?;
            SendMessageW(self.chk_size, BM_SETCHECK, WPARAM(BST_CHECKED as usize), LPARAM(0));

            self.chk_modified = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                w!("Modified"),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
                650, 10, 80, 25,
                self.hwnd,
                HMENU(ID_CHK_MODIFIED as _),
                instance,
                None,
            )?;
            SendMessageW(self.chk_modified, BM_SETCHECK, WPARAM(BST_CHECKED as usize), LPARAM(0));

            self.chk_path = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                w!("Path"),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
                740, 10, 60, 25,
                self.hwnd,
                HMENU(ID_CHK_PATH as _),
                instance,
                None,
            )?;
            SendMessageW(self.chk_path, BM_SETCHECK, WPARAM(BST_CHECKED as usize), LPARAM(0));

            // Add Help button
            self.help_button = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                w!("?"),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_PUSHBUTTON as u32),
                810, 10, 30, 25,
                self.hwnd,
                HMENU(ID_HELP_BUTTON as _),
                instance,
                None,
            )?;

            self.list_view = CreateWindowExW(
                WS_EX_CLIENTEDGE,
                WC_LISTVIEW,
                w!(""),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(
                    LVS_REPORT | LVS_SINGLESEL | LVS_SHOWSELALWAYS | LVS_OWNERDATA
                ),
                10, 45, 1160, 660,
                self.hwnd,
                HMENU(ID_LISTVIEW as _),
                instance,
                None,
            )?;

            SendMessageW(
                self.list_view,
                LVM_SETEXTENDEDLISTVIEWSTYLE,
                WPARAM(0),
                LPARAM((LVS_EX_FULLROWSELECT | LVS_EX_GRIDLINES | LVS_EX_DOUBLEBUFFER) as _),
            );

            self.setup_list_columns()?;

            // Add developer signature
            self.signature_label = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("Developed by David Landry"),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(SS_CENTER as u32),
                10, 710, 1160, 20,
                self.hwnd,
                HMENU(ID_SIGNATURE as _),
                instance,
                None,
            )?;

            self.status_bar = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                STATUSCLASSNAMEW,
                w!("Ready"),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(SBARS_SIZEGRIP),
                0, 0, 0, 0,
                self.hwnd,
                HMENU(ID_STATUSBAR as _),
                instance,
                None,
            )?;
        }

        Ok(())
    }
    
    fn add_tooltip(&self, control: HWND, text: &HSTRING) {
        unsafe {
            // Create tooltip window
            let tooltip = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("tooltips_class32"),
                &HSTRING::new(),
                WS_POPUP | WINDOW_STYLE(0x01), // TTS_ALWAYSTIP
                0, 0, 0, 0,
                self.hwnd,
                None,
                GetModuleHandleW(None).ok().unwrap_or_default(),
                None,
            ).ok();
            
            if let Some(tooltip_hwnd) = tooltip {
                // Prepare TOOLINFO structure
                #[repr(C)]
                #[allow(non_snake_case)]
                struct TOOLINFOW {
                    cbSize: u32,
                    uFlags: u32,
                    hwnd: HWND,
                    uId: usize,
                    rect: RECT,
                    hinst: HMODULE,
                    lpszText: PWSTR,
                    lParam: LPARAM,
                    lpReserved: *mut std::ffi::c_void,
                }
                
                let ti = TOOLINFOW {
                    cbSize: std::mem::size_of::<TOOLINFOW>() as u32,
                    uFlags: 0x01 | 0x10, // TTF_IDISHWND | TTF_SUBCLASS
                    hwnd: self.hwnd,
                    uId: control.0 as usize,
                    rect: RECT::default(),
                    hinst: GetModuleHandleW(None).ok().unwrap_or_default(),
                    lpszText: PWSTR(text.as_ptr() as *mut _),
                    lParam: LPARAM(0),
                    lpReserved: std::ptr::null_mut(),
                };
                
                // Add tool
                SendMessageW(
                    tooltip_hwnd,
                    0x0432, // TTM_ADDTOOLW
                    WPARAM(0),
                    LPARAM(&ti as *const _ as _),
                );
            }
        }
    }

    fn setup_list_columns(&self) -> Result<()> {
        unsafe {
            // Clear existing columns
            loop {
                if SendMessageW(self.list_view, LVM_DELETECOLUMN, WPARAM(0), LPARAM(0)).0 == 0 {
                    break;
                }
            }

            let mut col_index = 0;

            // Always show Name column
            let name_column = LVCOLUMNW {
                mask: LVCF_TEXT | LVCF_WIDTH | LVCF_FMT,
                fmt: LVCFMT_LEFT,
                cx: 400,
                pszText: PWSTR(w!("Name").as_ptr() as *mut _),
                ..Default::default()
            };
            SendMessageW(
                self.list_view,
                LVM_INSERTCOLUMNW,
                WPARAM(col_index),
                LPARAM(&name_column as *const _ as _),
            );
            col_index += 1;

            // Conditionally add other columns
            if self.show_type.load(Ordering::SeqCst) {
                let column = LVCOLUMNW {
                    mask: LVCF_TEXT | LVCF_WIDTH | LVCF_FMT,
                    fmt: LVCFMT_LEFT,
                    cx: 100,
                    pszText: PWSTR(w!("Type").as_ptr() as *mut _),
                    ..Default::default()
                };
                SendMessageW(
                    self.list_view,
                    LVM_INSERTCOLUMNW,
                    WPARAM(col_index),
                    LPARAM(&column as *const _ as _),
                );
                col_index += 1;
            }

            if self.show_size.load(Ordering::SeqCst) {
                let column = LVCOLUMNW {
                    mask: LVCF_TEXT | LVCF_WIDTH | LVCF_FMT,
                    fmt: LVCFMT_LEFT,
                    cx: 100,
                    pszText: PWSTR(w!("Size").as_ptr() as *mut _),
                    ..Default::default()
                };
                SendMessageW(
                    self.list_view,
                    LVM_INSERTCOLUMNW,
                    WPARAM(col_index),
                    LPARAM(&column as *const _ as _),
                );
                col_index += 1;
            }

            if self.show_modified.load(Ordering::SeqCst) {
                let column = LVCOLUMNW {
                    mask: LVCF_TEXT | LVCF_WIDTH | LVCF_FMT,
                    fmt: LVCFMT_LEFT,
                    cx: 150,
                    pszText: PWSTR(w!("Modified").as_ptr() as *mut _),
                    ..Default::default()
                };
                SendMessageW(
                    self.list_view,
                    LVM_INSERTCOLUMNW,
                    WPARAM(col_index),
                    LPARAM(&column as *const _ as _),
                );
                col_index += 1;
            }

            if self.show_path.load(Ordering::SeqCst) {
                let column = LVCOLUMNW {
                    mask: LVCF_TEXT | LVCF_WIDTH | LVCF_FMT,
                    fmt: LVCFMT_LEFT,
                    cx: 400,
                    pszText: PWSTR(w!("Path").as_ptr() as *mut _),
                    ..Default::default()
                };
                SendMessageW(
                    self.list_view,
                    LVM_INSERTCOLUMNW,
                    WPARAM(col_index),
                    LPARAM(&column as *const _ as _),
                );
            }
        }

        Ok(())
    }

    pub fn load_directory(&mut self, path: &Path) {
        // Show loading message immediately and set window title
        unsafe {
            let title = format!("File List Generator - {}", path.display());
            SetWindowTextW(self.hwnd, &HSTRING::from(title)).ok();
            SetWindowTextW(self.status_bar, &HSTRING::from("Scanning directory... 0 files found")).ok();
        }
    }

    fn sort_files(&self) {
        let mut files = self.filtered_files.lock().unwrap();
        let sort_col = *self.sort_column.lock().unwrap();
        let ascending = self.sort_ascending.load(Ordering::SeqCst);
        
        if sort_col < 0 {
            return; // No sorting
        }
        
        // Map visible column index to actual column type
        let mut actual_col = 0;
        let mut mapped_col = 0;
        
        // Column 0 is always Name
        if sort_col == 0 {
            mapped_col = 0;
        } else {
            actual_col = 1;
            if self.show_type.load(Ordering::SeqCst) {
                if sort_col == actual_col {
                    mapped_col = 1; // Type
                }
                actual_col += 1;
            }
            if self.show_size.load(Ordering::SeqCst) {
                if sort_col == actual_col {
                    mapped_col = 2; // Size
                }
                actual_col += 1;
            }
            if self.show_modified.load(Ordering::SeqCst) {
                if sort_col == actual_col {
                    mapped_col = 3; // Modified
                }
                actual_col += 1;
            }
            if self.show_path.load(Ordering::SeqCst) {
                if sort_col == actual_col {
                    mapped_col = 4; // Path
                }
            }
        }
        
        files.sort_by(|a, b| {
            let result = match mapped_col {
                0 => a.name.to_lowercase().cmp(&b.name.to_lowercase()), // Name
                1 => { // Type/Extension
                    let ext_a = a.extension.as_deref().unwrap_or("");
                    let ext_b = b.extension.as_deref().unwrap_or("");
                    ext_a.cmp(ext_b)
                }
                2 => a.size.cmp(&b.size), // Size
                3 => { // Modified
                    match (a.modified, b.modified) {
                        (Some(ta), Some(tb)) => ta.cmp(&tb),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                }
                4 => { // Path
                    let path_a = a.path.parent().map(|p| p.to_string_lossy().to_lowercase()).unwrap_or_default();
                    let path_b = b.path.parent().map(|p| p.to_string_lossy().to_lowercase()).unwrap_or_default();
                    path_a.cmp(&path_b)
                }
                _ => std::cmp::Ordering::Equal,
            };
            
            if ascending {
                result
            } else {
                result.reverse()
            }
        });
    }
    
    fn refresh_list_view(&self) {
        unsafe {
            let count = self.filtered_files.lock().unwrap().len();
            SendMessageW(
                self.list_view,
                LVM_SETITEMCOUNT,
                WPARAM(count),
                LPARAM(LVSICF_NOSCROLL as _),
            );
            let _ = InvalidateRect(self.list_view, None, false);
        }
    }

    fn update_status_bar(&self) {
        let files = self.files.lock().unwrap();
        let filtered = self.filtered_files.lock().unwrap();
        let mut animation_frame = self.scan_animation_frame.lock().unwrap();
        let is_scanning = self.is_scanning.load(Ordering::SeqCst);
        let elapsed_ms = self.scan_elapsed_ms.load(Ordering::SeqCst);
        
        let status = if is_scanning {
            // Currently scanning - show live progress
            *animation_frame = (*animation_frame + 1) % 4;
            let spinner = match *animation_frame {
                0 => "⠋",
                1 => "⠙",
                2 => "⠹",
                _ => "⠸",
            };
            
            let file_count = files.len();
            if elapsed_ms > 0 && file_count > 0 {
                let elapsed_secs = elapsed_ms as f64 / 1000.0;
                let files_per_sec = (file_count as f64 / elapsed_secs) as usize;
                format!("{} Scanning... {} files found • {:.1}s • {} files/sec", 
                    spinner, file_count, elapsed_secs, files_per_sec)
            } else {
                format!("{} Scanning directory... {} files found", spinner, file_count)
            }
        } else if files.is_empty() {
            "Ready".to_string()
        } else if files.len() != filtered.len() {
            // Showing filtered results
            let base_status = if filtered.is_empty() {
                format!("❌ No matches found (0 of {} files)", files.len())
            } else {
                format!("🔍 Showing {} of {} files", filtered.len(), files.len())
            };
            
            // Add timing info if we have it
            if elapsed_ms > 0 {
                let elapsed_secs = elapsed_ms as f64 / 1000.0;
                let files_per_sec = (files.len() as f64 / elapsed_secs) as usize;
                format!("{} • Scan time: {}ms ({:.1}s) • {} files/sec", 
                    base_status, elapsed_ms, elapsed_secs, files_per_sec)
            } else {
                base_status
            }
        } else {
            // Show all files with timing statistics
            let total_size: u64 = files.iter().map(|f| f.size).sum();
            let size_str = Self::format_file_size(total_size);
            
            if elapsed_ms > 0 {
                let elapsed_secs = elapsed_ms as f64 / 1000.0;
                let files_per_sec = (files.len() as f64 / elapsed_secs) as usize;
                format!("✅ {} files found in {}ms ({:.1}s) • {} files/sec • Total size: {}", 
                    files.len(), elapsed_ms, elapsed_secs, files_per_sec, size_str)
            } else {
                format!("✅ {} files found • Total size: {}", files.len(), size_str)
            }
        };
        
        unsafe {
            let wide = HSTRING::from(status);
            SetWindowTextW(self.status_bar, &wide).ok();
        }
    }
    
    fn format_file_size(size: u64) -> String {
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

    fn setup_search_thread(&mut self) {
        let (sender, receiver) = unbounded::<String>();
        self.search_sender = Some(sender);
        
        let files = Arc::clone(&self.files);
        let filtered_files = Arc::clone(&self.filtered_files);
        let filter = Arc::clone(&self.filter);
        let is_searching = Arc::clone(&self.is_searching);
        let hwnd = self.hwnd.0 as isize;
        
        thread::spawn(move || {
            while let Ok(search_text) = receiver.recv() {
                is_searching.store(true, Ordering::SeqCst);
                
                // Update filter
                filter.lock().unwrap().set_search(&search_text);
                
                // Perform filtering
                let files_lock = files.lock().unwrap();
                let filter_lock = filter.lock().unwrap();
                
                let mut filtered = Vec::with_capacity(files_lock.len() / 2);
                for file in files_lock.iter() {
                    if filter_lock.matches(file) {
                        filtered.push(file.clone());
                    }
                }
                
                drop(files_lock);
                drop(filter_lock);
                
                // Update filtered files
                *filtered_files.lock().unwrap() = filtered;
                
                is_searching.store(false, Ordering::SeqCst);
                
                // Notify UI to update
                unsafe {
                    let hwnd = HWND(hwnd as *mut _);
                    PostMessageW(hwnd, WM_UPDATE_SEARCH, WPARAM(0), LPARAM(0)).ok();
                }
            }
        });
    }
    
    fn handle_search(&mut self) {
        unsafe {
            let mut buffer = [0u16; 512];
            let len = GetWindowTextW(self.search_box, &mut buffer) as usize;
            let search_text = String::from_utf16_lossy(&buffer[..len]);
            
            // Send search request to background thread
            if let Some(ref sender) = self.search_sender {
                let _ = sender.send(search_text.clone());
            }
            
            // Update status to show searching
            if !search_text.is_empty() {
                SetWindowTextW(self.status_bar, &HSTRING::from("Searching...")).ok();
            } else {
                // Clear search - refresh immediately
                self.update_status_bar();
            }
        }
    }

    fn copy_selected_path(&self) {
        unsafe {
            let selected = SendMessageW(
                self.list_view,
                LVM_GETNEXTITEM,
                WPARAM(-1i32 as usize),
                LPARAM(LVNI_SELECTED as _),
            );
            
            if selected.0 >= 0 {
                if let Some(file) = self.filtered_files.lock().unwrap().get(selected.0 as usize) {
                    let path_str = file.path.to_string_lossy();
                    self.copy_to_clipboard(&path_str);
                }
            }
        }
    }
    
    fn copy_to_clipboard(&self, text: &str) {
        unsafe {
            if OpenClipboard(self.hwnd).is_ok() {
                let _ = EmptyClipboard();
                
                let wide_text: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
                let mem_size = wide_text.len() * 2;
                
                if let Ok(h_mem) = GlobalAlloc(GMEM_MOVEABLE, mem_size) {
                    let ptr = GlobalLock(h_mem);
                    if !ptr.is_null() {
                        std::ptr::copy_nonoverlapping(wide_text.as_ptr(), ptr as *mut u16, wide_text.len());
                        let _ = GlobalUnlock(h_mem);
                        let _ = SetClipboardData(CF_UNICODETEXT, HANDLE(h_mem.0 as _));
                    }
                }
                let _ = CloseClipboard();
            }
        }
    }
    
    fn show_context_menu(&self) {
        unsafe {
            let selected = SendMessageW(
                self.list_view,
                LVM_GETNEXTITEM,
                WPARAM(-1i32 as usize),
                LPARAM(LVNI_SELECTED as _),
            );
            
            if selected.0 < 0 {
                return; // No selection
            }
            
            let menu = CreatePopupMenu().unwrap();
            
            AppendMenuW(menu, MF_STRING, ID_CTX_OPEN as usize, w!("Open\tEnter")).ok();
            AppendMenuW(menu, MF_STRING, ID_CTX_OPEN_FOLDER as usize, w!("Open Containing Folder\tCtrl+O")).ok();
            AppendMenuW(menu, MF_SEPARATOR, 0, w!("")).ok();
            AppendMenuW(menu, MF_STRING, ID_CTX_COPY_PATH as usize, w!("Copy Full Path")).ok();
            AppendMenuW(menu, MF_STRING, ID_CTX_COPY_NAME as usize, w!("Copy File Name")).ok();
            
            let mut cursor_pos = POINT::default();
            GetCursorPos(&mut cursor_pos).ok();
            
            let cmd = TrackPopupMenuEx(
                menu,
                (TPM_RETURNCMD | TPM_RIGHTBUTTON).0,
                cursor_pos.x,
                cursor_pos.y,
                self.hwnd,
                None,
            );
            
            DestroyMenu(menu).ok();
            
            match cmd.0 as i32 {
                ID_CTX_OPEN => self.handle_list_item_activate(),
                ID_CTX_OPEN_FOLDER => self.open_containing_folder(),
                ID_CTX_COPY_PATH => self.copy_selected_path(),
                ID_CTX_COPY_NAME => self.copy_selected_name(),
                _ => {}
            }
        }
    }
    
    fn copy_list_to_clipboard(&self) {
        let files = self.filtered_files.lock().unwrap();
        if files.is_empty() {
            return;
        }

        let mut result = String::new();
        
        // Add header
        result.push_str("File List\n");
        result.push_str("=========\n\n");
        
        // Add column headers
        result.push_str("Name\t");
        if self.show_type.load(Ordering::SeqCst) {
            result.push_str("Type\t");
        }
        if self.show_size.load(Ordering::SeqCst) {
            result.push_str("Size\t");
        }
        if self.show_modified.load(Ordering::SeqCst) {
            result.push_str("Modified\t");
        }
        if self.show_path.load(Ordering::SeqCst) {
            result.push_str("Path");
        }
        result.push_str("\n");
        
        // Add separator
        for _ in 0..80 {
            result.push('-');
        }
        result.push_str("\n");
        
        // Add file data
        for file in files.iter() {
            result.push_str(&format!("{} {}\t", file.get_icon(), file.name));
            
            if self.show_type.load(Ordering::SeqCst) {
                let type_str = file.extension.clone()
                    .unwrap_or_else(|| "File".to_string())
                    .to_uppercase();
                result.push_str(&format!("{}\t", type_str));
            }
            
            if self.show_size.load(Ordering::SeqCst) {
                result.push_str(&format!("{}\t", file.size_formatted()));
            }
            
            if self.show_modified.load(Ordering::SeqCst) {
                result.push_str(&format!("{}\t", file.modified_formatted()));
            }
            
            if self.show_path.load(Ordering::SeqCst) {
                let path_str = file.path.parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                result.push_str(&path_str);
            }
            
            result.push_str("\n");
        }
        
        // Add summary
        result.push_str(&format!("\nTotal: {} files\n", files.len()));
        
        drop(files);
        self.copy_to_clipboard(&result);
    }
    
    fn copy_selected_name(&self) {
        unsafe {
            let selected = SendMessageW(
                self.list_view,
                LVM_GETNEXTITEM,
                WPARAM(-1i32 as usize),
                LPARAM(LVNI_SELECTED as _),
            );
            
            if selected.0 >= 0 {
                if let Some(file) = self.filtered_files.lock().unwrap().get(selected.0 as usize) {
                    self.copy_to_clipboard(&file.name);
                }
            }
        }
    }
    
    fn open_containing_folder(&self) {
        unsafe {
            let selected = SendMessageW(
                self.list_view,
                LVM_GETNEXTITEM,
                WPARAM(-1i32 as usize),
                LPARAM(LVNI_SELECTED as _),
            );
            
            if selected.0 >= 0 {
                let files = self.filtered_files.lock().unwrap();
                if let Some(file) = files.get(selected.0 as usize) {
                    let folder_path = if file.is_dir {
                        file.path.clone()
                    } else {
                        file.path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| file.path.clone())
                    };
                    drop(files); // Release lock before spawning thread
                    
                    thread::spawn(move || {
                        let path_str = folder_path.to_string_lossy();
                        let path_hstring = HSTRING::from(path_str.as_ref());
                        unsafe {
                            ShellExecuteW(
                                HWND::default(),
                                w!("explore"),
                                &path_hstring,
                                &HSTRING::new(),
                                &HSTRING::new(),
                                SW_SHOW,
                            );
                        }
                    });
                }
            }
        }
    }

    fn handle_list_item_activate(&self) {
        unsafe {
            let selected = SendMessageW(
                self.list_view,
                LVM_GETNEXTITEM,
                WPARAM(-1i32 as usize),
                LPARAM(LVNI_SELECTED as _),
            );
            
            if selected.0 >= 0 {
                let files = self.filtered_files.lock().unwrap();
                if let Some(file) = files.get(selected.0 as usize) {
                    let path = file.path.clone();
                    let is_dir = file.is_dir;
                    
                    // Drop the lock before spawning thread
                    drop(files);
                    
                    // Open file/folder in a separate thread to prevent UI freeze
                    thread::spawn(move || {
                        let path_str = path.to_string_lossy();
                        let path_hstring = HSTRING::from(path_str.as_ref());
                        
                        unsafe {
                            if is_dir {
                                ShellExecuteW(
                                    HWND::default(),
                                    w!("explore"),
                                    &path_hstring,
                                    &HSTRING::new(),
                                    &HSTRING::new(),
                                    SW_SHOW,
                                );
                            } else {
                                ShellExecuteW(
                                    HWND::default(),
                                    w!("open"),
                                    &path_hstring,
                                    &HSTRING::new(),
                                    &HSTRING::new(),
                                    SW_SHOW,
                                );
                            }
                        }
                    });
                }
            }
        }
    }

    fn select_first_item(&self) {
        unsafe {
            let count = self.filtered_files.lock().unwrap().len();
            if count > 0 {
                // Clear all selections first
                SendMessageW(
                    self.list_view,
                    LVM_SETITEMSTATE,
                    WPARAM(-1i32 as usize),
                    LPARAM(&LVITEMW {
                        mask: LVIF_STATE,
                        stateMask: LIST_VIEW_ITEM_STATE_FLAGS(LVIS_SELECTED.0 | LVIS_FOCUSED.0),
                        state: LIST_VIEW_ITEM_STATE_FLAGS(0),
                        ..Default::default()
                    } as *const _ as _),
                );
                
                // Select and focus first item
                SendMessageW(
                    self.list_view,
                    LVM_SETITEMSTATE,
                    WPARAM(0),
                    LPARAM(&LVITEMW {
                        mask: LVIF_STATE,
                        stateMask: LIST_VIEW_ITEM_STATE_FLAGS(LVIS_SELECTED.0 | LVIS_FOCUSED.0),
                        state: LIST_VIEW_ITEM_STATE_FLAGS(LVIS_SELECTED.0 | LVIS_FOCUSED.0),
                        ..Default::default()
                    } as *const _ as _),
                );
                
                // Ensure it's visible
                SendMessageW(
                    self.list_view,
                    LVM_ENSUREVISIBLE,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
        }
    }

    fn handle_dropped_files(&mut self, hdrop: isize) {
        unsafe {
            // Get the number of dropped files
            let count = DragQueryFileW(HDROP(hdrop as *mut _), 0xFFFFFFFF, None);
            
            if count > 0 {
                // Get the first dropped file path
                let mut buffer = vec![0u16; 260];
                let len = DragQueryFileW(
                    HDROP(hdrop as *mut _), 
                    0, 
                    Some(&mut buffer)
                );
                
                if len > 0 {
                    let path_str = String::from_utf16_lossy(&buffer[..len as usize]);
                    let mut path = std::path::PathBuf::from(path_str);
                    
                    // Check if it's a shortcut (.lnk file)
                    if let Some(ext) = path.extension() {
                        if ext.to_ascii_lowercase() == "lnk" {
                            // Try to resolve the shortcut
                            if let Some(target) = self.resolve_shortcut(&path) {
                                path = target;
                            }
                        }
                    }
                    
                    // Check if it's a directory (or resolved to a directory)
                    if path.is_dir() {
                        // Start scanning the dropped directory
                        self.start_new_scan(path);
                    }
                }
            }
            
            DragFinish(HDROP(hdrop as *mut _));
        }
    }
    
    fn resolve_shortcut(&self, lnk_path: &std::path::Path) -> Option<std::path::PathBuf> {
        unsafe {
            use windows::Win32::System::Com::*;
            use windows::Win32::UI::Shell::*;
            
            // Initialize COM
            let _ = CoInitialize(None);
            
            // Create IShellLink instance using class factory
            // CLSID_ShellLink = {00021401-0000-0000-C000-000000000046}
            let clsid = windows::core::GUID::from_u128(0x00021401_0000_0000_C000_000000000046);
            let shell_link: IShellLinkW = match CoCreateInstance(&clsid, None, CLSCTX_INPROC_SERVER) {
                Ok(sl) => sl,
                Err(_) => {
                    let _ = CoUninitialize();
                    return None;
                }
            };
            
            // Get IPersistFile interface
            let persist_file: IPersistFile = match shell_link.cast() {
                Ok(pf) => pf,
                Err(_) => {
                    let _ = CoUninitialize();
                    return None;
                }
            };
            
            // Load the shortcut file
            let wide_path = HSTRING::from(lnk_path.to_string_lossy().as_ref());
            if persist_file.Load(&wide_path, STGM_READ).is_err() {
                let _ = CoUninitialize();
                return None;
            }
            
            // Get the target path
            let mut target_buffer = vec![0u16; 260];
            
            if shell_link.GetPath(
                &mut target_buffer,
                std::ptr::null_mut(),
                0
            ).is_ok() {
                // Find the actual string length
                let len = target_buffer.iter().position(|&c| c == 0).unwrap_or(target_buffer.len());
                if len > 0 {
                    let target_str = String::from_utf16_lossy(&target_buffer[..len]);
                    let _ = CoUninitialize();
                    return Some(std::path::PathBuf::from(target_str));
                }
            }
            
            let _ = CoUninitialize();
            None
        }
    }
    
    fn paste_from_clipboard(&mut self) {
        unsafe {
            if OpenClipboard(self.hwnd).is_ok() {
                let h_data = GetClipboardData(CF_UNICODETEXT);
                if let Ok(handle) = h_data {
                    if !handle.is_invalid() {
                        let ptr = GlobalLock(HGLOBAL(handle.0 as *mut _));
                        if !ptr.is_null() {
                            let wide_ptr = ptr as *const u16;
                            let mut len = 0;
                            while *wide_ptr.offset(len) != 0 {
                                len += 1;
                            }
                            let wide_slice = std::slice::from_raw_parts(wide_ptr, len as usize);
                            let path_str = String::from_utf16_lossy(wide_slice);
                            
                            let _ = GlobalUnlock(HGLOBAL(handle.0 as *mut _));
                            
                            // Try to parse as path
                            let path = std::path::PathBuf::from(path_str.trim());
                            if path.is_dir() {
                                self.start_new_scan(path);
                            }
                        }
                    }
                }
                let _ = CloseClipboard();
            }
        }
    }
    
    fn show_help_dialog(&self) {
        unsafe {
            let help_text = "File List Generator - Keyboard Shortcuts & Tips\n\n\
                            KEYBOARD SHORTCUTS:\n\
                            • Ctrl+F: Focus search box\n\
                            • Ctrl+O: Open containing folder\n\
                            • Enter: Open selected file/folder\n\
                            • Escape: Clear search\n\
                            • Tab: Switch between search and list\n\
                            • Right-click: Show context menu\n\n\
                            SEARCH EXAMPLES:\n\
                            • filename - Search by name\n\
                            • .txt - Search by extension\n\
                            • >10mb - Files larger than 10MB\n\
                            • <1kb - Files smaller than 1KB\n\n\
                            FEATURES:\n\
                            • Double-click to open files/folders\n\
                            • Drag & drop folders to scan them\n\
                            • Use checkboxes to show/hide columns\n\
                            • Click 'Copy List' to export all data\n\n\
                            Developed by David Landry";
            
            MessageBoxW(
                self.hwnd,
                &HSTRING::from(help_text),
                w!("Help - File List Generator"),
                MB_OK | MB_ICONINFORMATION,
            );
        }
    }
    
    fn start_new_scan(&mut self, path: std::path::PathBuf) {
        // Clear existing data
        self.files.lock().unwrap().clear();
        self.filtered_files.lock().unwrap().clear();
        
        // Update UI
        self.load_directory(&path);
        
        // Start new scan
        let (sender, receiver) = crossbeam_channel::unbounded();
        self.set_update_receiver(receiver);
        
        let scanner_path = path.clone();
        std::thread::spawn(move || {
            let scanner = crate::scanner::Scanner::with_sender(sender);
            scanner.scan_directory(&scanner_path);
        });
    }

    fn handle_custom_draw(&self, lparam: isize) -> LRESULT {
        unsafe {
            let lpnmlvcd = lparam as *mut NMLVCUSTOMDRAW;
            let stage = (*lpnmlvcd).nmcd.dwDrawStage;
            
            match stage {
                CDDS_PREPAINT => {
                    // Request item notifications
                    LRESULT(CDRF_NOTIFYITEMDRAW as isize)
                }
                CDDS_ITEMPREPAINT => {
                    let item_index = (*lpnmlvcd).nmcd.dwItemSpec;
                    
                    // Alternating row colors
                    if item_index % 2 == 0 {
                        (*lpnmlvcd).clrTextBk = 0xF8F8F8; // Light gray background for even rows
                    } else {
                        (*lpnmlvcd).clrTextBk = 0xFFFFFF; // White for odd rows
                    }
                    
                    // Use standard black text for all files
                    (*lpnmlvcd).clrText = 0x000000;
                    
                    LRESULT(CDRF_NEWFONT as isize)
                }
                _ => LRESULT(0)
            }
        }
    }

    fn handle_display_info(&self, info: *mut NMLVDISPINFOW) {
        unsafe {
            let info = &mut *info;
            let index = info.item.iItem as usize;
            
            if let Some(file) = self.filtered_files.lock().unwrap().get(index) {
                if info.item.mask & LVIF_TEXT != LIST_VIEW_ITEM_FLAGS(0) {
                    let mut col_index = 0;
                    let mut text = String::new();
                    
                    // Name column is always first
                    if info.item.iSubItem == col_index {
                        text = format!("{} {}", file.get_icon(), file.name);
                    }
                    col_index += 1;
                    
                    // Type column
                    if self.show_type.load(Ordering::SeqCst) {
                        if info.item.iSubItem == col_index {
                            text = file.extension.clone()
                                .unwrap_or_else(|| "File".to_string())
                                .to_uppercase();
                        }
                        col_index += 1;
                    }
                    
                    // Size column
                    if self.show_size.load(Ordering::SeqCst) {
                        if info.item.iSubItem == col_index {
                            text = file.size_formatted();
                        }
                        col_index += 1;
                    }
                    
                    // Modified column
                    if self.show_modified.load(Ordering::SeqCst) {
                        if info.item.iSubItem == col_index {
                            text = file.modified_formatted();
                        }
                        col_index += 1;
                    }
                    
                    // Path column
                    if self.show_path.load(Ordering::SeqCst) {
                        if info.item.iSubItem == col_index {
                            text = file.path.parent()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_default();
                        }
                    }
                    
                    let wide = text.encode_utf16().chain(std::iter::once(0))
                        .take(info.item.cchTextMax as usize)
                        .collect::<Vec<_>>();
                    
                    std::ptr::copy_nonoverlapping(
                        wide.as_ptr(),
                        info.item.pszText.0,
                        wide.len().min(info.item.cchTextMax as usize)
                    );
                }
            }
        }
    }

    pub fn run_message_loop(&mut self) -> Result<()> {
        unsafe {
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).into() {
                // Handle special keys globally
                if msg.message == WM_KEYDOWN {
                    let key = msg.wParam.0;
                    let focused = GetFocus();
                    
                    match key {
                        // Escape - Clear search from anywhere
                        0x1B => {
                            SetWindowTextW(self.search_box, w!("")).ok();
                            self.handle_search();
                            let _ = SetFocus(self.list_view);
                            self.select_first_item();
                            continue;
                        }
                        // Enter in search box - Focus list and select first item
                        0x0D if focused == self.search_box => {
                            let _ = SetFocus(self.list_view);
                            self.select_first_item();
                            continue;
                        }
                        // Down arrow in search box - Move to list
                        0x28 if focused == self.search_box => { // VK_DOWN
                            let _ = SetFocus(self.list_view);
                            self.select_first_item();
                            continue;
                        }
                        // Tab - Navigate between search and list
                        0x09 => { // VK_TAB
                            if focused == self.search_box {
                                let _ = SetFocus(self.list_view);
                                self.select_first_item();
                            } else {
                                let _ = SetFocus(self.search_box);
                                SendMessageW(self.search_box, EM_SETSEL, WPARAM(0), LPARAM(-1));
                            }
                            continue;
                        }
                        _ => {}
                    }
                }
                
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        Ok(())
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_CREATE {
        let create_struct = lparam.0 as *const CREATESTRUCTW;
        let window = (*create_struct).lpCreateParams as *mut FileListWindow;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, window as _);
        return LRESULT(0);
    }

    let window_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut FileListWindow;
    if window_ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }

    let window = &mut *window_ptr;

    match msg {
        WM_HOTKEY => {
            match wparam.0 {
                1 => { // Ctrl+F
                    let _ = SetFocus(window.search_box);
                    SendMessageW(window.search_box, EM_SETSEL, WPARAM(0), LPARAM(-1));
                }
                3 => { // Ctrl+O
                    window.open_containing_folder();
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_DROPFILES => {
            window.handle_dropped_files(wparam.0 as isize);
            LRESULT(0)
        }
        WM_SIZE => {
            let width = lparam.0 & 0xFFFF;
            let height = (lparam.0 >> 16) & 0xFFFF;
            
            // Resize search box
            SetWindowPos(
                window.search_box,
                None,
                10, 10, 300, 25,
                SWP_NOZORDER,
            ).ok();
            
            // Keep button positions
            SetWindowPos(
                window.copy_button,
                None,
                320, 10, 80, 25,
                SWP_NOZORDER,
            ).ok();
            
            SetWindowPos(
                window.open_folder_button,
                None,
                410, 10, 90, 25,
                SWP_NOZORDER,
            ).ok();
            
            // Keep checkboxes and help button in position
            SetWindowPos(window.chk_type, None, 510, 10, 60, 25, SWP_NOZORDER).ok();
            SetWindowPos(window.chk_size, None, 580, 10, 60, 25, SWP_NOZORDER).ok();
            SetWindowPos(window.chk_modified, None, 650, 10, 80, 25, SWP_NOZORDER).ok();
            SetWindowPos(window.chk_path, None, 740, 10, 60, 25, SWP_NOZORDER).ok();
            SetWindowPos(window.help_button, None, 810, 10, 30, 25, SWP_NOZORDER).ok();
            
            // Resize list view (leave room for signature and status bar)
            SetWindowPos(
                window.list_view,
                None,
                10, 45, (width - 20) as i32, (height - 110) as i32,
                SWP_NOZORDER,
            ).ok();
            
            // Position signature above status bar
            SetWindowPos(
                window.signature_label,
                None,
                10, (height - 60) as i32, (width - 20) as i32, 20,
                SWP_NOZORDER,
            ).ok();
            
            SendMessageW(window.status_bar, WM_SIZE, wparam, lparam);
            
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as i32;
            let notification = (wparam.0 >> 16) as u32;
            
            match id {
                ID_SEARCHBOX if notification == EN_CHANGE as u32 => {
                    window.handle_search();
                }
                ID_COPY_BUTTON => {
                    window.copy_list_to_clipboard();
                }
                ID_OPEN_FOLDER_BUTTON => {
                    window.open_containing_folder();
                }
                ID_HELP_BUTTON => {
                    window.show_help_dialog();
                }
                ID_CHK_TYPE | ID_CHK_SIZE | ID_CHK_MODIFIED | ID_CHK_PATH => {
                    // Update visibility flags
                    let state = SendMessageW(HWND(lparam.0 as *mut _), BM_GETCHECK, WPARAM(0), LPARAM(0));
                    let checked = state.0 == BST_CHECKED as isize;
                    
                    match id {
                        ID_CHK_TYPE => window.show_type.store(checked, Ordering::SeqCst),
                        ID_CHK_SIZE => window.show_size.store(checked, Ordering::SeqCst),
                        ID_CHK_MODIFIED => window.show_modified.store(checked, Ordering::SeqCst),
                        ID_CHK_PATH => window.show_path.store(checked, Ordering::SeqCst),
                        _ => {}
                    }
                    
                    // Recreate columns and refresh list
                    window.setup_list_columns().ok();
                    window.refresh_list_view();
                }
                _ => {}
            }
            
            LRESULT(0)
        }
        WM_NOTIFY => {
            let nmhdr = lparam.0 as *const NMHDR;
            let code = (*nmhdr).code;
            
            match code {
                LVN_GETDISPINFOW => {
                    window.handle_display_info(lparam.0 as *mut NMLVDISPINFOW);
                }
                NM_CUSTOMDRAW => {
                    if (*nmhdr).idFrom == ID_LISTVIEW as usize {
                        return window.handle_custom_draw(lparam.0);
                    }
                }
                NM_DBLCLK => {
                    if (*nmhdr).idFrom == ID_LISTVIEW as usize {
                        window.handle_list_item_activate();
                    }
                }
                NM_RCLICK => {
                    if (*nmhdr).idFrom == ID_LISTVIEW as usize {
                        window.show_context_menu();
                    }
                }
                LVN_KEYDOWN => {
                    let key_info = lparam.0 as *const NMLVKEYDOWN;
                    if (*key_info).wVKey == VK_RETURN.0 {
                        window.handle_list_item_activate();
                    }
                }
                LVN_COLUMNCLICK => {
                    // Get the column that was clicked
                    let nm_listview = lparam.0 as *const NMLISTVIEW;
                    let clicked_col = (*nm_listview).iSubItem;
                    
                    // Update sort state
                    let mut current_sort_col = window.sort_column.lock().unwrap();
                    if *current_sort_col == clicked_col {
                        // Toggle sort direction
                        let current_ascending = window.sort_ascending.load(Ordering::SeqCst);
                        window.sort_ascending.store(!current_ascending, Ordering::SeqCst);
                    } else {
                        // New column, default to ascending
                        *current_sort_col = clicked_col;
                        window.sort_ascending.store(true, Ordering::SeqCst);
                    }
                    drop(current_sort_col);
                    
                    // Sort and refresh
                    window.sort_files();
                    window.refresh_list_view();
                }
                _ => {}
            }
            
            LRESULT(0)
        }
        WM_UPDATE_LIST => {
            window.refresh_list_view();
            window.update_status_bar();
            LRESULT(0)
        }
        WM_UPDATE_SEARCH => {
            window.refresh_list_view();
            window.update_status_bar();
            // Auto-select first item if search box has focus
            if GetFocus() == window.search_box {
                window.select_first_item();
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            // Unregister hotkeys
            let _ = UnregisterHotKey(hwnd, 1);
            let _ = UnregisterHotKey(hwnd, 3);
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
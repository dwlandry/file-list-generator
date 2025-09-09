use crate::scanner::FileInfo;

pub struct Filter {
    search_text: String,
    search_lower: String,
}

impl Filter {
    pub fn new() -> Self {
        Filter {
            search_text: String::new(),
            search_lower: String::new(),
        }
    }

    pub fn set_search(&mut self, text: &str) {
        self.search_text = text.to_string();
        self.search_lower = text.to_lowercase();
    }

    pub fn matches(&self, file: &FileInfo) -> bool {
        if self.search_text.is_empty() {
            return true;
        }

        let name_lower = file.name.to_lowercase();
        let path_lower = file.path.to_string_lossy().to_lowercase();
        
        if name_lower.contains(&self.search_lower) {
            return true;
        }

        if path_lower.contains(&self.search_lower) {
            return true;
        }

        if let Some(ref ext) = file.extension {
            if ext.contains(&self.search_lower) {
                return true;
            }
        }

        if self.search_text.starts_with('>') {
            let size_query = self.search_text[1..].trim();
            if let Ok(min_size) = parse_size(size_query) {
                return file.size > min_size;
            }
        }

        if self.search_text.starts_with('<') {
            let size_query = self.search_text[1..].trim();
            if let Ok(max_size) = parse_size(size_query) {
                return file.size < max_size;
            }
        }


        false
    }
}

fn parse_size(s: &str) -> Result<u64, ()> {
    let s = s.trim().to_lowercase();
    
    if s.is_empty() {
        return Err(());
    }

    let (number_part, unit_part) = if s.ends_with("kb") {
        (&s[..s.len()-2], "kb")
    } else if s.ends_with("mb") {
        (&s[..s.len()-2], "mb")
    } else if s.ends_with("gb") {
        (&s[..s.len()-2], "gb")
    } else if s.ends_with("k") {
        (&s[..s.len()-1], "k")
    } else if s.ends_with("m") {
        (&s[..s.len()-1], "m")
    } else if s.ends_with("g") {
        (&s[..s.len()-1], "g")
    } else {
        (s.as_str(), "")
    };

    let number: f64 = number_part.trim().parse().map_err(|_| ())?;

    let multiplier = match unit_part {
        "k" | "kb" => 1024.0,
        "m" | "mb" => 1024.0 * 1024.0,
        "g" | "gb" => 1024.0 * 1024.0 * 1024.0,
        _ => 1.0,
    };

    Ok((number * multiplier) as u64)
}
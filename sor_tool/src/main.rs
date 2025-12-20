//! SOR file tool - decrypt and re-encrypt .sor archives
//! 
//! Usage:
//!   sor_tool extract <input.sor> <password> <output_dir>
//!   sor_tool create <input_dir> <password> <output.sor>
//!   sor_tool rekey <input.sor> <old_password> <new_password> <output.sor>
//!   sor_tool list <input.sor>

use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        return;
    }
    
    match args[1].as_str() {
        "extract" => {
            if args.len() != 5 {
                println!("Usage: sor_tool extract <input.sor> <password> <output_dir>");
                return;
            }
            extract(&args[2], &args[3], &args[4]);
        }
        "create" => {
            if args.len() != 5 {
                println!("Usage: sor_tool create <input_dir> <password> <output.sor>");
                return;
            }
            create(&args[2], &args[3], &args[4]);
        }
        "rekey" => {
            if args.len() != 6 {
                println!("Usage: sor_tool rekey <input.sor> <old_password> <new_password> <output.sor>");
                return;
            }
            rekey(&args[2], &args[3], &args[4], &args[5]);
        }
        "list" => {
            if args.len() != 3 {
                println!("Usage: sor_tool list <input.sor>");
                return;
            }
            list(&args[2]);
        }
        _ => {
            print_usage();
        }
    }
}

fn print_usage() {
    println!("SOR file tool - decrypt and re-encrypt .sor archives");
    println!();
    println!("Usage:");
    println!("  sor_tool extract <input.sor> <password> <output_dir>");
    println!("  sor_tool create <input_dir> <password> <output.sor>");
    println!("  sor_tool rekey <input.sor> <old_password> <new_password> <output.sor>");
    println!("  sor_tool list <input.sor>");
    println!();
    println!("Known passwords:");
    println!("  acs.sor (old):    fergehtjw43435667w46egt367u46r");
    println!("  acs.sor (new):    ewtrhj654736z2g5q6bzhn6u");
    println!("  backgrounds.sor:  adfadsfbgh4534ewfgr");
}

/// XOR data with a repeating key
fn xor_crypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &b)| b ^ key[i % key.len()])
        .collect()
}

/// Read big-endian u16
fn read_be_u16(data: &[u8], offset: usize) -> u16 {
    ((data[offset] as u16) << 8) | (data[offset + 1] as u16)
}

/// Read big-endian u32
fn read_be_u32(data: &[u8], offset: usize) -> u32 {
    ((data[offset] as u32) << 24)
        | ((data[offset + 1] as u32) << 16)
        | ((data[offset + 2] as u32) << 8)
        | (data[offset + 3] as u32)
}

/// Write big-endian u16
fn write_be_u16(value: u16) -> [u8; 2] {
    [(value >> 8) as u8, value as u8]
}

/// Write big-endian u32
fn write_be_u32(value: u32) -> [u8; 4] {
    [
        (value >> 24) as u8,
        (value >> 16) as u8,
        (value >> 8) as u8,
        value as u8,
    ]
}

/// Parsed SOR file entry
struct FileEntry {
    filename: String,
    data: Vec<u8>, // Decrypted data
}

/// Parse SOR file and return entries (with decrypted data)
fn parse_sor(data: &[u8], key: &[u8]) -> Result<Vec<FileEntry>, String> {
    if data.len() < 6 {
        return Err("File too small".to_string());
    }
    
    // Read file count (big-endian u16 at offset 0)
    let file_count = read_be_u16(data, 0) as usize;
    
    if file_count == 0 {
        return Err("No files in archive".to_string());
    }
    
    // The offset table has (count-1) entries - the last file extends to end of archive
    let num_offsets = file_count - 1;
    let header_size = 2 + num_offsets * 4;
    if data.len() < header_size {
        return Err("File too small for header".to_string());
    }
    
    // Build cumulative offsets: [0, offset1, offset2, ..., end_of_data]
    let mut cum_offsets = vec![0u32];
    for i in 0..num_offsets {
        let offset = read_be_u32(data, 2 + i * 4);
        cum_offsets.push(offset);
    }
    // Last file extends to end of data section
    let data_section_size = (data.len() - header_size) as u32;
    cum_offsets.push(data_section_size);
    
    // Parse file entries
    let mut entries = Vec::with_capacity(file_count);
    
    for i in 0..file_count {
        let entry_start = header_size + cum_offsets[i] as usize;
        let entry_end = header_size + cum_offsets[i + 1] as usize;
        
        if entry_end > data.len() {
            return Err(format!("File {} extends past end of archive", i));
        }
        
        let entry_data = &data[entry_start..entry_end];
        
        if entry_data.len() < 2 {
            return Err(format!("File {} entry too small", i));
        }
        
        // Read filename length (big-endian u16)
        let fname_len = read_be_u16(entry_data, 0) as usize;
        
        if entry_data.len() < 2 + fname_len {
            return Err(format!("File {} has invalid filename length", i));
        }
        
        let filename = String::from_utf8_lossy(&entry_data[2..2 + fname_len]).to_string();
        let encrypted_data = &entry_data[2 + fname_len..];
        
        // Decrypt data
        let decrypted_data = xor_crypt(encrypted_data, key);
        
        entries.push(FileEntry {
            filename,
            data: decrypted_data,
        });
    }
    
    Ok(entries)
}

/// Build SOR file from entries
fn build_sor(entries: &[FileEntry], key: &[u8]) -> Vec<u8> {
    let file_count = entries.len();
    
    // Build encrypted entries first to calculate offsets
    let mut entry_data: Vec<Vec<u8>> = Vec::with_capacity(file_count);
    let mut cum_offset = 0u32;
    let mut cum_offsets = Vec::with_capacity(file_count);
    
    for entry in entries {
        let encrypted = xor_crypt(&entry.data, key);
        let fname_bytes = entry.filename.as_bytes();
        
        let mut data = Vec::new();
        data.extend_from_slice(&write_be_u16(fname_bytes.len() as u16));
        data.extend_from_slice(fname_bytes);
        data.extend_from_slice(&encrypted);
        
        cum_offset += data.len() as u32;
        cum_offsets.push(cum_offset);
        entry_data.push(data);
    }
    
    // Build output
    let mut output = Vec::new();
    
    // File count (big-endian u16)
    output.extend_from_slice(&write_be_u16(file_count as u16));
    
    // Cumulative offsets (big-endian u32) - only first (count-1) offsets
    // The last file extends to end of archive, so we don't store its end offset
    for offset in cum_offsets.iter().take(file_count - 1) {
        output.extend_from_slice(&write_be_u32(*offset));
    }
    
    // File entries
    for data in &entry_data {
        output.extend_from_slice(data);
    }
    
    output
}

fn list(input_path: &str) {
    let data = match fs::read(input_path) {
        Ok(d) => d,
        Err(e) => {
            println!("Error reading file: {}", e);
            return;
        }
    };
    
    if data.len() < 6 {
        println!("File too small");
        return;
    }
    
    let file_count = read_be_u16(&data, 0) as usize;
    let num_offsets = file_count - 1;
    let header_size = 2 + num_offsets * 4;
    
    println!("Archive: {}", input_path);
    println!("File count: {}", file_count);
    println!("Header size: {} (0x{:x})", header_size, header_size);
    println!();
    
    // Read cumulative offsets
    let mut cum_offsets = vec![0u32];
    for i in 0..num_offsets {
        let offset = read_be_u32(&data, 2 + i * 4);
        cum_offsets.push(offset);
    }
    // Last file extends to end
    let data_section_size = (data.len() - header_size) as u32;
    cum_offsets.push(data_section_size);
    
    // List files
    for i in 0..file_count {
        let entry_start = header_size + cum_offsets[i] as usize;
        let entry_end = header_size + cum_offsets[i + 1] as usize;
        let entry_size = entry_end - entry_start;
        
        if entry_end > data.len() {
            println!("[{:3}] Error: extends past end of file", i);
            break;
        }
        
        let entry_data = &data[entry_start..entry_end];
        
        if entry_data.len() < 2 {
            println!("[{:3}] Error: entry too small", i);
            continue;
        }
        
        let fname_len = read_be_u16(entry_data, 0) as usize;
        
        if entry_data.len() < 2 + fname_len {
            println!("[{:3}] Error: invalid filename length", i);
            continue;
        }
        
        let filename = String::from_utf8_lossy(&entry_data[2..2 + fname_len]);
        let data_size = entry_size - 2 - fname_len;
        
        println!("[{:3}] {} ({} bytes)", i, filename, data_size);
    }
}

fn extract(input_path: &str, password: &str, output_dir: &str) {
    let data = match fs::read(input_path) {
        Ok(d) => d,
        Err(e) => {
            println!("Error reading file: {}", e);
            return;
        }
    };
    
    let key = password.as_bytes();
    
    let entries = match parse_sor(&data, key) {
        Ok(e) => e,
        Err(e) => {
            println!("Error parsing archive: {}", e);
            return;
        }
    };
    
    // Create output directory
    if let Err(e) = fs::create_dir_all(output_dir) {
        println!("Error creating output directory: {}", e);
        return;
    }
    
    println!("Extracting {} files to {}", entries.len(), output_dir);
    
    let mut extracted = 0;
    for (i, entry) in entries.iter().enumerate() {
        let output_path = Path::new(output_dir).join(&entry.filename);
        
        match File::create(&output_path) {
            Ok(mut f) => {
                if let Err(e) = f.write_all(&entry.data) {
                    println!("[{:3}] Error writing {}: {}", i, entry.filename, e);
                } else {
                    extracted += 1;
                }
            }
            Err(e) => {
                println!("[{:3}] Error creating {}: {}", i, entry.filename, e);
            }
        }
    }
    
    println!("Extracted {} files", extracted);
}

fn create(input_dir: &str, password: &str, output_path: &str) {
    let key = password.as_bytes();
    
    // Read all files from input directory
    let mut entries: Vec<FileEntry> = Vec::new();
    
    let dir_entries = match fs::read_dir(input_dir) {
        Ok(e) => e,
        Err(e) => {
            println!("Error reading directory: {}", e);
            return;
        }
    };
    
    for dir_entry in dir_entries {
        let dir_entry = match dir_entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        
        let path = dir_entry.path();
        if !path.is_file() {
            continue;
        }
        
        let filename = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };
        
        let data = match fs::read(&path) {
            Ok(d) => d,
            Err(e) => {
                println!("Error reading {}: {}", filename, e);
                continue;
            }
        };
        
        entries.push(FileEntry { filename, data });
    }
    
    // Sort files by name for consistent ordering
    entries.sort_by(|a, b| a.filename.cmp(&b.filename));
    
    if entries.len() > 65535 {
        println!("Error: too many files (max 65535, got {})", entries.len());
        return;
    }
    
    println!("Creating archive with {} files", entries.len());
    
    let output = build_sor(&entries, key);
    
    // Write output
    match File::create(output_path) {
        Ok(mut f) => {
            if let Err(e) = f.write_all(&output) {
                println!("Error writing output: {}", e);
            } else {
                println!("Created {} ({} bytes)", output_path, output.len());
            }
        }
        Err(e) => {
            println!("Error creating output file: {}", e);
        }
    }
}

fn rekey(input_path: &str, old_password: &str, new_password: &str, output_path: &str) {
    let data = match fs::read(input_path) {
        Ok(d) => d,
        Err(e) => {
            println!("Error reading file: {}", e);
            return;
        }
    };
    
    let old_key = old_password.as_bytes();
    let new_key = new_password.as_bytes();
    
    // Parse with old key (decrypts data)
    let entries = match parse_sor(&data, old_key) {
        Ok(e) => e,
        Err(e) => {
            println!("Error parsing archive: {}", e);
            return;
        }
    };
    
    println!("Re-keying {} files", entries.len());
    println!("  Old key: {}", old_password);
    println!("  New key: {}", new_password);
    
    // Build with new key (re-encrypts data)
    let output = build_sor(&entries, new_key);
    
    // Write output
    match File::create(output_path) {
        Ok(mut f) => {
            if let Err(e) = f.write_all(&output) {
                println!("Error writing output: {}", e);
            } else {
                println!("Created {} ({} bytes)", output_path, output.len());
            }
        }
        Err(e) => {
            println!("Error creating output file: {}", e);
        }
    }
}

//! OBLIVION - LSM-Tree Key-Value Storage Engine
//! A high-performance, crash-recoverable storage engine
//! based on Log-Structured Merge Tree architecture.

use std::io::{self, BufRead, Write};

pub mod config;
pub mod engine;
pub mod error;
pub mod types;

use config::Config;
use engine::Oblivion;

fn main() {
    env_logger::init();

    println!();
    println!("  ╔═══════════════════════════════════════════╗");
    println!("  ║         OBLIVION Storage Engine           ║");
    println!("  ║      LSM-Tree Key-Value Store v1.0.0      ║");
    println!("  ╚═══════════════════════════════════════════╝");
    println!();
    println!("  Commands:");
    println!("    set <key> <value>  - Store a key-value pair");
    println!("    get <key>          - Retrieve a value by key");
    println!("    del <key>          - Delete a key");
    println!("    scan               - List all key-value pairs");
    println!("    info               - Show engine statistics");
    println!("    exit               - Shutdown engine");
    println!();

    let config = Config::default();
    let mut engine = match Oblivion::open(config) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("[ERROR] Failed to open engine: {}", err);
            std::process::exit(1);
        }
    };

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("oblivion> ");
        stdout.flush().unwrap();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap() == 0 {
            break; // EOF
        }

        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0].to_lowercase().as_str() {
            "set" | "put" => {
                if parts.len() < 3 {
                    println!("  Usage: set <key> <value>");
                    continue;
                }
                let key = parts[1].as_bytes().to_vec();
                let value = parts[2..].join(" ").as_bytes().to_vec();
                match engine.put(key, value) {
                    Ok(()) => println!("  OK"),
                    Err(e) => println!("  ERROR: {}", e),
                }
            }
            "get" => {
                if parts.len() < 2 {
                    println!("  Usage: get <key>");
                    continue;
                }
                let key = parts[1].as_bytes();
                match engine.get(key) {
                    Some(value) => match String::from_utf8(value) {
                        Ok(s) => println!("  \"{}\"", s),
                        Err(_) => println!("  <binary data>"),
                    },
                    None => println!("  (nil)"),
                }
            }
            "del" | "delete" => {
                if parts.len() < 2 {
                    println!("  Usage: del <key>");
                    continue;
                }
                let key = parts[1].as_bytes().to_vec();
                match engine.delete(key) {
                    Ok(()) => println!("  OK (deleted)"),
                    Err(e) => println!("  ERROR: {}", e),
                }
            }
            "scan" | "list" => {
                let entries = engine.scan();
                if entries.is_empty() {
                    println!("  (empty)");
                } else {
                    for (key, value) in &entries {
                        let k = String::from_utf8_lossy(key);
                        let v = String::from_utf8_lossy(value);
                        println!("  {} -> {}", k, v);
                    }
                    println!("  ({} entries)", entries.len());
                }
            }
            "info" | "stats" => {
                println!("  Entries:       {}", engine.len());
                println!("  MemTable size: {} bytes", engine.memtable_size());
            }
            "exit" | "quit" | "q" => {
                println!("  Shutting down OBLIVION...");
                break;
            }
            _ => {
                println!("  Unknown command: '{}'. Type 'exit' to quit.", parts[0]);
            }
        }
    }
}

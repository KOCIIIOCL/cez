#![allow(dead_code)]

mod ast;
mod codegen;
mod driver;
mod i18n;
mod lexer;
mod lsp;
mod parser;
mod sema;
mod token;
mod types;

use driver::{CompilerOptions, Driver};
use std::env;
use std::path::PathBuf;
use std::process;

fn main() {
    let raw_args: Vec<String> = env::args().collect();

    // Extract --lang=... or -L ... if provided
    let mut cli_lang = None;
    let mut filtered_args = Vec::new();

    let mut i = 0;
    while i < raw_args.len() {
        if i == 0 {
            filtered_args.push(raw_args[i].clone());
            i += 1;
            continue;
        }
        if raw_args[i].starts_with("--lang=") {
            cli_lang = Some(raw_args[i].trim_start_matches("--lang=").to_string());
        } else if raw_args[i] == "-L" && i + 1 < raw_args.len() {
            cli_lang = Some(raw_args[i + 1].clone());
            i += 1;
        } else {
            filtered_args.push(raw_args[i].clone());
        }
        i += 1;
    }

    let lang = i18n::Lang::detect(cli_lang.as_deref());
    let args = filtered_args;

    if args.first().map(|s| s.ends_with("cez-lsp")).unwrap_or(false) && (args.len() < 2 || args[1] == "lsp") {
        let mut server = lsp::CezLsp::new();
        server.run();
        return;
    }

    if args.len() < 2 {
        print!("{}", i18n::help_text(lang));
        process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "version" | "-v" | "--version" => {
            println!("{}", i18n::version_text(lang));
        }
        "help" | "-h" | "--help" => {
            print!("{}", i18n::help_text(lang));
        }
        "lsp" => {
            let mut server = lsp::CezLsp::new();
            server.run();
        }
        "hosts" | "free-hosts" => {
            let extra_args = if args.len() > 2 { &args[2..] } else { &[][..] };
            let status = std::process::Command::new("free-hosts")
                .args(extra_args)
                .status()
                .or_else(|_| {
                    std::process::Command::new("python3")
                        .arg("/home/low4rch/cez/tools/free_hosts.py")
                        .args(extra_args)
                        .status()
                });
            if let Ok(st) = status {
                if let Some(code) = st.code() {
                    process::exit(code);
                }
            }
        }
        "check" => {
            if args.len() < 3 {
                eprintln!("{}", i18n::msg_req_input_check(lang));
                process::exit(1);
            }
            let mut files = Vec::new();
            for arg in &args[2..] {
                if !arg.starts_with('-') {
                    files.push(PathBuf::from(arg));
                }
            }
            match Driver::compile_files_to_llvm_ir(&files, "x86_64-unknown-linux-gnu", false, lang) {
                Ok(_) => {
                    println!("{}", i18n::msg_check_passed(lang, files.len()));
                }
                Err(e) => {
                    eprintln!("{}", i18n::msg_semantic_error(lang, &e));
                    process::exit(1);
                }
            }
        }
        "emit-llvm" => {
            if args.len() < 3 {
                eprintln!("{}", i18n::msg_req_input_emit_llvm(lang));
                process::exit(1);
            }
            let mut options = CompilerOptions::default();
            options.lang = lang;
            options.emit_llvm = true;

            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "-o" => {
                        if i + 1 < args.len() {
                            options.output_file = Some(PathBuf::from(&args[i + 1]));
                            i += 1;
                        }
                    }
                    "--freestanding" => options.is_freestanding = true,
                    s if s.starts_with("--target=") => {
                        options.target = Some(s.trim_start_matches("--target=").to_string());
                    }
                    arg if !arg.starts_with('-') => {
                        options.input_files.push(PathBuf::from(arg));
                    }
                    _ => {}
                }
                i += 1;
            }

            match Driver::build(options) {
                Ok(path) => println!("{}", i18n::msg_emitted_llvm(lang, &path.display().to_string())),
                Err(e) => {
                    eprintln!("{}", i18n::msg_error(lang, &e));
                    process::exit(1);
                }
            }
        }
        "build" => {
            if args.len() < 3 {
                eprintln!("{}", i18n::msg_req_input_build(lang));
                process::exit(1);
            }
            let mut options = CompilerOptions::default();
            options.lang = lang;

            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "-o" => {
                        if i + 1 < args.len() {
                            options.output_file = Some(PathBuf::from(&args[i + 1]));
                            i += 1;
                        }
                    }
                    "-O0" => options.opt_level = "0".to_string(),
                    "-O1" => options.opt_level = "1".to_string(),
                    "-O2" => options.opt_level = "2".to_string(),
                    "-O3" => options.opt_level = "3".to_string(),
                    "--freestanding" => options.is_freestanding = true,
                    "--emit-llvm" => options.emit_llvm = true,
                    "--emit-obj" => options.emit_obj = true,
                    s if s.starts_with("--target=") => {
                        options.target = Some(s.trim_start_matches("--target=").to_string());
                    }
                    arg if !arg.starts_with('-') => {
                        options.input_files.push(PathBuf::from(arg));
                    }
                    _ => {}
                }
                i += 1;
            }

            match Driver::build(options) {
                Ok(path) => println!("{}", i18n::msg_built_successfully(lang, &path.display().to_string())),
                Err(e) => {
                    eprintln!("{}", i18n::msg_error(lang, &e));
                    process::exit(1);
                }
            }
        }
        "run" => {
            if args.len() < 3 {
                eprintln!("{}", i18n::msg_req_input_run(lang));
                process::exit(1);
            }
            let mut options = CompilerOptions::default();
            options.lang = lang;
            let mut run_args = Vec::new();

            let mut i = 2;
            let mut passed_double_dash = false;
            while i < args.len() {
                if passed_double_dash {
                    run_args.push(args[i].clone());
                } else if args[i] == "--" {
                    passed_double_dash = true;
                } else if args[i] == "--freestanding" {
                    options.is_freestanding = true;
                } else if !args[i].starts_with('-') && options.input_files.is_empty() {
                    options.input_files.push(PathBuf::from(&args[i]));
                } else if !options.input_files.is_empty() && !args[i].starts_with('-') && args[i].ends_with(".cez") {
                    options.input_files.push(PathBuf::from(&args[i]));
                } else {
                    run_args.push(args[i].clone());
                }
                i += 1;
            }

            if options.input_files.is_empty() {
                eprintln!("{}", i18n::msg_req_input_run(lang));
                process::exit(1);
            }

            // Create temporary binary for execution
            let stem = options.input_files[0].file_stem().unwrap().to_str().unwrap();
            let temp_bin = std::env::temp_dir().join(format!("cez_run_{}_{}", stem, process::id()));
            options.output_file = Some(temp_bin.clone());

            match Driver::run(options, &run_args) {
                Ok(code) => {
                    let _ = std::fs::remove_file(&temp_bin);
                    process::exit(code);
                }
                Err(e) => {
                    let _ = std::fs::remove_file(&temp_bin);
                    eprintln!("{}", i18n::msg_error(lang, &e));
                    process::exit(1);
                }
            }
        }
        unknown => {
            eprintln!("{}", i18n::msg_unknown_command(lang, unknown));
            process::exit(1);
        }
    }
}

#![allow(dead_code)]

mod ast;
mod codegen;
mod driver;
mod lexer;
mod parser;
mod sema;
mod token;
mod types;

use driver::{CompilerOptions, Driver};
use std::env;
use std::path::PathBuf;
use std::process;

fn print_help() {
    println!(
        r#"Cez Programming Language Compiler (v0.1.0)
Go syntax, LLVM backend, optimized for OSDev, low-level servers, and lightweight tools.

USAGE:
    cez <COMMAND> [OPTIONS] <FILE>

COMMANDS:
    build       Compile a Cez source file to executable or object file
    run         Compile and run a Cez program
    check       Typecheck a Cez source file without generating code
    emit-llvm   Emit LLVM IR (.ll) for a Cez source file
    version     Print compiler version
    help        Print this help message

OPTIONS:
    -o <FILE>           Output file path
    -O<LEVEL>           Optimization level: 0, 1, 2, 3 (default: 2)
    --freestanding      Freestanding / bare-metal mode (no libc, custom entrypoint)
    --target=<TRIPLE>   Target architecture triple (e.g. x86_64-unknown-none-elf)
    --emit-llvm         Emit LLVM IR instead of compiling
    --emit-obj          Emit relocatable object file (.o)

EXAMPLES:
    cez build main.cez -o myapp
    cez run main.cez
    cez build kernel.cez --freestanding -o kernel.o
    cez emit-llvm server.cez -o server.ll
"#
    );
}

fn print_version() {
    println!("cez version 0.1.0 (LLVM 22 backend)");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "version" | "-v" | "--version" => {
            print_version();
        }
        "help" | "-h" | "--help" => {
            print_help();
        }
        "check" => {
            if args.len() < 3 {
                eprintln!("Error: 'cez check' requires an input file.");
                process::exit(1);
            }
            let input_path = PathBuf::from(&args[2]);
            let source = match std::fs::read_to_string(&input_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error reading '{}': {}", input_path.display(), e);
                    process::exit(1);
                }
            };
            let mut lexer = lexer::Lexer::new(&source);
            let tokens = match lexer.tokenize_all() {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Lexer error: {}", e);
                    process::exit(1);
                }
            };
            let mut parser = parser::Parser::new(tokens);
            let program = match parser.parse_program() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Parser error: {}", e);
                    process::exit(1);
                }
            };
            let mut sema = sema::Sema::new();
            match sema.analyze_program(program) {
                Ok(_) => {
                    println!("Check passed: {} is valid Cez code.", input_path.display());
                }
                Err(e) => {
                    eprintln!("Semantic error: {}", e);
                    process::exit(1);
                }
            }
        }
        "emit-llvm" => {
            if args.len() < 3 {
                eprintln!("Error: 'cez emit-llvm' requires an input file.");
                process::exit(1);
            }
            let mut options = CompilerOptions::default();
            options.input_file = PathBuf::from(&args[2]);
            options.emit_llvm = true;

            let mut i = 3;
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
                    _ => {}
                }
                i += 1;
            }

            match Driver::build(options) {
                Ok(path) => println!("Emitted LLVM IR to {}", path.display()),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
        }
        "build" => {
            if args.len() < 3 {
                eprintln!("Error: 'cez build' requires an input file.");
                process::exit(1);
            }
            let mut options = CompilerOptions::default();
            options.input_file = PathBuf::from(&args[2]);

            let mut i = 3;
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
                    _ => {}
                }
                i += 1;
            }

            match Driver::build(options) {
                Ok(path) => println!("Built successfully: {}", path.display()),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
        }
        "run" => {
            if args.len() < 3 {
                eprintln!("Error: 'cez run' requires an input file.");
                process::exit(1);
            }
            let mut options = CompilerOptions::default();
            options.input_file = PathBuf::from(&args[2]);

            let mut run_args = Vec::new();
            let mut i = 3;
            while i < args.len() {
                if args[i] == "--" {
                    run_args.extend_from_slice(&args[i + 1..]);
                    break;
                } else if args[i] == "--freestanding" {
                    options.is_freestanding = true;
                } else {
                    run_args.push(args[i].clone());
                }
                i += 1;
            }

            // Create temporary binary for execution
            let stem = options.input_file.file_stem().unwrap().to_str().unwrap();
            let temp_bin = std::env::temp_dir().join(format!("cez_run_{}_{}", stem, process::id()));
            options.output_file = Some(temp_bin.clone());

            match Driver::run(options, &run_args) {
                Ok(code) => {
                    let _ = std::fs::remove_file(&temp_bin);
                    process::exit(code);
                }
                Err(e) => {
                    let _ = std::fs::remove_file(&temp_bin);
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
        }
        unknown => {
            eprintln!("Unknown command '{}'. Run 'cez help' for usage.", unknown);
            process::exit(1);
        }
    }
}

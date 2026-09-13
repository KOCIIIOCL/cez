use crate::codegen::llvm_ir::LlvmCodegen;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::sema::Sema;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct CompilerOptions {
    pub input_file: PathBuf,
    pub output_file: Option<PathBuf>,
    pub opt_level: String, // "0", "1", "2", "3", "s", "z"
    pub is_freestanding: bool,
    pub target: Option<String>,
    pub emit_llvm: bool,
    pub emit_obj: bool,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            input_file: PathBuf::new(),
            output_file: None,
            opt_level: "2".to_string(),
            is_freestanding: false,
            target: None,
            emit_llvm: false,
            emit_obj: false,
        }
    }
}

pub struct Driver;

impl Driver {
    pub fn compile_to_llvm_ir(source: &str, target_triple: &str, is_freestanding: bool) -> Result<String, String> {
        // 1. Lexer
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_all()?;

        // 2. Parser
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program()?;

        // 3. Sema
        let mut sema = Sema::new();
        let typed_program = sema.analyze_program(program)?;

        // 4. Codegen
        let mut codegen = LlvmCodegen::new(target_triple.to_string(), is_freestanding);
        let llvm_ir = codegen.generate(typed_program);

        Ok(llvm_ir)
    }

    pub fn build(options: CompilerOptions) -> Result<PathBuf, String> {
        let source = fs::read_to_string(&options.input_file)
            .map_err(|e| format!("Failed to read file '{}': {}", options.input_file.display(), e))?;

        let target_triple = options.target.clone().unwrap_or_else(|| {
            if options.is_freestanding {
                "x86_64-unknown-none-elf".to_string()
            } else {
                "x86_64-unknown-linux-gnu".to_string()
            }
        });

        let llvm_ir = Self::compile_to_llvm_ir(&source, &target_triple, options.is_freestanding)?;

        // If user just requested LLVM IR output
        if options.emit_llvm {
            let out_path = options.output_file.clone().unwrap_or_else(|| {
                options.input_file.with_extension("ll")
            });
            fs::write(&out_path, &llvm_ir)
                .map_err(|e| format!("Failed to write LLVM IR to '{}': {}", out_path.display(), e))?;
            return Ok(out_path);
        }

        // Write temporary .ll file
        let stem = options.input_file.file_stem().unwrap().to_str().unwrap();
        let temp_ll_path = std::env::temp_dir().join(format!("cez_{}_{}.ll", stem, std::process::id()));
        fs::write(&temp_ll_path, &llvm_ir)
            .map_err(|e| format!("Failed to write temporary LLVM IR to '{}': {}", temp_ll_path.display(), e))?;

        let final_output = options.output_file.clone().unwrap_or_else(|| {
            if options.emit_obj {
                options.input_file.with_extension("o")
            } else {
                PathBuf::from(stem)
            }
        });

        // Invoke clang to optimize and compile
        let mut cmd = Command::new("clang");
        cmd.arg(format!("-O{}", options.opt_level));
        cmd.arg(&temp_ll_path);
        cmd.arg("-o").arg(&final_output);

        if let Some(ref t) = options.target {
            cmd.arg(format!("--target={}", t));
        }

        if options.is_freestanding {
            cmd.arg("-ffreestanding");
            cmd.arg("-nostdlib");
            if options.emit_obj || !options.target.as_deref().unwrap_or("").contains("linux") {
                // For kernels or freestanding modules without crt, produce relocatable object or ELF
                if !cmd.get_args().any(|a| a == "-c" || a == "-r") {
                    cmd.arg("-c");
                }
            }
        }

        if options.emit_obj && !cmd.get_args().any(|a| a == "-c") {
            cmd.arg("-c");
        }

        let output = cmd.output().map_err(|e| format!("Failed to execute clang: {}", e))?;

        // Cleanup temporary .ll file
        let _ = fs::remove_file(&temp_ll_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Clang compilation failed:\n{}", stderr));
        }

        Ok(final_output)
    }

    pub fn run(options: CompilerOptions, run_args: &[String]) -> Result<i32, String> {
        let bin_path = Self::build(options)?;

        let mut child = Command::new(&bin_path)
            .args(run_args)
            .spawn()
            .map_err(|e| format!("Failed to execute '{}': {}", bin_path.display(), e))?;

        let status = child.wait().map_err(|e| format!("Execution failed: {}", e))?;

        // If executable was created in a temp dir or default, leave or clean up
        Ok(status.code().unwrap_or(-1))
    }
}

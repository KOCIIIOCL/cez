use crate::codegen::llvm_ir::LlvmCodegen;
use crate::i18n::{self, Lang};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::sema::Sema;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct CompilerOptions {
    pub input_files: Vec<PathBuf>,
    pub output_file: Option<PathBuf>,
    pub opt_level: String, // "0", "1", "2", "3", "s", "z"
    pub is_freestanding: bool,
    pub target: Option<String>,
    pub emit_llvm: bool,
    pub emit_obj: bool,
    pub lang: Lang,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            input_files: Vec::new(),
            output_file: None,
            opt_level: "2".to_string(),
            is_freestanding: false,
            target: None,
            emit_llvm: false,
            emit_obj: false,
            lang: Lang::En,
        }
    }
}

pub struct Driver;

impl Driver {
    pub fn compile_files_to_llvm_ir(
        files: &[PathBuf],
        target_triple: &str,
        is_freestanding: bool,
        lang: Lang,
    ) -> Result<String, String> {
        if files.is_empty() {
            return Err(i18n::msg_no_input_files(lang).to_string());
        }

        let mut loaded_files = std::collections::HashSet::new();
        let mut queue: std::collections::VecDeque<PathBuf> = files.iter().cloned().collect();
        for f in files {
            if let Ok(canon) = f.canonicalize() {
                loaded_files.insert(canon);
            } else {
                loaded_files.insert(f.clone());
            }
        }

        let mut merged_program: Option<crate::ast::Program> = None;

        while let Some(file) = queue.pop_front() {
            let source = fs::read_to_string(&file)
                .map_err(|e| i18n::msg_failed_read_file(lang, &file.display().to_string(), &e.to_string()))?;

            let mut lexer = Lexer::new(&source);
            let tokens = lexer.tokenize_all()?;

            let mut parser = Parser::new(tokens);
            let program = parser.parse_program()?;

            // Check imports to see if any point to .cez files or standard library modules
            for imp in &program.imports {
                let mut candidates = Vec::new();

                // CEZ_ROOT environment variable — highest priority for dev/custom installs
                if let Ok(root) = std::env::var("CEZ_ROOT") {
                    let root_path = PathBuf::from(&root);
                    candidates.push(root_path.join(format!("core/{}.cez", imp)));
                    candidates.push(root_path.join(format!("{}.cez", imp)));
                    candidates.push(root_path.join(format!("core/{}", imp)));
                }
                // Relative to running cez binary (e.g. ~/.local/bin/cez -> ~/.local/lib/cez/core/...)
                if let Ok(exe) = std::env::current_exe() {
                    if let Some(exe_dir) = exe.parent() {
                        candidates.push(exe_dir.join(format!("../lib/cez/core/{}.cez", imp)));
                        candidates.push(exe_dir.join(format!("../lib/cez/{}.cez", imp)));
                        candidates.push(exe_dir.join(format!("../core/{}.cez", imp)));
                        candidates.push(exe_dir.join(format!("core/{}.cez", imp)));
                    }
                }
                // ~/.local/lib/cez/core/ for user-local installs
                if let Ok(home) = std::env::var("HOME") {
                    candidates.push(PathBuf::from(format!("{}/.local/lib/cez/core/{}.cez", home, imp)));
                    candidates.push(PathBuf::from(format!("{}/.local/lib/cez/{}.cez", home, imp)));
                }
                // Relative to source file
                if let Some(parent) = file.parent() {
                    candidates.push(parent.join(format!("{}.cez", imp)));
                    candidates.push(parent.join(format!("core/{}.cez", imp)));
                    candidates.push(parent.join(imp));
                }
                // Relative to working directory
                candidates.push(PathBuf::from(format!("core/{}.cez", imp)));
                candidates.push(PathBuf::from(format!("core/{}", imp)));
                candidates.push(PathBuf::from(format!("std/{}.cez", imp)));
                candidates.push(PathBuf::from(format!("{}.cez", imp)));
                candidates.push(PathBuf::from(imp));

                // System paths (lowest priority)
                candidates.push(PathBuf::from(format!("/usr/local/lib/cez/core/{}.cez", imp)));
                candidates.push(PathBuf::from(format!("/usr/local/lib/cez/{}.cez", imp)));
                candidates.push(PathBuf::from(format!("/usr/lib/cez/core/{}.cez", imp)));
                candidates.push(PathBuf::from(format!("/usr/lib/cez/{}.cez", imp)));
                candidates.push(PathBuf::from(format!("/usr/lib/cez/{}", imp)));
                for cand in candidates {
                    if cand.is_file() {
                        let canon = cand.canonicalize().unwrap_or_else(|_| cand.clone());
                        if !loaded_files.contains(&canon) {
                            loaded_files.insert(canon);
                            queue.push_back(cand);
                            break;
                        }
                    }
                }
            }

            if let Some(ref mut merged) = merged_program {
                merged.decls.extend(program.decls);
                for imp in program.imports {
                    if !merged.imports.contains(&imp) {
                        merged.imports.push(imp);
                    }
                }
            } else {
                merged_program = Some(program);
            }
        }

        let program = merged_program.unwrap();

        // 3. Sema
        let mut sema = Sema::new();
        let typed_program = sema.analyze_program(program)?;

        // 4. Codegen
        let mut codegen = LlvmCodegen::new(target_triple.to_string(), is_freestanding);
        let llvm_ir = codegen.generate(typed_program);

        Ok(llvm_ir)
    }

    pub fn build(options: CompilerOptions) -> Result<PathBuf, String> {
        let lang = options.lang;
        if options.input_files.is_empty() {
            return Err(i18n::msg_no_input_files(lang).to_string());
        }

        let target_triple = options.target.clone().unwrap_or_else(|| {
            if options.is_freestanding {
                "x86_64-unknown-none-elf".to_string()
            } else {
                "x86_64-unknown-linux-gnu".to_string()
            }
        });

        let llvm_ir = Self::compile_files_to_llvm_ir(
            &options.input_files,
            &target_triple,
            options.is_freestanding,
            lang,
        )?;

        let primary_file = &options.input_files[0];

        // If user just requested LLVM IR output
        if options.emit_llvm {
            let out_path = options.output_file.clone().unwrap_or_else(|| {
                primary_file.with_extension("ll")
            });
            fs::write(&out_path, &llvm_ir)
                .map_err(|e| i18n::msg_failed_write_llvm(lang, &out_path.display().to_string(), &e.to_string()))?;
            return Ok(out_path);
        }

        // Write temporary .ll file
        let stem = primary_file.file_stem().unwrap().to_str().unwrap();
        let temp_ll_path = std::env::temp_dir().join(format!("cez_{}_{}.ll", stem, std::process::id()));
        fs::write(&temp_ll_path, &llvm_ir)
            .map_err(|e| i18n::msg_failed_write_llvm(lang, &temp_ll_path.display().to_string(), &e.to_string()))?;

        let final_output = options.output_file.clone().unwrap_or_else(|| {
            if options.emit_obj {
                primary_file.with_extension("o")
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

        let output = cmd.output().map_err(|e| i18n::msg_failed_execute_clang(lang, &e.to_string()))?;

        // Cleanup temporary .ll file
        let _ = fs::remove_file(&temp_ll_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(i18n::msg_clang_compilation_failed(lang, &stderr));
        }

        Ok(final_output)
    }

    pub fn run(options: CompilerOptions, run_args: &[String]) -> Result<i32, String> {
        let lang = options.lang;
        let bin_path = Self::build(options)?;

        let mut child = Command::new(&bin_path)
            .args(run_args)
            .spawn()
            .map_err(|e| i18n::msg_failed_execute_bin(lang, &bin_path.display().to_string(), &e.to_string()))?;

        let status = child.wait().map_err(|e| i18n::msg_execution_failed(lang, &e.to_string()))?;

        // If executable was created in a temp dir or default, leave or clean up
        Ok(status.code().unwrap_or(-1))
    }
}

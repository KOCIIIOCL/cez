#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Ru,
    Uk,
}

impl Lang {
    pub fn from_str(s: &str) -> Option<Self> {
        let lower = s.to_lowercase();
        if lower.starts_with("ru") {
            Some(Lang::Ru)
        } else if lower.starts_with("uk") || lower.starts_with("ua") {
            Some(Lang::Uk)
        } else if lower.starts_with("en") {
            Some(Lang::En)
        } else {
            None
        }
    }

    pub fn detect(cli_lang: Option<&str>) -> Self {
        if let Some(l) = cli_lang.and_then(Self::from_str) {
            return l;
        }
        if let Ok(val) = std::env::var("CEZ_LANG") {
            if let Some(l) = Self::from_str(&val) {
                return l;
            }
        }
        for var in &["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(val) = std::env::var(var) {
                if let Some(l) = Self::from_str(&val) {
                    return l;
                }
            }
        }
        Lang::En
    }

    pub fn code(&self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Ru => "ru",
            Lang::Uk => "uk",
        }
    }
}

pub fn help_text(lang: Lang) -> &'static str {
    match lang {
        Lang::En => {
            r#"Cez Programming Language Compiler (v0.1.0)
Go syntax, LLVM backend, optimized for OSDev, low-level servers, and lightweight tools.

USAGE:
    cez [OPTIONS] <COMMAND> [ARGS]

COMMANDS:
    build       Compile a Cez source file to executable or object file
    run         Compile and run a Cez program
    check       Typecheck a Cez source file without generating code
    emit-llvm   Emit LLVM IR (.ll) for a Cez source file
    lsp         Start Language Server Protocol (LSP) daemon
    version     Print compiler version
    help        Print this help message

OPTIONS:
    -o <FILE>           Output file path
    -O<LEVEL>           Optimization level: 0, 1, 2, 3 (default: 2)
    --freestanding      Freestanding / bare-metal mode (no libc, custom entrypoint)
    --target=<TRIPLE>   Target architecture triple (e.g. x86_64-unknown-none-elf)
    --emit-llvm         Emit LLVM IR instead of compiling
    --emit-obj          Emit relocatable object file (.o)
    --lang=<LANG>       Message language: en, ru, uk (or set $CEZ_LANG)

EXAMPLES:
    cez build main.cez -o myapp
    cez run main.cez
    cez build kernel.cez --freestanding -o kernel.o
    cez emit-llvm server.cez -o server.ll
"#
        }
        Lang::Ru => {
            r#"Компилятор языка программирования Cez (v0.1.0)
Go-синтаксис, бэкенд LLVM, оптимизирован для OSDev, низкоуровневых серверов и быстрых утилит.

ИСПОЛЬЗОВАНИЕ:
    cez [ОПЦИИ] <КОМАНДА> [АРГУМЕНТЫ]

КОМАНДЫ:
    build       Скомпилировать исходный файл Cez в исполняемый или объектный файл
    run         Скомпилировать и запустить программу на Cez
    check       Проверить типы исходных файлов Cez без генерации кода
    emit-llvm   Сгенерировать LLVM IR (.ll) для исходного файла Cez
    lsp         Запустить демон языкового сервера (LSP)
    version     Вывести версию компилятора
    help        Вывести это справочное сообщение

ОПЦИИ:
    -o <ФАЙЛ>           Путь к выходному файлу
    -O<УРОВЕНЬ>         Уровень оптимизации: 0, 1, 2, 3 (по умолчанию: 2)
    --freestanding      Режим freestanding / bare-metal (без libc, собственная точка входа)
    --target=<ЦЕЛЬ>     Целевая архитектура (например, x86_64-unknown-none-elf)
    --emit-llvm         Вывести LLVM IR вместо компиляции
    --emit-obj          Создать перемещаемый объектный файл (.o)
    --lang=<ЯЗЫК>       Язык сообщений: en, ru, uk (также через $CEZ_LANG)

ПРИМЕРЫ:
    cez build main.cez -o myapp
    cez run main.cez
    cez build kernel.cez --freestanding -o kernel.o
    cez emit-llvm server.cez -o server.ll
"#
        }
        Lang::Uk => {
            r#"Компілятор мови програмування Cez (v0.1.0)
Go-синтаксис, бекенд LLVM, оптимізований для OSDev, низькорівневих серверів та швидких утиліт.

ВИКОРИСТАННЯ:
    cez [ОПЦІЇ] <КОМАНДА> [АРГУМЕНТИ]

КОМАНДИ:
    build       Скомпілювати вихідний файл Cez у виконуваний або об'єктний файл
    run         Скомпілювати та запустити програму на Cez
    check       Перевірити типи вихідного файлу Cez без генерації коду
    emit-llvm   Згенерувати LLVM IR (.ll) для вихідного файлу Cez
    lsp         Запустити демон мовного сервера (LSP)
    version     Вивести версію компілятора
    help        Вивести це довідкове повідомлення

ОПЦІЇ:
    -o <ФАЙЛ>           Шлях до вихідного файлу
    -O<РІВЕНЬ>          Рівень оптимізації: 0, 1, 2, 3 (за замовчуванням: 2)
    --freestanding      Режим freestanding / bare-metal (без libc, власна точка входу)
    --target=<ЦІЛЬ>     Цільова архітектура (наприклад, x86_64-unknown-none-elf)
    --emit-llvm         Вивести LLVM IR замість повної компіляції
    --emit-obj          Створити переміщуваний об'єктний файл (.o)
    --lang=<МОВА>       Мова виводу: en, ru, uk (також через $CEZ_LANG)

ПРИКЛАДИ:
    cez build main.cez -o myapp
    cez run main.cez
    cez build kernel.cez --freestanding -o kernel.o
    cez emit-llvm server.cez -o server.ll
"#
        }
    }
}

pub fn version_text(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "cez version 0.1.0 (LLVM 22 backend)",
        Lang::Ru => "cez версия 0.1.0 (бэкенд LLVM 22)",
        Lang::Uk => "cez версія 0.1.0 (бекенд LLVM 22)",
    }
}

pub fn msg_built_successfully(lang: Lang, path: &str) -> String {
    match lang {
        Lang::En => format!("Built successfully: {}", path),
        Lang::Ru => format!("Успешно собрано: {}", path),
        Lang::Uk => format!("Успішно зібрано: {}", path),
    }
}

pub fn msg_emitted_llvm(lang: Lang, path: &str) -> String {
    match lang {
        Lang::En => format!("Emitted LLVM IR to {}", path),
        Lang::Ru => format!("LLVM IR сохранён в {}", path),
        Lang::Uk => format!("LLVM IR збережено у {}", path),
    }
}

pub fn msg_check_passed(lang: Lang, count: usize) -> String {
    match lang {
        Lang::En => format!("Check passed: all {} files are valid Cez code.", count),
        Lang::Ru => format!("Проверка пройдена: все {} файлов содержат корректный код Cez.", count),
        Lang::Uk => format!("Перевірку пройдено: всі {} файлів містять коректний код Cez.", count),
    }
}

pub fn msg_semantic_error(lang: Lang, err: &str) -> String {
    match lang {
        Lang::En => format!("Semantic error: {}", err),
        Lang::Ru => format!("Семантическая ошибка: {}", err),
        Lang::Uk => format!("Семантична помилка: {}", err),
    }
}

pub fn msg_req_input_build(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "Error: 'cez build' requires an input file.",
        Lang::Ru => "Ошибка: для 'cez build' требуется указать входной файл.",
        Lang::Uk => "Помилка: для 'cez build' потрібно вказати вхідний файл.",
    }
}

pub fn msg_req_input_run(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "Error: 'cez run' requires an input file.",
        Lang::Ru => "Ошибка: для 'cez run' требуется указать входной файл.",
        Lang::Uk => "Помилка: для 'cez run' потрібно вказати вхідний файл.",
    }
}

pub fn msg_req_input_check(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "Error: 'cez check' requires at least one input file.",
        Lang::Ru => "Ошибка: для 'cez check' требуется указать хотя бы один входной файл.",
        Lang::Uk => "Помилка: для 'cez check' потрібно вказати хоча б один вхідний файл.",
    }
}

pub fn msg_req_input_emit_llvm(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "Error: 'cez emit-llvm' requires an input file.",
        Lang::Ru => "Ошибка: для 'cez emit-llvm' требуется указать входной файл.",
        Lang::Uk => "Помилка: для 'cez emit-llvm' потрібно вказати вхідний файл.",
    }
}

pub fn msg_unknown_command(lang: Lang, cmd: &str) -> String {
    match lang {
        Lang::En => format!("Unknown command '{}'. Run 'cez help' for usage.", cmd),
        Lang::Ru => format!("Неизвестная команда '{}'. Запустите 'cez help' для справки.", cmd),
        Lang::Uk => format!("Невідома команда '{}'. Запустіть 'cez help' для довідки.", cmd),
    }
}

pub fn msg_error(lang: Lang, err: &str) -> String {
    match lang {
        Lang::En => format!("Error: {}", err),
        Lang::Ru => format!("Ошибка: {}", err),
        Lang::Uk => format!("Помилка: {}", err),
    }
}

pub fn msg_no_input_files(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "No input files provided",
        Lang::Ru => "Не указаны входные файлы",
        Lang::Uk => "Не вказано вхідні файли",
    }
}

pub fn msg_failed_read_file(lang: Lang, path: &str, err: &str) -> String {
    match lang {
        Lang::En => format!("Failed to read file '{}': {}", path, err),
        Lang::Ru => format!("Не удалось прочитать файл '{}': {}", path, err),
        Lang::Uk => format!("Не вдалося прочитати файл '{}': {}", path, err),
    }
}

pub fn msg_failed_write_llvm(lang: Lang, path: &str, err: &str) -> String {
    match lang {
        Lang::En => format!("Failed to write LLVM IR to '{}': {}", path, err),
        Lang::Ru => format!("Не удалось записать LLVM IR в '{}': {}", path, err),
        Lang::Uk => format!("Не вдалося записати LLVM IR у '{}': {}", path, err),
    }
}

pub fn msg_failed_execute_clang(lang: Lang, err: &str) -> String {
    match lang {
        Lang::En => format!("Failed to execute clang: {}", err),
        Lang::Ru => format!("Не удалось запустить clang: {}", err),
        Lang::Uk => format!("Не вдалося запустити clang: {}", err),
    }
}

pub fn msg_clang_compilation_failed(lang: Lang, stderr: &str) -> String {
    match lang {
        Lang::En => format!("Clang compilation failed:\n{}", stderr),
        Lang::Ru => format!("Ошибка компиляции Clang:\n{}", stderr),
        Lang::Uk => format!("Помилка компіляції Clang:\n{}", stderr),
    }
}

pub fn msg_failed_execute_bin(lang: Lang, path: &str, err: &str) -> String {
    match lang {
        Lang::En => format!("Failed to execute '{}': {}", path, err),
        Lang::Ru => format!("Не удалось запустить '{}': {}", path, err),
        Lang::Uk => format!("Не вдалося запустити '{}': {}", path, err),
    }
}

pub fn msg_execution_failed(lang: Lang, err: &str) -> String {
    match lang {
        Lang::En => format!("Execution failed: {}", err),
        Lang::Ru => format!("Ошибка при выполнении: {}", err),
        Lang::Uk => format!("Помилка під час виконання: {}", err),
    }
}

# Cez Programming Language

**Cez** — это системный язык программирования со знакомым, чистым и эргономичным синтаксисом **Go**, оптимизированный для:
1. **OSDev (первоклассный режим "лучший из лучших")**: нативная поддержка bare-metal/freestanding ядер, `@packed` структур для аппаратных дескрипторов (IDT, GDT, TSS, Page Tables), `@align(N)`, размещения данных в ELF-секциях (`@section(".multiboot")`), `@naked` функций без прологов и эпилогов для прерываний и переключения контекста, прямого ассемблера `inline_asm`, volatile операций для MMIO и кастомных аллокаторов.
2. **Низкоуровневых высокопроизводительных серверов**: zero-cost слайсы `[]T` без неявных копирований, быстрые арены памяти, `defer` для предсказуемой очистки ресурсов, прямой доступ к сокетам и системным вызовам без лишних уровней абстракций.
3. **Бытовых легких утилит и TUI**: лаконичный синтаксис Go (`:=`, `defer`, методы на структурах, срезы строк, ANSI эскейп-коды).
4. **Бэкенд на LLVM**: генерация переносимого, чистого LLVM IR с использованием мощных оптимизаторов (`-O1` .. `-O3`) и компоновщика LLD.
5. **Управление памятью**:
   - По умолчанию: ручное управление памятью, арены памяти и `defer` (нулевой оверхед, отсутствие скрытых пауз).
   - В перспективе: модульный GC (`--gc=none` по умолчанию, с возможностью подключения tracing GC для высокоуровневых сценариев).
6. **Подготовка к самохостингу**: компилятор спроектирован модульно (Lexer -> Parser -> AST -> Sema -> LLVM Codegen -> Driver), что обеспечивает простую трансляцию компилятора на сам Cez в будущем.

---

## Установка и сборка

Компилятор Cez не требует внешних зависимостей кроме Rust (для начальной сборки) и системного Clang/LLVM.

```bash
# Сборка компилятора
cargo build --release

# Бинарник компилятора находится в:
./target/release/cez --help
```

---

## Быстрый старт и CLI

```bash
# Скомпилировать и сразу запустить программу
cez run examples/hello.cez

# Запустить серверный пример парсинга сетевых пакетов
cez run examples/server_packet.cez

# Запустить интерактивную TUI панель мониторинга
cez run examples/tui_dashboard.cez

# Скомпилировать baremetal ядро ОС с Multiboot-заголовком без libc
cez build examples/kernel_multiboot.cez --freestanding -o kernel.o

# Проверить секции ядра
readelf -S kernel.o

# Сгенерировать чистый LLVM IR (.ll)
cez emit-llvm examples/hello.cez -o hello.ll
```

---

## Синтаксис и возможности языка

### 1. Базовый синтаксис в стиле Go
```go
package main

extern func puts(s *i8) i32

func main() int {
    defer puts("Deferred: executed in LIFO order at exit!")

    puts("Hello from Cez!")

    var a int = 20
    b := 22
    sum := a + b

    if sum == 42 {
        puts("Math check passed: 20 + 22 = 42")
    }

    return 0
}
```

### 2. Структуры, методы и `@packed` для сетевых протоколов и серверов
```go
package main

extern func printf(fmt *i8, ...) i32

// Сетевой заголовок без padding-байтов
type PacketHeader struct @packed {
    magic       u16
    version     u8
    opcode      u8
    payload_len u32
}

func (h *PacketHeader) IsValid() bool {
    return h.magic == 0x5054 && h.version == 1
}

func (h *PacketHeader) Summary() {
    printf("Packet[magic=0x%04X, ver=%d, size=%u bytes]\n", 
           u32(h.magic), u32(h.version), h.payload_len)
}

func main() int {
    var hdr PacketHeader = PacketHeader{
        magic: 0x5054,
        version: 1,
        opcode: 7,
        payload_len: 1024,
    }

    if hdr.IsValid() {
        hdr.Summary()
    }

    return 0
}
```

### 3. OSDev: Multiboot, прерывания, `@naked`, MMIO и inline ассемблер
```go
package main

// Multiboot 1 заголовок, помещаемый в секцию .multiboot с выравниванием 4 байта
type MultibootHeader struct @packed {
    magic    u32
    flags    u32
    checksum u32
}

var mb_header MultibootHeader @section(".multiboot") @align(4) = MultibootHeader{
    magic: 0x1BADB002,
    flags: 0x00000003,
    checksum: 0xE4524FFB,
}

// Таблица IDT дескрипторов прерываний
type IDTEntry struct @packed {
    offset_low  u16
    selector    u16
    ist         u8
    type_attr   u8
    offset_mid  u16
    offset_high u32
    reserved    u32
}

var idt_table [256]IDTEntry @align(4096)

// Порты ввода-вывода через встроенный ассемблер
func outb(port u16, val u8) {
    inline_asm("outb %0, %1" : : "{al}"(val), "{dx}"(port))
}

// Naked функция для обработчика прерывания
func isr_keyboard() @naked {
    inline_asm("cli")
    outb(0x20, 0x20)
    inline_asm("iretq")
}

// Точка входа ядра без пролога/эпилога
func kernel_entry() @export("_start") @naked {
    inline_asm("cli")

    // Прямая запись в текстовый буфер VGA (MMIO 0xB8000)
    vga := (*u16)(uintptr(0xb8000))
    @volatile_store(vga, u16(0x0A43)) // 'C' зеленым цветом

    inline_asm("hlt")
}
```

### 4. Арены памяти (`core/mem.cez`)
```go
// Инициализация легковесной арены
var arena Arena
arena.Init(buffer, 1024 * 1024) // 1MB
defer arena.Reset()

ptr := arena.Alloc(256)
```

---

## Архитектура компилятора

```
cez/
├── src/
│   ├── main.rs              # CLI утилита (build, run, check, emit-llvm)
│   ├── token.rs             # Токены, ключевые слова, спаны, ASI
│   ├── lexer.rs             # Лексический анализатор (авто-вставка точек с запятой)
│   ├── ast.rs               # Синтаксическое дерево (Go AST + OSDev атрибуты)
│   ├── parser.rs            # Рекурсивный спуск + Pratt parser для выражений
│   ├── types.rs             # Типизация (числа, структуры, указатели, слайсы, функции)
│   ├── sema.rs              # Семантический анализ, вывод типов, валидация OSDev
│   ├── codegen/
│   │   ├── mod.rs
│   │   └── llvm_ir.rs       # Генератор оптимизированного LLVM IR
│   └── driver.rs            # Оркестрация вызовов Clang/LLVM/LLD
├── core/
│   └── mem.cez              # Арены памяти, memcpy, memset
├── examples/
│   ├── hello.cez            # Базовый пример + defer
│   ├── server_packet.cez    # Сетевой серверный протокол + @packed
│   ├── kernel_multiboot.cez # OSDev ядро с Multiboot, IDT, MMIO, naked
│   └── tui_dashboard.cez    # Консольный TUI дашборд
└── tests/
    └── integration_tests.rs # E2E интеграционные тесты
```

---

## Лицензия
MIT

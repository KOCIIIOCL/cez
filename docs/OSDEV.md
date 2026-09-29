# Разработка операционных систем на языке Cez (OSDev Guide)

Язык **Cez** спроектирован с фокусом на разработку ядер операционных систем, гипервизоров и bare-metal прошивок. В режиме `--freestanding` компилятор не тянет libc или рантайм, генерируя исключительно чистый машинный код под целевую архитектуру.

---

## 1. Заголовок Multiboot 1

Спецификация Multiboot требует наличия 12-байтного заголовка в первых 8 КБ исполняемого файла ELF, выровненного по 4 байта. В Cez это реализуется одной типизированной структурой с атрибутами `@packed`, `@align` и `@section`:

```go
package main

type MultibootHeader struct @packed @align(4) @section(".multiboot") {
    magic    u32
    flags    u32
    checksum u32
}

// 0x1BADB002 — сигнатура Multiboot 1
// 0x00 — флаги
// 0xE4524FFE — контрольная сумма (-(magic + flags))
var mb_hdr MultibootHeader = MultibootHeader{
    magic:    0x1BADB002,
    flags:    0,
    checksum: 0xE4524FFE,
}
```

---

## 2. Точка входа ядра без прологов (`@naked`)

Точка входа ядра `_start` вызывается загрузчиком (GRUB, QEMU). Она не должна иметь стандартного пролога стека (`push rbp; mov rbp, rsp`):

```go
func _start() @naked @section(".text.boot") {
    inline_asm("cli")             // Отключаем аппаратные прерывания
    inline_asm("mov rsp, 0x90000") // Настраиваем начальный стек

    // Прямой вывод в видеобуфер VGA (0xB8000)
    vga := (*u16)(uintptr(0xB8000))
    *vga = u16(0x0F43) // 'C' с белым текстом на черном фоне

    kmain()

    // Остановка процессора при выходе
    inline_asm("hlt")
}
```

---

## 3. Дескрипторы прерываний (IDT) и GDT с `@packed`

Аппаратные структуры x86_64 требуют точного расположения байт без выравнивания компилятором:

```go
// 16-байтный дескриптор шлюза прерывания IDT x86_64
type IdtEntry struct @packed {
    offset_low  u16
    selector    u16
    ist         u8
    type_attr   u8
    offset_mid  u16
    offset_high u32
    zero        u32
}

type IdtPointer struct @packed {
    limit u16
    base  uintptr
}

// Таблица IDT на 256 шлюзов, выровненная по 4096 байт
var idt_table [256]IdtEntry @align(4096) @section(".bss")
var idt_ptr IdtPointer

func InitIDT() {
    idt_ptr.limit = u16(256 * 16 - 1)
    idt_ptr.base = uintptr(&idt_table)

    inline_asm("lidt [idt_ptr]")
}
```

---

## 4. Порты ввода-вывода (I/O Ports)

Взаимодействие с аппаратными контроллерами (PIC, PIT, Serial UART) через инструкции `in` и `out`:

```go
func outb(port u16, val u8) {
    inline_asm("out dx, al" : : "d"(port), "a"(val))
}

func inb(port u16) u8 {
    var val u8 = 0
    inline_asm("in al, dx" : "=a"(val) : "d"(port))
    return val
}
```

---

## 5. Прямой доступ к памяти (MMIO)

Работа с VGA-буфером, APIC, ACPI таблицами или сетевыми картами осуществляется через типизированные указатели:

```go
func ClearScreen() {
    vga := (*u16)(uintptr(0xB8000))
    blank := u16(0x0720) // Пробел с серым цветом

    for i := 0; i < 80 * 25; i += 1 {
        *(vga + i) = blank
    }
}
```

---

## 6. Сборка Bare-Metal ядра

Для компиляции ядра без стандартной библиотеки и libc используется флаг `--freestanding`:

```bash
# Компиляция ядра в объектный файл
cez build kernel.cez --freestanding -o kernel.o

# Проверка сгенерированных секций ELF
readelf -S kernel.o

# Линковка в готовый образ ядра
ld -m elf_x86_64 -T linker.ld -o kernel.bin kernel.o

# Запуск ядра в QEMU
qemu-system-x86_64 -kernel kernel.bin
```

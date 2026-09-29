; Keywords
[
  "break"
  "case"
  "const"
  "continue"
  "default"
  "defer"
  "else"
  "fallthrough"
  "for"
  "func"
  "if"
  "import"
  "package"
  "return"
  "struct"
  "switch"
  "type"
  "var"
] @keyword

; OSDev and Cez keywords
((identifier) @keyword
  (#match? @keyword "^(extern|inline_asm)$"))

; Functions and methods
(function_declaration
  name: (identifier) @function)

(call_expression
  function: (identifier) @function)

(call_expression
  function: (selector_expression
    field: (field_identifier) @function.method))

(method_declaration
  name: (field_identifier) @function.method)

; Types
(type_identifier) @type

; Literals
[
  (interpreted_string_literal)
  (raw_string_literal)
  (rune_literal)
] @string

(escape_sequence) @string.escape

[
  (int_literal)
  (float_literal)
] @number

[
  (true)
  (false)
  (nil)
] @constant.builtin

; Comments
(comment) @comment

; Operators
[
  "--"
  "-"
  "-="
  ":="
  "!"
  "!="
  "..."
  "*"
  "*="
  "/"
  "/="
  "&"
  "&&"
  "&="
  "%"
  "%="
  "^"
  "^="
  "+"
  "++"
  "+="
  "<"
  "<<"
  "<<="
  "<="
  "="
  "=="
  ">"
  ">="
  ">>"
  ">>="
  "|"
  "|="
  "||"
] @operator

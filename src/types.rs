use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructFieldType {
    pub name: String,
    pub ty: Type,
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructType {
    pub name: String,
    pub fields: Vec<StructFieldType>,
    pub is_packed: bool,
    pub explicit_align: Option<usize>,
    pub total_size: usize,
    pub alignment: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncType {
    pub params: Vec<Type>,
    pub ret_type: Box<Type>,
    pub is_variadic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Void,
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    Usize,
    Isize,
    Uintptr,
    F32,
    F64,
    String,
    Pointer(Box<Type>),
    Array(Box<Type>, usize),
    Slice(Box<Type>),
    Struct(StructType),
    Named(String), // Unresolved named type during parsing/early sema
    Function(FuncType),
}

impl Type {
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Type::I8
                | Type::I16
                | Type::I32
                | Type::I64
                | Type::U8
                | Type::U16
                | Type::U32
                | Type::U64
                | Type::Usize
                | Type::Isize
                | Type::Uintptr
        )
    }

    pub fn is_signed(&self) -> bool {
        matches!(self, Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::Isize)
    }

    pub fn is_unsigned(&self) -> bool {
        matches!(
            self,
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::Usize | Type::Uintptr
        )
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Type::F32 | Type::F64)
    }

    pub fn is_pointer(&self) -> bool {
        matches!(self, Type::Pointer(_))
    }

    pub fn is_slice(&self) -> bool {
        matches!(self, Type::Slice(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, Type::Array(_, _))
    }

    pub fn is_struct(&self) -> bool {
        matches!(self, Type::Struct(_))
    }

    pub fn is_void(&self) -> bool {
        matches!(self, Type::Void)
    }

    pub fn size_of(&self) -> usize {
        match self {
            Type::Void => 0,
            Type::Bool | Type::I8 | Type::U8 => 1,
            Type::I16 | Type::U16 => 2,
            Type::I32 | Type::U32 | Type::F32 => 4,
            Type::I64 | Type::U64 | Type::F64 | Type::Usize | Type::Isize | Type::Uintptr => 8,
            Type::Pointer(_) => 8,
            Type::String => 16, // { ptr: *i8, len: i64 }
            Type::Array(elem, len) => elem.size_of() * len,
            Type::Slice(_) => 24, // { ptr: *elem, len: i64, cap: i64 }
            Type::Struct(s) => s.total_size,
            Type::Function(_) => 8, // Function pointer
            Type::Named(_) => 8, // fallback / pointer width
        }
    }

    pub fn align_of(&self) -> usize {
        match self {
            Type::Void => 1,
            Type::Bool | Type::I8 | Type::U8 => 1,
            Type::I16 | Type::U16 => 2,
            Type::I32 | Type::U32 | Type::F32 => 4,
            Type::I64 | Type::U64 | Type::F64 | Type::Usize | Type::Isize | Type::Uintptr => 8,
            Type::Pointer(_) => 8,
            Type::String => 8,
            Type::Array(elem, _) => elem.align_of(),
            Type::Slice(_) => 8,
            Type::Struct(s) => s.alignment,
            Type::Function(_) => 8,
            Type::Named(_) => 8,
        }
    }

    pub fn to_llvm_type(&self) -> String {
        match self {
            Type::Void => "void".to_string(),
            Type::Bool => "i1".to_string(),
            Type::I8 | Type::U8 => "i8".to_string(),
            Type::I16 | Type::U16 => "i16".to_string(),
            Type::I32 | Type::U32 => "i32".to_string(),
            Type::I64 | Type::U64 | Type::Usize | Type::Isize | Type::Uintptr => "i64".to_string(),
            Type::F32 => "float".to_string(),
            Type::F64 => "double".to_string(),
            Type::Pointer(inner) => format!("{}*", inner.to_llvm_type()),
            Type::String => "{ i8*, i64 }".to_string(),
            Type::Array(elem, len) => format!("[{} x {}]", len, elem.to_llvm_type()),
            Type::Slice(elem) => format!("{{ {}*, i64, i64 }}", elem.to_llvm_type()),
            Type::Struct(s) => format!("%struct.{}", s.name),
            Type::Named(name) => format!("%struct.{}", name),
            Type::Function(f) => {
                let params_str: Vec<String> = f.params.iter().map(|p| p.to_llvm_type()).collect();
                format!("{} ({})*", f.ret_type.to_llvm_type(), params_str.join(", "))
            }
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Void => write!(f, "void"),
            Type::Bool => write!(f, "bool"),
            Type::I8 => write!(f, "i8"),
            Type::I16 => write!(f, "i16"),
            Type::I32 => write!(f, "i32"),
            Type::I64 => write!(f, "int"),
            Type::U8 => write!(f, "u8"),
            Type::U16 => write!(f, "u16"),
            Type::U32 => write!(f, "u32"),
            Type::U64 => write!(f, "uint"),
            Type::Usize => write!(f, "usize"),
            Type::Isize => write!(f, "isize"),
            Type::Uintptr => write!(f, "uintptr"),
            Type::F32 => write!(f, "f32"),
            Type::F64 => write!(f, "f64"),
            Type::String => write!(f, "string"),
            Type::Pointer(inner) => write!(f, "*{}", inner),
            Type::Array(inner, len) => write!(f, "[{}]{}", len, inner),
            Type::Slice(inner) => write!(f, "[]{}", inner),
            Type::Struct(s) => write!(f, "{}", s.name),
            Type::Named(name) => write!(f, "{}", name),
            Type::Function(func) => {
                let params: Vec<String> = func.params.iter().map(|p| p.to_string()).collect();
                write!(f, "func({}) {}", params.join(", "), func.ret_type)
            }
        }
    }
}

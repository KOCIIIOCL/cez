use crate::token::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum Attribute {
    Packed,
    Align(usize),
    Section(String),
    Naked,
    Interrupt,
    Export(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    BitClear,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    LogicalAnd,
    LogicalOr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,      // -
    Not,      // !
    BitNot,   // ^
    Deref,    // *
    AddrOf,   // &
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
    AndAssign,
    OrAssign,
    XorAssign,
    ShlAssign,
    ShrAssign,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamedFieldInit {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AsmOperandKind {
    In,
    Out,
    InOut,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AsmOperand {
    pub constraint: String,
    pub expr: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeNode {
    Named(String, Span),
    Pointer(Box<TypeNode>, Span),
    Array(Box<TypeNode>, usize, Span),
    Slice(Box<TypeNode>, Span),
}

impl TypeNode {
    pub fn span(&self) -> Span {
        match self {
            TypeNode::Named(_, s)
            | TypeNode::Pointer(_, s)
            | TypeNode::Array(_, _, s)
            | TypeNode::Slice(_, s) => *s,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    IntLit(i128, Span),
    FloatLit(f64, Span),
    StringLit(String, Span),
    CharLit(char, Span),
    BoolLit(bool, Span),
    Ident(String, Span),
    Binary(Box<Expr>, BinaryOp, Box<Expr>, Span),
    Unary(UnaryOp, Box<Expr>, Span),
    Call(Box<Expr>, Vec<Expr>, Span),
    MemberAccess(Box<Expr>, String, Span),
    Index(Box<Expr>, Box<Expr>, Span),
    Slice {
        expr: Box<Expr>,
        low: Option<Box<Expr>>,
        high: Option<Box<Expr>>,
        span: Span,
    },
    TypeCast(TypeNode, Box<Expr>, Span),
    StructLiteral {
        ty: TypeNode,
        fields: Vec<NamedFieldInit>,
        span: Span,
    },
    ArrayLiteral {
        ty: Option<TypeNode>,
        elements: Vec<Expr>,
        span: Span,
    },
    InlineAsm {
        asm_string: String,
        outputs: Vec<AsmOperand>,
        inputs: Vec<AsmOperand>,
        clobbers: Vec<String>,
        span: Span,
    },
    VolatileLoad {
        ptr: Box<Expr>,
        span: Span,
    },
    VolatileStore {
        ptr: Box<Expr>,
        val: Box<Expr>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::IntLit(_, s)
            | Expr::FloatLit(_, s)
            | Expr::StringLit(_, s)
            | Expr::CharLit(_, s)
            | Expr::BoolLit(_, s)
            | Expr::Ident(_, s)
            | Expr::Binary(_, _, _, s)
            | Expr::Unary(_, _, s)
            | Expr::Call(_, _, s)
            | Expr::MemberAccess(_, _, s)
            | Expr::Index(_, _, s)
            | Expr::Slice { span: s, .. }
            | Expr::TypeCast(_, _, s)
            | Expr::StructLiteral { span: s, .. }
            | Expr::ArrayLiteral { span: s, .. }
            | Expr::InlineAsm { span: s, .. }
            | Expr::VolatileLoad { span: s, .. }
            | Expr::VolatileStore { span: s, .. } => *s,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    pub values: Vec<Expr>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    VarDecl {
        name: String,
        ty: Option<TypeNode>,
        init: Option<Expr>,
        attributes: Vec<Attribute>,
        span: Span,
    },
    ConstDecl {
        name: String,
        ty: Option<TypeNode>,
        init: Expr,
        span: Span,
    },
    ShortVarDecl {
        name: String,
        init: Expr,
        span: Span,
    },
    Assign {
        target: Expr,
        op: AssignOp,
        value: Expr,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    Defer {
        call: Expr,
        span: Span,
    },
    If {
        init: Option<Box<Stmt>>,
        cond: Expr,
        then_block: Vec<Stmt>,
        else_block: Option<Vec<Stmt>>,
        span: Span,
    },
    For {
        init: Option<Box<Stmt>>,
        cond: Option<Expr>,
        post: Option<Box<Stmt>>,
        body: Vec<Stmt>,
        span: Span,
    },
    Switch {
        expr: Option<Expr>,
        cases: Vec<SwitchCase>,
        default_case: Option<Vec<Stmt>>,
        span: Span,
    },
    Break(Span),
    Continue(Span),
    Expr(Expr),
    Block(Vec<Stmt>, Span),
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub ty: TypeNode,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeDef {
    Struct {
        fields: Vec<StructField>,
        attributes: Vec<Attribute>,
    },
    Alias(TypeNode),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: TypeNode,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Receiver {
    pub name: String,
    pub ty: TypeNode,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncDecl {
    pub receiver: Option<Receiver>,
    pub name: String,
    pub params: Vec<Param>,
    pub ret_type: Option<TypeNode>,
    pub body: Option<Vec<Stmt>>,
    pub attributes: Vec<Attribute>,
    pub is_extern: bool,
    pub is_variadic: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    TypeDecl {
        name: String,
        def: TypeDef,
        span: Span,
    },
    Func(FuncDecl),
    GlobalVar {
        name: String,
        ty: Option<TypeNode>,
        init: Option<Expr>,
        attributes: Vec<Attribute>,
        span: Span,
    },
    Const {
        name: String,
        ty: Option<TypeNode>,
        init: Expr,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub package_name: String,
    pub imports: Vec<String>,
    pub decls: Vec<Decl>,
}

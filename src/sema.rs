use crate::ast::*;
use crate::token::Span;
use crate::types::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum TypedExprKind {
    IntLit(i128),
    FloatLit(f64),
    StringLit(String),
    CharLit(char),
    BoolLit(bool),
    LocalVar(String),
    GlobalVar(String),
    Binary(Box<TypedExpr>, BinaryOp, Box<TypedExpr>),
    Unary(UnaryOp, Box<TypedExpr>),
    Call {
        func_name: String,
        args: Vec<TypedExpr>,
    },
    MemberAccess {
        target: Box<TypedExpr>,
        field_name: String,
        field_index: usize,
    },
    Index {
        target: Box<TypedExpr>,
        index: Box<TypedExpr>,
    },
    Slice {
        target: Box<TypedExpr>,
        low: Option<Box<TypedExpr>>,
        high: Option<Box<TypedExpr>>,
    },
    Cast {
        expr: Box<TypedExpr>,
        target_ty: Type,
    },
    StructLiteral {
        struct_ty: StructType,
        fields: Vec<(usize, TypedExpr)>, // (field_index, value)
    },
    InlineAsm {
        asm_string: String,
        outputs: Vec<(String, TypedExpr)>,
        inputs: Vec<(String, TypedExpr)>,
        clobbers: Vec<String>,
    },
    VolatileLoad(Box<TypedExpr>),
    VolatileStore(Box<TypedExpr>, Box<TypedExpr>),
}

#[derive(Debug, Clone)]
pub struct TypedExpr {
    pub kind: TypedExprKind,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypedStmt {
    VarDecl {
        name: String,
        ty: Type,
        init: Option<TypedExpr>,
        span: Span,
    },
    Assign {
        target: TypedExpr,
        op: AssignOp,
        value: TypedExpr,
        span: Span,
    },
    Return {
        value: Option<TypedExpr>,
        span: Span,
    },
    Defer {
        stmt: Box<TypedStmt>,
        span: Span,
    },
    If {
        init: Option<Box<TypedStmt>>,
        cond: TypedExpr,
        then_block: Vec<TypedStmt>,
        else_block: Option<Vec<TypedStmt>>,
        span: Span,
    },
    For {
        init: Option<Box<TypedStmt>>,
        cond: Option<TypedExpr>,
        post: Option<Box<TypedStmt>>,
        body: Vec<TypedStmt>,
        span: Span,
    },
    Expr(TypedExpr),
    Block(Vec<TypedStmt>, Span),
    Break(Span),
    Continue(Span),
}

#[derive(Debug, Clone)]
pub struct TypedFuncDecl {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub ret_type: Type,
    pub body: Option<Vec<TypedStmt>>,
    pub attributes: Vec<Attribute>,
    pub is_extern: bool,
    pub is_variadic: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypedGlobalVar {
    pub name: String,
    pub ty: Type,
    pub init: Option<TypedExpr>,
    pub attributes: Vec<Attribute>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypedProgram {
    pub package_name: String,
    pub structs: HashMap<String, StructType>,
    pub globals: Vec<TypedGlobalVar>,
    pub funcs: Vec<TypedFuncDecl>,
}

pub struct Sema {
    types: HashMap<String, Type>,
    structs: HashMap<String, StructType>,
    methods: HashMap<(String, String), String>, // (Type, Method) -> MangledFunc
    funcs: HashMap<String, FuncType>,
    globals: HashMap<String, (Type, Vec<Attribute>)>,
    scopes: Vec<HashMap<String, Type>>,
    current_ret_type: Option<Type>,
}

impl Sema {
    pub fn new() -> Self {
        let mut types = HashMap::new();
        types.insert("void".to_string(), Type::Void);
        types.insert("bool".to_string(), Type::Bool);
        types.insert("i8".to_string(), Type::I8);
        types.insert("u8".to_string(), Type::U8);
        types.insert("byte".to_string(), Type::U8);
        types.insert("i16".to_string(), Type::I16);
        types.insert("u16".to_string(), Type::U16);
        types.insert("i32".to_string(), Type::I32);
        types.insert("u32".to_string(), Type::U32);
        types.insert("i64".to_string(), Type::I64);
        types.insert("u64".to_string(), Type::U64);
        types.insert("int".to_string(), Type::I64);
        types.insert("uint".to_string(), Type::U64);
        types.insert("usize".to_string(), Type::Usize);
        types.insert("isize".to_string(), Type::Isize);
        types.insert("uintptr".to_string(), Type::Uintptr);
        types.insert("f32".to_string(), Type::F32);
        types.insert("f64".to_string(), Type::F64);
        types.insert("string".to_string(), Type::String);

        let mut funcs = HashMap::new();
        funcs.insert(
            "printf".to_string(),
            FuncType {
                params: vec![Type::Pointer(Box::new(Type::I8))],
                ret_type: Box::new(Type::I32),
                is_variadic: true,
            },
        );
        funcs.insert(
            "puts".to_string(),
            FuncType {
                params: vec![Type::Pointer(Box::new(Type::I8))],
                ret_type: Box::new(Type::I32),
                is_variadic: false,
            },
        );
        funcs.insert(
            "exit".to_string(),
            FuncType {
                params: vec![Type::I32],
                ret_type: Box::new(Type::Void),
                is_variadic: false,
            },
        );
        funcs.insert(
            "malloc".to_string(),
            FuncType {
                params: vec![Type::Usize],
                ret_type: Box::new(Type::Pointer(Box::new(Type::U8))),
                is_variadic: false,
            },
        );
        funcs.insert(
            "free".to_string(),
            FuncType {
                params: vec![Type::Pointer(Box::new(Type::U8))],
                ret_type: Box::new(Type::Void),
                is_variadic: false,
            },
        );
        funcs.insert(
            "memcpy".to_string(),
            FuncType {
                params: vec![
                    Type::Pointer(Box::new(Type::U8)),
                    Type::Pointer(Box::new(Type::U8)),
                    Type::Usize,
                ],
                ret_type: Box::new(Type::Pointer(Box::new(Type::U8))),
                is_variadic: false,
            },
        );
        funcs.insert(
            "memset".to_string(),
            FuncType {
                params: vec![
                    Type::Pointer(Box::new(Type::U8)),
                    Type::I32,
                    Type::Usize,
                ],
                ret_type: Box::new(Type::Pointer(Box::new(Type::U8))),
                is_variadic: false,
            },
        );

        Self {
            types,
            structs: HashMap::new(),
            methods: HashMap::new(),
            funcs,
            globals: HashMap::new(),
            scopes: Vec::new(),
            current_ret_type: None,
        }
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn leave_scope(&mut self) {
        self.scopes.pop();
    }

    fn insert_local(&mut self, name: String, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, ty);
        }
    }

    fn lookup_var(&self, name: &str) -> Option<(Type, bool)> {
        // Search local scopes from innermost to outermost
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some((ty.clone(), false)); // false = local
            }
        }
        // Search globals
        if let Some((ty, _)) = self.globals.get(name) {
            return Some((ty.clone(), true)); // true = global
        }
        None
    }

    pub fn resolve_type_node(&self, node: &TypeNode) -> Result<Type, String> {
        match node {
            TypeNode::Named(name, span) => {
                if let Some(ty) = self.types.get(name) {
                    Ok(ty.clone())
                } else if let Some(st) = self.structs.get(name) {
                    Ok(Type::Struct(st.clone()))
                } else {
                    Err(format!("Unknown type '{}' at {}", name, span))
                }
            }
            TypeNode::Pointer(inner, _) => {
                let resolved = self.resolve_type_node(inner)?;
                Ok(Type::Pointer(Box::new(resolved)))
            }
            TypeNode::Array(inner, size, _) => {
                let resolved = self.resolve_type_node(inner)?;
                Ok(Type::Array(Box::new(resolved), *size))
            }
            TypeNode::Slice(inner, _) => {
                let resolved = self.resolve_type_node(inner)?;
                Ok(Type::Slice(Box::new(resolved)))
            }
        }
    }

    fn expr_to_type(&self, expr: &Expr) -> Option<Type> {
        match expr {
            Expr::Ident(name, _) => {
                if let Some(ty) = self.types.get(name) {
                    Some(ty.clone())
                } else if let Some(st) = self.structs.get(name) {
                    Some(Type::Struct(st.clone()))
                } else {
                    None
                }
            }
            Expr::Unary(UnaryOp::Deref, inner, _) => {
                let inner_ty = self.expr_to_type(inner)?;
                Some(Type::Pointer(Box::new(inner_ty)))
            }
            _ => None,
        }
    }

    pub fn analyze_program(&mut self, program: Program) -> Result<TypedProgram, String> {
        // Pass 1: Collect struct type declarations (skeletons)
        for decl in &program.decls {
            if let Decl::TypeDecl { name, def, span: _ } = decl {
                match def {
                    TypeDef::Struct { fields: _, attributes } => {
                        let is_packed = attributes.contains(&Attribute::Packed);
                        let mut explicit_align = None;
                        for attr in attributes {
                            if let Attribute::Align(n) = attr {
                                explicit_align = Some(*n);
                            }
                        }

                        let struct_placeholder = StructType {
                            name: name.clone(),
                            fields: Vec::new(),
                            is_packed,
                            explicit_align,
                            total_size: 0,
                            alignment: explicit_align.unwrap_or(1),
                        };
                        self.structs.insert(name.clone(), struct_placeholder.clone());
                        self.types.insert(name.clone(), Type::Struct(struct_placeholder));
                    }
                    TypeDef::Alias(_) => {}
                }
            }
        }

        // Pass 2: Resolve struct field types and compute layouts iteratively
        for _ in 0..16 {
            let mut changed = false;
            for decl in &program.decls {
                if let Decl::TypeDecl { name, def, span: _ } = decl {
                    match def {
                        TypeDef::Struct { fields, attributes } => {
                            let is_packed = attributes.contains(&Attribute::Packed);
                            let mut explicit_align = None;
                            for attr in attributes {
                                if let Attribute::Align(n) = attr {
                                    explicit_align = Some(*n);
                                }
                            }

                            let mut resolved_fields = Vec::new();
                            let mut current_offset = 0;
                            let mut max_align = explicit_align.unwrap_or(1);

                            for f in fields {
                                let field_ty = self.resolve_type_node(&f.ty)?;
                                let f_align = if is_packed { 1 } else { field_ty.align_of() };
                                if f_align > max_align {
                                    max_align = f_align;
                                }

                                if !is_packed && f_align > 0 {
                                    let padding = (f_align - (current_offset % f_align)) % f_align;
                                    current_offset += padding;
                                }

                                let offset = current_offset;
                                current_offset += field_ty.size_of();

                                resolved_fields.push(StructFieldType {
                                    name: f.name.clone(),
                                    ty: field_ty,
                                    offset,
                                });
                            }

                            let total_size = if !is_packed && max_align > 0 {
                                let pad = (max_align - (current_offset % max_align)) % max_align;
                                current_offset + pad
                            } else {
                                current_offset
                            };

                            let prev_size = self.structs.get(name).map(|s| s.total_size).unwrap_or(0);
                            let prev_field_count = self.structs.get(name).map(|s| s.fields.len()).unwrap_or(0);
                            if prev_size != total_size || prev_field_count != resolved_fields.len() {
                                changed = true;
                            }

                            let finalized = StructType {
                                name: name.clone(),
                                fields: resolved_fields,
                                is_packed,
                                explicit_align,
                                total_size,
                                alignment: max_align,
                            };

                            self.structs.insert(name.clone(), finalized.clone());
                            self.types.insert(name.clone(), Type::Struct(finalized));
                        }
                        TypeDef::Alias(ty_node) => {
                            let resolved = self.resolve_type_node(ty_node)?;
                            self.types.insert(name.clone(), resolved);
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }

        // Pass 3: Collect global variables, constants, and function signatures
        for decl in &program.decls {
            match decl {
                Decl::GlobalVar {
                    name,
                    ty,
                    init: _,
                    attributes,
                    span,
                } => {
                    let resolved_ty = if let Some(t) = ty {
                        self.resolve_type_node(t)?
                    } else {
                        return Err(format!("Global variable '{}' requires explicit type at {}", name, span));
                    };
                    self.globals.insert(name.clone(), (resolved_ty.clone(), attributes.clone()));
                }
                Decl::Const { name, ty, init: _, span: _ } => {
                    let resolved_ty = if let Some(t) = ty {
                        self.resolve_type_node(t)?
                    } else {
                        Type::I64
                    };
                    self.globals.insert(name.clone(), (resolved_ty, Vec::new()));
                }
                Decl::Func(f) => {
                    let ret_type = if let Some(ref r) = f.ret_type {
                        self.resolve_type_node(r)?
                    } else {
                        Type::Void
                    };

                    let mut param_types = Vec::new();
                    if let Some(ref recv) = f.receiver {
                        let recv_ty = self.resolve_type_node(&recv.ty)?;
                        param_types.push(recv_ty);
                    }
                    for p in &f.params {
                        param_types.push(self.resolve_type_node(&p.ty)?);
                    }

                    let func_mangled = if let Some(ref recv) = f.receiver {
                        let recv_ty = self.resolve_type_node(&recv.ty)?;
                        let type_name = match &recv_ty {
                            Type::Pointer(inner) => match &**inner {
                                Type::Struct(s) => s.name.clone(),
                                Type::Named(n) => n.clone(),
                                _ => "Recv".to_string(),
                            },
                            Type::Struct(s) => s.name.clone(),
                            Type::Named(n) => n.clone(),
                            _ => "Recv".to_string(),
                        };
                        let mangled = format!("{}_{}", type_name, f.name);
                        self.methods.insert((type_name, f.name.clone()), mangled.clone());
                        mangled
                    } else {
                        f.name.clone()
                    };

                    self.funcs.insert(
                        func_mangled,
                        FuncType {
                            params: param_types,
                            ret_type: Box::new(ret_type),
                            is_variadic: f.is_variadic,
                        },
                    );
                }
                Decl::TypeDecl { .. } => {}
            }
        }

        // Pass 4: Typecheck globals and function bodies
        let mut typed_globals = Vec::new();
        let mut typed_funcs = Vec::new();

        for decl in program.decls {
            match decl {
                Decl::GlobalVar {
                    name,
                    ty,
                    init,
                    attributes,
                    span,
                } => {
                    let ty = self.resolve_type_node(&ty.unwrap())?;
                    let typed_init = if let Some(init_expr) = init {
                        let mut typed_expr = self.check_expr(init_expr)?;
                        if typed_expr.ty != ty && !self.can_coerce(&typed_expr.ty, &ty) {
                            return Err(format!(
                                "Cannot initialize global '{}' of type '{}' with expression of type '{}' at {}",
                                name, ty, typed_expr.ty, span
                            ));
                        }
                        if typed_expr.ty != ty {
                            typed_expr = self.coerce_expr(typed_expr, ty.clone());
                        }
                        Some(typed_expr)
                    } else {
                        None
                    };

                    typed_globals.push(TypedGlobalVar {
                        name,
                        ty,
                        init: typed_init,
                        attributes,
                        span,
                    });
                }
                Decl::Const { name, ty, init, span } => {
                    let ty = if let Some(t) = ty {
                        self.resolve_type_node(&t)?
                    } else {
                        Type::I64
                    };
                    let typed_init = self.check_expr(init)?;
                    typed_globals.push(TypedGlobalVar {
                        name,
                        ty,
                        init: Some(typed_init),
                        attributes: Vec::new(),
                        span,
                    });
                }
                Decl::Func(f) => {
                    let typed_func = self.check_func(f)?;
                    typed_funcs.push(typed_func);
                }
                _ => {}
            }
        }

        Ok(TypedProgram {
            package_name: program.package_name,
            structs: self.structs.clone(),
            globals: typed_globals,
            funcs: typed_funcs,
        })
    }

    fn check_func(&mut self, f: FuncDecl) -> Result<TypedFuncDecl, String> {
        let ret_type = if let Some(ref r) = f.ret_type {
            self.resolve_type_node(r)?
        } else {
            Type::Void
        };

        let mut params = Vec::new();
        let func_name = if let Some(ref recv) = f.receiver {
            let recv_ty = self.resolve_type_node(&recv.ty)?;
            let type_name = match &recv_ty {
                Type::Pointer(inner) => match &**inner {
                    Type::Struct(s) => s.name.clone(),
                    Type::Named(n) => n.clone(),
                    _ => "Recv".to_string(),
                },
                Type::Struct(s) => s.name.clone(),
                Type::Named(n) => n.clone(),
                _ => "Recv".to_string(),
            };
            params.push((recv.name.clone(), recv_ty));
            format!("{}_{}", type_name, f.name)
        } else {
            f.name.clone()
        };

        for p in &f.params {
            let p_ty = self.resolve_type_node(&p.ty)?;
            params.push((p.name.clone(), p_ty));
        }

        self.enter_scope();
        for (name, ty) in &params {
            self.insert_local(name.clone(), ty.clone());
        }

        self.current_ret_type = Some(ret_type.clone());

        let typed_body = if let Some(body_stmts) = f.body {
            let mut stmts = Vec::new();
            for stmt in body_stmts {
                stmts.push(self.check_stmt(stmt)?);
            }
            Some(stmts)
        } else {
            None
        };

        self.current_ret_type = None;
        self.leave_scope();

        Ok(TypedFuncDecl {
            name: func_name,
            params,
            ret_type,
            body: typed_body,
            attributes: f.attributes,
            is_extern: f.is_extern,
            is_variadic: f.is_variadic,
            span: f.span,
        })
    }

    fn check_stmt(&mut self, stmt: Stmt) -> Result<TypedStmt, String> {
        match stmt {
            Stmt::VarDecl {
                name,
                ty,
                init,
                attributes: _,
                span,
            } => {
                let explicit_ty = if let Some(ref t) = ty {
                    Some(self.resolve_type_node(t)?)
                } else {
                    None
                };

                let typed_init = if let Some(init_expr) = init {
                    let expr = self.check_expr(init_expr)?;
                    Some(expr)
                } else {
                    None
                };

                let (final_ty, final_init) = match (explicit_ty, typed_init) {
                    (Some(t), Some(mut init)) => {
                        if t != init.ty && !self.can_coerce(&init.ty, &t) {
                            return Err(format!(
                                "Cannot assign value of type '{}' to variable '{}' of type '{}' at {}",
                                init.ty, name, t, span
                            ));
                        }
                        if t != init.ty {
                            init = self.coerce_expr(init, t.clone());
                        }
                        (t, Some(init))
                    }
                    (Some(t), None) => (t, None),
                    (None, Some(init)) => {
                        let ty = init.ty.clone();
                        (ty, Some(init))
                    }
                    (None, None) => {
                        return Err(format!("Variable '{}' requires either a type or an initializer at {}", name, span))
                    }
                };

                self.insert_local(name.clone(), final_ty.clone());

                Ok(TypedStmt::VarDecl {
                    name,
                    ty: final_ty,
                    init: final_init,
                    span,
                })
            }
            Stmt::ShortVarDecl { name, init, span } => {
                let typed_init = self.check_expr(init)?;
                let ty = typed_init.ty.clone();
                self.insert_local(name.clone(), ty.clone());
                Ok(TypedStmt::VarDecl {
                    name,
                    ty,
                    init: Some(typed_init),
                    span,
                })
            }
            Stmt::Assign {
                target,
                op,
                value,
                span,
            } => {
                let typed_target = self.check_expr(target)?;
                let mut typed_val = self.check_expr(value)?;

                if typed_target.ty != typed_val.ty && !self.can_coerce(&typed_val.ty, &typed_target.ty) {
                    return Err(format!(
                        "Cannot assign type '{}' to type '{}' at {}",
                        typed_val.ty, typed_target.ty, span
                    ));
                }
                if typed_target.ty != typed_val.ty {
                    typed_val = self.coerce_expr(typed_val, typed_target.ty.clone());
                }

                Ok(TypedStmt::Assign {
                    target: typed_target,
                    op,
                    value: typed_val,
                    span,
                })
            }
            Stmt::Return { value, span } => {
                let expected_ret = self.current_ret_type.clone().unwrap_or(Type::Void);
                let typed_val = if let Some(v) = value {
                    let mut expr = self.check_expr(v)?;
                    if expr.ty != expected_ret && !self.can_coerce(&expr.ty, &expected_ret) {
                        return Err(format!(
                            "Function returns '{}' but got '{}' at {}",
                            expected_ret, expr.ty, span
                        ));
                    }
                    if expr.ty != expected_ret {
                        expr = self.coerce_expr(expr, expected_ret.clone());
                    }
                    Some(expr)
                } else {
                    if expected_ret != Type::Void {
                        return Err(format!("Empty return in function returning '{}' at {}", expected_ret, span));
                    }
                    None
                };

                Ok(TypedStmt::Return {
                    value: typed_val,
                    span,
                })
            }
            Stmt::Defer { call, span } => {
                let typed_expr = self.check_expr(call)?;
                Ok(TypedStmt::Defer {
                    stmt: Box::new(TypedStmt::Expr(typed_expr)),
                    span,
                })
            }
            Stmt::If {
                init,
                cond,
                then_block,
                else_block,
                span,
            } => {
                self.enter_scope();
                let typed_init = if let Some(init_stmt) = init {
                    Some(Box::new(self.check_stmt(*init_stmt)?))
                } else {
                    None
                };

                let typed_cond = self.check_expr(cond)?;
                if typed_cond.ty != Type::Bool {
                    return Err(format!("If condition must be bool, got '{}' at {}", typed_cond.ty, span));
                }

                let mut typed_then = Vec::new();
                for s in then_block {
                    typed_then.push(self.check_stmt(s)?);
                }

                let typed_else = if let Some(else_stmts) = else_block {
                    let mut elses = Vec::new();
                    for s in else_stmts {
                        elses.push(self.check_stmt(s)?);
                    }
                    Some(elses)
                } else {
                    None
                };

                self.leave_scope();

                Ok(TypedStmt::If {
                    init: typed_init,
                    cond: typed_cond,
                    then_block: typed_then,
                    else_block: typed_else,
                    span,
                })
            }
            Stmt::For {
                init,
                cond,
                post,
                body,
                span,
            } => {
                self.enter_scope();
                let typed_init = if let Some(i) = init {
                    Some(Box::new(self.check_stmt(*i)?))
                } else {
                    None
                };

                let typed_cond = if let Some(c) = cond {
                    let tc = self.check_expr(c)?;
                    if tc.ty != Type::Bool {
                        return Err(format!("For loop condition must be bool, got '{}' at {}", tc.ty, span));
                    }
                    Some(tc)
                } else {
                    None
                };

                let typed_post = if let Some(p) = post {
                    Some(Box::new(self.check_stmt(*p)?))
                } else {
                    None
                };

                let mut typed_body = Vec::new();
                for s in body {
                    typed_body.push(self.check_stmt(s)?);
                }

                self.leave_scope();

                Ok(TypedStmt::For {
                    init: typed_init,
                    cond: typed_cond,
                    post: typed_post,
                    body: typed_body,
                    span,
                })
            }
            Stmt::Switch {
                expr,
                cases,
                default_case,
                span,
            } => {
                // Desugar switch into if-else chain or handle directly
                // We'll desugar `switch x { case 1: ... case 2: ... default: ... }` into if-else
                let typed_expr = if let Some(e) = expr {
                    Some(self.check_expr(e)?)
                } else {
                    None
                };

                // Desugar cases to if/else
                let mut current_else: Option<Vec<TypedStmt>> = None;
                if let Some(def_body) = default_case {
                    let mut b = Vec::new();
                    for s in def_body {
                        b.push(self.check_stmt(s)?);
                    }
                    current_else = Some(b);
                }

                for case in cases.into_iter().rev() {
                    let mut conds = Vec::new();
                    for val in case.values {
                        let t_val = self.check_expr(val)?;
                        let cond = if let Some(ref switch_target) = typed_expr {
                            TypedExpr {
                                kind: TypedExprKind::Binary(
                                    Box::new(switch_target.clone()),
                                    BinaryOp::Eq,
                                    Box::new(t_val),
                                ),
                                ty: Type::Bool,
                                span: case.span,
                            }
                        } else {
                            t_val
                        };
                        conds.push(cond);
                    }

                    let mut combined_cond = conds.remove(0);
                    for c in conds {
                        combined_cond = TypedExpr {
                            kind: TypedExprKind::Binary(
                                Box::new(combined_cond),
                                BinaryOp::LogicalOr,
                                Box::new(c),
                            ),
                            ty: Type::Bool,
                            span: case.span,
                        };
                    }

                    let mut case_body = Vec::new();
                    for s in case.body {
                        case_body.push(self.check_stmt(s)?);
                    }

                    let if_stmt = TypedStmt::If {
                        init: None,
                        cond: combined_cond,
                        then_block: case_body,
                        else_block: current_else,
                        span: case.span,
                    };

                    current_else = Some(vec![if_stmt]);
                }

                if let Some(chain) = current_else {
                    Ok(TypedStmt::Block(chain, span))
                } else {
                    Ok(TypedStmt::Block(Vec::new(), span))
                }
            }
            Stmt::Break(span) => Ok(TypedStmt::Break(span)),
            Stmt::Continue(span) => Ok(TypedStmt::Continue(span)),
            Stmt::Expr(expr) => {
                let typed_expr = self.check_expr(expr)?;
                Ok(TypedStmt::Expr(typed_expr))
            }
            Stmt::Block(stmts, span) => {
                self.enter_scope();
                let mut typed_stmts = Vec::new();
                for s in stmts {
                    typed_stmts.push(self.check_stmt(s)?);
                }
                self.leave_scope();
                Ok(TypedStmt::Block(typed_stmts, span))
            }
            Stmt::ConstDecl {
                name,
                ty,
                init,
                span,
            } => {
                let init_expr = self.check_expr(init)?;
                let final_ty = if let Some(t) = ty {
                    self.resolve_type_node(&t)?
                } else {
                    init_expr.ty.clone()
                };
                self.insert_local(name.clone(), final_ty.clone());
                Ok(TypedStmt::VarDecl {
                    name,
                    ty: final_ty,
                    init: Some(init_expr),
                    span,
                })
            }
        }
    }

    pub fn check_expr(&mut self, expr: Expr) -> Result<TypedExpr, String> {
        let span = expr.span();
        match expr {
            Expr::IntLit(val, s) => {
                // Default integer literal type is int (i64)
                Ok(TypedExpr {
                    kind: TypedExprKind::IntLit(val),
                    ty: Type::I64,
                    span: s,
                })
            }
            Expr::FloatLit(val, s) => Ok(TypedExpr {
                kind: TypedExprKind::FloatLit(val),
                ty: Type::F64,
                span: s,
            }),
            Expr::StringLit(val, s) => Ok(TypedExpr {
                kind: TypedExprKind::StringLit(val),
                ty: Type::String,
                span: s,
            }),
            Expr::CharLit(val, s) => Ok(TypedExpr {
                kind: TypedExprKind::CharLit(val),
                ty: Type::U8,
                span: s,
            }),
            Expr::BoolLit(val, s) => Ok(TypedExpr {
                kind: TypedExprKind::BoolLit(val),
                ty: Type::Bool,
                span: s,
            }),
            Expr::Ident(name, s) => {
                if let Some((ty, is_global)) = self.lookup_var(&name) {
                    let kind = if is_global {
                        TypedExprKind::GlobalVar(name)
                    } else {
                        TypedExprKind::LocalVar(name)
                    };
                    Ok(TypedExpr { kind, ty, span: s })
                } else if self.funcs.contains_key(&name) {
                    let f = self.funcs.get(&name).unwrap();
                    Ok(TypedExpr {
                        kind: TypedExprKind::GlobalVar(name),
                        ty: Type::Function(f.clone()),
                        span: s,
                    })
                } else {
                    Err(format!("Undefined variable or identifier '{}' at {}", name, s))
                }
            }
            Expr::Binary(lhs, op, rhs, s) => {
                let mut left = self.check_expr(*lhs)?;
                let mut right = self.check_expr(*rhs)?;

                // Numeric coercion for literals
                if left.ty != right.ty {
                    if self.can_coerce(&left.ty, &right.ty) {
                        left = self.coerce_expr(left, right.ty.clone());
                    } else if self.can_coerce(&right.ty, &left.ty) {
                        right = self.coerce_expr(right, left.ty.clone());
                    } else {
                        return Err(format!(
                            "Mismatched types in binary operation {:?}: '{}' and '{}' at {}",
                            op, left.ty, right.ty, s
                        ));
                    }
                }

                let ty = match op {
                    BinaryOp::Eq
                    | BinaryOp::NotEq
                    | BinaryOp::Lt
                    | BinaryOp::LtEq
                    | BinaryOp::Gt
                    | BinaryOp::GtEq
                    | BinaryOp::LogicalAnd
                    | BinaryOp::LogicalOr => Type::Bool,
                    _ => left.ty.clone(),
                };

                Ok(TypedExpr {
                    kind: TypedExprKind::Binary(Box::new(left), op, Box::new(right)),
                    ty,
                    span: s,
                })
            }
            Expr::Unary(op, inner, s) => {
                let typed_inner = self.check_expr(*inner)?;
                match op {
                    UnaryOp::AddrOf => {
                        let ptr_ty = Type::Pointer(Box::new(typed_inner.ty.clone()));
                        Ok(TypedExpr {
                            kind: TypedExprKind::Unary(UnaryOp::AddrOf, Box::new(typed_inner)),
                            ty: ptr_ty,
                            span: s,
                        })
                    }
                    UnaryOp::Deref => {
                        if let Type::Pointer(elem_ty) = typed_inner.ty.clone() {
                            Ok(TypedExpr {
                                kind: TypedExprKind::Unary(UnaryOp::Deref, Box::new(typed_inner)),
                                ty: *elem_ty,
                                span: s,
                            })
                        } else {
                            Err(format!("Cannot dereference non-pointer type '{}' at {}", typed_inner.ty, s))
                        }
                    }
                    UnaryOp::Not => {
                        if typed_inner.ty != Type::Bool {
                            return Err(format!("Cannot apply '!' to non-boolean type '{}' at {}", typed_inner.ty, s));
                        }
                        Ok(TypedExpr {
                            kind: TypedExprKind::Unary(UnaryOp::Not, Box::new(typed_inner)),
                            ty: Type::Bool,
                            span: s,
                        })
                    }
                    UnaryOp::Neg | UnaryOp::BitNot => {
                        let ty = typed_inner.ty.clone();
                        Ok(TypedExpr {
                            kind: TypedExprKind::Unary(op, Box::new(typed_inner)),
                            ty,
                            span: s,
                        })
                    }
                }
            }
            Expr::Call(callee, args, s) => {
                // Check if callee is a type cast: e.g. `uintptr(ptr)`, `u32(x)`, `(*u16)(x)`
                if let Some(target_ty) = self.expr_to_type(&callee) {
                    if args.len() == 1 {
                        let arg = self.check_expr(args.into_iter().next().unwrap())?;
                        return Ok(TypedExpr {
                            kind: TypedExprKind::Cast {
                                expr: Box::new(arg),
                                target_ty: target_ty.clone(),
                            },
                            ty: target_ty,
                            span: s,
                        });
                    }
                }

                // Check if callee is a package call or a method call: `recv.Method(...)`
                if let Expr::MemberAccess(ref target, ref method_name, _) = *callee {
                    if let Expr::Ident(ref pkg_name, _) = **target {
                        if self.lookup_var(pkg_name).is_none() {
                            let candidate_names = [
                                format!("{}_{}", pkg_name, method_name),
                                method_name.clone(),
                                if pkg_name == "fmt" && method_name == "Printf" { "printf".to_string() } else { "".to_string() },
                                if pkg_name == "fmt" && method_name == "Println" { "Println".to_string() } else { "".to_string() },
                                if pkg_name == "fmt" && method_name == "Print" { "Print".to_string() } else { "".to_string() },
                                if pkg_name == "fmt" && (method_name == "Scanf" || method_name == "Scan" || method_name == "Scanln") {
                                    if self.funcs.contains_key("fmt_Scanln") {
                                        "fmt_Scanln".to_string()
                                    } else if self.funcs.contains_key("Scanln") {
                                        "Scanln".to_string()
                                    } else if self.funcs.contains_key("scanf") {
                                        "scanf".to_string()
                                    } else {
                                        "".to_string()
                                    }
                                } else {
                                    "".to_string()
                                },
                                if pkg_name == "os" && method_name == "Exit" { "exit".to_string() } else { "".to_string() },
                            ];

                            let mut resolved_func = None;
                            for cand in &candidate_names {
                                if !cand.is_empty() && self.funcs.contains_key(cand) {
                                    resolved_func = Some(cand.clone());
                                    break;
                                }
                            }
                            if resolved_func.is_none() && pkg_name == "fmt" && method_name == "Println" {
                                if self.funcs.contains_key("puts") {
                                    resolved_func = Some("puts".to_string());
                                }
                            }

                            if let Some(func_name) = resolved_func {
                                let func_sig = self.funcs.get(&func_name).cloned().unwrap();
                                if func_sig.is_variadic {
                                    if args.len() < func_sig.params.len() {
                                        return Err(format!(
                                            "Variadic function '{}.{}' expects at least {} arguments, but got {} at {}",
                                            pkg_name, method_name, func_sig.params.len(), args.len(), s
                                        ));
                                    }
                                } else if func_sig.params.len() != args.len() {
                                    return Err(format!(
                                        "Function '{}.{}' expects {} arguments, but got {} at {}",
                                        pkg_name, method_name, func_sig.params.len(), args.len(), s
                                    ));
                                }

                                let mut checked_args = Vec::new();
                                for (i, arg_expr) in args.into_iter().enumerate() {
                                    let mut a = self.check_expr(arg_expr)?;
                                    if i < func_sig.params.len() {
                                        let param_ty = &func_sig.params[i];
                                        if a.ty != *param_ty {
                                            if self.can_coerce(&a.ty, param_ty) {
                                                a = self.coerce_expr(a, param_ty.clone());
                                            } else {
                                                return Err(format!(
                                                    "Argument of type '{}' does not match expected parameter type '{}' in call to '{}.{}' at {}",
                                                    a.ty, param_ty, pkg_name, method_name, s
                                                ));
                                            }
                                        }
                                    } else if a.ty == Type::String {
                                        a = self.coerce_expr(a, Type::Pointer(Box::new(Type::I8)));
                                    } else if a.ty == Type::I8 || a.ty == Type::I16 {
                                        a = self.coerce_expr(a, Type::I32);
                                    } else if a.ty == Type::U8 || a.ty == Type::U16 {
                                        a = self.coerce_expr(a, Type::U32);
                                    } else if a.ty == Type::F32 {
                                        a = self.coerce_expr(a, Type::F64);
                                    } else {
                                        let elem_opt = match a.ty {
                                            Type::Pointer(ref inner) => match **inner {
                                                Type::Array(ref elem, _) => Some(elem.clone()),
                                                _ => None,
                                            },
                                            _ => None,
                                        };
                                        if let Some(elem) = elem_opt {
                                            a = self.coerce_expr(a, Type::Pointer(elem));
                                        }
                                    }
                                    checked_args.push(a);
                                }

                                // Universal %d support: rewrite %d to %lld for 64-bit integer arguments in Printf
                                if (func_name == "printf" || func_name == "fmt_Printf" || (pkg_name == "fmt" && method_name == "Printf"))
                                    && !checked_args.is_empty()
                                {
                                    self.rewrite_printf_format(&mut checked_args);
                                }

                                return Ok(TypedExpr {
                                    kind: TypedExprKind::Call {
                                        func_name,
                                        args: checked_args,
                                    },
                                    ty: *func_sig.ret_type,
                                    span: s,
                                });
                            }
                        }
                    }

                    // 2. Struct method call
                    let typed_target = self.check_expr(*target.clone())?;
                    let (type_name, is_ptr) = match &typed_target.ty {
                        Type::Pointer(inner) => match &**inner {
                            Type::Struct(st) => (st.name.clone(), true),
                            Type::Named(n) => (n.clone(), true),
                            _ => ("".to_string(), true),
                        },
                        Type::Struct(st) => (st.name.clone(), false),
                        Type::Named(n) => (n.clone(), false),
                        _ => ("".to_string(), false),
                    };

                    if let Some(mangled_name) = self.methods.get(&(type_name.clone(), method_name.clone())).cloned() {
                        let func_sig = self.funcs.get(&mangled_name).cloned().ok_or_else(|| {
                            format!("Method '{}' found but missing function signature at {}", mangled_name, s)
                        })?;

                        // First parameter is the receiver
                        let mut call_args = Vec::new();
                        let expected_recv_ty = &func_sig.params[0];

                        // Auto-address-of or auto-deref if needed
                        let recv_arg = if expected_recv_ty.is_pointer() && !is_ptr {
                            let recv_ty = typed_target.ty.clone();
                            TypedExpr {
                                kind: TypedExprKind::Unary(UnaryOp::AddrOf, Box::new(typed_target)),
                                ty: Type::Pointer(Box::new(recv_ty)),
                                span: s,
                            }
                        } else {
                            typed_target
                        };

                        call_args.push(recv_arg);

                        for arg in args {
                            call_args.push(self.check_expr(arg)?);
                        }

                        return Ok(TypedExpr {
                            kind: TypedExprKind::Call {
                                func_name: mangled_name,
                                args: call_args,
                            },
                            ty: *func_sig.ret_type,
                            span: s,
                        });
                    }
                }

                // Regular function call
                let func_name = match *callee {
                    Expr::Ident(ref name, _) => name.clone(),
                    _ => return Err(format!("Indirect function calls not yet supported at {}", s)),
                };

                let func_sig = self.funcs.get(&func_name).cloned().ok_or_else(|| {
                    format!("Call to undefined function '{}' at {}", func_name, s)
                })?;

                if func_sig.is_variadic {
                    if args.len() < func_sig.params.len() {
                        return Err(format!(
                            "Variadic function '{}' expects at least {} arguments, but got {} at {}",
                            func_name,
                            func_sig.params.len(),
                            args.len(),
                            s
                        ));
                    }
                } else if func_sig.params.len() != args.len() {
                    return Err(format!(
                        "Function '{}' expects {} arguments, but got {} at {}",
                        func_name,
                        func_sig.params.len(),
                        args.len(),
                        s
                    ));
                }

                let mut checked_args = Vec::new();
                for (i, arg_expr) in args.into_iter().enumerate() {
                    let mut a = self.check_expr(arg_expr)?;
                    if i < func_sig.params.len() {
                        let param_ty = &func_sig.params[i];
                        if a.ty != *param_ty {
                            if self.can_coerce(&a.ty, param_ty) {
                                a = self.coerce_expr(a, param_ty.clone());
                            } else {
                                return Err(format!(
                                    "Argument of type '{}' does not match expected parameter type '{}' in call to '{}' at {}",
                                    a.ty, param_ty, func_name, s
                                ));
                            }
                        }
                    } else if a.ty == Type::String {
                        a = self.coerce_expr(a, Type::Pointer(Box::new(Type::I8)));
                    } else if a.ty == Type::I8 || a.ty == Type::I16 {
                        a = self.coerce_expr(a, Type::I32);
                    } else if a.ty == Type::U8 || a.ty == Type::U16 {
                        a = self.coerce_expr(a, Type::U32);
                    } else if a.ty == Type::F32 {
                        a = self.coerce_expr(a, Type::F64);
                    } else {
                        let elem_opt = match a.ty {
                            Type::Pointer(ref inner) => match **inner {
                                Type::Array(ref elem, _) => Some(elem.clone()),
                                _ => None,
                            },
                            _ => None,
                        };
                        if let Some(elem) = elem_opt {
                            a = self.coerce_expr(a, Type::Pointer(elem));
                        }
                    }
                    checked_args.push(a);
                }

                // Universal %d support: rewrite %d to %lld for 64-bit integer arguments in printf
                if (func_name == "printf" || func_name == "fmt_Printf") && !checked_args.is_empty() {
                    self.rewrite_printf_format(&mut checked_args);
                }

                Ok(TypedExpr {
                    kind: TypedExprKind::Call {
                        func_name,
                        args: checked_args,
                    },
                    ty: *func_sig.ret_type,
                    span: s,
                })
            }
            Expr::MemberAccess(target, field_name, s) => {
                let typed_target = self.check_expr(*target)?;
                let (struct_name, is_pointer) = match &typed_target.ty {
                    Type::Struct(st) => (st.name.clone(), false),
                    Type::Pointer(inner) => match &**inner {
                        Type::Struct(st) => (st.name.clone(), true),
                        _ => return Err(format!("Cannot access member on pointer to non-struct at {}", s)),
                    },
                    _ => return Err(format!("Cannot access member on non-struct type '{}' at {}", typed_target.ty, s)),
                };

                let struct_def = self.structs.get(&struct_name).cloned().ok_or_else(|| {
                    format!("Struct '{}' not found at {}", struct_name, s)
                })?;

                let (idx, field_ty) = struct_def
                    .fields
                    .iter()
                    .enumerate()
                    .find(|(_, f)| f.name == field_name)
                    .map(|(i, f)| (i, f.ty.clone()))
                    .ok_or_else(|| format!("Struct '{}' has no field '{}' at {}", struct_def.name, field_name, s))?;

                let adjusted_target = if is_pointer {
                    // Auto-deref
                    TypedExpr {
                        kind: TypedExprKind::Unary(UnaryOp::Deref, Box::new(typed_target)),
                        ty: Type::Struct(struct_def),
                        span: s,
                    }
                } else {
                    typed_target
                };

                Ok(TypedExpr {
                    kind: TypedExprKind::MemberAccess {
                        target: Box::new(adjusted_target),
                        field_name,
                        field_index: idx,
                    },
                    ty: field_ty,
                    span: s,
                })
            }
            Expr::Index(target, index, s) => {
                let typed_target = self.check_expr(*target)?;
                let typed_index = self.check_expr(*index)?;

                if !typed_index.ty.is_integer() {
                    return Err(format!("Index must be integer, got '{}' at {}", typed_index.ty, s));
                }

                let elem_ty = match &typed_target.ty {
                    Type::Array(inner, _) => *inner.clone(),
                    Type::Slice(inner) => *inner.clone(),
                    Type::Pointer(inner) => *inner.clone(),
                    _ => return Err(format!("Cannot index non-array/slice/pointer type '{}' at {}", typed_target.ty, s)),
                };

                Ok(TypedExpr {
                    kind: TypedExprKind::Index {
                        target: Box::new(typed_target),
                        index: Box::new(typed_index),
                    },
                    ty: elem_ty,
                    span: s,
                })
            }
            Expr::Slice {
                expr,
                low,
                high,
                span: s,
            } => {
                let typed_target = self.check_expr(*expr)?;
                let typed_low = if let Some(l) = low {
                    Some(Box::new(self.check_expr(*l)?))
                } else {
                    None
                };
                let typed_high = if let Some(h) = high {
                    Some(Box::new(self.check_expr(*h)?))
                } else {
                    None
                };

                let elem_ty = match &typed_target.ty {
                    Type::Array(inner, _) => *inner.clone(),
                    Type::Slice(inner) => *inner.clone(),
                    Type::Pointer(inner) => *inner.clone(),
                    _ => return Err(format!("Cannot slice non-indexable type '{}' at {}", typed_target.ty, s)),
                };

                Ok(TypedExpr {
                    kind: TypedExprKind::Slice {
                        target: Box::new(typed_target),
                        low: typed_low,
                        high: typed_high,
                    },
                    ty: Type::Slice(Box::new(elem_ty)),
                    span: s,
                })
            }
            Expr::TypeCast(ty_node, inner, s) => {
                let target_ty = self.resolve_type_node(&ty_node)?;
                let typed_inner = self.check_expr(*inner)?;

                Ok(TypedExpr {
                    kind: TypedExprKind::Cast {
                        expr: Box::new(typed_inner),
                        target_ty: target_ty.clone(),
                    },
                    ty: target_ty,
                    span: s,
                })
            }
            Expr::StructLiteral { ty, fields, span: s } => {
                let struct_ty = match self.resolve_type_node(&ty)? {
                    Type::Struct(st) => self.structs.get(&st.name).cloned().unwrap_or(st),
                    _ => return Err(format!("Struct literal expects struct type at {}", s)),
                };

                let mut inits = Vec::new();
                for f in fields {
                    let (idx, field_def) = struct_ty
                        .fields
                        .iter()
                        .enumerate()
                        .find(|(_, sf)| sf.name == f.name)
                        .ok_or_else(|| {
                            format!("Struct '{}' has no field named '{}' at {}", struct_ty.name, f.name, f.span)
                        })?;

                    let mut val = self.check_expr(f.value)?;
                    if val.ty != field_def.ty && !self.can_coerce(&val.ty, &field_def.ty) {
                        return Err(format!(
                            "Cannot assign value of type '{}' to field '{}' of type '{}' at {}",
                            val.ty, f.name, field_def.ty, f.span
                        ));
                    }
                    if val.ty != field_def.ty {
                        val = self.coerce_expr(val, field_def.ty.clone());
                    }

                    inits.push((idx, val));
                }

                Ok(TypedExpr {
                    kind: TypedExprKind::StructLiteral {
                        struct_ty: struct_ty.clone(),
                        fields: inits,
                    },
                    ty: Type::Struct(struct_ty),
                    span: s,
                })
            }
            Expr::ArrayLiteral { .. } => Err(format!("Array literal not implemented at {}", span)),
            Expr::InlineAsm {
                asm_string,
                outputs,
                inputs,
                clobbers,
                span: s,
            } => {
                let mut checked_outs = Vec::new();
                for op in outputs {
                    let e = self.check_expr(op.expr)?;
                    checked_outs.push((op.constraint, e));
                }

                let mut checked_ins = Vec::new();
                for op in inputs {
                    let e = self.check_expr(op.expr)?;
                    checked_ins.push((op.constraint, e));
                }

                Ok(TypedExpr {
                    kind: TypedExprKind::InlineAsm {
                        asm_string,
                        outputs: checked_outs,
                        inputs: checked_ins,
                        clobbers,
                    },
                    ty: Type::Void,
                    span: s,
                })
            }
            Expr::VolatileLoad { ptr, span: s } => {
                let typed_ptr = self.check_expr(*ptr)?;
                if let Type::Pointer(elem_ty) = typed_ptr.ty.clone() {
                    Ok(TypedExpr {
                        kind: TypedExprKind::VolatileLoad(Box::new(typed_ptr)),
                        ty: *elem_ty,
                        span: s,
                    })
                } else {
                    Err(format!("@volatile_load requires pointer argument, got '{}' at {}", typed_ptr.ty, s))
                }
            }
            Expr::VolatileStore { ptr, val, span: s } => {
                let typed_ptr = self.check_expr(*ptr)?;
                let mut typed_val = self.check_expr(*val)?;

                if let Type::Pointer(elem_ty) = &typed_ptr.ty {
                    if typed_val.ty != **elem_ty && !self.can_coerce(&typed_val.ty, elem_ty) {
                        return Err(format!(
                            "@volatile_store expected value of type '{}', got '{}' at {}",
                            elem_ty, typed_val.ty, s
                        ));
                    }
                    if typed_val.ty != **elem_ty {
                        typed_val = self.coerce_expr(typed_val, *elem_ty.clone());
                    }

                    Ok(TypedExpr {
                        kind: TypedExprKind::VolatileStore(Box::new(typed_ptr), Box::new(typed_val)),
                        ty: Type::Void,
                        span: s,
                    })
                } else {
                    Err(format!("@volatile_store requires pointer argument, got '{}' at {}", typed_ptr.ty, s))
                }
            }
        }
    }

    fn can_coerce(&self, from: &Type, to: &Type) -> bool {
        if from == to {
            return true;
        }
        // Implicit integer widening or integer conversion
        if from.is_integer() && to.is_integer() {
            return true;
        }
        // Pointer to uintptr/usize or vice-versa
        if (from.is_pointer() && matches!(to, Type::Uintptr | Type::Usize))
            || (matches!(from, Type::Uintptr | Type::Usize) && to.is_pointer())
        {
            return true;
        }
        // String to *i8 or *u8 for C ABI interop
        if from == &Type::String {
            if let Type::Pointer(inner) = to {
                if matches!(**inner, Type::I8 | Type::U8) {
                    return true;
                }
            }
        }
        // Pointer to pointer (e.g. *u8 to *u16 for low-level osdev)
        if from.is_pointer() && to.is_pointer() {
            return true;
        }
        // Pointer to array to pointer to element: *[N]T -> *T
        if let Type::Pointer(ref from_inner) = from {
            if let Type::Array(ref elem, _) = **from_inner {
                if let Type::Pointer(ref to_inner) = to {
                    if **elem == **to_inner {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn coerce_expr(&self, expr: TypedExpr, target_ty: Type) -> TypedExpr {
        if expr.ty == target_ty {
            return expr;
        }
        let span = expr.span;
        TypedExpr {
            kind: TypedExprKind::Cast {
                expr: Box::new(expr),
                target_ty: target_ty.clone(),
            },
            ty: target_ty,
            span,
        }
    }

    fn rewrite_printf_format(&self, checked_args: &mut [TypedExpr]) {
        if checked_args.is_empty() {
            return;
        }
        let maybe_fmt_str = match &checked_args[0].kind {
            TypedExprKind::StringLit(s) => Some((s.clone(), false)),
            TypedExprKind::Cast { expr: inner, .. } => match &inner.kind {
                TypedExprKind::StringLit(s) => Some((s.clone(), true)),
                _ => None,
            },
            _ => None,
        };
        if let Some((fmt_str, was_cast)) = maybe_fmt_str {
            let extra_args = &checked_args[1..];
            let mut new_fmt = String::new();
            let mut chars = fmt_str.chars().peekable();
            let mut arg_idx = 0;

            while let Some(ch) = chars.next() {
                if ch == '%' {
                    if chars.peek() == Some(&'%') {
                        new_fmt.push('%');
                        new_fmt.push(chars.next().unwrap());
                        continue;
                    }
                    let mut prefix = String::new();
                    let mut spec_char = None;
                    while let Some(&c) = chars.peek() {
                        if c.is_alphabetic() {
                            spec_char = Some(chars.next().unwrap());
                            break;
                        } else {
                            prefix.push(chars.next().unwrap());
                        }
                    }

                    if let Some(spec) = spec_char {
                        if spec == 'd' || spec == 'i' {
                            if arg_idx < extra_args.len() {
                                let arg_ty = &extra_args[arg_idx].ty;
                                if arg_ty.is_unsigned() {
                                    if arg_ty.size_of() == 8 {
                                        new_fmt.push('%');
                                        new_fmt.push_str(&prefix);
                                        new_fmt.push_str("llu");
                                    } else {
                                        new_fmt.push('%');
                                        new_fmt.push_str(&prefix);
                                        new_fmt.push('u');
                                    }
                                } else {
                                    if arg_ty.size_of() == 8 {
                                        new_fmt.push('%');
                                        new_fmt.push_str(&prefix);
                                        new_fmt.push_str("lld");
                                    } else {
                                        new_fmt.push('%');
                                        new_fmt.push_str(&prefix);
                                        new_fmt.push('d');
                                    }
                                }
                            } else {
                                new_fmt.push('%');
                                new_fmt.push_str(&prefix);
                                new_fmt.push(spec);
                            }
                        } else if spec == 'u' {
                            if arg_idx < extra_args.len() {
                                let arg_ty = &extra_args[arg_idx].ty;
                                if arg_ty.size_of() == 8 {
                                    new_fmt.push('%');
                                    new_fmt.push_str(&prefix);
                                    new_fmt.push_str("llu");
                                } else {
                                    new_fmt.push('%');
                                    new_fmt.push_str(&prefix);
                                    new_fmt.push('u');
                                }
                            } else {
                                new_fmt.push('%');
                                new_fmt.push_str(&prefix);
                                new_fmt.push('u');
                            }
                        } else if spec == 'x' || spec == 'X' {
                            if arg_idx < extra_args.len() {
                                let arg_ty = &extra_args[arg_idx].ty;
                                if arg_ty.size_of() == 8 {
                                    new_fmt.push('%');
                                    new_fmt.push_str(&prefix);
                                    new_fmt.push_str("ll");
                                    new_fmt.push(spec);
                                } else {
                                    new_fmt.push('%');
                                    new_fmt.push_str(&prefix);
                                    new_fmt.push(spec);
                                }
                            } else {
                                new_fmt.push('%');
                                new_fmt.push_str(&prefix);
                                new_fmt.push(spec);
                            }
                        } else {
                            new_fmt.push('%');
                            new_fmt.push_str(&prefix);
                            new_fmt.push(spec);
                        }
                        arg_idx += 1;
                    } else {
                        new_fmt.push('%');
                        new_fmt.push_str(&prefix);
                    }
                } else {
                    new_fmt.push(ch);
                }
            }
            if was_cast {
                if let TypedExprKind::Cast { ref mut expr, .. } = checked_args[0].kind {
                    expr.kind = TypedExprKind::StringLit(new_fmt);
                }
            } else {
                checked_args[0].kind = TypedExprKind::StringLit(new_fmt);
            }
        }
    }
}

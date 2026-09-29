use crate::ast::{AssignOp, Attribute, BinaryOp, UnaryOp};
use crate::sema::*;
use crate::types::*;
use std::collections::HashMap;

fn format_llvm_float(f: f64) -> String {
    let mut s = format!("{:e}", f);
    if let Some(e_idx) = s.find('e') {
        if !s[..e_idx].contains('.') {
            s.insert_str(e_idx, ".0");
        }
    } else if !s.contains('.') {
        s.push_str(".0");
    }
    s
}

pub struct LlvmCodegen {
    target_triple: String,
    is_freestanding: bool,
    out: String,
    reg_counter: usize,
    label_counter: usize,
    string_literals: Vec<(String, usize)>, // (content, global_id)
    locals: HashMap<String, (String, Type)>, // var_name -> (llvm_reg, Type)
    defer_stack: Vec<TypedStmt>,
    loop_stack: Vec<(String, String)>, // (post_label, exit_label)
    current_block_terminated: bool,
    current_func_name: Option<String>,
}

impl LlvmCodegen {
    pub fn new(target_triple: String, is_freestanding: bool) -> Self {
        Self {
            target_triple,
            is_freestanding,
            out: String::new(),
            reg_counter: 1,
            label_counter: 1,
            string_literals: Vec::new(),
            locals: HashMap::new(),
            defer_stack: Vec::new(),
            loop_stack: Vec::new(),
            current_block_terminated: false,
            current_func_name: None,
        }
    }

    fn new_reg(&mut self) -> String {
        let r = format!("%r{}", self.reg_counter);
        self.reg_counter += 1;
        r
    }

    fn new_label(&mut self, prefix: &str) -> String {
        let l = format!("{}.{}", prefix, self.label_counter);
        self.label_counter += 1;
        l
    }

    fn emit_line(&mut self, line: &str) {
        self.out.push_str(line);
        self.out.push('\n');
    }

    fn emit_instruction(&mut self, inst: &str) {
        if !self.current_block_terminated {
            self.out.push_str("  ");
            self.out.push_str(inst);
            self.out.push('\n');
        }
    }

    pub fn generate(&mut self, program: TypedProgram) -> String {
        // Module header
        self.emit_line("; ModuleID = 'cez_module'");
        self.emit_line(&format!("target triple = \"{}\"", self.target_triple));
        if self.target_triple.contains("x86_64") {
            self.emit_line("target datalayout = \"e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128\"");
        }
        self.emit_line("");

        // 1. Struct type definitions
        for (name, st) in &program.structs {
            let fields_str: Vec<String> = st.fields.iter().map(|f| f.ty.to_llvm_type()).collect();
            if st.is_packed {
                self.emit_line(&format!("%struct.{} = type <{{ {} }}>", name, fields_str.join(", ")));
            } else {
                self.emit_line(&format!("%struct.{} = type {{ {} }}", name, fields_str.join(", ")));
            }
        }
        self.emit_line("");

        // 2. Global variables
        for g in &program.globals {
            let mut sec_str = String::new();
            let mut align_str = format!("align {}", g.ty.align_of());

            for attr in &g.attributes {
                match attr {
                    Attribute::Section(sec) => sec_str = format!(", section \"{}\"", sec),
                    Attribute::Align(a) => align_str = format!("align {}", a),
                    _ => {}
                }
            }

            let init_str = if let Some(ref init) = g.init {
                match &init.kind {
                    TypedExprKind::IntLit(n) => format!("{}", n),
                    TypedExprKind::FloatLit(f) => format_llvm_float(*f),
                    TypedExprKind::BoolLit(b) => if *b { "true".to_string() } else { "false".to_string() },
                    TypedExprKind::StringLit(s) => {
                        let id = self.string_literals.len();
                        self.string_literals.push((s.clone(), id));
                        let len = s.as_bytes().len();
                        let total_len = len + 1;
                        if g.ty.is_pointer() {
                            format!("getelementptr inbounds ([{} x i8], [{} x i8]* @.str.{}, i64 0, i64 0)", total_len, total_len, id)
                        } else {
                            format!("{{ i8* getelementptr inbounds ([{} x i8], [{} x i8]* @.str.{}, i64 0, i64 0), i64 {} }}", total_len, total_len, id, len)
                        }
                    }
                    TypedExprKind::Cast { expr: inner, .. } => {
                        if let TypedExprKind::StringLit(s) = &inner.kind {
                            let id = self.string_literals.len();
                            self.string_literals.push((s.clone(), id));
                            let len = s.as_bytes().len();
                            let total_len = len + 1;
                            format!("getelementptr inbounds ([{} x i8], [{} x i8]* @.str.{}, i64 0, i64 0)", total_len, total_len, id)
                        } else {
                            "zeroinitializer".to_string()
                        }
                    }
                    _ => "zeroinitializer".to_string(),
                }
            } else {
                "zeroinitializer".to_string()
            };

            self.emit_line(&format!(
                "@{} = global {} {}{}, {}",
                g.name,
                g.ty.to_llvm_type(),
                init_str,
                sec_str,
                align_str
            ));
        }
        self.emit_line("");

        // 3. Declarations for built-in or external functions
        if !self.is_freestanding {
            let func_names: Vec<&str> = program.funcs.iter().map(|f| f.name.as_str()).collect();
            if !func_names.contains(&"printf") {
                self.emit_line("declare i32 @printf(i8*, ...)");
            }
            if !func_names.contains(&"puts") {
                self.emit_line("declare i32 @puts(i8*)");
            }
            if !func_names.contains(&"malloc") {
                self.emit_line("declare i8* @malloc(i64)");
            }
            if !func_names.contains(&"free") {
                self.emit_line("declare void @free(i8*)");
            }
            if !func_names.contains(&"memcpy") {
                self.emit_line("declare i8* @memcpy(i8*, i8*, i64)");
            }
            if !func_names.contains(&"memset") {
                self.emit_line("declare i8* @memset(i8*, i32, i64)");
            }
            if !func_names.contains(&"exit") {
                self.emit_line("declare void @exit(i32)");
            }
            self.emit_line("");
        }

        // 4. Functions
        for f in &program.funcs {
            self.generate_func(f);
        }

        // 5. String literals constants appended to module
        let string_literals = self.string_literals.clone();
        for (content, id) in &string_literals {
            let escaped = escape_llvm_string(content);
            let len = content.as_bytes().len() + 1; // null-terminated
            self.emit_line(&format!(
                "@.str.{} = private unnamed_addr constant [{} x i8] c\"{}\\00\", align 1",
                id, len, escaped
            ));
        }

        self.out.clone()
    }

    fn generate_func(&mut self, f: &TypedFuncDecl) {
        self.reg_counter = 1;
        self.locals.clear();
        self.defer_stack.clear();
        self.loop_stack.clear();
        self.current_block_terminated = false;
        self.current_func_name = Some(f.name.clone());

        let ret_ty_str = if f.name == "main" && f.ret_type == Type::Void && !self.is_freestanding {
            "i32".to_string()
        } else {
            f.ret_type.to_llvm_type()
        };

        let mut param_strs = Vec::new();
        for (name, ty) in &f.params {
            param_strs.push(format!("{} %arg.{}", ty.to_llvm_type(), name));
        }
        if f.is_variadic {
            param_strs.push("...".to_string());
        }

        let mut func_attrs = Vec::new();
        let mut export_name = f.name.clone();

        for attr in &f.attributes {
            match attr {
                Attribute::Naked => func_attrs.push("naked".to_string()),
                Attribute::Section(sec) => func_attrs.push(format!("section \"{}\"", sec)),
                Attribute::Export(sym) => export_name = sym.clone(),
                Attribute::Interrupt => func_attrs.push("\"x86_intr\"".to_string()),
                _ => {}
            }
        }

        let attr_str = if func_attrs.is_empty() {
            String::new()
        } else {
            format!(" {}", func_attrs.join(" "))
        };

        if f.is_extern || f.body.is_none() {
            self.emit_line(&format!(
                "declare {} @{}({}){}",
                ret_ty_str,
                export_name,
                param_strs.join(", "),
                attr_str
            ));
            self.emit_line("");
            return;
        }

        self.emit_line(&format!(
            "define {} @{}({}){} {{",
            ret_ty_str,
            export_name,
            param_strs.join(", "),
            attr_str
        ));
        self.emit_line("entry:");

        // Allocate space for parameters and store incoming arguments
        for (name, ty) in &f.params {
            let alloca_reg = self.new_reg();
            let align = ty.align_of();
            self.emit_instruction(&format!(
                "{} = alloca {}, align {}",
                alloca_reg,
                ty.to_llvm_type(),
                align
            ));
            self.emit_instruction(&format!(
                "store {} %arg.{}, {}* {}, align {}",
                ty.to_llvm_type(),
                name,
                ty.to_llvm_type(),
                alloca_reg,
                align
            ));
            self.locals.insert(name.clone(), (alloca_reg, ty.clone()));
        }

        // Generate function body
        if let Some(ref stmts) = f.body {
            for stmt in stmts {
                self.generate_stmt(stmt, &f.ret_type);
            }
        }

        // If the function has not terminated, execute defers and emit default ret
        if !self.current_block_terminated {
            self.execute_defers();
            if f.name == "main" && f.ret_type == Type::Void && !self.is_freestanding {
                self.emit_instruction("ret i32 0");
            } else if f.ret_type == Type::Void {
                self.emit_instruction("ret void");
            } else if f.name == "main" && f.ret_type.is_integer() {
                self.emit_instruction("ret i32 0");
            } else {
                self.emit_instruction(&format!("ret {} zeroinitializer", ret_ty_str));
            }
            self.current_block_terminated = true;
        }

        self.emit_line("}");
        self.emit_line("");
    }

    fn execute_defers(&mut self) {
        let defers = self.defer_stack.clone();
        for d in defers.iter().rev() {
            self.generate_stmt(d, &Type::Void);
        }
    }

    fn generate_stmt(&mut self, stmt: &TypedStmt, ret_ty: &Type) {
        if self.current_block_terminated {
            return;
        }

        match stmt {
            TypedStmt::VarDecl { name, ty, init, .. } => {
                let alloca_reg = self.new_reg();
                let align = ty.align_of();
                self.emit_instruction(&format!(
                    "{} = alloca {}, align {}",
                    alloca_reg,
                    ty.to_llvm_type(),
                    align
                ));
                self.locals.insert(name.clone(), (alloca_reg.clone(), ty.clone()));

                if let Some(init_expr) = init {
                    let val_reg = self.generate_expr(init_expr);
                    self.emit_instruction(&format!(
                        "store {} {}, {}* {}, align {}",
                        init_expr.ty.to_llvm_type(),
                        val_reg,
                        ty.to_llvm_type(),
                        alloca_reg,
                        align
                    ));
                }
            }
            TypedStmt::Assign {
                target,
                op,
                value,
                ..
            } => {
                let val_reg = self.generate_expr(value);
                let target_ptr = self.generate_expr_ptr(target);

                let final_val = match op {
                    AssignOp::Assign => val_reg,
                    AssignOp::AddAssign => {
                        let cur = self.new_reg();
                        let align = target.ty.align_of();
                        self.emit_instruction(&format!(
                            "{} = load {}, {}* {}, align {}",
                            cur,
                            target.ty.to_llvm_type(),
                            target.ty.to_llvm_type(),
                            target_ptr,
                            align
                        ));
                        let res = self.new_reg();
                        self.emit_instruction(&format!(
                            "{} = add {} {}, {}",
                            res,
                            target.ty.to_llvm_type(),
                            cur,
                            val_reg
                        ));
                        res
                    }
                    AssignOp::SubAssign => {
                        let cur = self.new_reg();
                        let align = target.ty.align_of();
                        self.emit_instruction(&format!(
                            "{} = load {}, {}* {}, align {}",
                            cur,
                            target.ty.to_llvm_type(),
                            target.ty.to_llvm_type(),
                            target_ptr,
                            align
                        ));
                        let res = self.new_reg();
                        self.emit_instruction(&format!(
                            "{} = sub {} {}, {}",
                            res,
                            target.ty.to_llvm_type(),
                            cur,
                            val_reg
                        ));
                        res
                    }
                    AssignOp::MulAssign => {
                        let cur = self.new_reg();
                        let align = target.ty.align_of();
                        self.emit_instruction(&format!(
                            "{} = load {}, {}* {}, align {}",
                            cur,
                            target.ty.to_llvm_type(),
                            target.ty.to_llvm_type(),
                            target_ptr,
                            align
                        ));
                        let res = self.new_reg();
                        self.emit_instruction(&format!(
                            "{} = mul {} {}, {}",
                            res,
                            target.ty.to_llvm_type(),
                            cur,
                            val_reg
                        ));
                        res
                    }
                    _ => val_reg,
                };

                let align = target.ty.align_of();
                self.emit_instruction(&format!(
                    "store {} {}, {}* {}, align {}",
                    target.ty.to_llvm_type(),
                    final_val,
                    target.ty.to_llvm_type(),
                    target_ptr,
                    align
                ));
            }
            TypedStmt::Return { value, .. } => {
                let ret_val_reg = if let Some(ref v) = value {
                    Some(self.generate_expr(v))
                } else {
                    None
                };

                // Execute deferred calls in LIFO order
                self.execute_defers();

                if let Some(r) = ret_val_reg {
                    self.emit_instruction(&format!("ret {} {}", ret_ty.to_llvm_type(), r));
                } else if self.current_func_name.as_deref() == Some("main")
                    && *ret_ty == Type::Void
                    && !self.is_freestanding
                {
                    self.emit_instruction("ret i32 0");
                } else {
                    self.emit_instruction("ret void");
                }
                self.current_block_terminated = true;
            }
            TypedStmt::Defer { stmt, .. } => {
                self.defer_stack.push(*stmt.clone());
            }
            TypedStmt::If {
                init,
                cond,
                then_block,
                else_block,
                ..
            } => {
                if let Some(i) = init {
                    self.generate_stmt(i, ret_ty);
                }

                let cond_reg = self.generate_expr(cond);
                let then_label = self.new_label("then");
                let else_label = self.new_label("else");
                let merge_label = self.new_label("if.end");

                let has_else = else_block.is_some();
                let false_target = if has_else { &else_label } else { &merge_label };

                self.emit_instruction(&format!(
                    "br i1 {}, label %{}, label %{}",
                    cond_reg, then_label, false_target
                ));

                // Then block
                self.emit_line(&format!("{}:", then_label));
                self.current_block_terminated = false;
                for s in then_block {
                    self.generate_stmt(s, ret_ty);
                }
                if !self.current_block_terminated {
                    self.emit_instruction(&format!("br label %{}", merge_label));
                }

                // Else block
                if let Some(elses) = else_block {
                    self.emit_line(&format!("{}:", else_label));
                    self.current_block_terminated = false;
                    for s in elses {
                        self.generate_stmt(s, ret_ty);
                    }
                    if !self.current_block_terminated {
                        self.emit_instruction(&format!("br label %{}", merge_label));
                    }
                }

                self.emit_line(&format!("{}:", merge_label));
                self.current_block_terminated = false;
            }
            TypedStmt::For {
                init,
                cond,
                post,
                body,
                ..
            } => {
                if let Some(i) = init {
                    self.generate_stmt(i, ret_ty);
                }

                let cond_label = self.new_label("loop.cond");
                let body_label = self.new_label("loop.body");
                let post_label = self.new_label("loop.post");
                let exit_label = self.new_label("loop.exit");

                self.emit_instruction(&format!("br label %{}", cond_label));

                // Loop Cond
                self.emit_line(&format!("{}:", cond_label));
                self.current_block_terminated = false;
                if let Some(c) = cond {
                    let c_reg = self.generate_expr(c);
                    self.emit_instruction(&format!(
                        "br i1 {}, label %{}, label %{}",
                        c_reg, body_label, exit_label
                    ));
                } else {
                    self.emit_instruction(&format!("br label %{}", body_label));
                }

                // Loop Body
                self.emit_line(&format!("{}:", body_label));
                self.current_block_terminated = false;
                self.loop_stack.push((post_label.clone(), exit_label.clone()));
                for s in body {
                    self.generate_stmt(s, ret_ty);
                }
                self.loop_stack.pop();
                if !self.current_block_terminated {
                    self.emit_instruction(&format!("br label %{}", post_label));
                }

                // Loop Post
                self.emit_line(&format!("{}:", post_label));
                self.current_block_terminated = false;
                if let Some(p) = post {
                    self.generate_stmt(p, ret_ty);
                }
                self.emit_instruction(&format!("br label %{}", cond_label));

                // Loop Exit
                self.emit_line(&format!("{}:", exit_label));
                self.current_block_terminated = false;
            }
            TypedStmt::Expr(expr) => {
                self.generate_expr(expr);
            }
            TypedStmt::Block(stmts, ..) => {
                for s in stmts {
                    self.generate_stmt(s, ret_ty);
                }
            }
            TypedStmt::Break(..) => {
                if let Some((_, exit_lbl)) = self.loop_stack.last() {
                    let exit_lbl = exit_lbl.clone();
                    self.emit_instruction(&format!("br label %{}", exit_lbl));
                    self.current_block_terminated = true;
                }
            }
            TypedStmt::Continue(..) => {
                if let Some((post_lbl, _)) = self.loop_stack.last() {
                    let post_lbl = post_lbl.clone();
                    self.emit_instruction(&format!("br label %{}", post_lbl));
                    self.current_block_terminated = true;
                }
            }
        }
    }

    fn generate_expr(&mut self, expr: &TypedExpr) -> String {
        match &expr.kind {
            TypedExprKind::IntLit(n) => format!("{}", n),
            TypedExprKind::FloatLit(f) => format_llvm_float(*f),
            TypedExprKind::BoolLit(b) => if *b { "true".to_string() } else { "false".to_string() },
            TypedExprKind::CharLit(c) => format!("{}", *c as u8),
            TypedExprKind::StringLit(s) => {
                let id = self.string_literals.len();
                self.string_literals.push((s.clone(), id));
                let str_len = s.as_bytes().len();
                let str_total_len = str_len + 1;

                // Create { i8* ptr, i64 len } string struct
                let gep_reg = self.new_reg();
                self.emit_instruction(&format!(
                    "{} = getelementptr inbounds [{} x i8], [{} x i8]* @.str.{}, i64 0, i64 0",
                    gep_reg, str_total_len, str_total_len, id
                ));

                let str_val1 = self.new_reg();
                self.emit_instruction(&format!(
                    "{} = insertvalue {{ i8*, i64 }} undef, i8* {}, 0",
                    str_val1, gep_reg
                ));
                let str_val2 = self.new_reg();
                self.emit_instruction(&format!(
                    "{} = insertvalue {{ i8*, i64 }} {}, i64 {}, 1",
                    str_val2, str_val1, str_len
                ));
                str_val2
            }
            TypedExprKind::LocalVar(name) => {
                let (alloca_reg, ty) = self.locals.get(name).cloned().unwrap();
                let load_reg = self.new_reg();
                let align = ty.align_of();
                self.emit_instruction(&format!(
                    "{} = load {}, {}* {}, align {}",
                    load_reg,
                    ty.to_llvm_type(),
                    ty.to_llvm_type(),
                    alloca_reg,
                    align
                ));
                load_reg
            }
            TypedExprKind::GlobalVar(name) => {
                let load_reg = self.new_reg();
                let align = expr.ty.align_of();
                self.emit_instruction(&format!(
                    "{} = load {}, {}* @{}, align {}",
                    load_reg,
                    expr.ty.to_llvm_type(),
                    expr.ty.to_llvm_type(),
                    name,
                    align
                ));
                load_reg
            }
            TypedExprKind::Binary(lhs, op, rhs) => {
                let l_reg = self.generate_expr(lhs);
                let r_reg = self.generate_expr(rhs);
                let res = self.new_reg();
                let ty_str = lhs.ty.to_llvm_type();

                let is_float = lhs.ty.is_float();
                let is_signed = lhs.ty.is_signed();

                let inst = if is_float {
                    match op {
                        BinaryOp::Add => format!("{} = fadd {} {}, {}", res, ty_str, l_reg, r_reg),
                        BinaryOp::Sub => format!("{} = fsub {} {}, {}", res, ty_str, l_reg, r_reg),
                        BinaryOp::Mul => format!("{} = fmul {} {}, {}", res, ty_str, l_reg, r_reg),
                        BinaryOp::Div => format!("{} = fdiv {} {}, {}", res, ty_str, l_reg, r_reg),
                        BinaryOp::Mod => format!("{} = frem {} {}, {}", res, ty_str, l_reg, r_reg),
                        BinaryOp::Eq => format!("{} = fcmp oeq {} {}, {}", res, ty_str, l_reg, r_reg),
                        BinaryOp::NotEq => format!("{} = fcmp one {} {}, {}", res, ty_str, l_reg, r_reg),
                        BinaryOp::Lt => format!("{} = fcmp olt {} {}, {}", res, ty_str, l_reg, r_reg),
                        BinaryOp::LtEq => format!("{} = fcmp ole {} {}, {}", res, ty_str, l_reg, r_reg),
                        BinaryOp::Gt => format!("{} = fcmp ogt {} {}, {}", res, ty_str, l_reg, r_reg),
                        BinaryOp::GtEq => format!("{} = fcmp oge {} {}, {}", res, ty_str, l_reg, r_reg),
                        _ => format!("{} = fadd {} {}, {}", res, ty_str, l_reg, r_reg),
                    }
                } else {
                    match op {
                        BinaryOp::Add => format!("{} = add {} {}, {}", res, ty_str, l_reg, r_reg),
                        BinaryOp::Sub => format!("{} = sub {} {}, {}", res, ty_str, l_reg, r_reg),
                        BinaryOp::Mul => format!("{} = mul {} {}, {}", res, ty_str, l_reg, r_reg),
                        BinaryOp::Div => {
                            if is_signed {
                                format!("{} = sdiv {} {}, {}", res, ty_str, l_reg, r_reg)
                            } else {
                                format!("{} = udiv {} {}, {}", res, ty_str, l_reg, r_reg)
                            }
                        }
                        BinaryOp::Mod => {
                            if is_signed {
                                format!("{} = srem {} {}, {}", res, ty_str, l_reg, r_reg)
                            } else {
                                format!("{} = urem {} {}, {}", res, ty_str, l_reg, r_reg)
                            }
                        }
                        BinaryOp::BitAnd => format!("{} = and {} {}, {}", res, ty_str, l_reg, r_reg),
                        BinaryOp::BitOr => format!("{} = or {} {}, {}", res, ty_str, l_reg, r_reg),
                        BinaryOp::BitXor => format!("{} = xor {} {}, {}", res, ty_str, l_reg, r_reg),
                        BinaryOp::Shl => format!("{} = shl {} {}, {}", res, ty_str, l_reg, r_reg),
                        BinaryOp::Shr => {
                            if is_signed {
                                format!("{} = ashr {} {}, {}", res, ty_str, l_reg, r_reg)
                            } else {
                                format!("{} = lshr {} {}, {}", res, ty_str, l_reg, r_reg)
                            }
                        }
                        BinaryOp::Eq => format!("{} = icmp eq {} {}, {}", res, ty_str, l_reg, r_reg),
                        BinaryOp::NotEq => format!("{} = icmp ne {} {}, {}", res, ty_str, l_reg, r_reg),
                        BinaryOp::Lt => {
                            if is_signed {
                                format!("{} = icmp slt {} {}, {}", res, ty_str, l_reg, r_reg)
                            } else {
                                format!("{} = icmp ult {} {}, {}", res, ty_str, l_reg, r_reg)
                            }
                        }
                        BinaryOp::LtEq => {
                            if is_signed {
                                format!("{} = icmp sle {} {}, {}", res, ty_str, l_reg, r_reg)
                            } else {
                                format!("{} = icmp ule {} {}, {}", res, ty_str, l_reg, r_reg)
                            }
                        }
                        BinaryOp::Gt => {
                            if is_signed {
                                format!("{} = icmp sgt {} {}, {}", res, ty_str, l_reg, r_reg)
                            } else {
                                format!("{} = icmp ugt {} {}, {}", res, ty_str, l_reg, r_reg)
                            }
                        }
                        BinaryOp::GtEq => {
                            if is_signed {
                                format!("{} = icmp sge {} {}, {}", res, ty_str, l_reg, r_reg)
                            } else {
                                format!("{} = icmp uge {} {}, {}", res, ty_str, l_reg, r_reg)
                            }
                        }
                        BinaryOp::LogicalAnd => format!("{} = and i1 {}, {}", res, l_reg, r_reg),
                        BinaryOp::LogicalOr => format!("{} = or i1 {}, {}", res, l_reg, r_reg),
                        BinaryOp::BitClear => {
                            // Go's `&^`: a &^ b is a & (~b)
                            let not_r = self.new_reg();
                            self.emit_instruction(&format!("{} = xor {} {}, -1", not_r, ty_str, r_reg));
                            format!("{} = and {} {}, {}", res, ty_str, l_reg, not_r)
                        }
                    }
                };

                self.emit_instruction(&inst);
                res
            }
            TypedExprKind::Unary(op, inner) => {
                match op {
                    UnaryOp::Neg => {
                        let in_reg = self.generate_expr(inner);
                        let res = self.new_reg();
                        if inner.ty.is_float() {
                            self.emit_instruction(&format!(
                                "{} = fneg {} {}",
                                res,
                                inner.ty.to_llvm_type(),
                                in_reg
                            ));
                        } else {
                            self.emit_instruction(&format!(
                                "{} = sub {} 0, {}",
                                res,
                                inner.ty.to_llvm_type(),
                                in_reg
                            ));
                        }
                        res
                    }
                    UnaryOp::Not => {
                        let in_reg = self.generate_expr(inner);
                        let res = self.new_reg();
                        self.emit_instruction(&format!("{} = xor i1 {}, true", res, in_reg));
                        res
                    }
                    UnaryOp::BitNot => {
                        let in_reg = self.generate_expr(inner);
                        let res = self.new_reg();
                        self.emit_instruction(&format!(
                            "{} = xor {} {}, -1",
                            res,
                            inner.ty.to_llvm_type(),
                            in_reg
                        ));
                        res
                    }
                    UnaryOp::AddrOf => self.generate_expr_ptr(inner),
                    UnaryOp::Deref => {
                        let ptr_reg = self.generate_expr(inner);
                        let res = self.new_reg();
                        let align = expr.ty.align_of();
                        self.emit_instruction(&format!(
                            "{} = load {}, {}* {}, align {}",
                            res,
                            expr.ty.to_llvm_type(),
                            expr.ty.to_llvm_type(),
                            ptr_reg,
                            align
                        ));
                        res
                    }
                }
            }
            TypedExprKind::Call { func_name, args } => {
                let mut arg_strs = Vec::new();
                for a in args {
                    let r = self.generate_expr(a);
                    arg_strs.push(format!("{} {}", a.ty.to_llvm_type(), r));
                }

                if expr.ty == Type::Void {
                    self.emit_instruction(&format!("call void @{}({})", func_name, arg_strs.join(", ")));
                    "".to_string()
                } else {
                    let res = self.new_reg();
                    self.emit_instruction(&format!(
                        "{} = call {} @{}({})",
                        res,
                        expr.ty.to_llvm_type(),
                        func_name,
                        arg_strs.join(", ")
                    ));
                    res
                }
            }
            TypedExprKind::MemberAccess {
                target,
                field_index,
                ..
            } => {
                let target_ptr = self.generate_expr_ptr(target);
                let gep_reg = self.new_reg();
                let struct_llvm = target.ty.to_llvm_type();

                self.emit_instruction(&format!(
                    "{} = getelementptr inbounds {}, {}* {}, i32 0, i32 {}",
                    gep_reg, struct_llvm, struct_llvm, target_ptr, field_index
                ));

                let load_reg = self.new_reg();
                let align = expr.ty.align_of();
                self.emit_instruction(&format!(
                    "{} = load {}, {}* {}, align {}",
                    load_reg,
                    expr.ty.to_llvm_type(),
                    expr.ty.to_llvm_type(),
                    gep_reg,
                    align
                ));
                load_reg
            }
            TypedExprKind::Index { target, index } => {
                let target_ptr = if matches!(target.ty, Type::Pointer(_)) {
                    self.generate_expr(target)
                } else if matches!(target.ty, Type::Slice(_)) {
                    let slice_val = self.generate_expr(target);
                    let ptr_reg = self.new_reg();
                    self.emit_instruction(&format!(
                        "{} = extractvalue {} {}, 0",
                        ptr_reg, target.ty.to_llvm_type(), slice_val
                    ));
                    ptr_reg
                } else {
                    self.generate_expr_ptr(target)
                };
                let idx_reg = self.generate_expr(index);

                let gep_reg = self.new_reg();
                let elem_llvm = expr.ty.to_llvm_type();

                self.emit_instruction(&format!(
                    "{} = getelementptr inbounds {}, {}* {}, i64 {}",
                    gep_reg, elem_llvm, elem_llvm, target_ptr, idx_reg
                ));

                let load_reg = self.new_reg();
                let align = expr.ty.align_of();
                self.emit_instruction(&format!(
                    "{} = load {}, {}* {}, align {}",
                    load_reg, elem_llvm, elem_llvm, gep_reg, align
                ));
                load_reg
            }
            TypedExprKind::Slice { target, low, high } => {
                let target_ptr = self.generate_expr_ptr(target);
                let low_reg = if let Some(l) = low {
                    self.generate_expr(l)
                } else {
                    "0".to_string()
                };

                let elem_ty = match &expr.ty {
                    Type::Slice(elem) => elem,
                    _ => unreachable!(),
                };
                let elem_llvm = elem_ty.to_llvm_type();

                // Offset the pointer: target_ptr + low
                let sub_ptr = self.new_reg();
                self.emit_instruction(&format!(
                    "{} = getelementptr inbounds {}, {}* {}, i64 {}",
                    sub_ptr, elem_llvm, elem_llvm, target_ptr, low_reg
                ));

                let high_reg = if let Some(h) = high {
                    self.generate_expr(h)
                } else {
                    "0".to_string() // fallback
                };

                let len_reg = self.new_reg();
                self.emit_instruction(&format!("{} = sub i64 {}, {}", len_reg, high_reg, low_reg));

                // Create slice struct { elem_ptr*, len, cap }
                let slice_llvm = expr.ty.to_llvm_type();
                let s1 = self.new_reg();
                self.emit_instruction(&format!(
                    "{} = insertvalue {} undef, {}* {}, 0",
                    s1, slice_llvm, elem_llvm, sub_ptr
                ));
                let s2 = self.new_reg();
                self.emit_instruction(&format!(
                    "{} = insertvalue {} {}, i64 {}, 1",
                    s2, slice_llvm, s1, len_reg
                ));
                let s3 = self.new_reg();
                self.emit_instruction(&format!(
                    "{} = insertvalue {} {}, i64 {}, 2",
                    s3, slice_llvm, s2, len_reg
                ));
                s3
            }
            TypedExprKind::Cast { expr: inner, target_ty } => {
                let in_reg = self.generate_expr(inner);
                let in_ty = &inner.ty;

                if in_ty == target_ty {
                    return in_reg;
                }

                let res = self.new_reg();
                let from_llvm = in_ty.to_llvm_type();
                let to_llvm = target_ty.to_llvm_type();

                if in_ty == &Type::String && target_ty.is_pointer() {
                    let ptr_reg = self.new_reg();
                    self.emit_instruction(&format!(
                        "{} = extractvalue {{ i8*, i64 }} {}, 0",
                        ptr_reg, in_reg
                    ));
                    if to_llvm != "i8*" {
                        self.emit_instruction(&format!(
                            "{} = bitcast i8* {} to {}",
                            res, ptr_reg, to_llvm
                        ));
                    } else {
                        return ptr_reg;
                    }
                } else if in_ty == &Type::String && target_ty.is_integer() {
                    let ptr_reg = self.new_reg();
                    self.emit_instruction(&format!(
                        "{} = extractvalue {{ i8*, i64 }} {}, 0",
                        ptr_reg, in_reg
                    ));
                    self.emit_instruction(&format!(
                        "{} = ptrtoint i8* {} to {}",
                        res, ptr_reg, to_llvm
                    ));
                } else if in_ty.is_float() && target_ty.is_float() {
                    let from_size = in_ty.size_of();
                    let to_size = target_ty.size_of();
                    if from_size < to_size {
                        self.emit_instruction(&format!(
                            "{} = fpext {} {} to {}",
                            res, from_llvm, in_reg, to_llvm
                        ));
                    } else if from_size > to_size {
                        self.emit_instruction(&format!(
                            "{} = fptrunc {} {} to {}",
                            res, from_llvm, in_reg, to_llvm
                        ));
                    } else {
                        return in_reg;
                    }
                } else if in_ty.is_integer() && target_ty.is_float() {
                    if in_ty.is_signed() {
                        self.emit_instruction(&format!(
                            "{} = sitofp {} {} to {}",
                            res, from_llvm, in_reg, to_llvm
                        ));
                    } else {
                        self.emit_instruction(&format!(
                            "{} = uitofp {} {} to {}",
                            res, from_llvm, in_reg, to_llvm
                        ));
                    }
                } else if in_ty.is_float() && target_ty.is_integer() {
                    if target_ty.is_signed() {
                        self.emit_instruction(&format!(
                            "{} = fptosi {} {} to {}",
                            res, from_llvm, in_reg, to_llvm
                        ));
                    } else {
                        self.emit_instruction(&format!(
                            "{} = fptoui {} {} to {}",
                            res, from_llvm, in_reg, to_llvm
                        ));
                    }
                } else if in_ty.is_integer() && target_ty.is_integer() {
                    let from_size = in_ty.size_of();
                    let to_size = target_ty.size_of();
                    if from_size < to_size {
                        if in_ty.is_signed() {
                            self.emit_instruction(&format!(
                                "{} = sext {} {} to {}",
                                res, from_llvm, in_reg, to_llvm
                            ));
                        } else {
                            self.emit_instruction(&format!(
                                "{} = zext {} {} to {}",
                                res, from_llvm, in_reg, to_llvm
                            ));
                        }
                    } else if from_size > to_size {
                        self.emit_instruction(&format!(
                            "{} = trunc {} {} to {}",
                            res, from_llvm, in_reg, to_llvm
                        ));
                    } else {
                        self.emit_instruction(&format!(
                            "{} = bitcast {} {} to {}",
                            res, from_llvm, in_reg, to_llvm
                        ));
                    }
                } else if in_ty.is_pointer() && (target_ty.is_integer()) {
                    self.emit_instruction(&format!(
                        "{} = ptrtoint {} {} to {}",
                        res, from_llvm, in_reg, to_llvm
                    ));
                } else if in_ty.is_integer() && target_ty.is_pointer() {
                    self.emit_instruction(&format!(
                        "{} = inttoptr {} {} to {}",
                        res, from_llvm, in_reg, to_llvm
                    ));
                } else if in_ty.is_pointer() && target_ty.is_pointer() {
                    self.emit_instruction(&format!(
                        "{} = bitcast {} {} to {}",
                        res, from_llvm, in_reg, to_llvm
                    ));
                } else {
                    self.emit_instruction(&format!(
                        "{} = bitcast {} {} to {}",
                        res, from_llvm, in_reg, to_llvm
                    ));
                }

                res
            }
            TypedExprKind::StructLiteral { struct_ty, fields } => {
                let mut cur = "undef".to_string();
                let struct_llvm = format!("%struct.{}", struct_ty.name);

                for (idx, val) in fields {
                    let val_reg = self.generate_expr(val);
                    let next = self.new_reg();
                    self.emit_instruction(&format!(
                        "{} = insertvalue {} {}, {} {}, {}",
                        next,
                        struct_llvm,
                        cur,
                        val.ty.to_llvm_type(),
                        val_reg,
                        idx
                    ));
                    cur = next;
                }
                cur
            }
            TypedExprKind::InlineAsm {
                asm_string,
                outputs: _,
                inputs,
                clobbers: _,
            } => {
                // LLVM inline asm: call void asm sideeffect "...", "..."(...)
                let mut in_regs = Vec::new();
                let mut constraints = Vec::new();

                for (c, expr) in inputs {
                    let r = self.generate_expr(expr);
                    in_regs.push(format!("{} {}", expr.ty.to_llvm_type(), r));
                    constraints.push(c.clone());
                }

                let constr_str = constraints.join(",");
                let mut formatted_asm = asm_string.clone();
                for i in 0..10 {
                    formatted_asm = formatted_asm.replace(&format!("%{}", i), &format!("${}", i));
                }

                self.emit_instruction(&format!(
                    "call void asm sideeffect \"{}\", \"{}\"({})",
                    formatted_asm,
                    constr_str,
                    in_regs.join(", ")
                ));
                "".to_string()
            }
            TypedExprKind::VolatileLoad(ptr) => {
                let ptr_reg = self.generate_expr(ptr);
                let res = self.new_reg();
                let elem_llvm = expr.ty.to_llvm_type();
                let align = expr.ty.align_of();
                self.emit_instruction(&format!(
                    "{} = load volatile {}, {}* {}, align {}",
                    res, elem_llvm, elem_llvm, ptr_reg, align
                ));
                res
            }
            TypedExprKind::VolatileStore(ptr, val) => {
                let ptr_reg = self.generate_expr(ptr);
                let val_reg = self.generate_expr(val);
                let elem_llvm = val.ty.to_llvm_type();
                let align = val.ty.align_of();
                self.emit_instruction(&format!(
                    "store volatile {} {}, {}* {}, align {}",
                    elem_llvm, val_reg, elem_llvm, ptr_reg, align
                ));
                "".to_string()
            }
        }
    }

    fn generate_expr_ptr(&mut self, expr: &TypedExpr) -> String {
        match &expr.kind {
            TypedExprKind::LocalVar(name) => {
                let (alloca_reg, _) = self.locals.get(name).unwrap();
                alloca_reg.clone()
            }
            TypedExprKind::GlobalVar(name) => format!("@{}", name),
            TypedExprKind::Unary(UnaryOp::Deref, inner) => self.generate_expr(inner),
            TypedExprKind::MemberAccess {
                target,
                field_index,
                ..
            } => {
                let target_ptr = self.generate_expr_ptr(target);
                let gep_reg = self.new_reg();
                let struct_llvm = target.ty.to_llvm_type();
                self.emit_instruction(&format!(
                    "{} = getelementptr inbounds {}, {}* {}, i32 0, i32 {}",
                    gep_reg, struct_llvm, struct_llvm, target_ptr, field_index
                ));
                gep_reg
            }
            TypedExprKind::Index { target, index } => {
                let target_ptr = if matches!(target.ty, Type::Pointer(_)) {
                    self.generate_expr(target)
                } else if matches!(target.ty, Type::Slice(_)) {
                    let slice_val = self.generate_expr(target);
                    let ptr_reg = self.new_reg();
                    self.emit_instruction(&format!(
                        "{} = extractvalue {} {}, 0",
                        ptr_reg, target.ty.to_llvm_type(), slice_val
                    ));
                    ptr_reg
                } else {
                    self.generate_expr_ptr(target)
                };
                let idx_reg = self.generate_expr(index);
                let gep_reg = self.new_reg();
                let elem_llvm = expr.ty.to_llvm_type();
                self.emit_instruction(&format!(
                    "{} = getelementptr inbounds {}, {}* {}, i64 {}",
                    gep_reg, elem_llvm, elem_llvm, target_ptr, idx_reg
                ));
                gep_reg
            }
            _ => {
                // If expression is a value, spill to alloca and return alloca ptr
                let val_reg = self.generate_expr(expr);
                let alloca_reg = self.new_reg();
                let align = expr.ty.align_of();
                self.emit_instruction(&format!(
                    "{} = alloca {}, align {}",
                    alloca_reg,
                    expr.ty.to_llvm_type(),
                    align
                ));
                self.emit_instruction(&format!(
                    "store {} {}, {}* {}, align {}",
                    expr.ty.to_llvm_type(),
                    val_reg,
                    expr.ty.to_llvm_type(),
                    alloca_reg,
                    align
                ));
                alloca_reg
            }
        }
    }
}

fn escape_llvm_string(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        match *b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\22"),
            b'\n' => out.push_str("\\0A"),
            b'\r' => out.push_str("\\0D"),
            b'\t' => out.push_str("\\09"),
            32..=126 => out.push(*b as char),
            other => out.push_str(&format!("\\{:02X}", other)),
        }
    }
    out
}

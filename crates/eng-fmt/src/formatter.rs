use eng_parser::ast::*;

/// Re-emit an Englist module in canonical format.
/// Output is always syntactically valid Englist (begin/end style).
pub fn format_module(module: &Module) -> String {
    let mut f = Formatter::new();
    f.fmt_module(module);
    f.output
}

struct Formatter {
    output: String,
    indent: usize,
}

const INDENT: &str = "    "; // 4 spaces

impl Formatter {
    fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
        }
    }

    fn line(&mut self, s: &str) {
        if s.is_empty() {
            self.output.push('\n');
        } else {
            for _ in 0..self.indent {
                self.output.push_str(INDENT);
            }
            self.output.push_str(s);
            self.output.push('\n');
        }
    }

    fn blank(&mut self) {
        self.output.push('\n');
    }

    fn indent(&mut self) {
        self.indent += 1;
    }
    fn dedent(&mut self) {
        self.indent = self.indent.saturating_sub(1);
    }

    fn fmt_module(&mut self, module: &Module) {
        for (i, item) in module.items.iter().enumerate() {
            if i > 0 {
                self.blank();
            }
            self.fmt_item(item);
        }
    }

    fn fmt_item(&mut self, item: &Item) {
        match item {
            Item::Package(p) => self.line(&format!("package {}", p.name)),
            Item::Module(m) => self.line(&format!("module {}", m.name)),
            Item::Use(u) => {
                let path = u
                    .path
                    .iter()
                    .map(|id| id.name.as_str())
                    .collect::<Vec<_>>()
                    .join(".");
                self.line(&format!("use {}", path));
            }
            Item::Function(f) => self.fmt_function(f),
            Item::Type(t) => self.fmt_type_def(t),
            Item::Route(r) => {
                self.line(&format!("route \"{}\"", r.path));
                self.indent();
                self.line("returns");
                self.fmt_block(&r.handler);
                self.dedent();
            }
            Item::Statement(s) => self.fmt_stmt(s),
            Item::Enum(e) => {
                self.line(&format!("enum {} {{", e.name.name));
                self.indent();
                for variant in &e.variants {
                    if let Some(payload) = &variant.payload {
                        self.line(&format!("{} {}", variant.name.name, fmt_type(payload)));
                    } else {
                        self.line(&format!("{}", variant.name.name));
                    }
                }
                self.dedent();
                self.line("}");
            }
        }
    }

    fn fmt_function(&mut self, f: &FunctionDef) {
        let vis = match f.visibility {
            Visibility::Public => "public ",
            Visibility::Private => "",
            Visibility::Internal => "internal ",
        };
        let params = f
            .params
            .iter()
            .map(|p| format!("{} {}", fmt_type(&p.ty), p.name.name))
            .collect::<Vec<_>>()
            .join(", ");
        self.line(&format!("{}function {}({})", vis, f.name.name, params));

        if let Some(ret) = &f.ret_type {
            self.line(&format!("returns {}", fmt_type(ret)));
        }
        if !f.effects.is_empty() {
            let effects: Vec<_> = f.effects.iter().map(|e| e.name.as_str()).collect();
            self.line(&format!("effects {}", effects.join(", ")));
        }
        self.fmt_block_begin_end(&f.body);
    }

    fn fmt_type_def(&mut self, t: &TypeDef) {
        self.line(&format!("type {}", t.name.name));
        if !t.capabilities.is_empty() {
            self.indent();
            let caps: Vec<_> = t.capabilities.iter().map(|c| c.name.as_str()).collect();
            self.line(&format!("requires {}", caps.join(", ")));
            self.dedent();
        }
    }

    fn fmt_block_begin_end(&mut self, block: &Block) {
        self.line("begin");
        self.indent();
        for stmt in &block.stmts {
            self.fmt_stmt(stmt);
        }
        self.dedent();
        self.line("end");
    }

    fn fmt_block(&mut self, block: &Block) {
        self.indent();
        for stmt in &block.stmts {
            self.fmt_stmt(stmt);
        }
        self.dedent();
    }

    fn fmt_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let(l) => {
                let mut parts = vec!["let".to_string()];
                if l.mutable {
                    parts.push("mutable".into());
                }
                parts.push(l.name.name.clone());
                parts.push("be".into());
                if let Some(ty) = &l.ty {
                    parts.push(fmt_type(ty));
                }
                if let Some(val) = &l.value {
                    if l.ty.is_some() {
                        parts.push("=".into());
                    }
                    parts.push(fmt_expr(val));
                }
                self.line(&parts.join(" "));
            }
            Stmt::Return(r) => {
                if let Some(val) = &r.value {
                    self.line(&format!("return {}", fmt_expr(val)));
                } else {
                    self.line("return");
                }
            }
            Stmt::If(i) => {
                self.line(&format!("if {}", fmt_expr(&i.condition)));
                self.line("then");
                self.fmt_block_begin_end(&i.then_block);
                if let Some(else_block) = &i.otherwise {
                    self.line("otherwise");
                    self.fmt_block_begin_end(else_block);
                }
            }
            Stmt::When(w) => {
                self.line(&format!("when {}", fmt_expr(&w.condition)));
                self.fmt_block_begin_end(&w.then_block);
                if let Some(else_block) = &w.otherwise {
                    self.line("otherwise");
                    self.fmt_block_begin_end(else_block);
                }
            }
            Stmt::Repeat(r) => self.fmt_repeat(r, false),
            Stmt::ParallelRepeat(r) => self.fmt_repeat(r, true),
            Stmt::Match(m) => {
                self.line(&format!("match {}", fmt_expr(&m.subject)));
                self.indent();
                for case in &m.cases {
                    self.line(&format!("case {}", fmt_pattern(&case.pattern)));
                    self.fmt_block_begin_end(&case.body);
                }
                if let Some(otherwise) = &m.otherwise {
                    self.line("otherwise");
                    self.fmt_block_begin_end(otherwise);
                }
                self.dedent();
            }
            Stmt::Assign(a) => {
                let op = match a.op {
                    AssignOp::Assign => "be",
                    AssignOp::AddAssign => "+=",
                    AssignOp::SubAssign => "-=",
                    AssignOp::MulAssign => "*=",
                    AssignOp::DivAssign => "/=",
                };
                self.line(&format!(
                    "{} {} {}",
                    fmt_expr(&a.target),
                    op,
                    fmt_expr(&a.value)
                ));
            }
            Stmt::Spawn(s) => self.line(&format!("spawn {}", s.actor.name)),
            Stmt::Send(s) => self.line(&format!("send {}", fmt_expr(&s.message))),
            Stmt::Receive(r) => {
                let binding = r
                    .binding
                    .as_ref()
                    .map(|b| format!(" {}", b.name))
                    .unwrap_or_default();
                self.line(&format!("receive{}", binding));
            }
            Stmt::Transaction(t) => {
                self.line("transaction");
                self.fmt_block_begin_end(&t.body);
                self.line("commit");
            }
            Stmt::Expr(e) => self.line(&fmt_expr(e)),
        }
    }

    fn fmt_repeat(&mut self, r: &RepeatStmt, parallel: bool) {
        let prefix = if parallel { "parallel " } else { "" };
        match r {
            RepeatStmt::ForEvery {
                var,
                iterable,
                body,
                ..
            } => {
                self.line(&format!(
                    "{}repeat for every {} {}",
                    prefix,
                    var.name,
                    fmt_expr(iterable)
                ));
                self.fmt_block_begin_end(body);
            }
            RepeatStmt::While {
                condition, body, ..
            } => {
                self.line(&format!("{}repeat while {}", prefix, fmt_expr(condition)));
                self.fmt_block_begin_end(body);
            }
            RepeatStmt::Count { times, body, .. } => {
                self.line(&format!("{}repeat {} times", prefix, fmt_expr(times)));
                self.fmt_block_begin_end(body);
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Expression and type formatters (pure functions — no mutable state)
// ─────────────────────────────────────────────────────────────────────────────

fn fmt_expr(expr: &Expr) -> String {
    match expr {
        Expr::Lit { value, .. } => match value {
            Literal::Int(i) => i.to_string(),
            Literal::Float(f) => f.to_string(),
            Literal::Bool(b) => b.to_string(),
            Literal::Text(s) => format!("\"{}\"", s),
            Literal::Unit => "()".into(),
        },
        Expr::Ident(id) => id.name.clone(),
        Expr::GenericInst { base, args, .. } => {
            let args_str = args.iter().map(fmt_type).collect::<Vec<_>>().join(", ");
            format!("{}<{}>", base.name, args_str)
        }
        Expr::Call { callee, args, .. } => {
            let args_str = args.iter().map(fmt_expr).collect::<Vec<_>>().join(", ");
            format!("{}({})", fmt_expr(callee), args_str)
        }
        Expr::BinOp {
            left, op, right, ..
        } => {
            let op_str = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Mod => "%",
                BinOp::Eq => "==",
                BinOp::NotEq => "!=",
                BinOp::Lt => "<",
                BinOp::Gt => ">",
                BinOp::LtEq => "<=",
                BinOp::GtEq => ">=",
                BinOp::And => "and",
                BinOp::Or => "or",
                BinOp::IsBelow => "is below",
                BinOp::IsAbove => "is above",
                BinOp::Exceeds => "exceeds",
            };
            format!("{} {} {}", fmt_expr(left), op_str, fmt_expr(right))
        }
        Expr::UnOp { op, operand, .. } => {
            let op_str = match op {
                UnOp::Neg => "-",
                UnOp::Not => "not ",
                UnOp::Deref => "deref ",
                UnOp::Borrow(true) => "borrow mutable ",
                UnOp::Borrow(false) => "borrow ",
            };
            format!("{}{}", op_str, fmt_expr(operand))
        }
        Expr::Field { object, field, .. } => {
            format!("{}.{}", fmt_expr(object), field.name)
        }
        Expr::Index { object, index, .. } => {
            format!("{}[{}]", fmt_expr(object), fmt_expr(index))
        }
        Expr::List { elements, .. } => {
            let inner = elements.iter().map(fmt_expr).collect::<Vec<_>>().join(", ");
            format!("[{}]", inner)
        }
        Expr::Block(_) => "( block )".into(),
        Expr::StructLit { ty, fields, .. } => {
            let fields_str = fields
                .iter()
                .map(|(id, expr)| format!("{}: {}", id.name, fmt_expr(expr)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{} {{ {} }}", fmt_expr(ty), fields_str)
        }
        Expr::MacroCall { name, args, .. } => {
            let args_str = args.iter().map(fmt_expr).collect::<Vec<_>>().join(", ");
            format!("{}!({})", name.name, args_str)
        }
        Expr::PostfixTry { inner, .. } => {
            format!("{}?", fmt_expr(inner))
        }
    }
}

fn fmt_type(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Named(id) => id.name.clone(),
        TypeExpr::List(t) => format!("List of {}", fmt_type(t)),
        TypeExpr::Dict { key, val } => {
            format!("Dictionary from {} to {}", fmt_type(key), fmt_type(val))
        }
        TypeExpr::Optional(t) => format!("{}?", fmt_type(t)),
        TypeExpr::Result(t) => format!("Result of {}", fmt_type(t)),
        TypeExpr::Generic { base, args } => {
            let inner = args.iter().map(fmt_type).collect::<Vec<_>>().join(", ");
            format!("{}<{}>", base.name, inner)
        }
        TypeExpr::Reference { mutable, inner } => {
            if *mutable {
                format!("borrow mutable {}", fmt_type(inner))
            } else {
                format!("borrow {}", fmt_type(inner))
            }
        }
    }
}

fn fmt_pattern(pat: &Pattern) -> String {
    match pat {
        Pattern::Constructor(id) => id.name.clone(),
        Pattern::Bind(id) => id.name.clone(),
        Pattern::Wildcard(_) => "_".into(),
        Pattern::Literal(Literal::Int(i)) => i.to_string(),
        Pattern::Literal(Literal::Bool(b)) => b.to_string(),
        Pattern::Literal(Literal::Text(s)) => format!("\"{}\"", s),
        Pattern::Literal(_) => "_".into(),
    }
}

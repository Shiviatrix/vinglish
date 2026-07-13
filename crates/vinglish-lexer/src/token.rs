/// Every token the Vinglish lexer can produce.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // ── Keywords ──────────────────────────────────────────────────────────────
    Let,
    Be, // `be`  (assignment / type hint)
    Function,
    Returns,
    Begin,
    End,
    If,
    Then,
    Otherwise,
    When,
    Repeat,
    For,
    Every,
    While,
    Match,
    Case,
    Parallel,
    Spawn,
    Send,
    Receive,
    Transaction,
    Commit,
    Compile,
    Use,
    Package,
    Module,
    Public,
    Private,
    Internal,
    Type,
    Requires,
    Effects,
    Foreign,
    Export,
    Using,
    Arena,
    On,
    Is,    // structural equality / identity keyword
    Below, // `is below`  →  <
    Above, // `is above`  →  >
    And,
    Or,
    Not,
    Borrow,
    Deref,
    Return,
    Import,
    Route,
    Of,   // `List of Number`
    From, // `Dictionary from Text to User`
    To,   // (same)
    Mutable,
    Exceeds,   // `exceeds`  →  >
    Unstable,  // reserved for medical domain DSL
    Recommend, // "recommend" keyword
    Prove,     // proof assistant hook
    Theorem,
    Induction,
    Every2, // `every frame` → loop variant (aliased)

    // ── Built-in type names ───────────────────────────────────────────────────
    Number,  // integer alias
    Decimal, // float alias
    Text,    // string alias
    Boolean, // bool alias

    // ── Literals ──────────────────────────────────────────────────────────────
    Integer(i64),
    Float(f64),
    StringLit(String),
    True,
    False,

    // ── Identifier ────────────────────────────────────────────────────────────
    Ident(String),

    // ── Operators ─────────────────────────────────────────────────────────────
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,       // ==
    NotEq,    // !=
    Lt,       // <
    Gt,       // >
    LtEq,     // <=
    GtEq,     // >=
    PlusEq,   // +=
    MinusEq,  // -=
    StarEq,   // *=
    SlashEq,  // /=
    Arrow,    // ->
    FatArrow, // =>

    // ── Punctuation ───────────────────────────────────────────────────────────
    Dot,
    Comma,
    Colon,
    Semicolon,
    QuestionMark,
    Bang,
    Pipe,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,

    // ── Structural ────────────────────────────────────────────────────────────
    Newline,
    Indent,
    Dedent,
    EOF,

    // ── (filtered before reaching parser) ─────────────────────────────────────
    Comment(String),
}

impl Token {
    /// Map a word to a keyword token, applying natural-language aliases.
    pub fn from_word(s: &str) -> Option<Token> {
        match s {
            "let" => Some(Token::Let),
            "be" => Some(Token::Be),
            "function" => Some(Token::Function),
            "returns" => Some(Token::Returns),
            "begin" => Some(Token::Begin),
            "end" => Some(Token::End),
            "if" => Some(Token::If),
            "then" => Some(Token::Then),
            "otherwise" => Some(Token::Otherwise),
            "when" => Some(Token::When),
            "repeat" => Some(Token::Repeat),
            "for" => Some(Token::For),
            "every" => Some(Token::Every),
            "while" => Some(Token::While),
            "match" => Some(Token::Match),
            "case" => Some(Token::Case),
            "parallel" => Some(Token::Parallel),
            "spawn" => Some(Token::Spawn),
            "send" => Some(Token::Send),
            "receive" => Some(Token::Receive),
            "transaction" => Some(Token::Transaction),
            "commit" => Some(Token::Commit),
            "compile" => Some(Token::Compile),
            "use" => Some(Token::Use),
            "package" => Some(Token::Package),
            "module" => Some(Token::Module),
            "public" => Some(Token::Public),
            "private" => Some(Token::Private),
            "internal" => Some(Token::Internal),
            "type" => Some(Token::Type),
            "requires" => Some(Token::Requires),
            "effects" => Some(Token::Effects),
            "foreign" => Some(Token::Foreign),
            "export" => Some(Token::Export),
            "using" => Some(Token::Using),
            "arena" => Some(Token::Arena),
            "on" => Some(Token::On),
            "is" => Some(Token::Is),
            "below" => Some(Token::Below),
            "above" => Some(Token::Above),
            "and" => Some(Token::And),
            "or" => Some(Token::Or),
            "not" => Some(Token::Not),
            "borrow" => Some(Token::Borrow),
            "deref" => Some(Token::Deref),
            "return" => Some(Token::Return),
            "import" => Some(Token::Import),
            "route" => Some(Token::Route),
            "of" => Some(Token::Of),
            "from" => Some(Token::From),
            "to" => Some(Token::To),
            "mutable" => Some(Token::Mutable),
            "exceeds" => Some(Token::Exceeds),
            "unstable" => Some(Token::Unstable),
            "recommend" => Some(Token::Recommend),
            "prove" => Some(Token::Prove),
            "theorem" => Some(Token::Theorem),
            "induction" => Some(Token::Induction),
            // Canonical type names
            "number" => Some(Token::Number),
            "decimal" => Some(Token::Decimal),
            "text" => Some(Token::Text),
            "boolean" => Some(Token::Boolean),
            // Literal words
            "true" => Some(Token::True),
            "false" => Some(Token::False),
            // Natural-language aliases — compiler treats these as equivalent
            "compute" | "calculate" | "determine" => Some(Token::Ident("calculate".to_string())),
            "modify" | "mutate" => Some(Token::Ident("mutate".to_string())),
            "create" | "make" => Some(Token::Ident("create".to_string())),
            "destroy" | "delete" => Some(Token::Ident("delete".to_string())),
            "show" | "display" | "print" | "println" => Some(Token::Ident(s.to_string())),
            _ => None,
        }
    }

    /// Human-readable description for use in error messages.
    pub fn describe(&self) -> &'static str {
        match self {
            Token::Let => "`let`",
            Token::Be => "`be`",
            Token::Function => "`function`",
            Token::Returns => "`returns`",
            Token::Begin => "`begin`",
            Token::End => "`end`",
            Token::If => "`if`",
            Token::Then => "`then`",
            Token::Otherwise => "`otherwise`",
            Token::When => "`when`",
            Token::Repeat => "`repeat`",
            Token::For => "`for`",
            Token::Every => "`every`",
            Token::While => "`while`",
            Token::Match => "`match`",
            Token::Case => "`case`",
            Token::Parallel => "`parallel`",
            Token::Spawn => "`spawn`",
            Token::Send => "`send`",
            Token::Receive => "`receive`",
            Token::Transaction => "`transaction`",
            Token::Commit => "`commit`",
            Token::Compile => "`compile`",
            Token::Use => "`use`",
            Token::Package => "`package`",
            Token::Module => "`module`",
            Token::Public => "`public`",
            Token::Private => "`private`",
            Token::Internal => "`internal`",
            Token::Type => "`type`",
            Token::Requires => "`requires`",
            Token::Effects => "`effects`",
            Token::Foreign => "`foreign`",
            Token::Export => "`export`",
            Token::Using => "`using`",
            Token::Arena => "`arena`",
            Token::On => "`on`",
            Token::Is => "`is`",
            Token::Below => "`below`",
            Token::Above => "`above`",
            Token::And => "`and`",
            Token::Or => "`or`",
            Token::Not => "`not`",
            Token::Borrow => "`borrow`",
            Token::Deref => "`deref`",
            Token::Return => "`return`",
            Token::Import => "`import`",
            Token::Route => "`route`",
            Token::Of => "`of`",
            Token::From => "`from`",
            Token::To => "`to`",
            Token::Mutable => "`mutable`",
            Token::Exceeds => "`exceeds`",
            Token::Unstable => "`unstable`",
            Token::Recommend => "`recommend`",
            Token::Prove => "`prove`",
            Token::Theorem => "`theorem`",
            Token::Induction => "`induction`",
            Token::Every2 => "`every`",
            Token::Number => "`number`",
            Token::Decimal => "`decimal`",
            Token::Text => "`text`",
            Token::Boolean => "`boolean`",
            Token::Integer(_) => "integer literal",
            Token::Float(_) => "float literal",
            Token::StringLit(_) => "string literal",
            Token::True => "`true`",
            Token::False => "`false`",
            Token::Ident(_) => "identifier",
            Token::Plus => "`+`",
            Token::Minus => "`-`",
            Token::Star => "`*`",
            Token::Slash => "`/`",
            Token::Percent => "`%`",
            Token::Eq => "`==`",
            Token::NotEq => "`!=`",
            Token::Lt => "`<`",
            Token::Gt => "`>`",
            Token::LtEq => "`<=`",
            Token::GtEq => "`>=`",
            Token::PlusEq => "`+=`",
            Token::MinusEq => "`-=`",
            Token::StarEq => "`*=`",
            Token::SlashEq => "`/=`",
            Token::Arrow => "`->`",
            Token::FatArrow => "`=>`",
            Token::Dot => "`.`",
            Token::Comma => "`,`",
            Token::Colon => "`:`",
            Token::Semicolon => "`;`",
            Token::QuestionMark => "`?`",
            Token::Bang => "`!`",
            Token::Pipe => "`|`",
            Token::LParen => "`(`",
            Token::RParen => "`)`",
            Token::LBracket => "`[`",
            Token::RBracket => "`]`",
            Token::LBrace => "`{`",
            Token::RBrace => "`}`",
            Token::Newline => "newline",
            Token::Indent => "indent",
            Token::Dedent => "dedent",
            Token::EOF => "end of file",
            Token::Comment(_) => "comment",
        }
    }
}

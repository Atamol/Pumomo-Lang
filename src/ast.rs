use crate::pumomo::Ins;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Clone, Copy, Debug)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Clone, Copy, Debug)]
pub enum Sink {
    /// 最高出力を
    Out,
    /// 豚キムチ
    Bare,
    /// お尻の穴が
    Err,
}

#[derive(Clone, Debug)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Pumomo,
    Array(Vec<Expr>),
    Ident(String),
    Index(Box<Expr>, Box<Expr>),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    /// おちんちんくださぁい？
    ReadLine,
    /// いっぱいちょうだい？
    ReadAll,
    /// 完全合体合金おちんちん ... ムッキュ
    Gattai { args: Vec<Expr>, body: Vec<Ins> },
}

#[derive(Clone, Debug)]
pub enum Target {
    Var(String),
    Index(Expr, Expr),
}

#[derive(Clone, Debug)]
pub struct Stmt {
    pub line: usize,
    pub kind: StmtKind,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
    Let(String, Expr),
    Const(String, Expr),
    Assign(Target, Expr),
    Say(Sink, Expr),
    If {
        arms: Vec<(Expr, Vec<Stmt>)>,
        otherwise: Option<Vec<Stmt>>,
    },
    While(Expr, Vec<Stmt>),
    For {
        var: String,
        from: Expr,
        to: Expr,
        body: Vec<Stmt>,
    },
    ForEach {
        var: String,
        iter: Expr,
        body: Vec<Stmt>,
    },
    Loop(Vec<Stmt>),
    Break,
    Continue,
    Return(Option<Expr>),
    Assert(Expr),
    Exit(Option<Expr>),
    Panic(Option<Expr>),
    Throw(Expr),
    Try {
        body: Vec<Stmt>,
        catch_var: Option<String>,
        catch_body: Vec<Stmt>,
    },
    Expr(Expr),
}

#[derive(Clone, Debug)]
pub struct Func {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub struct Program {
    pub funcs: Vec<Func>,
}

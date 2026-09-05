use crate::ast::*;
use crate::error::{PResult, PumomoError};
use crate::lexer::{PHRASE, Tok, Token};

pub fn parse(toks: Vec<Token>) -> PResult<Program> {
    Parser { t: toks, i: 0 }.program()
}

struct Parser {
    t: Vec<Token>,
    i: usize,
}

fn closes_block(t: &Tok) -> bool {
    matches!(t, Tok::End | Tok::ElseIf | Tok::Else | Tok::Catch | Tok::Eof)
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.t[self.i.min(self.t.len() - 1)].kind
    }

    fn line(&self) -> usize {
        self.t[self.i.min(self.t.len() - 1)].line
    }

    fn next(&mut self) -> Tok {
        let k = self.t[self.i.min(self.t.len() - 1)].kind.clone();
        if self.i < self.t.len() - 1 {
            self.i += 1;
        }
        k
    }

    fn eat(&mut self, want: &Tok) -> bool {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(want) {
            self.next();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, want: &Tok, hint: &str) -> PResult<Tok> {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(want) {
            Ok(self.next())
        } else {
            Err(PumomoError::syntax(
                self.line(),
                format!("{hint} が要る．あるのは「{}」", self.peek().describe()),
            ))
        }
    }

    fn ident(&mut self, hint: &str) -> PResult<String> {
        match self.next() {
            Tok::Ident(s) => Ok(s),
            other => Err(PumomoError::syntax(
                self.line(),
                format!("{hint} には名前が要る．あるのは「{}」", other.describe()),
            )),
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Tok::Newline) {
            self.next();
        }
    }

    fn program(&mut self) -> PResult<Program> {
        self.skip_newlines();
        if !self.eat(&Tok::Incantation) {
            return Err(PumomoError::syntax(
                self.line(),
                format!("まず喘げ．ファイルの頭に {PHRASE} が要る"),
            ));
        }
        let mut funcs = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Tok::Eof) {
                break;
            }
            if !self.eat(&Tok::Kanjiro) {
                return Err(PumomoError::syntax(
                    self.line(),
                    format!(
                        "外に出せるのは 緩次郎 だけ．あるのは「{}」",
                        self.peek().describe()
                    ),
                ));
            }
            funcs.push(self.func()?);
        }
        Ok(Program { funcs })
    }

    fn func(&mut self) -> PResult<Func> {
        let line = self.line();
        let name = self.ident("緩次郎")?;
        let mut params = Vec::new();
        if self.eat(&Tok::LParen) {
            while !self.eat(&Tok::RParen) {
                params.push(self.ident("引数")?);
                if !self.eat(&Tok::Comma) {
                    self.expect(&Tok::RParen, "引数を閉じる )")?;
                    break;
                }
            }
        }
        let body = self.block()?;
        self.expect(&Tok::End, "ムッキュ")?;
        Ok(Func { name, params, body, line })
    }

    /// Leaves the block terminator for the caller to consume
    fn block(&mut self) -> PResult<Vec<Stmt>> {
        let mut out = Vec::new();
        loop {
            self.skip_newlines();
            if closes_block(self.peek()) {
                break;
            }
            out.push(self.statement()?);
        }
        Ok(out)
    }

    fn end_of_stmt(&mut self) -> PResult<()> {
        if matches!(self.peek(), Tok::Newline) {
            self.next();
            return Ok(());
        }
        if closes_block(self.peek()) {
            return Ok(());
        }
        Err(PumomoError::syntax(
            self.line(),
            format!(
                "1行に文が2つある．「{}」の前で改行しろ",
                self.peek().describe()
            ),
        ))
    }

    fn bare(&self) -> bool {
        matches!(self.peek(), Tok::Newline) || closes_block(self.peek())
    }

    fn statement(&mut self) -> PResult<Stmt> {
        let line = self.line();
        let kind = match self.peek() {
            Tok::Let => {
                self.next();
                let name = self.ident("いっぱい入っちゃう")?;
                self.expect(&Tok::Assign, "=")?;
                let v = self.expr()?;
                self.end_of_stmt()?;
                StmtKind::Let(name, v)
            }
            Tok::Const => {
                self.next();
                let name = self.ident("おまんこロックマン")?;
                self.expect(&Tok::Assign, "=")?;
                let v = self.expr()?;
                self.end_of_stmt()?;
                StmtKind::Const(name, v)
            }
            Tok::Println | Tok::Print | Tok::Stderr => {
                let sink = match self.next() {
                    Tok::Println => Sink::Out,
                    Tok::Print => Sink::Bare,
                    _ => Sink::Err,
                };
                let v = self.expr()?;
                self.end_of_stmt()?;
                StmtKind::Say(sink, v)
            }
            Tok::Break => {
                self.next();
                self.end_of_stmt()?;
                StmtKind::Break
            }
            Tok::Continue => {
                self.next();
                self.end_of_stmt()?;
                StmtKind::Continue
            }
            Tok::Return => {
                self.next();
                let v = if self.bare() { None } else { Some(self.expr()?) };
                self.end_of_stmt()?;
                StmtKind::Return(v)
            }
            Tok::Assert => {
                self.next();
                let v = self.expr()?;
                self.end_of_stmt()?;
                StmtKind::Assert(v)
            }
            Tok::Exit => {
                self.next();
                let v = if self.bare() { None } else { Some(self.expr()?) };
                self.end_of_stmt()?;
                StmtKind::Exit(v)
            }
            Tok::Panic => {
                self.next();
                let v = if self.bare() { None } else { Some(self.expr()?) };
                self.end_of_stmt()?;
                StmtKind::Panic(v)
            }
            Tok::Throw => {
                self.next();
                let v = self.expr()?;
                self.end_of_stmt()?;
                StmtKind::Throw(v)
            }
            Tok::If => return self.if_stmt(),
            Tok::While => return self.while_stmt(),
            Tok::For => return self.for_stmt(),
            Tok::ForEach => return self.foreach_stmt(),
            Tok::Loop => return self.loop_stmt(),
            Tok::Try => return self.try_stmt(),
            _ => {
                let e = self.expr()?;
                if self.eat(&Tok::Assign) {
                    let target = match e {
                        Expr::Ident(n) => Target::Var(n),
                        Expr::Index(a, b) => Target::Index(*a, *b),
                        _ => return Err(PumomoError::syntax(line, "そこには代入できない")),
                    };
                    let v = self.expr()?;
                    self.end_of_stmt()?;
                    StmtKind::Assign(target, v)
                } else {
                    self.end_of_stmt()?;
                    StmtKind::Expr(e)
                }
            }
        };
        Ok(Stmt { line, kind })
    }

    fn if_stmt(&mut self) -> PResult<Stmt> {
        let line = self.line();
        self.expect(&Tok::If, "ドーラ")?;
        let mut arms = vec![(self.expr()?, self.block()?)];
        while self.eat(&Tok::ElseIf) {
            let cond = self.expr()?;
            arms.push((cond, self.block()?));
        }
        let otherwise = if self.eat(&Tok::Else) {
            Some(self.block()?)
        } else {
            None
        };
        self.expect(&Tok::End, "ドーラ を閉じる ムッキュ")?;
        Ok(Stmt { line, kind: StmtKind::If { arms, otherwise } })
    }

    fn while_stmt(&mut self) -> PResult<Stmt> {
        let line = self.line();
        self.expect(&Tok::While, "動いてる中で")?;
        let cond = self.expr()?;
        let body = self.block()?;
        self.expect(&Tok::End, "動いてる中で を閉じる ムッキュ")?;
        Ok(Stmt { line, kind: StmtKind::While(cond, body) })
    }

    fn for_stmt(&mut self) -> PResult<Stmt> {
        let line = self.line();
        self.expect(&Tok::For, "いっぱいいっぱいゆうじろう")?;
        let var = self.ident("いっぱいいっぱいゆうじろう")?;
        self.expect(&Tok::Assign, "=")?;
        let from = self.expr()?;
        self.expect(&Tok::DotDot, "..")?;
        let to = self.expr()?;
        let body = self.block()?;
        self.expect(&Tok::End, "いっぱいいっぱいゆうじろう を閉じる ムッキュ")?;
        Ok(Stmt { line, kind: StmtKind::For { var, from, to, body } })
    }

    fn foreach_stmt(&mut self) -> PResult<Stmt> {
        let line = self.line();
        self.expect(&Tok::ForEach, "Delight! Beast! おまんこの中でEasy! Easy!")?;
        let var = self.ident("Delight! Beast! おまんこの中でEasy! Easy!")?;
        self.expect(&Tok::Assign, "=")?;
        let iter = self.expr()?;
        let body = self.block()?;
        self.expect(&Tok::End, "Delight! Beast! おまんこの中でEasy! Easy! を閉じる ムッキュ")?;
        Ok(Stmt { line, kind: StmtKind::ForEach { var, iter, body } })
    }

    fn loop_stmt(&mut self) -> PResult<Stmt> {
        let line = self.line();
        self.expect(&Tok::Loop, "ジャッキーチェン")?;
        let body = self.block()?;
        self.expect(&Tok::End, "ジャッキーチェン を閉じる ムッキュ")?;
        Ok(Stmt { line, kind: StmtKind::Loop(body) })
    }

    fn try_stmt(&mut self) -> PResult<Stmt> {
        let line = self.line();
        self.expect(&Tok::Try, "細菌入り味噌")?;
        let body = self.block()?;
        self.expect(&Tok::Catch, "オ゛ア゛ッ゛")?;
        let catch_var = match self.peek() {
            Tok::Ident(_) => Some(self.ident("オ゛ア゛ッ゛")?),
            _ => None,
        };
        let catch_body = self.block()?;
        self.expect(&Tok::End, "細菌入り味噌 を閉じる ムッキュ")?;
        Ok(Stmt { line, kind: StmtKind::Try { body, catch_var, catch_body } })
    }

    fn expr(&mut self) -> PResult<Expr> {
        self.binary(0)
    }

    fn binary(&mut self, min: u8) -> PResult<Expr> {
        let mut lhs = self.unary()?;
        loop {
            let Some((op, prec)) = binop(self.peek()) else { break };
            if prec < min {
                break;
            }
            self.next();
            let rhs = self.binary(prec + 1)?;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> PResult<Expr> {
        match self.peek() {
            Tok::Minus => {
                self.next();
                Ok(Expr::Unary(UnOp::Neg, Box::new(self.unary()?)))
            }
            Tok::Bang => {
                self.next();
                Ok(Expr::Unary(UnOp::Not, Box::new(self.unary()?)))
            }
            _ => self.postfix(),
        }
    }

    fn postfix(&mut self) -> PResult<Expr> {
        let mut e = self.primary()?;
        while self.eat(&Tok::LBracket) {
            let idx = self.expr()?;
            self.expect(&Tok::RBracket, "]")?;
            e = Expr::Index(Box::new(e), Box::new(idx));
        }
        Ok(e)
    }

    fn args(&mut self) -> PResult<Vec<Expr>> {
        let mut args = Vec::new();
        while !self.eat(&Tok::RParen) {
            args.push(self.expr()?);
            if !self.eat(&Tok::Comma) {
                self.expect(&Tok::RParen, "引数を閉じる )")?;
                break;
            }
        }
        Ok(args)
    }

    fn primary(&mut self) -> PResult<Expr> {
        let line = self.line();
        match self.next() {
            Tok::Int(n) => Ok(Expr::Int(n)),
            Tok::Float(x) => Ok(Expr::Float(x)),
            Tok::Str(s) => Ok(Expr::Str(s)),
            Tok::True => Ok(Expr::Bool(true)),
            Tok::False => Ok(Expr::Bool(false)),
            Tok::Null => Ok(Expr::Pumomo),
            Tok::ReadLine => Ok(Expr::ReadLine),
            Tok::ReadAll => Ok(Expr::ReadAll),
            Tok::Ident(name) => {
                if self.eat(&Tok::LParen) {
                    Ok(Expr::Call(name, self.args()?))
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            Tok::LParen => {
                let e = self.expr()?;
                self.expect(&Tok::RParen, ")")?;
                Ok(e)
            }
            Tok::LBracket => {
                let mut items = Vec::new();
                while !self.eat(&Tok::RBracket) {
                    items.push(self.expr()?);
                    if !self.eat(&Tok::Comma) {
                        self.expect(&Tok::RBracket, "]")?;
                        break;
                    }
                }
                Ok(Expr::Array(items))
            }
            Tok::Gattai => {
                let args = if self.eat(&Tok::LParen) {
                    self.args()?
                } else {
                    Vec::new()
                };
                match self.next() {
                    Tok::GattaiBody(body) => Ok(Expr::Gattai { args, body }),
                    _ => Err(PumomoError::syntax(line, "完全合体合金おちんちん の中身が読めない")),
                }
            }
            other => Err(PumomoError::syntax(
                line,
                format!("式が要る．あるのは「{}」", other.describe()),
            )),
        }
    }
}

fn binop(t: &Tok) -> Option<(BinOp, u8)> {
    Some(match t {
        Tok::OrOr => (BinOp::Or, 1),
        Tok::AndAnd => (BinOp::And, 2),
        Tok::Eq => (BinOp::Eq, 3),
        Tok::Ne => (BinOp::Ne, 3),
        Tok::Lt => (BinOp::Lt, 4),
        Tok::Le => (BinOp::Le, 4),
        Tok::Gt => (BinOp::Gt, 4),
        Tok::Ge => (BinOp::Ge, 4),
        Tok::Plus => (BinOp::Add, 5),
        Tok::Minus => (BinOp::Sub, 5),
        Tok::Star => (BinOp::Mul, 6),
        Tok::Slash => (BinOp::Div, 6),
        Tok::Percent => (BinOp::Mod, 6),
        _ => return None,
    })
}

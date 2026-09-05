use crate::ast::*;
use crate::error::{ErrKind, PResult, PumomoError};
use crate::pumomo;
use crate::value::Value;
use std::collections::{HashMap, VecDeque};
use std::io::{BufWriter, Write, stdin, stdout};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_DEPTH: usize = 512;

enum Flow {
    Normal,
    Break,
    Continue,
    Return(Value),
}

#[derive(Clone)]
struct Slot {
    v: Value,
    konst: bool,
}

struct IoState {
    out: BufWriter<std::io::Stdout>,
    pending: VecDeque<char>,
    eof: bool,
}

impl IoState {
    fn fill(&mut self) -> bool {
        if !self.pending.is_empty() {
            return true;
        }
        if self.eof {
            return false;
        }
        let _ = self.out.flush();
        let mut line = String::new();
        match stdin().read_line(&mut line) {
            Ok(0) | Err(_) => {
                self.eof = true;
                false
            }
            Ok(_) => {
                self.pending.extend(line.chars());
                true
            }
        }
    }

    fn read_line(&mut self) -> Option<String> {
        if !self.fill() {
            return None;
        }
        let mut s = String::new();
        while let Some(c) = self.pending.pop_front() {
            if c == '\n' {
                break;
            }
            s.push(c);
        }
        Some(s.trim_end_matches('\r').to_string())
    }

    fn read_all(&mut self) -> String {
        let mut s: String = self.pending.drain(..).collect();
        let _ = self.out.flush();
        let _ = std::io::Read::read_to_string(&mut stdin(), &mut s);
        self.eof = true;
        s
    }
}

impl pumomo::Io for IoState {
    fn out(&mut self, s: &str) {
        let _ = self.out.write_all(s.as_bytes());
    }

    fn read_char(&mut self) -> Option<char> {
        if !self.fill() {
            return None;
        }
        self.pending.pop_front()
    }
}

pub struct Interp {
    funcs: HashMap<String, Func>,
    scopes: Vec<HashMap<String, Slot>>,
    base: usize,
    depth: usize,
    io: IoState,
}

pub const ENTRY: &str = "ガバ穴ダディー";

pub fn run(program: &Program) -> PResult<i32> {
    let mut funcs = HashMap::new();
    for f in &program.funcs {
        if funcs.insert(f.name.clone(), f.clone()).is_some() {
            return Err(PumomoError::runtime(
                f.line,
                format!("緩次郎 {} が二人いる", f.name),
            ));
        }
    }
    if !funcs.contains_key(ENTRY) {
        return Err(PumomoError::runtime(
            0,
            format!("{ENTRY} がいない．緩次郎 {ENTRY}() を書け"),
        ));
    }
    let mut it = Interp {
        funcs,
        scopes: vec![HashMap::new()],
        base: 0,
        depth: 0,
        io: IoState {
            out: BufWriter::new(stdout()),
            pending: VecDeque::new(),
            eof: false,
        },
    };
    let r = it.call(ENTRY, Vec::new(), 0);
    let _ = it.io.out.flush();
    match r {
        Ok(_) => Ok(0),
        Err(e) => match e.kind {
            ErrKind::Exit(code) => Ok(code),
            _ => Err(e),
        },
    }
}

impl Interp {
    fn say(&mut self, sink: Sink, s: &str) {
        match sink {
            Sink::Out => {
                let _ = writeln!(self.io.out, "{s}");
            }
            Sink::Bare => {
                let _ = write!(self.io.out, "{s}");
            }
            Sink::Err => {
                let _ = self.io.out.flush();
                eprintln!("{s}");
            }
        }
    }

    fn lookup(&self, name: &str) -> Option<&Slot> {
        self.scopes[self.base..].iter().rev().find_map(|s| s.get(name))
    }

    fn declare(&mut self, name: String, v: Value, konst: bool) {
        self.scopes.last_mut().unwrap().insert(name, Slot { v, konst });
    }

    fn assign(&mut self, name: &str, v: Value, line: usize) -> PResult<()> {
        for s in self.scopes[self.base..].iter_mut().rev() {
            if let Some(slot) = s.get_mut(name) {
                if slot.konst {
                    return Err(PumomoError::runtime(
                        line,
                        format!("{name} は おまんこロックマン なので動かせない"),
                    ));
                }
                slot.v = v;
                return Ok(());
            }
        }
        Err(PumomoError::runtime(
            line,
            format!("{name} は穴が空いている．まず いっぱい入っちゃう {name} = ..."),
        ))
    }

    fn call(&mut self, name: &str, args: Vec<Value>, line: usize) -> PResult<Value> {
        let Some(f) = self.funcs.get(name).cloned() else {
            return Err(PumomoError::runtime(line, format!("緩次郎 {name} はいない")));
        };
        if args.len() != f.params.len() {
            return Err(PumomoError::runtime(
                line,
                format!(
                    "{name} は引数 {} 個．渡したのは {} 個",
                    f.params.len(),
                    args.len()
                ),
            ));
        }
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            self.depth -= 1;
            return Err(PumomoError::runtime(line, "ガバ穴が深すぎる (再帰しすぎ)"));
        }
        let saved_base = self.base;
        let mut frame = HashMap::new();
        for (p, a) in f.params.iter().zip(args) {
            frame.insert(p.clone(), Slot { v: a, konst: false });
        }
        self.base = self.scopes.len();
        self.scopes.push(frame);

        let flow = self.exec_block(&f.body);

        self.scopes.truncate(self.base);
        self.base = saved_base;
        self.depth -= 1;

        match flow? {
            Flow::Return(v) => Ok(v),
            _ => Ok(Value::Pumomo),
        }
    }

    fn exec_block(&mut self, stmts: &[Stmt]) -> PResult<Flow> {
        self.scopes.push(HashMap::new());
        let mut result = Ok(Flow::Normal);
        for s in stmts {
            match self.exec(s) {
                Ok(Flow::Normal) => {}
                other => {
                    result = other;
                    break;
                }
            }
        }
        self.scopes.pop();
        result
    }

    /// Runs one lap, false means stop and out carries a pending Return
    fn spin(&mut self, body: &[Stmt], out: &mut Option<Flow>) -> PResult<bool> {
        match self.exec_block(body)? {
            Flow::Break => Ok(false),
            Flow::Return(v) => {
                *out = Some(Flow::Return(v));
                Ok(false)
            }
            _ => Ok(true),
        }
    }

    fn exec(&mut self, s: &Stmt) -> PResult<Flow> {
        let line = s.line;
        match &s.kind {
            StmtKind::Let(name, e) => {
                let v = self.eval(e, line)?;
                self.declare(name.clone(), v, false);
            }
            StmtKind::Const(name, e) => {
                let v = self.eval(e, line)?;
                self.declare(name.clone(), v, true);
            }
            StmtKind::Assign(target, e) => {
                let v = self.eval(e, line)?;
                match target {
                    Target::Var(name) => self.assign(name, v, line)?,
                    Target::Index(base, idx) => {
                        let b = self.eval(base, line)?;
                        let i = self.eval(idx, line)?;
                        let Value::Array(a) = b else {
                            return Err(PumomoError::runtime(
                                line,
                                format!("{} には入らない", b.type_name()),
                            ));
                        };
                        let i = as_index(&i, line)?;
                        let mut a = a.borrow_mut();
                        if i >= a.len() {
                            return Err(PumomoError::runtime(
                                line,
                                format!("{i} 番目は無い (長さ {})", a.len()),
                            ));
                        }
                        a[i] = v;
                    }
                }
            }
            StmtKind::Say(sink, e) => {
                let v = self.eval(e, line)?;
                let s = v.to_string();
                self.say(*sink, &s);
            }
            StmtKind::If { arms, otherwise } => {
                for (cond, body) in arms {
                    if self.eval(cond, line)?.truthy() {
                        return self.exec_block(body);
                    }
                }
                if let Some(body) = otherwise {
                    return self.exec_block(body);
                }
            }
            StmtKind::While(cond, body) => {
                let mut out = None;
                while self.eval(cond, line)?.truthy() {
                    if !self.spin(body, &mut out)? {
                        break;
                    }
                }
                if let Some(f) = out {
                    return Ok(f);
                }
            }
            StmtKind::Loop(body) => {
                let mut out = None;
                while self.spin(body, &mut out)? {}
                if let Some(f) = out {
                    return Ok(f);
                }
            }
            StmtKind::For { var, from, to, body } => {
                let from = as_int(&self.eval(from, line)?, line)?;
                let to = as_int(&self.eval(to, line)?, line)?;
                let mut out = None;
                let mut i = from;
                while i <= to {
                    self.scopes.push(HashMap::new());
                    self.declare(var.clone(), Value::Int(i), false);
                    let go = self.spin(body, &mut out);
                    self.scopes.pop();
                    if !go? {
                        break;
                    }
                    i += 1;
                }
                if let Some(f) = out {
                    return Ok(f);
                }
            }
            StmtKind::ForEach { var, iter, body } => {
                let items = match self.eval(iter, line)? {
                    Value::Array(a) => a.borrow().clone(),
                    Value::Str(s) => s.chars().map(|c| Value::str(c.to_string())).collect(),
                    other => {
                        return Err(PumomoError::runtime(
                            line,
                            format!("{} の中には入れない", other.type_name()),
                        ));
                    }
                };
                let mut out = None;
                for it in items {
                    self.scopes.push(HashMap::new());
                    self.declare(var.clone(), it, false);
                    let go = self.spin(body, &mut out);
                    self.scopes.pop();
                    if !go? {
                        break;
                    }
                }
                if let Some(f) = out {
                    return Ok(f);
                }
            }
            StmtKind::Break => return Ok(Flow::Break),
            StmtKind::Continue => return Ok(Flow::Continue),
            StmtKind::Return(e) => {
                let v = match e {
                    Some(e) => self.eval(e, line)?,
                    None => Value::Pumomo,
                };
                return Ok(Flow::Return(v));
            }
            StmtKind::Assert(e) => {
                if !self.eval(e, line)?.truthy() {
                    return Err(PumomoError::runtime(line, "ムーア！我慢できぬ (条件が破れた)"));
                }
            }
            StmtKind::Exit(e) => {
                let code = match e {
                    Some(e) => as_int(&self.eval(e, line)?, line)? as i32,
                    None => 0,
                };
                return Err(PumomoError::exit(code));
            }
            StmtKind::Panic(e) => {
                let msg = match e {
                    Some(e) => self.eval(e, line)?.to_string(),
                    None => "もう嫌...".to_string(),
                };
                return Err(PumomoError::panic(line, msg));
            }
            StmtKind::Throw(e) => {
                let v = self.eval(e, line)?;
                return Err(PumomoError::thrown(line, v));
            }
            StmtKind::Try { body, catch_var, catch_body } => match self.exec_block(body) {
                Err(e) if e.catchable() => {
                    self.scopes.push(HashMap::new());
                    if let Some(name) = catch_var {
                        let v = e.as_value();
                        self.declare(name.clone(), v, false);
                    }
                    let flow = self.exec_block(catch_body);
                    self.scopes.pop();
                    return flow;
                }
                other => return other,
            },
            StmtKind::Expr(e) => {
                self.eval(e, line)?;
            }
        }
        Ok(Flow::Normal)
    }

    fn eval(&mut self, e: &Expr, line: usize) -> PResult<Value> {
        Ok(match e {
            Expr::Int(n) => Value::Int(*n),
            Expr::Float(x) => Value::Float(*x),
            Expr::Str(s) => Value::str(s.clone()),
            Expr::Bool(b) => Value::Bool(*b),
            Expr::Pumomo => Value::Pumomo,
            Expr::Array(items) => {
                let mut v = Vec::with_capacity(items.len());
                for it in items {
                    v.push(self.eval(it, line)?);
                }
                Value::array(v)
            }
            Expr::Ident(name) => self.lookup(name).map(|s| s.v.clone()).ok_or_else(|| {
                PumomoError::runtime(line, format!("{name} は穴が空いている (未定義)"))
            })?,
            Expr::Index(base, idx) => {
                let b = self.eval(base, line)?;
                let i = self.eval(idx, line)?;
                let i = as_index(&i, line)?;
                match b {
                    Value::Array(a) => a.borrow().get(i).cloned().ok_or_else(|| {
                        PumomoError::runtime(
                            line,
                            format!("{i} 番目は無い (長さ {})", a.borrow().len()),
                        )
                    })?,
                    Value::Str(s) => s
                        .chars()
                        .nth(i)
                        .map(|c| Value::str(c.to_string()))
                        .ok_or_else(|| PumomoError::runtime(line, format!("{i} 文字目は無い")))?,
                    other => {
                        return Err(PumomoError::runtime(
                            line,
                            format!("{} は添字で引けない", other.type_name()),
                        ));
                    }
                }
            }
            Expr::Unary(op, inner) => {
                let v = self.eval(inner, line)?;
                match op {
                    UnOp::Not => Value::Bool(!v.truthy()),
                    UnOp::Neg => match v {
                        Value::Int(n) => Value::Int(-n),
                        Value::Float(x) => Value::Float(-x),
                        other => {
                            return Err(PumomoError::runtime(
                                line,
                                format!("{} には符号が付かない", other.type_name()),
                            ));
                        }
                    },
                }
            }
            Expr::Binary(op, a, b) => {
                match op {
                    BinOp::And => {
                        let l = self.eval(a, line)?;
                        return Ok(Value::Bool(l.truthy() && self.eval(b, line)?.truthy()));
                    }
                    BinOp::Or => {
                        let l = self.eval(a, line)?;
                        return Ok(Value::Bool(l.truthy() || self.eval(b, line)?.truthy()));
                    }
                    _ => {}
                }
                let l = self.eval(a, line)?;
                let r = self.eval(b, line)?;
                binary(*op, l, r, line)?
            }
            Expr::Call(name, args) => {
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval(a, line)?);
                }
                if self.funcs.contains_key(name) {
                    self.call(name, vals, line)?
                } else {
                    builtin(name, vals, line)?
                }
            }
            Expr::ReadLine => match self.io.read_line() {
                Some(s) => Value::str(s),
                None => Value::Pumomo,
            },
            Expr::ReadAll => Value::str(self.io.read_all()),
            Expr::Gattai { args, body } => {
                let mut stack = Vec::new();
                for a in args {
                    let v = self.eval(a, line)?;
                    stack.push(as_int(&v, line)?);
                }
                pumomo::execute(body, &mut stack, &mut self.io)?;
                match stack.pop() {
                    Some(n) => Value::Int(n),
                    None => Value::Pumomo,
                }
            }
        })
    }
}

fn as_int(v: &Value, line: usize) -> PResult<i64> {
    match v {
        Value::Int(n) => Ok(*n),
        Value::Float(x) => Ok(*x as i64),
        Value::Bool(b) => Ok(*b as i64),
        other => Err(PumomoError::runtime(
            line,
            format!("{} は数にならない", other.type_name()),
        )),
    }
}

fn as_index(v: &Value, line: usize) -> PResult<usize> {
    let n = as_int(v, line)?;
    usize::try_from(n).map_err(|_| PumomoError::runtime(line, format!("{n} 番目は無い")))
}

fn num_pair(a: &Value, b: &Value) -> Option<(f64, f64)> {
    let f = |v: &Value| match v {
        Value::Int(n) => Some(*n as f64),
        Value::Float(x) => Some(*x),
        _ => None,
    };
    Some((f(a)?, f(b)?))
}

fn binary(op: BinOp, l: Value, r: Value, line: usize) -> PResult<Value> {
    use BinOp::*;
    match op {
        Eq => return Ok(Value::Bool(l.eq(&r))),
        Ne => return Ok(Value::Bool(!l.eq(&r))),
        _ => {}
    }
    if op == Add && (matches!(l, Value::Str(_)) || matches!(r, Value::Str(_))) {
        return Ok(Value::str(format!("{l}{r}")));
    }
    if matches!(op, Lt | Le | Gt | Ge)
        && let (Value::Str(a), Value::Str(b)) = (&l, &r)
    {
        let c = a.cmp(b);
        return Ok(Value::Bool(match op {
            Lt => c.is_lt(),
            Le => c.is_le(),
            Gt => c.is_gt(),
            _ => c.is_ge(),
        }));
    }
    if let (Value::Int(a), Value::Int(b)) = (&l, &r) {
        let (a, b) = (*a, *b);
        return Ok(match op {
            Add => Value::Int(a + b),
            Sub => Value::Int(a - b),
            Mul => Value::Int(a * b),
            Div | Mod if b == 0 => {
                return Err(PumomoError::runtime(line, "0 で割った．穴が無い"));
            }
            Div => Value::Int(a.div_euclid(b)),
            Mod => Value::Int(a.rem_euclid(b)),
            Lt => Value::Bool(a < b),
            Le => Value::Bool(a <= b),
            Gt => Value::Bool(a > b),
            Ge => Value::Bool(a >= b),
            _ => unreachable!(),
        });
    }
    let Some((a, b)) = num_pair(&l, &r) else {
        return Err(PumomoError::runtime(
            line,
            format!("{} と {} は混ざらない", l.type_name(), r.type_name()),
        ));
    };
    Ok(match op {
        Add => Value::Float(a + b),
        Sub => Value::Float(a - b),
        Mul => Value::Float(a * b),
        Div if b == 0.0 => return Err(PumomoError::runtime(line, "0 で割った．穴が無い")),
        Div => Value::Float(a / b),
        Mod if b == 0.0 => return Err(PumomoError::runtime(line, "0 で割った余りは無い")),
        Mod => Value::Float(a.rem_euclid(b)),
        Lt => Value::Bool(a < b),
        Le => Value::Bool(a <= b),
        Gt => Value::Bool(a > b),
        Ge => Value::Bool(a >= b),
        _ => unreachable!(),
    })
}

pub const BUILTINS: &[(&str, &str)] = &[
    ("いっぱいガラム", "ガバ穴 / 文字列の長さ"),
    ("おちんちん喋らせてくださぁい？", "文字列にする"),
    ("ペロペロ舐め値", "数にする"),
    ("入っちゃう入っちゃう入っちゃう", "ガバ穴 の末尾に足す"),
    ("後ろ後ろが", "ガバ穴 の末尾を抜く"),
    ("中が気持ちいいモコモコ", "中にあるか調べる"),
    ("いっぱいギロチン", "切り出す (両端を含む)"),
    ("一揆しちゃったぁ", "ひっくり返す"),
    ("ヒヤシンス", "指定ミリ秒だけ止まる"),
    ("しゃぶりTIME", "今の時刻 (ミリ秒)"),
    ("ニンニンニンニンニン", "文字コードから文字を作る"),
    ("カプチーノ", "先頭の文字の文字コード"),
];

const KEYS: &[&str] = &[
    "いっぱいガラム",
    "おちんちん喋らせてくださぁい",
    "ペロペロ舐め値",
    "入っちゃう",
    "後ろ後ろが",
    "中が気持ちいいモコモコ",
    "いっぱいギロチン",
    "一揆しちゃった",
    "ヒヤシンス",
    "しゃぶりTIME",
    "ニン",
    "カプチーノ",
];

fn canonical(name: &str) -> String {
    let keys = crate::lexer::lookup_keys(name);
    keys.iter()
        .find(|k| KEYS.contains(&k.as_str()))
        .cloned()
        .unwrap_or_else(|| keys[0].clone())
}

fn arity(name: &str, args: &[Value], want: usize, line: usize) -> PResult<()> {
    if args.len() == want {
        Ok(())
    } else {
        Err(PumomoError::runtime(
            line,
            format!("{name} は引数 {want} 個．渡したのは {} 個", args.len()),
        ))
    }
}

fn as_array(v: &Value, who: &str, line: usize) -> PResult<std::rc::Rc<std::cell::RefCell<Vec<Value>>>> {
    match v {
        Value::Array(a) => Ok(a.clone()),
        other => Err(PumomoError::runtime(
            line,
            format!("{who} に渡すのは ガバ穴．来たのは {}", other.type_name()),
        )),
    }
}

fn builtin(name: &str, mut args: Vec<Value>, line: usize) -> PResult<Value> {
    match canonical(name).as_str() {
        "いっぱいガラム" => {
            arity(name, &args, 1, line)?;
            Ok(Value::Int(match &args[0] {
                Value::Array(a) => a.borrow().len() as i64,
                Value::Str(s) => s.chars().count() as i64,
                other => {
                    return Err(PumomoError::runtime(
                        line,
                        format!("{} に長さは無い", other.type_name()),
                    ));
                }
            }))
        }
        "おちんちん喋らせてくださぁい" => {
            arity(name, &args, 1, line)?;
            Ok(Value::str(args[0].to_string()))
        }
        "ペロペロ舐め値" => {
            arity(name, &args, 1, line)?;
            Ok(match &args[0] {
                Value::Int(n) => Value::Int(*n),
                Value::Float(x) => Value::Float(*x),
                Value::Bool(b) => Value::Int(*b as i64),
                Value::Str(s) => {
                    let t = s.trim();
                    if let Ok(n) = t.parse::<i64>() {
                        Value::Int(n)
                    } else if let Ok(x) = t.parse::<f64>() {
                        Value::Float(x)
                    } else {
                        return Err(PumomoError::runtime(
                            line,
                            format!("「{t}」は数として読めない"),
                        ));
                    }
                }
                other => {
                    return Err(PumomoError::runtime(
                        line,
                        format!("{} は数にならない", other.type_name()),
                    ));
                }
            })
        }
        "入っちゃう" => {
            arity(name, &args, 2, line)?;
            let v = args.pop().unwrap();
            let a = as_array(&args[0], name, line)?;
            a.borrow_mut().push(v);
            Ok(args[0].clone())
        }
        "後ろ後ろが" => {
            arity(name, &args, 1, line)?;
            let a = as_array(&args[0], name, line)?;
            let popped = a.borrow_mut().pop();
            Ok(popped.unwrap_or(Value::Pumomo))
        }
        "中が気持ちいいモコモコ" => {
            arity(name, &args, 2, line)?;
            Ok(Value::Bool(match (&args[0], &args[1]) {
                (Value::Array(a), needle) => a.borrow().iter().any(|v| v.eq(needle)),
                (Value::Str(s), Value::Str(t)) => s.contains(t.as_str()),
                (hay, _) => {
                    return Err(PumomoError::runtime(
                        line,
                        format!("{} の中は覗けない", hay.type_name()),
                    ));
                }
            }))
        }
        "いっぱいギロチン" => {
            arity(name, &args, 3, line)?;
            let from = as_index(&args[1], line)?;
            let to = as_int(&args[2], line)?;
            match &args[0] {
                Value::Array(a) => {
                    let a = a.borrow();
                    let end = (to + 1).clamp(0, a.len() as i64) as usize;
                    Ok(Value::array(a[from.min(end)..end].to_vec()))
                }
                Value::Str(s) => {
                    let cs: Vec<char> = s.chars().collect();
                    let end = (to + 1).clamp(0, cs.len() as i64) as usize;
                    Ok(Value::str(cs[from.min(end)..end].iter().collect::<String>()))
                }
                other => Err(PumomoError::runtime(
                    line,
                    format!("{} は切れない", other.type_name()),
                )),
            }
        }
        "一揆しちゃった" => {
            arity(name, &args, 1, line)?;
            match &args[0] {
                Value::Array(a) => {
                    let mut v = a.borrow().clone();
                    v.reverse();
                    Ok(Value::array(v))
                }
                Value::Str(s) => Ok(Value::str(s.chars().rev().collect::<String>())),
                other => Err(PumomoError::runtime(
                    line,
                    format!("{} はひっくり返らない", other.type_name()),
                )),
            }
        }
        "ヒヤシンス" => {
            arity(name, &args, 1, line)?;
            let ms = as_int(&args[0], line)?.max(0) as u64;
            std::thread::sleep(std::time::Duration::from_millis(ms));
            Ok(Value::Pumomo)
        }
        "しゃぶりTIME" => {
            arity(name, &args, 0, line)?;
            let ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            Ok(Value::Int(ms))
        }
        "ニン" => {
            arity(name, &args, 1, line)?;
            let n = as_int(&args[0], line)?;
            let c = u32::try_from(n)
                .ok()
                .and_then(char::from_u32)
                .ok_or_else(|| PumomoError::runtime(line, format!("{n} は文字にならない")))?;
            Ok(Value::str(c.to_string()))
        }
        "カプチーノ" => {
            arity(name, &args, 1, line)?;
            let Value::Str(s) = &args[0] else {
                return Err(PumomoError::runtime(line, "カプチーノ の引数は文字列"));
            };
            let c = s
                .chars()
                .next()
                .ok_or_else(|| PumomoError::runtime(line, "空文字にコードは無い"))?;
            Ok(Value::Int(c as i64))
        }
        _ => Err(PumomoError::runtime(line, format!("緩次郎 {name} はいない"))),
    }
}

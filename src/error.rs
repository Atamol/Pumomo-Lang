use crate::value::Value;
use std::fmt;

#[derive(Clone, Copy, PartialEq)]
pub enum ErrKind {
    Syntax,
    Runtime,
    /// raised by 細菌入っちゃう
    Thrown,
    /// raised by ｵﾚﾓｲｯﾁｬｳ
    Panic,
    /// Not an error, オッパゲドン rides the same path to unwind
    Exit(i32),
}

#[derive(Clone)]
pub struct PumomoError {
    pub kind: ErrKind,
    pub line: usize,
    pub msg: String,
    pub payload: Option<Value>,
}

impl PumomoError {
    pub fn syntax(line: usize, msg: impl Into<String>) -> Self {
        Self { kind: ErrKind::Syntax, line, msg: msg.into(), payload: None }
    }

    pub fn runtime(line: usize, msg: impl Into<String>) -> Self {
        Self { kind: ErrKind::Runtime, line, msg: msg.into(), payload: None }
    }

    pub fn thrown(line: usize, v: Value) -> Self {
        Self { kind: ErrKind::Thrown, line, msg: v.to_string(), payload: Some(v) }
    }

    pub fn panic(line: usize, msg: impl Into<String>) -> Self {
        Self { kind: ErrKind::Panic, line, msg: msg.into(), payload: None }
    }

    pub fn exit(code: i32) -> Self {
        Self { kind: ErrKind::Exit(code), line: 0, msg: String::new(), payload: None }
    }

    pub fn catchable(&self) -> bool {
        matches!(self.kind, ErrKind::Runtime | ErrKind::Thrown)
    }

    pub fn as_value(&self) -> Value {
        self.payload.clone().unwrap_or_else(|| Value::str(self.msg.clone()))
    }

    fn banner(&self) -> &'static str {
        match self.kind {
            ErrKind::Syntax => "オ゛ア゛ッ゛",
            ErrKind::Runtime | ErrKind::Thrown => "細菌入っちゃう！",
            ErrKind::Panic => "ｳｧｧ!!ｵﾚﾓｲｯﾁｬｳｩｩｩ!!!",
            ErrKind::Exit(_) => "オッパゲドン",
        }
    }
}

impl fmt::Display for PumomoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            write!(f, "{} {}行目: {}", self.banner(), self.line, self.msg)
        } else {
            write!(f, "{} {}", self.banner(), self.msg)
        }
    }
}

pub type PResult<T> = Result<T, PumomoError>;

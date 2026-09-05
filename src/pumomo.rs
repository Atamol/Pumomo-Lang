//! 完全合体合金おちんちん, the 17-mora stack machine

use crate::error::{PResult, PumomoError};

#[derive(Clone, Copy, Debug)]
pub enum Op {
    Push(i64),
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Dup,
    Drop,
    Swap,
    EmitNum,
    EmitChar,
    Input,
    // Both hold the partner's index
    LoopStart(usize),
    LoopEnd(usize),
    Halt,
}

#[derive(Clone, Copy, Debug)]
pub struct Ins {
    pub op: Op,
    pub line: usize,
}

pub trait Io {
    fn out(&mut self, s: &str);
    fn read_char(&mut self) -> Option<char>;
}

/// Ops keyed by their mora, longest first so ちょっ beats ちょ
const TABLE: &[(&str, Op)] = &[
    ("ちょっ", Op::LoopStart(0)),
    ("ちゃっ", Op::LoopEnd(0)),
    ("さっ", Op::EmitChar),
    ("ちょ", Op::Drop),
    ("ちゃ", Op::Swap),
    ("さ", Op::EmitNum),
    ("え", Op::Add),
    ("ん", Op::Sub),
    ("ぐ", Op::Mul),
    ("げ", Op::Div),
    ("ぎ", Op::Mod),
    ("お", Op::Dup),
];

pub fn assemble(src: &str, first_line: usize) -> PResult<Vec<Ins>> {
    let chars: Vec<char> = src.chars().collect();
    let mut out: Vec<Ins> = Vec::new();
    let mut opens: Vec<usize> = Vec::new();
    let mut i = 0;
    let mut line = first_line;

    'scan: while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            line += 1;
            i += 1;
            continue;
        }
        if c.is_whitespace() || c == '　' {
            i += 1;
            continue;
        }
        if c == '#' || c == '※' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == 'ぷ' {
            i += 1;
            let mut n: i64 = 0;
            while i < chars.len() && chars[i] == 'も' {
                n += 1;
                i += 1;
            }
            out.push(Ins { op: Op::Push(n), line });
            continue;
        }
        if c == 'も' {
            return Err(PumomoError::syntax(line, "ぷ を挟まずに も が出てきた．数はいつも ぷ から始まる"));
        }
        if c == '！' || c == '!' {
            out.push(Ins { op: Op::Halt, line });
            i += 1;
            continue;
        }
        if c == '？' || c == '?' {
            out.push(Ins { op: Op::Input, line });
            i += 1;
            continue;
        }
        for (mora, op) in TABLE {
            let m: Vec<char> = mora.chars().collect();
            if chars[i..].starts_with(&m[..]) {
                match op {
                    Op::LoopStart(_) => {
                        opens.push(out.len());
                        out.push(Ins { op: Op::LoopStart(0), line });
                    }
                    Op::LoopEnd(_) => {
                        let start = opens.pop().ok_or_else(|| {
                            PumomoError::syntax(line, "ちゃっ が余っている．開いていない ちょっ を閉じた")
                        })?;
                        let here = out.len();
                        out[start].op = Op::LoopStart(here);
                        out.push(Ins { op: Op::LoopEnd(start), line });
                    }
                    _ => out.push(Ins { op: *op, line }),
                }
                i += m.len();
                continue 'scan;
            }
        }
        return Err(PumomoError::syntax(
            line,
            format!("ダディーは「{c}」とは喘がない"),
        ));
    }

    if let Some(start) = opens.pop() {
        return Err(PumomoError::syntax(out[start].line, "ちょっ が閉じられていない．ちゃっ を足せ"));
    }
    Ok(out)
}

pub fn execute(code: &[Ins], stack: &mut Vec<i64>, io: &mut dyn Io) -> PResult<()> {
    let mut pc = 0;
    while pc < code.len() {
        let Ins { op, line } = code[pc];
        let pop = |st: &mut Vec<i64>| -> PResult<i64> {
            st.pop()
                .ok_or_else(|| PumomoError::runtime(line, "スタックがガバガバ．積む前に取り出した"))
        };
        match op {
            Op::Push(n) => stack.push(n),
            Op::Add => {
                let (b, a) = (pop(stack)?, pop(stack)?);
                stack.push(a + b);
            }
            Op::Sub => {
                let (b, a) = (pop(stack)?, pop(stack)?);
                stack.push(a - b);
            }
            Op::Mul => {
                let (b, a) = (pop(stack)?, pop(stack)?);
                stack.push(a * b);
            }
            Op::Div => {
                let (b, a) = (pop(stack)?, pop(stack)?);
                if b == 0 {
                    return Err(PumomoError::runtime(line, "0 で割った．穴が無い"));
                }
                stack.push(a.div_euclid(b));
            }
            Op::Mod => {
                let (b, a) = (pop(stack)?, pop(stack)?);
                if b == 0 {
                    return Err(PumomoError::runtime(line, "0 で割った余りは無い"));
                }
                stack.push(a.rem_euclid(b));
            }
            Op::Dup => {
                let a = *stack
                    .last()
                    .ok_or_else(|| PumomoError::runtime(line, "スタックがガバガバ．お は空を複製できない"))?;
                stack.push(a);
            }
            Op::Drop => {
                pop(stack)?;
            }
            Op::Swap => {
                let (b, a) = (pop(stack)?, pop(stack)?);
                stack.push(b);
                stack.push(a);
            }
            Op::EmitNum => {
                let a = pop(stack)?;
                io.out(&a.to_string());
            }
            Op::EmitChar => {
                let a = pop(stack)?;
                let c = u32::try_from(a)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or_else(|| PumomoError::runtime(line, format!("{a} は文字にならない")))?;
                io.out(&c.to_string());
            }
            Op::Input => {
                let v = io.read_char().map(|c| c as i64).unwrap_or(-1);
                stack.push(v);
            }
            Op::LoopStart(end) => {
                let top = *stack
                    .last()
                    .ok_or_else(|| PumomoError::runtime(line, "スタックがガバガバ．ちょっ が判定できない"))?;
                if top == 0 {
                    pc = end + 1;
                    continue;
                }
            }
            Op::LoopEnd(start) => {
                pc = start;
                continue;
            }
            Op::Halt => return Ok(()),
        }
        pc += 1;
    }
    Ok(())
}

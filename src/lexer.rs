use crate::error::{PResult, PumomoError};
use crate::pumomo::{self, Ins};

/// Canonical opening line, normalize_incant absorbs the variants
pub const PHRASE: &str = "ぷももえんぐえげぎおんもえちょっちょっちゃっさっ！";

#[derive(Clone, Debug)]
pub enum Tok {
    Incantation,
    Kanjiro,
    Let,
    Const,
    Println,
    Print,
    Stderr,
    ReadLine,
    ReadAll,
    If,
    ElseIf,
    Else,
    While,
    For,
    ForEach,
    Loop,
    Break,
    Continue,
    Return,
    Assert,
    Exit,
    Panic,
    Try,
    Catch,
    Throw,
    End,
    True,
    False,
    Null,
    Gattai,
    GattaiBody(Vec<Ins>),

    Ident(String),
    Int(i64),
    Float(f64),
    Str(String),

    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Assign,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    OrOr,
    Bang,
    DotDot,
    Newline,
    Eof,
}

impl Tok {
    pub fn describe(&self) -> String {
        match self {
            Tok::Incantation => PHRASE.to_string(),
            Tok::Ident(s) => s.clone(),
            Tok::Int(n) => n.to_string(),
            Tok::Float(x) => x.to_string(),
            Tok::Str(s) => format!("「{s}」"),
            Tok::GattaiBody(_) => "完全合体合金おちんちん の中身".to_string(),
            Tok::Newline => "改行".to_string(),
            Tok::Eof => "ファイルの終わり".to_string(),
            other => keyword_text(other).unwrap_or("記号").to_string(),
        }
    }
}

pub fn keyword_text(t: &Tok) -> Option<&'static str> {
    Some(match t {
        Tok::Kanjiro => "緩次郎",
        Tok::Let => "いっぱい入っちゃう",
        Tok::Const => "おまんこロックマン",
        Tok::Println => "最高出力を",
        Tok::Print => "豚キムチ",
        Tok::Stderr => "お尻の穴が",
        Tok::ReadLine => "おちんちんくださぁい？",
        Tok::ReadAll => "いっぱいちょうだい？",
        Tok::If => "ドーラ",
        Tok::ElseIf => "銅鑼",
        Tok::Else => "ドアラ",
        Tok::While => "動いてる中で",
        Tok::For => "いっぱいいっぱいゆうじろう",
        Tok::ForEach => "Delight! Beast! おまんこの中でEasy! Easy!",
        Tok::Loop => "ジャッキーチェン",
        Tok::Break => "そこッ",
        Tok::Continue => "てっ",
        Tok::Return => "一緒に帰りたい",
        Tok::Assert => "ムーア我慢できぬ",
        Tok::Exit => "オッパゲドン",
        Tok::Panic => "ｵﾚﾓｲｯﾁｬｳ",
        Tok::Try => "細菌入り味噌",
        Tok::Catch => "オ゛ア゛ッ゛",
        Tok::Throw => "細菌入っちゃう",
        Tok::End => "ムッキュ",
        Tok::True => "ンキモチイイ",
        Tok::False => "もう嫌",
        Tok::Null => "ぷもも",
        Tok::Gattai => "完全合体合金おちんちん",
        _ => return None,
    })
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: Tok,
    pub line: usize,
}

const HALF: &str = "｡｢｣､･ｦｧｨｩｪｫｬｭｮｯｰｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜﾝ";
const FULL: &str = "。「」、・ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン";

fn wide_kana(c: char) -> Option<char> {
    HALF.chars().position(|h| h == c).and_then(|i| FULL.chars().nth(i))
}

fn voiced(c: char) -> Option<char> {
    Some(match c {
        'カ' => 'ガ', 'キ' => 'ギ', 'ク' => 'グ', 'ケ' => 'ゲ', 'コ' => 'ゴ',
        'サ' => 'ザ', 'シ' => 'ジ', 'ス' => 'ズ', 'セ' => 'ゼ', 'ソ' => 'ゾ',
        'タ' => 'ダ', 'チ' => 'ヂ', 'ツ' => 'ヅ', 'テ' => 'デ', 'ト' => 'ド',
        'ハ' => 'バ', 'ヒ' => 'ビ', 'フ' => 'ブ', 'ヘ' => 'ベ', 'ホ' => 'ボ',
        'ウ' => 'ヴ',
        _ => return None,
    })
}

fn semi_voiced(c: char) -> Option<char> {
    Some(match c {
        'ハ' => 'パ', 'ヒ' => 'ピ', 'フ' => 'プ', 'ヘ' => 'ペ', 'ホ' => 'ポ',
        _ => return None,
    })
}

/// Trailing-only decoration, ゃゅょ stay out since they are part of the syllable
const TRAILING: &str = "ぁぃぅぇぉっァィゥェォッーｰ？?！!～~…";

/// Half-width kana widened and dakuten dropped, identifiers get no more folding than this
pub fn fold(s: &str) -> String {
    let cs: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if matches!(c, '\u{3099}' | '\u{309A}' | '\u{309B}' | '\u{309C}' | 'ﾞ' | 'ﾟ')
            || DECOR.contains(c)
        {
            i += 1;
            continue;
        }
        let base = wide_kana(c).unwrap_or(c);
        let mark = cs.get(i + 1).copied();
        let combined = match mark {
            Some('ﾞ') => voiced(base),
            Some('ﾟ') => semi_voiced(base),
            _ => None,
        };
        match combined {
            Some(v) => {
                out.push(v);
                i += 2;
            }
            None => {
                out.push(base);
                i += 1;
            }
        }
    }
    out
}

fn trim_tail(s: &str) -> String {
    let mut out = s.to_string();
    while out.chars().next_back().is_some_and(|c| TRAILING.contains(c)) {
        out.pop();
    }
    out
}

pub fn normalize_word(s: &str) -> String {
    trim_tail(&fold(s))
}

/// keys to try for a 語録, including the unit a repeated one folds to
pub fn lookup_keys(raw: &str) -> Vec<String> {
    let folded = fold(raw);
    let trimmed = trim_tail(&folded);
    let mut out = vec![trimmed.clone()];
    for base in [&folded, &trimmed] {
        let cs: Vec<char> = base.chars().collect();
        for p in 1..=cs.len() / 2 {
            if cs.len() % p != 0 {
                continue;
            }
            if cs.chunks(p).all(|c| c == &cs[..p]) {
                let unit = trim_tail(&cs[..p].iter().collect::<String>());
                if !unit.is_empty() && !out.contains(&unit) {
                    out.push(unit);
                }
            }
        }
    }
    out
}

fn keyword(word: &str) -> Option<Tok> {
    lookup_keys(word).into_iter().find_map(|k| keyword_key(&k))
}

fn keyword_key(key: &str) -> Option<Tok> {
    Some(match key {
        "緩次郎" => Tok::Kanjiro,
        "いっぱい入っちゃう" => Tok::Let,
        "おまんこロックマン" => Tok::Const,
        "最高出力を" | "最高出力" => Tok::Println,
        "豚キムチ" => Tok::Print,
        "お尻の穴が" => Tok::Stderr,
        "おちんちんくださぁい" | "おおきいおちんちんくださぁい" => Tok::ReadLine,
        "いっぱいちょうだい" => Tok::ReadAll,
        "ドーラ" => Tok::If,
        "銅鑼" => Tok::ElseIf,
        "ドアラ" => Tok::Else,
        "動いてる中で" => Tok::While,
        "いっぱいいっぱいゆうじろう" => Tok::For,
        "ジャッキーチェン" => Tok::Loop,
        "そこ" => Tok::Break,
        "て" => Tok::Continue,
        "一緒に帰りたい" => Tok::Return,
        "ムーア我慢できぬ" => Tok::Assert,
        "オッパゲドン" => Tok::Exit,
        "オレモイッチャウ" => Tok::Panic,
        "細菌入り味噌" => Tok::Try,
        "オア" => Tok::Catch,
        "細菌入っちゃう" => Tok::Throw,
        "ムッキュ" => Tok::End,
        "ンキモチイイ" => Tok::True,
        "もう嫌" => Tok::False,
        "ぷもも" => Tok::Null,
        "完全合体合金おちんちん" => Tok::Gattai,
        _ => return None,
    })
}

const KEEPS_QUESTION: &[&str] = &[
    "おちんちんくださぁい",
    "おおきいおちんちんくださぁい",
    "いっぱいちょうだい",
    "おちんちん喋らせてくださぁい",
];

/// 語録 with punctuation inside, matched loosely instead of by word scan
const FOREACH_FORMS: &[&str] = &[
    "Delight! Beast! おまんこの中でEasy! Easy!",
    "Delight! Beast! OMANCOの中でEasy! Easy!",
];
const PRINTLN_FORMS: &[&str] = &["最高出力を"];
const WHILE_FORMS: &[&str] = &["動いてる中で"];
const LET_FORMS: &[&str] = &["いっぱい入っちゃう"];
const ASSERT_FORMS: &[&str] = &["ムーア我慢できぬ"];

/// dropped anywhere, unlike TRAILING
const DECOR: &str = "↑↓⤴⤵＾～！";

/// Nothing can follow these but a 語録 flourish, so a trailing ! is not negation
fn no_expr_after(t: &Tok) -> bool {
    matches!(
        t,
        Tok::Int(_)
            | Tok::Float(_)
            | Tok::Str(_)
            | Tok::Ident(_)
            | Tok::True
            | Tok::False
            | Tok::Null
            | Tok::RParen
            | Tok::RBracket
            | Tok::GattaiBody(_)
            | Tok::Incantation
            | Tok::End
            | Tok::Loop
            | Tok::Break
            | Tok::Continue
            | Tok::Else
            | Tok::Try
    )
}
const PANIC_FORMS: &[&str] = &[
    "ｳｧｧ!!ｵﾚﾓｲｯﾁｬｳ",
    "ウァァ!!オレモイッチャウ",
    "ｵﾚﾓｲｯﾁｬｳ",
    "オレモイッチャウ",
];
const SCREAM: &str = "ｳｩｧｱｲｨウゥァアイィ!！";

const INCANT_CHARS: &[char] = &[
    'ぷ', 'も', 'え', 'ん', 'ぐ', 'げ', 'ぎ', 'お', 'ち', 'ょ', 'ゃ', 'さ', 'っ', '！', '!',
];

fn normalize_incant(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if matches!(c, 'っ' | '！' | '!') || c.is_whitespace() {
            continue;
        }
        if out.chars().last() == Some(c) {
            continue;
        }
        out.push(c);
    }
    out
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric()
        || c == '_'
        || matches!(c, '\u{3099}' | '\u{309A}' | '\u{309B}' | '\u{309C}')
        || c == 'ー'
        || c == '々'
        || DECOR.contains(c)
        || matches!(c as u32, 0xFF66..=0xFF9F)
}

#[derive(Clone, Copy)]
enum Gattai {
    AfterKw,
    InArgs(usize),
    BodyNext,
}

pub fn lex(src: &str) -> PResult<Vec<Token>> {
    Lexer {
        s: src.chars().collect(),
        i: 0,
        line: 1,
        out: Vec::new(),
        paren: 0,
        gattai: Vec::new(),
    }
    .run()
}

struct Lexer {
    s: Vec<char>,
    i: usize,
    line: usize,
    out: Vec<Token>,
    paren: usize,
    gattai: Vec<Gattai>,
}

impl Lexer {
    fn at(&self, k: usize) -> Option<char> {
        self.s.get(self.i + k).copied()
    }

    fn push(&mut self, kind: Tok) {
        self.out.push(Token { kind, line: self.line });
    }

    fn skip_trivia(&mut self) {
        while let Some(c) = self.at(0) {
            match c {
                '\n' => {
                    self.line += 1;
                    self.i += 1;
                }
                '#' | '※' => {
                    while self.at(0).is_some_and(|c| c != '\n') {
                        self.i += 1;
                    }
                }
                c if c.is_whitespace() => self.i += 1,
                _ => break,
            }
        }
    }

    /// Grab the 合体合金 body verbatim, up to its ムッキュ
    fn gattai_body(&mut self) -> PResult<Vec<Ins>> {
        let start_line = self.line;
        let mut raw = String::new();
        loop {
            let Some(c) = self.at(0) else {
                return Err(PumomoError::syntax(
                    start_line,
                    "完全合体合金おちんちん が ムッキュ で閉じられていない",
                ));
            };
            if c == '#' || c == '※' {
                while let Some(c) = self.at(0) {
                    if c == '\n' {
                        break;
                    }
                    raw.push(c);
                    self.i += 1;
                }
                continue;
            }
            if c == 'ム' && self.s[self.i..].starts_with(&['ム', 'ッ', 'キ', 'ュ']) {
                self.i += 4;
                break;
            }
            if c == '\n' {
                self.line += 1;
            }
            raw.push(c);
            self.i += 1;
        }
        pumomo::assemble(&raw, start_line)
    }

    fn run(mut self) -> PResult<Vec<Token>> {
        loop {
            match self.gattai.last().copied() {
                Some(Gattai::AfterKw) => {
                    self.skip_trivia();
                    if self.at(0) == Some('(') || self.at(0) == Some('（') {
                        let base = self.paren;
                        *self.gattai.last_mut().unwrap() = Gattai::InArgs(base);
                    } else {
                        self.gattai.pop();
                        let body = self.gattai_body()?;
                        self.push(Tok::GattaiBody(body));
                        continue;
                    }
                }
                Some(Gattai::BodyNext) => {
                    self.gattai.pop();
                    self.skip_trivia();
                    let body = self.gattai_body()?;
                    self.push(Tok::GattaiBody(body));
                    continue;
                }
                _ => {}
            }

            let Some(c) = self.at(0) else { break };

            if c == '\n' {
                self.i += 1;
                let fresh = matches!(self.out.last().map(|t| &t.kind), None | Some(Tok::Newline));
                if self.paren == 0 && !fresh {
                    self.push(Tok::Newline);
                }
                self.line += 1;
                continue;
            }
            if c.is_whitespace() || DECOR.contains(c) {
                self.i += 1;
                continue;
            }
            if c == '!'
                && self.at(1) != Some('=')
                && self.out.last().is_some_and(|t| no_expr_after(&t.kind))
            {
                self.i += 1;
                continue;
            }
            // Exactly two dots is the range operator, any other run is the 語録 ellipsis
            if c == '.' || c == '…' {
                let mut n = 0;
                while self.at(n).is_some_and(|c| c == '.' || c == '…') {
                    n += 1;
                }
                if n == 2 && self.at(1) == Some('.') {
                    self.push(Tok::DotDot);
                }
                self.i += n;
                continue;
            }
            if c == '#' || c == '※' {
                while self.at(0).is_some_and(|c| c != '\n') {
                    self.i += 1;
                }
                continue;
            }
            if c == '「' || c == '\u{22}' {
                self.string(c)?;
                continue;
            }
            if c.is_ascii_digit() {
                self.number()?;
                continue;
            }
            if self.try_phrase() {
                continue;
            }
            if c == 'ぷ' && self.try_incantation() {
                continue;
            }
            if is_ident_char(c) {
                self.word();
                continue;
            }
            self.symbol()?;
        }
        self.push(Tok::Eof);
        Ok(self.out)
    }

    /// Matches form while skipping decoration, returns how many source chars it ate
    fn match_form(&self, form: &str) -> Option<usize> {
        let skip = |c: char| "!！ 　.…、，".contains(c) || DECOR.contains(c);
        let mut si = self.i;
        let mut f = form.chars().peekable();
        loop {
            while f.peek().is_some_and(|c| skip(*c)) {
                f.next();
            }
            let Some(fc) = f.next() else { break };
            while si < self.s.len() && skip(self.s[si]) {
                si += 1;
            }
            let sc = *self.s.get(si)?;
            if sc.to_lowercase().ne(fc.to_lowercase()) {
                return None;
            }
            si += 1;
        }
        Some(si - self.i)
    }

    fn try_phrase(&mut self) -> bool {
        for (forms, tail, tok) in [
            (FOREACH_FORMS, "", Tok::ForEach),
            (PANIC_FORMS, SCREAM, Tok::Panic),
            (PRINTLN_FORMS, "", Tok::Println),
            (WHILE_FORMS, "", Tok::While),
            (LET_FORMS, "", Tok::Let),
            (ASSERT_FORMS, "", Tok::Assert),
        ] {
            for form in forms {
                let Some(n) = self.match_form(form) else { continue };
                let mut end = self.i + n;
                while self
                    .s
                    .get(end)
                    .is_some_and(|c| *c == '!' || *c == '！' || tail.contains(*c))
                {
                    end += 1;
                }
                if self.s.get(end).copied().is_some_and(is_ident_char) {
                    continue;
                }
                self.i = end;
                self.push(tok);
                return true;
            }
        }
        false
    }

    fn try_incantation(&mut self) -> bool {
        let mut j = self.i;
        while j < self.s.len() && INCANT_CHARS.contains(&self.s[j]) {
            j += 1;
        }
        let raw: String = self.s[self.i..j].iter().collect();
        if normalize_incant(&raw) != normalize_incant(PHRASE) {
            return false;
        }
        self.push(Tok::Incantation);
        self.i = j;
        true
    }

    fn word(&mut self) {
        let start = self.i;
        while self.at(0).is_some_and(is_ident_char) {
            self.i += 1;
        }
        let raw: String = self.s[start..self.i].iter().collect();
        if KEEPS_QUESTION.contains(&normalize_word(&raw).as_str())
            && self.at(0).is_some_and(|c| c == '？' || c == '?')
        {
            self.i += 1;
        }
        match keyword(&raw) {
            Some(Tok::Gattai) => {
                self.push(Tok::Gattai);
                self.gattai.push(Gattai::AfterKw);
            }
            Some(k) => self.push(k),
            None => self.push(Tok::Ident(fold(&raw))),
        }
    }

    fn number(&mut self) -> PResult<()> {
        let start = self.i;
        while self.at(0).is_some_and(|c| c.is_ascii_digit() || c == '_') {
            self.i += 1;
        }
        let float = self.at(0) == Some('.') && self.at(1).is_some_and(|c| c.is_ascii_digit());
        if float {
            self.i += 1;
            while self.at(0).is_some_and(|c| c.is_ascii_digit() || c == '_') {
                self.i += 1;
            }
        }
        let text: String = self.s[start..self.i].iter().filter(|c| **c != '_').collect();
        if float {
            let v = text
                .parse::<f64>()
                .map_err(|_| PumomoError::syntax(self.line, format!("{text} は数として読めない")))?;
            self.push(Tok::Float(v));
        } else {
            let v = text
                .parse::<i64>()
                .map_err(|_| PumomoError::syntax(self.line, format!("{text} は大きすぎる")))?;
            self.push(Tok::Int(v));
        }
        Ok(())
    }

    fn string(&mut self, open: char) -> PResult<()> {
        let start_line = self.line;
        self.i += 1;
        let mut buf = String::new();
        let mut depth = 1usize;
        loop {
            let Some(c) = self.at(0) else {
                return Err(PumomoError::syntax(start_line, "文字列が閉じていない"));
            };
            if c == '\\' {
                self.i += 1;
                let e = self.at(0).unwrap_or('\\');
                buf.push(match e {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '0' => '\0',
                    other => other,
                });
                self.i += 1;
                continue;
            }
            if open == '「' {
                if c == '「' {
                    depth += 1;
                } else if c == '」' {
                    depth -= 1;
                    if depth == 0 {
                        self.i += 1;
                        break;
                    }
                }
            } else if c == '\u{22}' {
                self.i += 1;
                break;
            }
            if c == '\n' {
                self.line += 1;
            }
            buf.push(c);
            self.i += 1;
        }
        self.out.push(Token { kind: Tok::Str(buf), line: start_line });
        Ok(())
    }

    fn symbol(&mut self) -> PResult<()> {
        let c = self.s[self.i];
        let n = self.at(1);
        let (tok, len) = match (c, n) {
            ('=', Some('=')) => (Tok::Eq, 2),
            ('!', Some('=')) => (Tok::Ne, 2),
            ('<', Some('=')) => (Tok::Le, 2),
            ('>', Some('=')) => (Tok::Ge, 2),
            ('&', Some('&')) => (Tok::AndAnd, 2),
            ('|', Some('|')) => (Tok::OrOr, 2),
            ('=', _) => (Tok::Assign, 1),
            ('<', _) => (Tok::Lt, 1),
            ('>', _) => (Tok::Gt, 1),
            ('!', _) => (Tok::Bang, 1),
            ('+', _) => (Tok::Plus, 1),
            ('-', _) => (Tok::Minus, 1),
            ('*', _) => (Tok::Star, 1),
            ('/', _) => (Tok::Slash, 1),
            ('%', _) => (Tok::Percent, 1),
            ('(' | '（', _) => (Tok::LParen, 1),
            (')' | '）', _) => (Tok::RParen, 1),
            ('[' | '［', _) => (Tok::LBracket, 1),
            (']' | '］', _) => (Tok::RBracket, 1),
            (',' | '、' | '，', _) => (Tok::Comma, 1),
            _ => {
                return Err(PumomoError::syntax(
                    self.line,
                    format!("「{c}」はぷももえんの音ではない"),
                ));
            }
        };
        match tok {
            Tok::LParen | Tok::LBracket => self.paren += 1,
            Tok::RParen | Tok::RBracket => {
                self.paren = self.paren.saturating_sub(1);
                if let Some(Gattai::InArgs(base)) = self.gattai.last().copied() {
                    if self.paren == base {
                        *self.gattai.last_mut().unwrap() = Gattai::BodyNext;
                    }
                }
            }
            _ => {}
        }
        self.push(tok);
        self.i += len;
        Ok(())
    }
}

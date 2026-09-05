use pumomo::interp::BUILTINS;
use pumomo::lexer::PHRASE;
use std::process::ExitCode;

const KEYWORDS: &[(&str, &str)] = &[
    (PHRASE, "開始宣言．ファイルの頭に要る"),
    ("緩次郎", "関数を定義する"),
    ("ガバ穴ダディー", "最初に呼ばれる関数の名前"),
    ("ムッキュ", "ブロックを閉じる"),
    ("いっぱい入っちゃう", "変数を宣言する"),
    ("おまんこロックマン", "定数を宣言する"),
    ("最高出力を", "標準出力へ書いて改行する"),
    ("豚キムチ", "標準出力へ書く (改行しない)"),
    ("お尻の穴が", "標準エラー出力へ書く"),
    ("おちんちんくださぁい？", "標準入力から1行読む"),
    ("いっぱいちょうだい？", "標準入力を最後まで読む"),
    ("ドーラ", "もし"),
    ("銅鑼", "そうでなくて，もし"),
    ("ドアラ", "そうでなければ"),
    ("動いてる中で", "条件が続く間まわす"),
    ("いっぱいいっぱいゆうじろう", "範囲でまわす (a..b は両端を含む)"),
    ("Delight! Beast! おまんこの中でEasy! Easy!", "ガバ穴や文字列の中をまわす"),
    ("ジャッキーチェン", "止めるまでまわす"),
    ("そこッ", "繰り返しを抜ける"),
    ("てっ", "次の周回へ飛ぶ"),
    ("一緒に帰りたい", "値を返す"),
    ("ムーア我慢できぬ", "条件が偽なら落ちる"),
    ("オッパゲドン", "プログラムを終える"),
    ("ｳｧｧ!!ｵﾚﾓｲｯﾁｬｳｩｩｩ!!!", "捕まえられない異常終了"),
    ("細菌入り味噌", "例外を捕まえる区間"),
    ("オ゛ア゛ッ゛", "捕まえた例外を受ける"),
    ("細菌入っちゃう", "例外を投げる"),
    ("ンキモチイイ", "真"),
    ("もう嫌", "偽"),
    ("ぷもも", "値が無いこと"),
    ("完全合体合金おちんちん", "低レベル層 (17音のスタックマシン) を開く"),
];

const MORA: &[(&str, &str)] = &[
    ("ぷ + も*n", "も の数を積む (ぷ だけなら 0)"),
    ("え", "足す"),
    ("ん", "引く"),
    ("ぐ", "掛ける"),
    ("げ", "割る"),
    ("ぎ", "余り"),
    ("お", "複製"),
    ("ちょ", "捨てる"),
    ("ちゃ", "入れ替える"),
    ("さ", "数として出す"),
    ("さっ", "文字として出す"),
    ("ちょっ", "頂点が0でない間まわす"),
    ("ちゃっ", "ちょっ へ戻る"),
    ("？", "1文字読む (終端なら -1)"),
    ("！", "そこで止める"),
];

fn width(s: &str) -> usize {
    s.chars()
        .map(|c| match c as u32 {
            0x1100..=0x115F
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE6F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6 => 2,
            _ => 1,
        })
        .sum()
}

fn table(rows: &[(&str, &str)]) {
    let w = rows
        .iter()
        .map(|(k, _)| width(k))
        .filter(|w| *w <= 32)
        .max()
        .unwrap_or(20)
        + 2;
    for (k, d) in rows {
        let pad = " ".repeat(w.saturating_sub(width(k)).max(1));
        println!("  {k}{pad}{d}");
    }
}

fn usage() {
    println!("ぷももえん (PUMOMO-Lang)");
    println!();
    println!("使い方:");
    println!("  pumomo <file.pmm>       走らせる");
    println!("  pumomo --tokens <file>  字句を見る");
    println!("  pumomo --ast <file>     構文木を見る");
    println!("  pumomo 語録             予約語と17音の一覧");
    println!("  pumomo --help           これ");
}

fn goroku() {
    println!("== 予約語 ==");
    table(KEYWORDS);
    println!();
    println!("== 組み込み ==");
    table(BUILTINS);
    println!();
    println!("== 完全合体合金おちんちん の17音 ==");
    table(MORA);
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mode, path) = match args.as_slice() {
        [] => {
            usage();
            return ExitCode::SUCCESS;
        }
        [a] if a == "--help" || a == "-h" => {
            usage();
            return ExitCode::SUCCESS;
        }
        [a] if a == "語録" || a == "goroku" => {
            goroku();
            return ExitCode::SUCCESS;
        }
        [f] => ("run", f.clone()),
        [m, f] if m == "--tokens" => ("tokens", f.clone()),
        [m, f] if m == "--ast" => ("ast", f.clone()),
        [m, f] if m == "run" => ("run", f.clone()),
        _ => {
            usage();
            return ExitCode::from(2);
        }
    };

    let src = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("オ゛ア゛ッ゛ {path} が読めない: {e}");
            return ExitCode::from(2);
        }
    };

    let result = (|| {
        let toks = pumomo::lexer::lex(&src)?;
        if mode == "tokens" {
            for t in &toks {
                println!("{:>4}: {:?}", t.line, t.kind);
            }
            return Ok(0);
        }
        let prog = pumomo::parser::parse(toks)?;
        if mode == "ast" {
            println!("{prog:#?}");
            return Ok(0);
        }
        pumomo::interp::run(&prog)
    })();

    match result {
        Ok(code) => ExitCode::from(code.clamp(0, 255) as u8),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

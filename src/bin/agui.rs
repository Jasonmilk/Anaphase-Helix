//! 投影的唯一真源出口：读 session-events jsonl，吐 AG-UI 1.0 事件（JSONL）。
//! 为什么是 bin 而不是 web 里再写一遍规则：**投影只许有一份**（两份真源必漂）。
//! 用法: agui <events.jsonl>   （stdin 也接受，传 - 即可）
use std::io::{self, BufRead, Write};

fn main() {
    let arg = std::env::args().nth(1).unwrap_or_else(|| "-".to_string());
    let reader: Box<dyn BufRead> = if arg == "-" {
        Box::new(io::BufReader::new(io::stdin()))
    } else {
        match std::fs::File::open(&arg) {
            Ok(f) => Box::new(io::BufReader::new(f)),
            Err(e) => {
                eprintln!("UNMEASURABLE: 打不开 {arg}: {e}");
                std::process::exit(3);
            }
        }
    };
    let out = io::stdout();
    let mut w = out.lock();
    let mut n = 0usize;
    for line in reader.lines() {
        let line = match line { Ok(l) => l, Err(_) => continue };
        if line.trim().is_empty() { continue; }
        let ev: serde_json::Value = match serde_json::from_str(&line) { Ok(v) => v, Err(_) => continue };
        for e in anaphase::agui::project(&ev) {
            writeln!(w, "{e}").ok();
            n += 1;
        }
    }
    eprintln!("agui: {n} 条（源 {arg}）");
}

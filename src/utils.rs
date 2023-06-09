use anyhow::{Ok, Result};
use console::{style, Style};
use similar::{ChangeTag, TextDiff};
use std::fmt::{self, Write};
use std::io::Write as _;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

struct Line(Option<usize>);

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            None => write!(f, "    "),
            Some(idx) => write!(f, "{:<4}", idx + 1),
        }
    }
}

pub fn diff_text(text1: &str, text2: &str) -> Result<String> {
    let mut output = String::new();
    let diff = TextDiff::from_lines(text1, text2);

    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            writeln!(&mut output, "{:-^1$}", "-", 80)?;
        }
        for op in group {
            for change in diff.iter_inline_changes(op) {
                let (sign, s) = match change.tag() {
                    ChangeTag::Delete => ("-", Style::new().red()),
                    ChangeTag::Insert => ("+", Style::new().green()),
                    ChangeTag::Equal => (" ", Style::new().dim()),
                };
                write!(
                    &mut output,
                    "{}{} |{}",
                    style(Line(change.old_index())).dim(),
                    style(Line(change.new_index())).dim(),
                    s.apply_to(sign).bold(),
                )?;
                for (emphasized, value) in change.iter_strings_lossy() {
                    if emphasized {
                        write!(&mut output, "{}", s.apply_to(value).underlined().on_black())?;
                    } else {
                        write!(&mut output, "{}", s.apply_to(value))?;
                    }
                }
                if change.missing_newline() {
                    writeln!(&mut output)?;
                }
            }
        }
    }

    Ok(output)
}

pub fn highlight_text(text: &str, extension: &str) -> Result<String> {
    // Load these once at the start of your program
    // 加载语法集和主题集
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = ps
        .find_syntax_by_extension(extension)
        .expect("extension not found");

    let mut higlin = HighlightLines::new(syntax, &ts.themes.iter().collect::<Vec<_>>()[1].1);
    let mut output = String::new();
    for line in LinesWithEndings::from(text) {
        let ranges = higlin.highlight_line(line, &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        write!(&mut output, "{}", escaped)?;
    }

    Ok(output)
}

// 判断是否为默认值
pub fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}

// 接受一个Result<>类型的参数，如果出错，并且输出，打印出错误信息，并且给错误信息上色
pub fn print_error(result: Result<()>) -> Result<()> {
    if let Err(e) = result {
        let stderr = std::io::stderr();
        let mut stderr = stderr.lock();
        if atty::is(atty::Stream::Stderr) {
            let color = Style::new().red();
            writeln!(stderr, "{}", color.apply_to(format!("{:?}", e)))?;
        } else {
            writeln!(stderr, "{:?}", e)?;
        }
    }
    Ok(())
}

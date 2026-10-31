#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Header { level: u8, text: String },
    CodeBlock { lang: String, code: String },
    Bullet { prefix: String, text: String },
    Paragraph(String),
}

/// Простий і надійний парсер Markdown для структурованого рендерингу в Direct2D
pub fn parse_markdown(input: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim_end();
        let trimmed = line.trim();

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        // Блок коду (```lang ... ```)
        if trimmed.starts_with("```") {
            let lang = trimmed.trim_start_matches('`').trim().to_string();
            let mut code_lines = Vec::new();
            i += 1;
            while i < lines.len() {
                let code_line = lines[i];
                if code_line.trim().starts_with("```") {
                    i += 1;
                    break;
                }
                code_lines.push(code_line);
                i += 1;
            }
            blocks.push(Block::CodeBlock {
                lang,
                code: code_lines.join("\n"),
            });
            continue;
        }

        // Заголовки (#, ##, ###)
        if trimmed.starts_with("### ") {
            blocks.push(Block::Header {
                level: 3,
                text: clean_formatting(&trimmed[4..]),
            });
            i += 1;
            continue;
        } else if trimmed.starts_with("## ") {
            blocks.push(Block::Header {
                level: 2,
                text: clean_formatting(&trimmed[3..]),
            });
            i += 1;
            continue;
        } else if trimmed.starts_with("# ") {
            blocks.push(Block::Header {
                level: 1,
                text: clean_formatting(&trimmed[2..]),
            });
            i += 1;
            continue;
        }

        // Списки (*, -, або 1., 2.)
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            blocks.push(Block::Bullet {
                prefix: "•".to_string(),
                text: clean_formatting(&trimmed[2..]),
            });
            i += 1;
            continue;
        }

        // Нумеровані списки (1., 2., a., b.)
        if let Some(pos) = trimmed.find(". ") {
            let prefix = &trimmed[..pos];
            if prefix.chars().all(|c| c.is_ascii_digit() || c.is_ascii_alphabetic()) && prefix.len() <= 3 {
                blocks.push(Block::Bullet {
                    prefix: format!("{}.", prefix),
                    text: clean_formatting(&trimmed[pos + 2..]),
                });
                i += 1;
                continue;
            }
        }

        // Звичайний параграф
        let mut para_lines = vec![clean_formatting(trimmed)];
        i += 1;
        while i < lines.len() {
            let next_line = lines[i].trim();
            if next_line.is_empty()
                || next_line.starts_with("```")
                || next_line.starts_with("#")
                || next_line.starts_with("- ")
                || next_line.starts_with("* ")
            {
                break;
            }
            para_lines.push(clean_formatting(next_line));
            i += 1;
        }

        blocks.push(Block::Paragraph(para_lines.join(" ")));
    }

    blocks
}

/// Очищає зайві зірочки та символи форматування для комфортного читання
fn clean_formatting(text: &str) -> String {
    // Прибираємо подвійні зірочки **bold** але зберігаємо сам текст
    text.replace("**", "").replace("__", "")
}

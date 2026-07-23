use crate::error::RqlError;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    From,
    Where,
    And,
    Or,
    Not,
    In,
    Within,
    Before,
    After,
    Group,
    By,
    Order,
    Asc,
    Desc,
    As,
    True,
    False,
    Null,
    Ident(String),
    String(String),
    Number(f64, Option<String>),
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    Like,
    LParen,
    RParen,
    Comma,
    Dot,
    Eof,
}

impl Token {
    fn keyword(s: &str) -> Option<Token> {
        match s.to_lowercase().as_str() {
            "from" => Some(Token::From),
            "where" => Some(Token::Where),
            "and" => Some(Token::And),
            "or" => Some(Token::Or),
            "not" => Some(Token::Not),
            "in" => Some(Token::In),
            "within" => Some(Token::Within),
            "before" => Some(Token::Before),
            "after" => Some(Token::After),
            "group" => Some(Token::Group),
            "by" => Some(Token::By),
            "order" => Some(Token::Order),
            "asc" => Some(Token::Asc),
            "desc" => Some(Token::Desc),
            "as" => Some(Token::As),
            "true" => Some(Token::True),
            "false" => Some(Token::False),
            "null" => Some(Token::Null),
            _ => None,
        }
    }
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self { chars: input.chars().collect(), pos: 0 }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, RqlError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace();
            if self.pos >= self.chars.len() {
                tokens.push(Token::Eof);
                return Ok(tokens);
            }
            tokens.push(self.next_token()?);
        }
    }

    pub fn tokenize_one(&mut self) -> Result<Token, RqlError> {
        self.skip_whitespace();
        if self.pos >= self.chars.len() {
            return Ok(Token::Eof);
        }
        self.next_token()
    }

    fn next_token(&mut self) -> Result<Token, RqlError> {
        let c = self.chars[self.pos];
        if c == '"' || c == '\'' {
            return self.read_string(c);
        }
        if c.is_ascii_digit() || (c == '-' && self.pos + 1 < self.chars.len() && self.chars[self.pos + 1].is_ascii_digit()) {
            return self.read_number();
        }
        if c.is_ascii_alphabetic() || c == '_' {
            return Ok(self.read_ident_or_keyword());
        }

        match c {
            '(' => { self.pos += 1; Ok(Token::LParen) }
            ')' => { self.pos += 1; Ok(Token::RParen) }
            ',' => { self.pos += 1; Ok(Token::Comma) }
            '.' => { self.pos += 1; Ok(Token::Dot) }
            '=' => { self.pos += 1; Ok(Token::Eq) }
            '!' => {
                if self.pos + 1 < self.chars.len() && self.chars[self.pos + 1] == '=' {
                    self.pos += 2;
                    Ok(Token::Ne)
                } else {
                    Err(RqlError::lex(self.pos, "expected '=' after '!'"))
                }
            }
            '<' => {
                if self.pos + 1 < self.chars.len() && self.chars[self.pos + 1] == '=' {
                    self.pos += 2;
                    Ok(Token::Le)
                } else {
                    self.pos += 1;
                    Ok(Token::Lt)
                }
            }
            '>' => {
                if self.pos + 1 < self.chars.len() && self.chars[self.pos + 1] == '=' {
                    self.pos += 2;
                    Ok(Token::Ge)
                } else {
                    self.pos += 1;
                    Ok(Token::Gt)
                }
            }
            '~' => {
                if self.pos + 1 < self.chars.len() && self.chars[self.pos + 1] == '=' {
                    self.pos += 2;
                    Ok(Token::Like)
                } else {
                    Err(RqlError::lex(self.pos, "expected '=' after '~'"))
                }
            }
            _ => Err(RqlError::lex(self.pos, format!("unexpected character '{c}'"))),
        }
    }

    fn read_string(&mut self, quote: char) -> Result<Token, RqlError> {
        let start = self.pos;
        self.pos += 1;
        let mut s = String::new();
        loop {
            if self.pos >= self.chars.len() {
                return Err(RqlError::lex(start, "unterminated string literal"));
            }
            let c = self.chars[self.pos];
            if c == '\\' {
                self.pos += 1;
                if self.pos < self.chars.len() {
                    s.push(match self.chars[self.pos] {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '"' => '"',
                        '\'' => '\'',
                        '\\' => '\\',
                        x => x,
                    });
                    self.pos += 1;
                }
            } else if c == quote {
                self.pos += 1;
                return Ok(Token::String(s));
            } else {
                s.push(c);
                self.pos += 1;
            }
        }
    }

    fn read_number(&mut self) -> Result<Token, RqlError> {
        let start = self.pos;
        if self.chars[self.pos] == '-' {
            self.pos += 1;
        }
        while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        if self.pos < self.chars.len() && self.chars[self.pos] == '.' {
            self.pos += 1;
            while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }
        let num_str: String = self.chars[start..self.pos].iter().collect();
        let num: f64 = num_str.parse().map_err(|_| RqlError::lex(start, format!("invalid number '{num_str}'")))?;

        let unit = if self.pos < self.chars.len() && self.chars[self.pos].is_ascii_alphabetic() {
            let ustart = self.pos;
            while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_alphabetic() {
                self.pos += 1;
            }
            Some(self.chars[ustart..self.pos].iter().collect())
        } else {
            None
        };

        Ok(Token::Number(num, unit))
    }

    fn read_ident_or_keyword(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.chars.len() && (self.chars[self.pos].is_ascii_alphanumeric() || self.chars[self.pos] == '_') {
            self.pos += 1;
        }
        let word: String = self.chars[start..self.pos].iter().collect();
        Token::keyword(&word).unwrap_or(Token::Ident(word))
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }
}

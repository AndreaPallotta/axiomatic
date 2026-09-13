use super::fol::{Equality, Term};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Ident(String),
    Number(String),
    Plus,
    Star,
    Minus,
    Slash,
    Caret,
    Ampersand,
    Pipe,
    Bang,
    Equal,
    Le,
    Ge,
    Lt,
    Gt,
    LParen,
    RParen,
    Comma,
}

/// Tokenizes an algebraic or logical formula
fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' | '\r' | '\n' => {
                chars.next();
            }
            '+' => {
                tokens.push(Token::Plus);
                chars.next();
            }
            '*' | '·' => {
                tokens.push(Token::Star);
                chars.next();
            }
            '-' => {
                tokens.push(Token::Minus);
                chars.next();
            }
            '/' => {
                tokens.push(Token::Slash);
                chars.next();
            }
            '^' => {
                tokens.push(Token::Caret);
                chars.next();
            }
            '&' | '∧' => {
                tokens.push(Token::Ampersand);
                chars.next();
            }
            '|' | '∨' => {
                tokens.push(Token::Pipe);
                chars.next();
            }
            '!' | '¬' | '~' => {
                tokens.push(Token::Bang);
                chars.next();
            }
            '=' => {
                tokens.push(Token::Equal);
                chars.next();
            }
            '<' => {
                chars.next();
                if let Some(&'=') = chars.peek() {
                    chars.next();
                    tokens.push(Token::Le);
                } else {
                    tokens.push(Token::Lt);
                }
            }
            '>' => {
                chars.next();
                if let Some(&'=') = chars.peek() {
                    chars.next();
                    tokens.push(Token::Ge);
                } else {
                    tokens.push(Token::Gt);
                }
            }
            '≤' => {
                tokens.push(Token::Le);
                chars.next();
            }
            '≥' => {
                tokens.push(Token::Ge);
                chars.next();
            }
            '(' => {
                tokens.push(Token::LParen);
                chars.next();
            }
            ')' => {
                tokens.push(Token::RParen);
                chars.next();
            }
            ',' => {
                tokens.push(Token::Comma);
                chars.next();
            }
            '0'..='9' => {
                let mut num = String::new();
                while let Some(&d) = chars.peek() {
                    if d.is_ascii_digit() {
                        num.push(d);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Number(num));
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let mut ident = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' {
                        ident.push(ch);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Ident(ident));
            }
            _ => return Err(format!("Unexpected character: '{}'", c)),
        }
    }

    Ok(tokens)
}

/// Recursive descent parser for terms
struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let t = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(t)
        } else {
            None
        }
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        match self.next() {
            Some(t) if t == expected => Ok(()),
            Some(t) => Err(format!("Expected {:?}, found {:?}", expected, t)),
            None => Err(format!("Expected {:?}, found EOF", expected)),
        }
    }

    /// Parses an equation or relational proposition: LHS = RHS, LHS <= RHS, LHS >= RHS, etc.
    fn parse_equality(&mut self) -> Result<Equality, String> {
        let lhs = self.parse_expr()?;
        if let Some(tok) = self.peek() {
            match tok {
                Token::Equal => {
                    self.next();
                    let rhs = self.parse_expr()?;
                    Ok(Equality::new(lhs, rhs))
                }
                Token::Le => {
                    self.next();
                    let rhs = self.parse_expr()?;
                    Ok(Equality::new(Term::func("<=", vec![lhs, rhs]), Term::constant("true")))
                }
                Token::Ge => {
                    self.next();
                    let rhs = self.parse_expr()?;
                    Ok(Equality::new(Term::func("<=", vec![rhs, lhs]), Term::constant("true")))
                }
                Token::Lt => {
                    self.next();
                    let rhs = self.parse_expr()?;
                    Ok(Equality::new(Term::func("<", vec![lhs, rhs]), Term::constant("true")))
                }
                Token::Gt => {
                    self.next();
                    let rhs = self.parse_expr()?;
                    Ok(Equality::new(Term::func("<", vec![rhs, lhs]), Term::constant("true")))
                }
                _ => {
                    if let Term::Func(ref op, _) = lhs {
                        if op == "<=" || op == "<" {
                            return Ok(Equality::new(lhs, Term::constant("true")));
                        }
                    }
                    Err(format!("Expected '=' or relational operator, found {:?}", tok))
                }
            }
        } else {
            if let Term::Func(ref op, _) = lhs {
                if op == "<=" || op == "<" {
                    return Ok(Equality::new(lhs, Term::constant("true")));
                }
            }
            Err("Expected '=' or relational operator, found EOF".to_string())
        }
    }

    /// Parses relational comparisons: A <= B, A >= B, A < B, A > B
    fn parse_expr(&mut self) -> Result<Term, String> {
        let mut left = self.parse_additive()?;

        while let Some(tok) = self.peek() {
            match tok {
                Token::Le => {
                    self.next();
                    let right = self.parse_additive()?;
                    left = Term::func("<=", vec![left, right]);
                }
                Token::Ge => {
                    self.next();
                    let right = self.parse_additive()?;
                    left = Term::func("<=", vec![right, left]);
                }
                Token::Lt => {
                    self.next();
                    let right = self.parse_additive()?;
                    left = Term::func("<", vec![left, right]);
                }
                Token::Gt => {
                    self.next();
                    let right = self.parse_additive()?;
                    left = Term::func("<", vec![right, left]);
                }
                _ => break,
            }
        }

        Ok(left)
    }

    /// Parses addition, subtraction, and Boolean OR: A + B, A - B, A | B
    fn parse_additive(&mut self) -> Result<Term, String> {
        let mut left = self.parse_multiplicative()?;

        while let Some(tok) = self.peek() {
            match tok {
                Token::Plus => {
                    self.next();
                    let right = self.parse_multiplicative()?;
                    left = Term::func("+", vec![left, right]);
                }
                Token::Minus => {
                    self.next();
                    let right = self.parse_multiplicative()?;
                    left = Term::func("+", vec![left, Term::func("-", vec![right])]);
                }
                Token::Pipe => {
                    self.next();
                    let right = self.parse_multiplicative()?;
                    left = Term::func("|", vec![left, right]);
                }
                _ => break,
            }
        }

        Ok(left)
    }

    /// Parses multiplication and Boolean AND: A * B, A & B
    fn parse_multiplicative(&mut self) -> Result<Term, String> {
        let mut left = self.parse_primary()?;

        while let Some(tok) = self.peek() {
            match tok {
                Token::Star => {
                    self.next();
                    let right = self.parse_primary()?;
                    left = Term::func("*", vec![left, right]);
                }
                Token::Slash => {
                    self.next();
                    let right = self.parse_primary()?;
                    left = Term::func("/", vec![left, right]);
                }
                Token::Caret => {
                    self.next();
                    let right = self.parse_primary()?;
                    left = Term::func("^", vec![left, right]);
                }
                Token::Ampersand => {
                    self.next();
                    let right = self.parse_primary()?;
                    left = Term::func("&", vec![left, right]);
                }
                _ => break,
            }
        }

        Ok(left)
    }

    /// Parses atoms, variables, functions, unary negation, and parenthesized expressions
    fn parse_primary(&mut self) -> Result<Term, String> {
        match self.next() {
            Some(Token::LParen) => {
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            Some(Token::Number(num)) => Ok(Term::constant(&num)),
            Some(Token::Ident(name)) => {
                if let Some(Token::LParen) = self.peek() {
                    self.next();
                    let mut args = Vec::new();
                    if let Some(Token::RParen) = self.peek() {
                        self.next();
                    } else {
                        loop {
                            args.push(self.parse_expr()?);
                            match self.peek() {
                                Some(Token::Comma) => {
                                    self.next();
                                }
                                Some(Token::RParen) => {
                                    self.next();
                                    break;
                                }
                                _ => return Err("Expected ',' or ')' in argument list".to_string()),
                            }
                        }
                    }
                    Ok(Term::func(&name, args))
                } else if name == "0" || name == "1" || name == "U" {
                    Ok(Term::constant(&name))
                } else {
                    Ok(Term::constant(&name))
                }
            }
            Some(Token::Minus) => {
                let inner = self.parse_primary()?;
                Ok(Term::func("-", vec![inner]))
            }
            Some(Token::Bang) => {
                let inner = self.parse_primary()?;
                Ok(Term::func("!", vec![inner]))
            }
            Some(tok) => Err(format!("Unexpected token in expression: {:?}", tok)),
            None => Err("Unexpected end of expression".to_string()),
        }
    }
}

/// Parses a string representation of an equality into an `Equality` AST
pub fn parse_conjecture(input: &str) -> Result<Equality, String> {
    let tokens = tokenize(input)?;
    let mut parser = Parser::new(tokens);
    let eq = parser.parse_equality()?;
    if parser.pos < parser.tokens.len() {
        return Err(format!(
            "Trailing unparsed tokens from position {}",
            parser.pos
        ));
    }
    Ok(eq)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_conjecture() {
        let eq = parse_conjecture("(x + 0) = (0 + x)").unwrap();
        assert_eq!(eq.lhs.to_string(), "(x + 0)");
        assert_eq!(eq.rhs.to_string(), "(0 + x)");
    }

    #[test]
    fn test_parse_nested_conjecture() {
        let eq = parse_conjecture("((x + 0) * 1) = (1 * x)").unwrap();
        assert_eq!(eq.lhs.to_string(), "((x + 0) * 1)");
        assert_eq!(eq.rhs.to_string(), "(1 * x)");
    }

    #[test]
    fn test_parse_boolean_logic() {
        let eq = parse_conjecture("!(a & b) = (!a | !b)").unwrap();
        assert_eq!(eq.lhs.to_string(), "!(a & b)");
    }

    #[test]
    fn test_parse_calculus() {
        let eq = parse_conjecture("D(x + 0) = 1").unwrap();
        assert_eq!(eq.lhs.to_string(), "D((x + 0))");
    }
}

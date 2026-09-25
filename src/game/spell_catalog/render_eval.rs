//! `$?` conditions and `${...}` arithmetic of spell descriptions.

use super::CatalogSpell;
use super::render::{Renderer, Unresolved, ValueToken, parse_value_token};

/// Parsed `${...}` body.
#[derive(Debug, PartialEq)]
pub(super) enum Expr {
    Number(f64),
    Value(ValueToken),
    Neg(Box<Expr>),
    Binary(char, Box<Expr>, Box<Expr>),
    Call(Function, Vec<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Function {
    Abs,
    Floor,
    Ceil,
    Max,
    Min,
    Gt,
    Gte,
    Lt,
    Lte,
    Cond,
}

impl Function {
    fn named(name: &str) -> Option<Self> {
        Some(match name.to_ascii_lowercase().as_str() {
            "abs" => Self::Abs,
            "floor" => Self::Floor,
            "ceil" => Self::Ceil,
            "max" => Self::Max,
            "min" => Self::Min,
            "gt" => Self::Gt,
            "gte" => Self::Gte,
            "lt" => Self::Lt,
            "lte" => Self::Lte,
            "cond" => Self::Cond,
            _ => return None,
        })
    }

    fn apply(self, args: &[f64]) -> Result<f64, Unresolved> {
        let flag = |value: bool| if value { 1.0 } else { 0.0 };
        Ok(match (self, args) {
            (Self::Abs, [x]) => x.abs(),
            (Self::Floor, [x]) => x.floor(),
            (Self::Ceil, [x]) => x.ceil(),
            (Self::Max, [a, b]) => a.max(*b),
            (Self::Min, [a, b]) => a.min(*b),
            (Self::Gt, [a, b]) => flag(a > b),
            (Self::Gte, [a, b]) => flag(a >= b),
            (Self::Lt, [a, b]) => flag(a < b),
            (Self::Lte, [a, b]) => flag(a <= b),
            (Self::Cond, [c, a, b]) => {
                if *c != 0.0 {
                    *a
                } else {
                    *b
                }
            }
            _ => return Err(Unresolved),
        })
    }
}

impl Expr {
    pub(super) fn eval(
        &self,
        value: &dyn Fn(&ValueToken) -> Result<f64, Unresolved>,
    ) -> Result<f64, Unresolved> {
        Ok(match self {
            Self::Number(number) => *number,
            Self::Value(token) => value(token)?,
            Self::Neg(inner) => -inner.eval(value)?,
            Self::Binary(op, lhs, rhs) => {
                let (lhs, rhs) = (lhs.eval(value)?, rhs.eval(value)?);
                match op {
                    '+' => lhs + rhs,
                    '-' => lhs - rhs,
                    '*' => lhs * rhs,
                    _ if rhs == 0.0 => return Err(Unresolved),
                    _ => lhs / rhs,
                }
            }
            Self::Call(function, args) => {
                let args = args
                    .iter()
                    .map(|arg| arg.eval(value))
                    .collect::<Result<Vec<_>, _>>()?;
                function.apply(&args)?
            }
        })
    }
}

pub(super) fn parse_expr(text: &str) -> Result<Expr, Unresolved> {
    let mut parser = Parser { text, pos: 0 };
    let expr = parser.sum()?;
    parser.skip_space();
    if parser.pos == text.len() {
        Ok(expr)
    } else {
        Err(Unresolved)
    }
}

/// Evaluates a `$?` condition for the viewing player.
pub(super) fn eval_condition(
    text: &str,
    renderer: &Renderer,
    spell: &CatalogSpell,
) -> Result<bool, Unresolved> {
    let mut parser = Parser { text, pos: 0 };
    let result = parser.or(renderer, spell)?;
    parser.skip_space();
    if parser.pos == text.len() {
        Ok(result)
    } else {
        Err(Unresolved)
    }
}

struct Parser<'t> {
    text: &'t str,
    pos: usize,
}

impl Parser<'_> {
    fn rest(&self) -> &str {
        &self.text[self.pos..]
    }

    fn skip_space(&mut self) {
        let trimmed = self.rest().trim_start();
        self.pos = self.text.len() - trimmed.len();
    }

    fn eat(&mut self, token: &str) -> bool {
        self.skip_space();
        let found = self.rest().starts_with(token);
        if found {
            self.pos += token.len();
        }
        found
    }

    fn sum(&mut self) -> Result<Expr, Unresolved> {
        let mut lhs = self.product()?;
        loop {
            let op = if self.eat("+") {
                '+'
            } else if self.eat("-") {
                '-'
            } else {
                return Ok(lhs);
            };
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(self.product()?));
        }
    }

    fn product(&mut self) -> Result<Expr, Unresolved> {
        let mut lhs = self.unary()?;
        loop {
            let op = if self.eat("*") {
                '*'
            } else if self.eat("/") {
                '/'
            } else {
                return Ok(lhs);
            };
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(self.unary()?));
        }
    }

    fn unary(&mut self) -> Result<Expr, Unresolved> {
        if self.eat("-") {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        if self.eat("(") {
            let inner = self.sum()?;
            return if self.eat(")") {
                Ok(inner)
            } else {
                Err(Unresolved)
            };
        }
        self.skip_space();
        if self
            .rest()
            .starts_with(|ch: char| ch.is_ascii_digit() || ch == '.')
        {
            return self.number().map(Expr::Number);
        }
        if self.eat("$") {
            return self.dollar();
        }
        Err(Unresolved)
    }

    fn number(&mut self) -> Result<f64, Unresolved> {
        self.skip_space();
        let len = self
            .rest()
            .bytes()
            .take_while(|byte| byte.is_ascii_digit() || *byte == b'.')
            .count();
        let number = self.rest()[..len].parse().map_err(|_| Unresolved)?;
        self.pos += len;
        Ok(number)
    }

    /// After a `$` inside an expression: a function call or a value token.
    fn dollar(&mut self) -> Result<Expr, Unresolved> {
        let name_len = self
            .rest()
            .bytes()
            .take_while(u8::is_ascii_alphabetic)
            .count();
        let function = Function::named(&self.rest()[..name_len]);
        if let Some(function) = function.filter(|_| self.rest()[name_len..].starts_with('(')) {
            self.pos += name_len + 1;
            let mut args = vec![self.sum()?];
            while self.eat(",") {
                args.push(self.sum()?);
            }
            return if self.eat(")") {
                Ok(Expr::Call(function, args))
            } else {
                Err(Unresolved)
            };
        }
        let token = parse_value_token(self.rest()).ok_or(Unresolved)?;
        self.pos += token.len;
        Ok(Expr::Value(token))
    }

    fn or(&mut self, renderer: &Renderer, spell: &CatalogSpell) -> Result<bool, Unresolved> {
        let mut result = self.and(renderer, spell)?;
        while self.eat("|") {
            result |= self.and(renderer, spell)?;
        }
        Ok(result)
    }

    fn and(&mut self, renderer: &Renderer, spell: &CatalogSpell) -> Result<bool, Unresolved> {
        let mut result = self.not(renderer, spell)?;
        while self.eat("&") {
            result &= self.not(renderer, spell)?;
        }
        Ok(result)
    }

    fn not(&mut self, renderer: &Renderer, spell: &CatalogSpell) -> Result<bool, Unresolved> {
        if self.eat("!") {
            return Ok(!self.not(renderer, spell)?);
        }
        if self.eat("(") {
            let inner = self.or(renderer, spell)?;
            return if self.eat(")") {
                Ok(inner)
            } else {
                Err(Unresolved)
            };
        }
        self.skip_space();
        if self.rest().starts_with('$') {
            return self.comparison(renderer, spell);
        }
        self.player_term(renderer)
    }

    /// `s123` known spell, `a123` aura on the player, `c2` second spec of the class.
    fn player_term(&mut self, renderer: &Renderer) -> Result<bool, Unresolved> {
        let kind = self.rest().chars().next().ok_or(Unresolved)?;
        self.pos += kind.len_utf8();
        let id = self.number()? as u32;
        let ctx = renderer.ctx;
        match kind {
            's' | 'S' => Ok(ctx.known_spells.contains(&id)),
            'a' | 'A' => Ok(ctx.auras.contains(&id)),
            'c' | 'C' => {
                let spec = ctx
                    .spec_id
                    .and_then(|spec| renderer.catalog.tabs.specs.get(&spec));
                spec.map(|spec| spec.order_index + 1 == id)
                    .ok_or(Unresolved)
            }
            _ => Err(Unresolved),
        }
    }

    /// `$w3>0`, `$s1=5`; a bare value is true when non-zero.
    fn comparison(
        &mut self,
        renderer: &Renderer,
        spell: &CatalogSpell,
    ) -> Result<bool, Unresolved> {
        let lhs = self.unary()?;
        let lhs = renderer.evaluate(&lhs, spell)?;
        let op = ["<=", ">=", "!=", "<", ">", "="]
            .into_iter()
            .find(|op| self.eat(op));
        let Some(op) = op else {
            return Ok(lhs != 0.0);
        };
        let rhs = self.unary()?;
        let rhs = renderer.evaluate(&rhs, spell)?;
        Ok(match op {
            "<=" => lhs <= rhs,
            ">=" => lhs >= rhs,
            "!=" => lhs != rhs,
            "<" => lhs < rhs,
            ">" => lhs > rhs,
            _ => lhs == rhs,
        })
    }
}

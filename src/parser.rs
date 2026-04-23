use crate::lexer::Token;
use crate::ast::*;
use anyhow::{Result, anyhow};
use logos::Logos;

pub struct Parser<'a> {
    lexer: logos::Lexer<'a, Token>,
    peeked: Option<(Token, String, logos::Span)>,
    input: &'a str,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            lexer: Token::lexer(input),
            peeked: None,
            input,
        }
    }

    fn peek(&mut self) -> Result<Token> {
        if self.peeked.is_none() {
            let token = match self.lexer.next() {
                Some(Ok(t)) => t,
                Some(Err(_)) => {
                    return Err(anyhow!("Lex error at offset {}", self.lexer.span().start));
                }
                None => Token::EndTerm, // EOF
            };
            let text = self.lexer.slice().to_string();
            let span = self.lexer.span();
            self.peeked = Some((token, text, span));
        }
        Ok(self.peeked.as_ref().unwrap().0)
    }

    fn next(&mut self) -> Result<Token> {
        self.peek()?;
        let (token, _, _) = self.peeked.take().unwrap();
        Ok(token)
    }

    fn current_text(&mut self) -> Result<String> {
        if let Some((_, text, _)) = &self.peeked {
            Ok(text.clone())
        } else {
            // This happens if next() was called and then current_text() is called without peek()
            // We should have peeked already.
            Err(anyhow!("Internal error: current_text called without peek"))
        }
    }

    fn expect(&mut self, expected: Token) -> Result<()> {
        let token = self.next()?;
        if token == expected {
            Ok(())
        } else {
            Err(anyhow!("Expected {:?}, got {:?}", expected, token))
        }
    }

    pub fn parse_class(&mut self) -> Result<ClassDef> {
        let name = self.parse_identifier()?;
        self.expect(Token::Equal)?;
        
        let super_class = if self.peek()? == Token::Identifier {
            Some(self.parse_identifier()?)
        } else {
            None
        };
        
        self.expect(Token::NewTerm)?;
        
        let instance_fields = self.parse_fields()?;
        let mut instance_methods = Vec::new();
        while self.is_pattern_start()? {
            instance_methods.push(self.parse_method()?);
        }
        
        let mut class_fields = Vec::new();
        let mut class_methods = Vec::new();
        if self.peek()? == Token::Separator {
            self.next()?;
            class_fields = self.parse_fields()?;
            while self.is_pattern_start()? {
                class_methods.push(self.parse_method()?);
            }
        }
        
        self.expect(Token::EndTerm)?;
        
        Ok(ClassDef {
            name,
            super_class,
            instance_fields,
            instance_methods,
            class_fields,
            class_methods,
        })
    }

    fn parse_fields(&mut self) -> Result<Vec<String>> {
        if self.peek()? == Token::Or {
            self.next()?;
            let mut fields = Vec::new();
            while self.peek()? == Token::Identifier {
                fields.push(self.parse_identifier()?);
            }
            self.expect(Token::Or)?;
            Ok(fields)
        } else {
            Ok(Vec::new())
        }
    }

    fn parse_method(&mut self) -> Result<MethodDef> {
        let signature = self.parse_signature()?;
        self.expect(Token::Equal)?;
        
        let body = if self.peek()? == Token::Primitive {
            self.next()?;
            MethodBody::Primitive
        } else {
            MethodBody::Block(self.parse_method_block()?)
        };
        
        Ok(MethodDef { signature, body })
    }

    fn parse_signature(&mut self) -> Result<Signature> {
        let token = self.peek()?;
        match token {
            Token::Identifier | Token::Primitive => {
                Ok(Signature::Unary(self.parse_identifier()?))
            }
            Token::Keyword => {
                let mut parts = Vec::new();
                while self.peek()? == Token::Keyword {
                    let key = self.current_text()?;
                    self.next()?;
                    let arg = self.parse_identifier()?;
                    parts.push((key, arg));
                }
                Ok(Signature::Keyword(parts))
            }
            _ => {
                let op = self.parse_binary_selector()?;
                let arg = self.parse_identifier()?;
                Ok(Signature::Binary(op, arg))
            }
        }
    }

    fn parse_identifier(&mut self) -> Result<String> {
        self.peek()?; // Ensure peeked
        let text = self.current_text()?;
        self.expect(Token::Identifier).or_else(|_| self.expect(Token::Primitive))?;
        Ok(text)
    }

    fn parse_binary_selector(&mut self) -> Result<String> {
        self.peek()?;
        let text = self.current_text()?;
        self.next()?;
        Ok(text)
    }

    fn is_pattern_start(&mut self) -> Result<bool> {
        match self.peek()? {
            Token::Identifier | Token::Primitive | Token::Keyword |
            Token::Or | Token::Comma | Token::Minus | Token::Equal | 
            Token::Not | Token::And | Token::Star | Token::Div | Token::Mod | 
            Token::Plus | Token::More | Token::Less | Token::At | Token::Per | Token::OperatorSequence => Ok(true),
            _ => Ok(false),
        }
    }

    fn parse_method_block(&mut self) -> Result<Block> {
        self.expect(Token::NewTerm)?;
        let block = self.parse_block_contents()?;
        self.expect(Token::EndTerm)?;
        Ok(block)
    }

    fn parse_block_contents(&mut self) -> Result<Block> {
        let mut locals = Vec::new();
        if self.peek()? == Token::Or {
            self.next()?;
            while self.peek()? == Token::Identifier {
                locals.push(self.parse_identifier()?);
            }
            self.expect(Token::Or)?;
        }
        
        let mut body = Vec::new();
        
        while let Ok(token) = self.peek() {
            if token == Token::EndTerm || token == Token::EndBlock {
                break;
            }
            if token == Token::Exit {
                self.next()?;
                body.push(Expression::Return(Box::new(self.parse_expression()?)));
                if let Ok(Token::Period) = self.peek() {
                    self.next()?;
                }
                break;
            }
            body.push(self.parse_expression()?);
            if let Ok(Token::Period) = self.peek() {
                self.next()?;
            } else {
                let next = self.peek()?;
                if next != Token::EndTerm && next != Token::EndBlock {
                    let offset = self.lexer.span().start;
                    return Err(anyhow!("Expected '.' or end of block, got {:?} ('{}') at offset {}", next, self.input[self.lexer.span()].to_string(), offset));
                }
                break;
            }
        }
        
        Ok(Block { parameters: Vec::new(), locals, body })
    }

    pub fn parse_expression(&mut self) -> Result<Expression> {
        self.parse_evaluation()
    }

    fn parse_evaluation(&mut self) -> Result<Expression> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek()? {
                Token::Identifier | Token::Primitive => {
                    let name = self.parse_identifier()?;
                    expr = Expression::Message(Box::new(expr), Message::Unary(name));
                }
                Token::Or | Token::Comma | Token::Minus | Token::Equal | 
                Token::Not | Token::And | Token::Star | Token::Div | Token::Mod | 
                Token::Plus | Token::More | Token::Less | Token::At | Token::Per | Token::OperatorSequence => {
                    let op = self.parse_binary_selector()?;
                    let arg = self.parse_primary()?;
                    let mut arg_expr = arg;
                    while let Token::Identifier | Token::Primitive = self.peek()? {
                         let name = self.parse_identifier()?;
                         arg_expr = Expression::Message(Box::new(arg_expr), Message::Unary(name));
                    }
                    expr = Expression::Message(Box::new(expr), Message::Binary(op, Box::new(arg_expr)));
                }
                Token::Keyword => {
                    let mut parts = Vec::new();
                    while self.peek()? == Token::Keyword {
                        let key = self.current_text()?;
                        self.next()?;
                        let arg = self.parse_primary()?;
                        let mut arg_expr = arg;
                        while let Token::Identifier | Token::Primitive = self.peek()? {
                             let name = self.parse_identifier()?;
                             arg_expr = Expression::Message(Box::new(arg_expr), Message::Unary(name));
                        }
                        while let Token::Or | Token::Comma | Token::Minus | Token::Equal | 
                            Token::Not | Token::And | Token::Star | Token::Div | Token::Mod | 
                            Token::Plus | Token::More | Token::Less | Token::At | Token::Per | Token::OperatorSequence = self.peek()? {
                            let op = self.parse_binary_selector()?;
                            let b_arg = self.parse_primary()?;
                            let mut b_arg_expr = b_arg;
                            while let Token::Identifier | Token::Primitive = self.peek()? {
                                 let name = self.parse_identifier()?;
                                 b_arg_expr = Expression::Message(Box::new(b_arg_expr), Message::Unary(name));
                            }
                            arg_expr = Expression::Message(Box::new(arg_expr), Message::Binary(op, Box::new(b_arg_expr)));
                        }
                        parts.push((key, Box::new(arg_expr)));
                    }
                    expr = Expression::Message(Box::new(expr), Message::Keyword(parts));
                    break;
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expression> {
        let token = self.peek()?;
        match token {
            Token::Identifier | Token::Primitive => {
                let name = self.parse_identifier()?;
                if self.peek()? == Token::Assign {
                    self.next()?;
                    let val = self.parse_expression()?;
                    Ok(Expression::Assignment(name, Box::new(val)))
                } else {
                    Ok(Expression::Variable(name))
                }
            }
            Token::NewTerm => {
                self.next()?;
                let expr = self.parse_expression()?;
                self.expect(Token::EndTerm)?;
                Ok(expr)
            }
            Token::NewBlock => {
                Ok(Expression::Block(self.parse_nested_block()?))
            }
            Token::Pound | Token::Integer | Token::Double | Token::STString | Token::Minus => {
                Ok(Expression::Literal(self.parse_literal()?))
            }
            _ => {
                let txt = self.current_text()?;
                let offset = self.lexer.span().start;
                Err(anyhow!("Unexpected token in primary: {:?} ('{}') at offset {}", token, txt, offset))
            }
        }
    }

    fn parse_nested_block(&mut self) -> Result<Block> {
        self.expect(Token::NewBlock)?;
        let mut parameters = Vec::new();
        if self.peek()? == Token::Colon {
            while self.peek()? == Token::Colon {
                self.next()?;
                parameters.push(self.parse_identifier()?);
            }
            self.expect(Token::Or)?;
        }
        let mut block = self.parse_block_contents()?;
        block.parameters = parameters;
        self.expect(Token::EndBlock)?;
        Ok(block)
    }

    fn parse_literal(&mut self) -> Result<Literal> {
        let token = self.peek()?;
        match token {
            Token::Integer => {
                let s = self.current_text()?;
                self.next()?;
                let bi = s.parse::<num_bigint::BigInt>()?;
                Ok(Literal::Integer(bi))
            }
            Token::Double => {
                let s = self.current_text()?;
                self.next()?;
                Ok(Literal::Double(s.parse()?))
            }
            Token::Minus => {
                self.next()?;
                let next = self.peek()?;
                match next {
                    Token::Integer => {
                        let s = self.current_text()?;
                        self.next()?;
                        let bi = format!("-{}", s).parse::<num_bigint::BigInt>()?;
                        Ok(Literal::Integer(bi))
                    }
                    Token::Double => {
                        let s = self.current_text()?;
                        self.next()?;
                        Ok(Literal::Double(-s.parse::<f64>()?))
                    }
                    _ => Err(anyhow!("Expected number after Minus in literal")),
                }
            }
            Token::STString => {
                let s = self.current_text()?;
                self.next()?;
                Ok(Literal::String(s[1..s.len()-1].to_string()))
            }
            Token::Pound => {
                self.next()?;
                let next = self.peek()?;
                match next {
                    Token::NewTerm => {
                        self.next()?;
                        let mut items = Vec::new();
                        while self.peek()? != Token::EndTerm {
                            items.push(self.parse_literal()?);
                        }
                        self.expect(Token::EndTerm)?;
                        Ok(Literal::Array(items))
                    }
                    _ => {
                         let mut last_span = self.lexer.span();
                         let mut sym = match self.next()? {
                             Token::Identifier | Token::Primitive | Token::Keyword |
                             Token::Or | Token::Comma | Token::Minus | Token::Equal | 
                             Token::Not | Token::And | Token::Star | Token::Div | Token::Mod | 
                             Token::Plus | Token::More | Token::Less | Token::At | Token::Per | Token::OperatorSequence => self.lexer.slice().to_string(),
                             Token::STString => {
                                 let s = self.lexer.slice().to_string();
                                 s[1..s.len()-1].to_string()
                             }
                             _ => return Err(anyhow!("Invalid symbol after #")),
                         };
                         while let Ok(Token::Keyword) = self.peek() {
                             let next_span = self.lexer.span();
                             if next_span.start == last_span.end {
                                 self.next()?;
                                 sym.push_str(&self.lexer.slice());
                                 last_span = self.lexer.span();
                             } else {
                                 break;
                             }
                         }
                         Ok(Literal::Symbol(sym))
                    }
                }
            }
            _ => Err(anyhow!("Expected literal, got {:?}", token)),
        }
    }
}

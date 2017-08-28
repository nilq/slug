use std::rc::Rc;

use super::*;
use super::ParserError;
use super::lexer::TokenType;

pub struct Parser {
    traveler: Traveler,
}

#[allow(dead_code)]
impl Parser {
    pub fn new(traveler: Traveler) -> Parser {
        Parser {
            traveler,
        }
    }

    pub fn parse(&mut self) -> ParserResult<Vec<Statement>> {
        let mut stack = Vec::new();
        while self.traveler.remaining() > 2 {
            if let Some(s) = self.statement()? {
                stack.push(s);
                self.traveler.next();
            }
        }

        Ok(stack)
    }
    
    pub fn skip_whitespace(&mut self) -> ParserResult<()> {
        while self.traveler.current_content() == "\n" || self.traveler.current().token_type == TokenType::EOL {
            self.traveler.next();
        }
        
        Ok(())
    }
    
    pub fn statement(&mut self) -> ParserResult<Option<Statement>> {
        match self.traveler.current().token_type {
            TokenType::EOL => {
                self.traveler.next();
                self.statement()
            },
            
            TokenType::Keyword => match self.traveler.current_content().as_str() {
                "fun" => {
                    self.traveler.next();

                    if self.traveler.current().token_type == TokenType::Identifier {
                        let name = Rc::new(self.traveler.current_content());
                        self.traveler.next();
                        
                        match self.traveler.current_content().as_str() {
                            "(" => {
                                self.traveler.next();

                                let mut param_names = Vec::new();
                                let mut param_types = Vec::new();
                                
                                while self.traveler.current_content() != ")" {
                                    param_names.push(self.traveler.expect(TokenType::Identifier)?);
                                    self.traveler.next();

                                    if self.traveler.current().token_type == TokenType::Type {
                                        param_types.push(get_type(&self.traveler.expect(TokenType::Type)?).unwrap());
                                        self.traveler.next();
                                    } else {
                                        param_types.push(Type::Any);
                                    }

                                    if self.traveler.current_content() == "," {
                                        self.traveler.next();
                                    }
                                }
                                
                                self.traveler.next();
                                
                                let mut t = None;
                                
                                if self.traveler.current().token_type == TokenType::Type {
                                    t = get_type(&self.traveler.current_content());
                                    self.traveler.next();
                                }

                                self.traveler.expect_content(":")?;
                                self.traveler.next();
                                
                                let body;
                                
                                if self.traveler.current_content() == "\n" {
                                    self.traveler.next();
                                    
                                    body = Rc::new(self.block()?);
                                } else {
                                    body = Rc::new(vec![Statement::Expression(Rc::new(self.expression()?))])
                                }
                                
                                Ok(Some(Statement::Fun {
                                    name,
                                    param_names: Rc::new(param_names),
                                    param_types: Rc::new(param_types),
                                    t,
                                    body,
                                }))
                            },
                            
                            _ => {
                                self.traveler.next();
                                
                                let mut t = None;
                                
                                if self.traveler.current().token_type == TokenType::Type {
                                    t = get_type(&self.traveler.current_content());
                                    self.traveler.next();
                                }

                                self.traveler.expect_content(":")?;
                                self.traveler.next();
                                
                                let body;
                                
                                if self.traveler.current_content() == "\n" {
                                    self.traveler.next();
                                    
                                    body = Rc::new(self.block()?);
                                } else {
                                    body = Rc::new(vec![Statement::Expression(Rc::new(self.expression()?))])
                                }
                                
                                Ok(Some(Statement::Fun {
                                    name,
                                    param_names: Rc::new(Vec::new()),
                                    param_types: Rc::new(Vec::new()),
                                    t,
                                    body,
                                }))
                            },
                        }

                    } else {
                        self.traveler.prev();
                        Ok(None)
                    }
                },
                _ => Err(ParserError::new_pos(self.traveler.current().position, &format!("unexpected: {}", self.traveler.current_content()))),
            },
            
            _ => Ok(Some(Statement::Expression(Rc::new(self.expression()?)))),
        }
    }
    
    pub fn term(&mut self) -> ParserResult<Expression> {
        self.skip_whitespace()?;
        match self.traveler.current().token_type {
            TokenType::EOL => {
                self.traveler.next();
                match self.traveler.current().token_type {
                    TokenType::Block(_) => return Ok(Expression::Block(Rc::new(self.block()?))),
                    TokenType::EOL      => return Ok(Expression::EOF),
                    _ => (),
                }
            },
            _ => (),
        }

        match self.traveler.current().token_type {
            TokenType::IntLiteral    => Ok(Expression::NumberLiteral(self.traveler.current_content().parse::<f64>().unwrap())),
            TokenType::FloatLiteral  => Ok(Expression::NumberLiteral(self.traveler.current_content().parse::<f64>().unwrap())),
            TokenType::BoolLiteral   => Ok(Expression::BoolLiteral(self.traveler.current_content() == "true")),
            TokenType::StringLiteral => Ok(Expression::StringLiteral(Rc::new(self.traveler.current_content().clone()))),
            TokenType::Identifier    => {
                let id = Expression::Identifier(Rc::new(self.traveler.current_content()));
                let name = Rc::new(self.traveler.current_content());
                
                self.traveler.next();

                if let Some(t) = self.types()? {                    
                    match self.traveler.current_content().as_str() {
                        "=" => {
                            self.traveler.next();
                            let expr = self.expression()?;
                            self.traveler.next();

                            return Ok(Expression::Definition(Some(t), name, Some(Rc::new(expr))))
                        },

                        _ => return Ok(Expression::Definition(Some(t), name, None)),
                    }
                }
                
                match self.traveler.current().token_type {
                    TokenType::IntLiteral |
                    TokenType::FloatLiteral |
                    TokenType::BoolLiteral |
                    TokenType::StringLiteral |
                    TokenType::Identifier |
                    TokenType::Symbol => {
                        if self.traveler.current().token_type == TokenType::Symbol {
                            match self.traveler.current_content().as_str() {
                                "(" | ")" | "," => (),
                                "!"       => return Ok(Expression::Call(Rc::new(id), Rc::new(vec!()))),
                                "="       => {
                                    self.traveler.next();
                                    let expr = self.expression()?;

                                    self.traveler.next();

                                    return Ok(Expression::Definition(None, name, Some(Rc::new(expr))))
                                },
                                
                                _   => return Err(ParserError::new_pos(self.traveler.current().position, &format!("unexpected: {}", self.traveler.current_content()))),
                            }
                        } else {
                            let call = self.call(id)?;

                            self.traveler.next();

                            return Ok(call)
                        }
                    },

                    _ => ()
                }

                self.traveler.prev();

                Ok(id)
            },
            TokenType::Symbol => match self.traveler.current_content().as_str() {
                "," | ")" => { // bad hack here
                    self.traveler.next();
                    return self.term()
                },
                _ => Err(ParserError::new_pos(self.traveler.current().position, &format!("unexpected symbol: {}", self.traveler.current_content()))),
            },
            _ => Err(ParserError::new_pos(self.traveler.current().position, &format!("unexpected: {:#?}", self.traveler.current()))),
        }
    }
    
    fn block(&mut self) -> ParserResult<Vec<Statement>> {
        match self.traveler.current().token_type {
            TokenType::Block(ref v) => {
                let mut p = Parser::new(Traveler::new(v.clone()));
                Ok(try!(p.parse()))
            },
            _ => Err(ParserError::new_pos(self.traveler.current().position, &format!("expected block, found: {}", self.traveler.current_content()))),
        }
    }

    fn call(&mut self, caller: Expression) -> ParserResult<Expression> {
        let mut args = Vec::new();

        while self.traveler.current_content() != ")" && self.traveler.current_content() != "\n" {
            args.push(try!(self.expression()));

            self.traveler.next();

            if self.traveler.current_content() == "," {
                self.traveler.next();
            }
        }

        Ok(Expression::Call(Rc::new(caller), Rc::new(args)))
    }
    
    fn types(&mut self) -> ParserResult<Option<Type>> {
        match self.traveler.current().token_type {
            TokenType::Type   => {
                let t = Ok(Some(get_type(&self.traveler.current_content()).unwrap()));
                self.traveler.next();
                t
            },
            TokenType::Symbol => match self.traveler.current_content().as_str() {
                "[" => {
                    self.traveler.next(); // a

                    let mut len = None;

                    if self.traveler.current_content() != "]" {
                        len = Some(Rc::new(self.expression()?));
                        self.traveler.next();
                    }
                    
                    self.traveler.expect_content("]")?;
                    self.traveler.next(); // b
                    
                    if self.traveler.current().token_type == TokenType::Type {
                        let t = Type::Array(len, Rc::new(get_type(&self.traveler.current_content()).unwrap()));
                        self.traveler.next();
                        
                        Ok(Some(t))
                    } else {
                        self.traveler.prev(); // b
                        self.traveler.prev(); // a
                        Ok(None)
                    }
                },
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }

    fn expression(&mut self) -> ParserResult<Expression> {
        if self.traveler.current_content() == "\n" {
            self.traveler.next();
        }
        
        let expr = self.term()?;
        
        self.traveler.next();
        
        if self.traveler.remaining() > 0 {
            if self.traveler.current().token_type == TokenType::Operator {
                return self.operation(expr)
            }
            
            self.traveler.prev();
        }

        Ok(expr)
    }
    
    fn operation(&mut self, expression: Expression) -> ParserResult<Expression> {
        let mut ex_stack = vec![expression];
        let mut op_stack: Vec<(Operand, u8)> = Vec::new();

        op_stack.push(get_operand(&self.traveler.current_content()).unwrap());
        self.traveler.next();

        if self.traveler.current_content() == "\n" {
            self.traveler.next();
        }

        ex_stack.push(self.term()?);

        let mut done = false;
        while ex_stack.len() > 1 {
            if !done && self.traveler.next() {
                if self.traveler.current().token_type != TokenType::Operator {
                    self.traveler.prev();
                    done = true;
                    continue
                }

                let (op, precedence) = get_operand(&self.traveler.current_content()).unwrap();

                if precedence >= op_stack.last().unwrap().1 {
                    let left  = ex_stack.pop().unwrap();
                    let right = ex_stack.pop().unwrap();

                    ex_stack.push(Expression::Operation {
                        right: Rc::new(left),
                        op:    op_stack.pop().unwrap().0,
                        left:  Rc::new(right)
                    });

                    self.traveler.next();

                    ex_stack.push(self.term()?);
                    op_stack.push((op, precedence));

                    continue
                }

                self.traveler.next();

                ex_stack.push(self.term()?);
                op_stack.push((op, precedence));
            }

            let left  = ex_stack.pop().unwrap();
            let right = ex_stack.pop().unwrap();

            ex_stack.push(Expression::Operation {
                right: Rc::new(left),
                op:    op_stack.pop().unwrap().0,
                left:  Rc::new(right)
            });
        }

        Ok(ex_stack.pop().unwrap())
    }
}

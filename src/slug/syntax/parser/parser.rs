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
            } else {
                stack.push(Statement::Expression(Rc::new(self.expression()?)))
            }
        }

        Ok(stack)
    }
    
    pub fn skip_whitespace(&mut self) -> ParserResult<()> {
        while self.traveler.current_content() == "\n" || self.traveler.current().token_type == TokenType::EOL {
            self.traveler.next();
            
            if self.traveler.remaining() < 2 {
                break
            }
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
                                    param_names.push(Rc::new(self.traveler.expect(TokenType::Identifier)?));
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
                        if self.traveler.current().token_type == TokenType::Type {
                            self.traveler.prev();
                            
                            Ok(Some(Statement::Expression(Rc::new(self.expression()?))))
                        } else {
                            self.traveler.prev();
                            Ok(None)
                        }
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
            TokenType::EOL => return Ok(Expression::EOF),

            TokenType::Block(_) => {
                let block = self.block()?;
                
                if block.len() > 1 {
                    return Err(ParserError::new_pos(self.traveler.current().position, &format!("can't termize several elements")))
                } else {
                    match block.get(0) {
                        Some(s) => match *s {
                            Statement::Expression(ref e) => {
                                let ref ex = *e.clone();
                                return Ok(ex.clone())
                            },
                            _ => (),
                        },
                        None => return Err(ParserError::new_pos(self.traveler.current().position, &format!("")))
                    }
                    return Ok(Expression::Block(Rc::new(self.block()?)))
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
                let mut id = Expression::Identifier(Rc::new(self.traveler.current_content()));

                self.traveler.next();

                if let Some(t) = self.types()? {                    
                    match self.traveler.current_content().as_str() {
                        "=" => {
                            self.traveler.next();
                            let expr = self.expression()?;
                            
                            return Ok(Expression::Definition(Some(t), Rc::new(id), Some(Rc::new(expr))))
                        },

                        _ => return Ok(Expression::Definition(Some(t), Rc::new(id), None)),
                    }
                } else if self.traveler.current_content() == "." {
                    self.traveler.next();

                    match self.expression()? {
                        Expression::Definition(ref t, ref a, ref b) => {
                            id = Expression::Definition(t.clone(), Rc::new(Expression::Index(Rc::new(id), a.clone())), b.clone());
                            self.traveler.next();
                        },
                        Expression::Call(ref a, ref args) => {
                            id = Expression::Call(Rc::new(Expression::Index(Rc::new(id), a.clone())), args.clone());
                            self.traveler.next();
                        },
                        e => {
                            id = Expression::Index(Rc::new(id), Rc::new(e));
                            self.traveler.next();
                        },
                    }
                }

                match self.traveler.current().token_type {
                    TokenType::IntLiteral |
                    TokenType::FloatLiteral |
                    TokenType::BoolLiteral |
                    TokenType::StringLiteral |
                    TokenType::Identifier => {
                        let call = self.call(id)?;

                        Ok(call)
                    },

                    TokenType::Symbol => match self.traveler.current_content().as_str() {
                        "(" | ")" | "," => {
                            self.traveler.prev();

                            Ok(id)
                        },
                        "!"       => Ok(Expression::Call(Rc::new(id), Rc::new(vec!()))),
                        "="       => {                            
                            self.traveler.next();
                            let expr = self.expression()?;

                            Ok(Expression::Definition(None, Rc::new(id), Some(Rc::new(expr))))
                        },

                        _   => Err(ParserError::new_pos(self.traveler.current().position, &format!("unexpected: {}", self.traveler.current_content()))),
                    },
                    _ => {
                        self.traveler.prev();

                        Ok(id)
                    },
                }
            },
            TokenType::Keyword => match self.traveler.current_content().as_str() {
                "fun" => {
                    self.traveler.next();
                    
                    match self.traveler.current_content().as_str() {
                        "(" => {
                            self.traveler.next();

                            let mut param_names = Vec::new();
                            let mut param_types = Vec::new();

                            while self.traveler.current_content() != ")" {
                                param_names.push(Rc::new(self.traveler.expect(TokenType::Identifier)?));
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

                            Ok(Expression::Fun {
                                param_names: Rc::new(param_names),
                                param_types: Rc::new(param_types),
                                t,
                                body,
                            })
                        },
                        
                        _ => {
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

                            Ok(Expression::Fun {
                                param_names: Rc::new(Vec::new()),
                                param_types: Rc::new(Vec::new()),
                                t,
                                body,
                            })
                        },
                    }
                },

                _ => Err(ParserError::new_pos(self.traveler.current().position, &format!("unexpected symbol: {}", self.traveler.current_content()))),
            },
            TokenType::Symbol => match self.traveler.current_content().as_str() {
                "[" => {
                    self.traveler.next();
                    
                    let mut body = Vec::new();

                    while self.traveler.current_content() != "]" {
                        self.skip_whitespace()?;

                        let expr = self.expression()?;

                        if expr == Expression::EOF {
                            println!("eow");
                            while self.traveler.current_content() != "]" {
                                println!("hey: {}", self.traveler.current_content());
                                self.traveler.prev();
                            }
                            break
                        }

                        body.push(expr);

                        self.skip_whitespace()?;
                        
                        self.traveler.next();
                        println!("next here: {}", self.traveler.current_content());

                        if self.traveler.current_content() == "," {
                            self.traveler.next();
                        }
                        
                        self.skip_whitespace()?;
                        println!("next white: {}", self.traveler.current_content())
                    }
                    
                    println!("body: {:#?}", body);

                    println!("nextest: {}", self.traveler.current_content());

                    self.traveler.next();

                    println!("nextest2: {:?}", self.traveler.current().token_type);
                    
                    Ok(Expression::DictLiteral(Rc::new(body)))
                },
                "(" => {
                    self.traveler.next();
                    let expr = self.expression()?;
                    self.traveler.next();

                    self.skip_whitespace()?;

                    self.traveler.expect_content(")")?;

                    self.traveler.next();

                    match self.traveler.current().token_type {
                        TokenType::IntLiteral |
                        TokenType::FloatLiteral |
                        TokenType::BoolLiteral |
                        TokenType::StringLiteral |
                        TokenType::Identifier |
                        TokenType::Symbol => {
                            if self.traveler.current().token_type == TokenType::Symbol {
                                match self.traveler.current_content().as_str() {
                                    "!"  => {
                                        self.traveler.next();
                                        return Ok(Expression::Call(Rc::new(expr), Rc::new(vec!())));
                                    },
                                    "(" => (),
                                    "=" => {
                                        self.traveler.next();
                                        let expr_right = self.expression()?;

                                        return Ok(Expression::Definition(None, Rc::new(expr), Some(Rc::new(expr_right))))
                                    },
                                    _   => return Err(ParserError::new_pos(self.traveler.current().position, &format!("unexpected: {}", self.traveler.current_content()))),
                                }
                            }

                            let call = self.call(expr)?;

                            self.traveler.next();

                            return Ok(call)
                        },

                        _ => (),
                    }

                    self.traveler.prev();

                    Ok(expr)
                },

                _ => Err(ParserError::new_pos(self.traveler.current().position, &format!("unexpected symbol: {}", self.traveler.current_content()))),
            },
            _ => Err(ParserError::new_pos(self.traveler.current().position, &format!("unexpected: {:#?}", self.traveler.current_content()))),
        }
    }
    
    fn block(&mut self) -> ParserResult<Vec<Statement>> {        
        match self.traveler.current().token_type.clone() {
            TokenType::Block(ref v) => {
                let mut p = Parser::new(Traveler::new(v.clone()));
                Ok(try!(p.parse()))
            },
            _ => Ok(vec![Statement::Expression(Rc::new(self.expression()?))]),
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
        
        if self.traveler.current_content() == ")" {
            self.traveler.next();
        }
        
        Ok(Expression::Call(Rc::new(caller), Rc::new(args)))
    }
    
    fn types(&mut self) -> ParserResult<Option<Type>> {
        match self.traveler.current().token_type {
            TokenType::Type   => {
                let t = get_type(&self.traveler.current_content()).unwrap();
                self.traveler.next();

                match self.traveler.current_content().as_str() {
                    ".." => {
                        self.traveler.next();
                        
                        Ok(Some(Type::Array(Rc::new(t))))
                    },

                    _ => Ok(Some(t))
                }
            },
            _ => Ok(None),
        }
    }

    fn expression(&mut self) -> ParserResult<Expression> {
        if self.traveler.current_content() == "\n" {
            self.traveler.next();
        }
        
        let expr = self.term()?;

        if expr == Expression::EOF {
            return Ok(expr)
        }

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
                
                self.traveler.next();

                if precedence >= op_stack.last().unwrap().1 {
                    let left  = ex_stack.pop().unwrap();
                    let right = ex_stack.pop().unwrap();

                    ex_stack.push(Expression::Operation {
                        right: Rc::new(left),
                        op:    op_stack.pop().unwrap().0,
                        left:  Rc::new(right)
                    });

                    ex_stack.push(self.term()?);
                    op_stack.push((op, precedence));

                    continue
                }

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
        
        self.traveler.next();

        Ok(ex_stack.pop().unwrap())
    }
}

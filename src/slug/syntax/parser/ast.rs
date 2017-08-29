use std::rc::Rc;

use super::{ParserResult, ParserError};
use super::super::{SymTab, TypeTab};

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Block(Rc<Vec<Statement>>),
    NumberLiteral(f64),
    StringLiteral(Rc<String>),
    Identifier(Rc<String>),
    BoolLiteral(bool),
    DictLiteral(Rc<Vec<Expression>>),
    Call(Rc<Expression>, Rc<Vec<Expression>>),
    Definition(Option<Type>, Rc<String>, Option<Rc<Expression>>),
    EOF,
    Operation {
        left:  Rc<Expression>,
        op:    Operand,
        right: Rc<Expression>,
    },
    Fun {
        param_names: Rc<Vec<Rc<String>>>,
        param_types: Rc<Vec<Type>>,
        t:           Option<Type>,
        body:        Rc<Vec<Statement>>,
    },
}

#[allow(dead_code)]
impl Expression {
    pub fn get_type(&self, sym: &Rc<SymTab>, env: &Rc<TypeTab>) -> ParserResult<Type> {
        match *self {
            Expression::NumberLiteral(_)  => Ok(Type::Num),
            Expression::StringLiteral(_)  => Ok(Type::Str),
            Expression::BoolLiteral(_)    => Ok(Type::Bool),
            Expression::Identifier(ref n) => match sym.get_name(&*n) {
                Some((i, env_index)) => {
                    Ok(env.get_type(i, env_index).unwrap())
                },
                None => Err(ParserError::new(&format!("unexpected use of: {}", n))),
            },
            
            Expression::Definition(ref t, _, ref expr) => {
                match *t {
                    Some(ref tp) => return Ok(tp.clone()),
                    None         => if let &Some(ref e) = expr {
                        Ok(e.get_type(sym, env)?)
                    } else {
                        Ok(Type::Any)
                    },
                }
            },
            
            Expression::Fun { ref t, ref param_names, ref param_types, ref body, } => {
                let mut tp = Vec::new();
                
                if let &Some(ref t) = t {
                    tp.push(t.clone())
                } else {
                    tp.push(Type::Any)
                }
                
                for t in param_types.iter() {
                    tp.push(t.clone())
                }
                
                Ok(Type::Fun(Rc::new(tp)))
            },
            
            Expression::Call(ref id, ref args) => match id.get_type(sym, env)? {
                Type::Fun(ref params) => Ok(params.get(0).unwrap().clone()),
                Type::Any => Ok(Type::Any),
                _         => Err(ParserError::new(&format!("{}: can't call non-fun", id))),
            },
            
            Expression::Operation { ref left, ref op, ref right, } => Ok(op.operate((left.get_type(sym, env)?, right.get_type(sym, env)?))?),
            
            _ => Ok(Type::Undefined),
        }
    }
    
    pub fn visit(&self, sym: &Rc<SymTab>, env: &Rc<TypeTab>) -> ParserResult<()> {
        match *self {
            Expression::Identifier(ref id) => match sym.get_name(&*id) {
                Some(_) => {
                    Ok(())
                },
                None => Err(ParserError::new(&format!("use of undeclared: {}", id))),
            },
            
            Expression::DictLiteral(ref body) => {
                let local_sym = Rc::new(SymTab::new(sym.clone(), &Vec::new()));
                let local_env = Rc::new(TypeTab::new(env.clone(), &Vec::new()));

                for s in body.iter() {
                    s.visit(&local_sym, &local_env)?
                }

                Ok(())
            },

            Expression::Definition(ref t, ref name, ref e) => {
                if let &Some(ref expr) = e {
                    expr.visit(sym, env)?;

                    let tp = match *t {
                        Some(ref tt) => {
                            let right_hand = &expr.get_type(sym, env)?;
                            if !tt.compare(right_hand) {
                                return Err(ParserError::new(&format!("{}: expected '{:?}', got '{:?}'", name, tt, right_hand)))
                            }
                            tt.clone()
                        },
                        None => Type::Any,
                    };
                    
                    match sym.get_name(&name) {
                        Some((i, env_index)) => {
                            match env.get_type(i, env_index) {
                                Ok(tp2) => if !tp2.compare(&tp) {
                                    return Err(ParserError::new(&format!("{}: can't mutate type", name)))
                                },
                                Err(e) => return Err(ParserError::new(&format!("{}", e))),
                            }
                        },
                        None => (),
                    }
                
                    let index = sym.add_name(name);
                    if index >= env.size() {
                        env.grow();
                    }

                    if let Err(e) = env.set_type(index, 0, tp) {
                        Err(ParserError::new(&format!("error setting type: {}", e)))
                    } else {
                        Ok(())
                    }
                } else {
                    Ok(())
                }
            },

            Expression::Fun { ref t, ref param_names, ref param_types, ref body, } => {
                let local_sym = Rc::new(SymTab::new(sym.clone(), &param_names.as_slice()));
                let local_env = Rc::new(TypeTab::new(env.clone(), &param_types));

                for statement in body.iter() {
                    statement.visit(&local_sym, &local_env)?;

                    if let &Some(ref t) = t {
                        let returned_type = statement.get_type(&local_sym, &local_env)?;
                        if returned_type != *t {
                            match *t {
                                Type::Any => (),
                                _         => return Err(ParserError::new(&format!("fun mismatched return type: '{:?}' got '{:?}'", t, returned_type))),
                            }
                        }
                    }
                }

                Ok(())
            },

            Expression::Call(ref id, ref args) => {
                match try!(id.get_type(sym, env)) {
                    Type::Fun(ref params) => {
                        let mut arg_types = Vec::new();

                        for arg in args.iter() {
                            arg_types.push(try!(arg.get_type(sym, env)));
                        }

                        match params[params.len() - 1] {
                            Type::Many(ref t) => {
                                if params[1..params.len() - 1].to_vec() != arg_types.as_slice()[1 .. params.len() - 1].to_vec() {
                                    Err(ParserError::new(&format!("{}: supplied very wrong args", id)))
                                } else {
                                    for arg_t in arg_types[params.len() - 1 ..].iter() {
                                        if !arg_t.compare(&**t) {
                                            return Err(ParserError::new(&format!("{}: expected '{:?}' got '{:?}'", id, t, arg_t)))
                                        }
                                    }
                                    Ok(())
                                }
                            },
                            _ => if params[1..].to_vec() != arg_types.as_slice() {
                                Err(ParserError::new(&format!("{}: supplied very wrong args", id)))
                            } else {
                                Ok(())
                            },
                        }
                    },

                    Type::Any => Ok(()),

                    _ => Err(ParserError::new(&format!("{}: calling non-funs is a sin", id))),
                }
            }

            _ => Ok(())
        }
    }
    
    pub fn lua(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            _ => Ok(()),
        }
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.lua(f)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Expression(Rc<Expression>),
    Fun {
        name:        Rc<String>,
        param_names: Rc<Vec<Rc<String>>>,
        param_types: Rc<Vec<Type>>,
        t:           Option<Type>,
        body:        Rc<Vec<Statement>>,
    },
}

impl Statement {
    pub fn visit(&self, sym: &Rc<SymTab>, env: &Rc<TypeTab>) -> ParserResult<()> {
        match *self {
            Statement::Expression(ref e) => e.visit(sym, env),
            Statement::Fun { ref name, ref t, ref param_names, ref param_types, ref body, } => {
                match sym.get_name(&name) {
                    Some((_, _)) => return Err(ParserError::new(&format!("{}: already declared", name))),
                    None => {
                        let index = sym.add_name(&name);
                        if index >= env.size() {
                            env.grow();
                        }

                        if let Err(e) = env.set_type(index, 0, try!(self.get_type(sym, env))) {
                            panic!("error setting type: {}", e)
                        }
                    },
                }

                let local_sym = Rc::new(SymTab::new(sym.clone(), &param_names));
                let local_env = Rc::new(TypeTab::new(env.clone(), &param_types));

                if let &Some(ref t) = t {
                    for statement in body.iter() {
                        statement.visit(&local_sym, &local_env)?;
                        if statement.get_type(&local_sym, &local_env)? != *t {
                            match *t {
                                Type::Any => (),
                                _ => return Err(ParserError::new(&format!("{}: mismatched return type", name))),
                            }
                        }
                    }
                }
                
                Ok(())
            },
        }
    }

    pub fn get_type(&self, sym: &Rc<SymTab>, env: &Rc<TypeTab>) -> ParserResult<Type> {
        match *self {
            Statement::Expression(ref e) => e.get_type(sym, env),
            Statement::Fun {ref name, ref t, ref param_names, ref param_types, ref body, } => {
                let mut tp = Vec::new();
                
                if let &Some(ref t) = t {
                    tp.push(t.clone())
                } else {
                    tp.push(Type::Any)
                }
                
                for t in param_types.iter() {
                    tp.push(t.clone())
                }

                Ok(Type::Fun(Rc::new(tp)))
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Str, Num, Bool, Any, Nil, Array(Option<Rc<Expression>>, Rc<Type>), Undefined,
    Fun(Rc<Vec<Type>>), Many(Rc<Type>),
}

#[allow(unused)]
impl Type {
    pub fn compare(&self, other: &Type) -> bool {
        if self == &Type::Any || other == &Type::Any {
            true
        } else {
            self == other
        }
    }
}

pub fn get_type(v: &str) -> Option<Type> {
    match v {
        "str"  => Some(Type::Str),
        "num"  => Some(Type::Num),
        "bool" => Some(Type::Bool),
        "any"  => Some(Type::Any),
        "nil"  => Some(Type::Nil),
        _      => None,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    Pow,
    Mul, Div, Mod,
    Add, Sub,
    Equal, NEqual,
    Lt, Gt, LtEqual, GtEqual,
    And, Or, Not,
}

impl Operand {
    pub fn operate(&self, lr: (Type, Type)) -> ParserResult<Type> {
        match *self {
            Operand::Pow => match lr {
                (Type::Num, Type::Num) => Ok(Type::Num),
                (Type::Any, Type::Num) => Ok(Type::Any),
                (Type::Num, Type::Any) => Ok(Type::Any),
                (Type::Str, Type::Num) => Ok(Type::Str),
                (Type::Str, Type::Any) => Ok(Type::Any),
                (Type::Any, Type::Any) => Ok(Type::Any),
                (a, b) => Err(ParserError::new(&format!("failed to pow: {:?} and {:?}", a, b))),
            },

            Operand::Mul => match lr {
                (Type::Num, Type::Num)  => Ok(Type::Num),
                (Type::Any, Type::Num)  => Ok(Type::Any),
                (Type::Num, Type::Any)  => Ok(Type::Any),
                (Type::Str, Type::Num)  => Ok(Type::Str),
                (Type::Str, Type::Str)  => Ok(Type::Str),
                (Type::Any, Type::Any)  => Ok(Type::Any),
                (a, b) => Err(ParserError::new(&format!("failed to multiply: {:?} and {:?}", a, b))),
            },

            Operand::Div => match lr {
                (Type::Num, Type::Num)  => Ok(Type::Num),
                (Type::Any, Type::Num)  => Ok(Type::Any),
                (Type::Num, Type::Any)  => Ok(Type::Any),
                (Type::Any, Type::Any)  => Ok(Type::Any),
                (a, b) => Err(ParserError::new(&format!("failed to divide: {:?} and {:?}", a, b))),
            },

            Operand::Mod => match lr {
                (Type::Num, Type::Num)  => Ok(Type::Num),
                (Type::Any, Type::Num)  => Ok(Type::Any),
                (Type::Num, Type::Any)  => Ok(Type::Any),
                (Type::Any, Type::Any)  => Ok(Type::Any),
                (a, b) => Err(ParserError::new(&format!("failed to mod: {:?} and {:?}", a, b))),
            },

            Operand::Add => match lr {
                (Type::Num, Type::Num)  => Ok(Type::Num),
                (Type::Any, Type::Num)  => Ok(Type::Any),
                (Type::Num, Type::Any)  => Ok(Type::Any),
                (Type::Str, Type::Num)  => Ok(Type::Str),
                (Type::Str, Type::Str)  => Ok(Type::Str),
                (Type::Str, Type::Bool) => Ok(Type::Str),
                (Type::Any, Type::Any)  => Ok(Type::Any),
                (a, b) => Err(ParserError::new(&format!("failed to add: {:?} and {:?}", a, b))),
            },

            Operand::Sub => match lr {
                (Type::Num, Type::Num)  => Ok(Type::Num),
                (Type::Any, Type::Num)  => Ok(Type::Any),
                (Type::Num, Type::Any)  => Ok(Type::Any),
                (Type::Str, Type::Num)  => Ok(Type::Str),
                (Type::Str, Type::Str)  => Ok(Type::Str),
                (Type::Any, Type::Any)  => Ok(Type::Any),
                (a, b) => Err(ParserError::new(&format!("failed to subtract: {:?} and {:?}", a, b))),
            },

            Operand::Equal | Operand::NEqual => Ok(Type::Bool),

            Operand::Lt | Operand::Gt | Operand::LtEqual | Operand::GtEqual => match lr {
                (a @ Type::Bool, b @ _) => Err(ParserError::new(&format!("failed to '{:?} < {:?}'", a, b))),
                (a @ _, b @ Type::Bool) => Err(ParserError::new(&format!("failed to '{:?} < {:?}'", a, b))),
                (a @ Type::Str, b @ _)  => Err(ParserError::new(&format!("failed to '{:?} < {:?}'", a, b))),
                (a @ _, b @ Type::Str)  => Err(ParserError::new(&format!("failed to '{:?} < {:?}'", a, b))),
                _ => Ok(Type::Bool),
            },

            Operand::And | Operand::Or | Operand::Not => Ok(Type::Bool),
        }
    }

    pub fn translate_lua(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Operand::Pow     => write!(f, "^"),
            Operand::Mul     => write!(f, "*"),
            Operand::Div     => write!(f, "/"),
            Operand::Mod     => write!(f, "%"),
            Operand::Add     => write!(f, "+"),
            Operand::Sub     => write!(f, "-"),
            Operand::Equal   => write!(f, "=="),
            Operand::NEqual  => write!(f, "~="),
            Operand::Lt      => write!(f, "<"),
            Operand::Gt      => write!(f, ">"),
            Operand::LtEqual => write!(f, "<="),
            Operand::GtEqual => write!(f, ">="),
            Operand::And     => write!(f, "and"),
            Operand::Or      => write!(f, "or"),
            Operand::Not     => write!(f, "not"),
        }
    }
}

pub fn get_operand(v: &str) -> Option<(Operand, u8)> {
    match v {
        "^"   => Some((Operand::Pow, 0)),
        "*"   => Some((Operand::Mul, 1)),
        "/"   => Some((Operand::Div, 1)),
        "%"   => Some((Operand::Mod, 1)),
        "+"   => Some((Operand::Add, 2)),
        "-"   => Some((Operand::Sub, 2)),
        "=="  => Some((Operand::Equal, 3)),
        "!="  => Some((Operand::NEqual, 3)),
        "<"   => Some((Operand::Lt, 4)),
        ">"   => Some((Operand::Gt, 4)),
        "<="  => Some((Operand::LtEqual, 4)),
        ">="  => Some((Operand::GtEqual, 4)),
        "!"   => Some((Operand::Not, 4)),
        "and" => Some((Operand::And, 4)),
        "or"  => Some((Operand::Or, 4)),
        _ => None,
    }
}

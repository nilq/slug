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
    Index(Rc<Expression>, Rc<Expression>),
    Definition(Option<Type>, Rc<Expression>, Option<Rc<Expression>>),
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

            Expression::Index(ref a, ref b) => {
                match a.get_type(&sym, &env)? {
                    Type::Array(ref t) => {
                        let ref tt = *t.clone(); // uhm ..
                        Ok(tt.clone())
                    },
                    _ => Err(ParserError::new(&format!("{:?}: trying to index '{:?}'", a, b)))
                }
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

            Expression::DictLiteral(ref content) => {
                if let Some(t) = content.get(0) {
                    Ok(Type::Array(Rc::new(t.get_type(&sym, &env)?)))
                } else {
                    Ok(Type::Array(Rc::new(Type::Nil)))
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
            
            Expression::Index(ref a, ref b) => { 
                a.visit(&sym, &env)?;

                Ok(())
            },

            Expression::DictLiteral(ref content) => {
                let local_sym = Rc::new(SymTab::new(sym.clone(), &Vec::new()));
                let local_env = Rc::new(TypeTab::new(env.clone(), &Vec::new()));

                let mut tp = Type::Any;

                if let Some(t) = content.get(0) {
                    tp = t.clone().get_type(&sym, &env)?;
                }

                for s in content.iter() {
                    let t = s.get_type(&sym, &env)?;
                    if !tp.compare(&t) {
                        return Err(ParserError::new(&format!("mismatched array type: expected '{:?}' got '{:?}'", tp, t)))
                    }

                    s.visit(&local_sym, &local_env)?
                }

                Ok(())
            },

            Expression::Definition(ref t, ref id, ref e) => {
                if let &Some(ref expr) = e {
                    expr.visit(sym, env)?;

                    let tp = match *t {
                        Some(ref tt) => {
                            let right_hand = &expr.get_type(sym, env)?;
                            if !tt.compare(right_hand) {
                                return Err(ParserError::new(&format!("{}: expected '{:?}', got '{:?}'", id, tt, right_hand)))
                            }
                            tt.clone()
                        },
                        None => expr.get_type(sym, env)?,
                    };
                    
                    match **id {
                        Expression::Identifier(ref name) => {
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
                        },

                        Expression::Index(ref a, ref b) => {
                            a.visit(&sym, &env)?;

                            Ok(())
                        },
                        
                        _ => Err(ParserError::new(&format!("{}: failed to assign", id))),
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
                match id.get_type(sym, env)? {
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
    
    pub fn global(&self) -> Rc<String> {
        match *self {
            Expression::Definition(_, ref name, ref expr) => {
                if let &Some(ref e) = expr {
                    Rc::new(format!("{} = {}", name, e))
                } else {
                    Rc::new(format!("{}", name))
                }
            },
            
            _ => Rc::new(format!("{}", self)),
        }
    }


    pub fn lua(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Expression::Block(ref statements) => {
                for s in statements.iter() {
                    s.lua(f)?;
                }

                Ok(())
            },
            Expression::NumberLiteral(ref n) => write!(f, "{}", n),
            Expression::StringLiteral(ref n) => write!(f, "\"{}\"", n),
            Expression::BoolLiteral(ref n)   => write!(f, "{}", n),
            Expression::Identifier(ref n)    => write!(f, "{}", n),
            Expression::Definition(_, ref name, ref expr) => {
                match **name {
                    Expression::Index(_, _) => {
                        if let &Some(ref e) = expr {
                            writeln!(f, "{} = {}", name, e)?;
                        }
                    },
                    _ => (),
                }
                if let &Some(ref e) = expr {
                    writeln!(f, "local {} = {}", name, e)
                } else {
                    writeln!(f, "local {}", name)
                }
            },
            
            Expression::Index(ref a, ref b) => {
                match **b {
                    Expression::Identifier(_) => write!(f, "{}.{}", a, b),
                    _ => write!(f, "{}[{}]", a, b),
                }
            },

            Expression::DictLiteral(ref body)  => {
                write!(f, "{{")?;
                
                for e in body.iter() {
                    write!(f, "{},", e.global())?;
                }
                
                write!(f, "}}")
            },

            Expression::Call(ref id, ref args) => {
                write!(f, "{}", id)?;
                write!(f, "(")?;

                let mut acc = 1;
                for e in args.iter() {
                    write!(f, "{}", e)?;
                    if acc != args.len() {
                        write!(f, ",")?;
                    }
                    acc += 1;
                }

                write!(f, ")")
            },

            Expression::Fun {
                ref t, ref param_names, ref param_types, ref body,
            } => {
                write!(f, "function")?;

                write!(f, "(")?;

                for e in param_names.iter() {
                    write!(f, "{}", e)?;
                    if e != param_names.last().unwrap() {
                        write!(f, ",")?;
                    }
                }
                
                writeln!(f, ")")?;
                
                for s in body.iter() {
                    if s == body.last().unwrap() {
                        match *s {
                            Statement::Expression(ref e) => { write!(f, "return {}\n", e)?; },
                            _ => { write!(f, "{}", s)?; },
                        }
                    } else {
                        write!(f, "{}", s)?;
                    }
                }
                
                write!(f, "end")
            },
            
            Expression::Operation {
                ref left, ref op, ref right,
            } => {
                write!(f, "{}", left)?;
                write!(f, " {} ", op)?;
                write!(f, "{}", right)
            },

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

    pub fn lua(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Statement::Expression(ref e) => write!(f, "{}", e),
            Statement::Fun {
                ref name, ref t, ref param_names, ref param_types, ref body,
            } => {
                write!(f, "function")?;
                
                write!(f, "{}", name)?;

                write!(f, "(")?;

                for e in param_names.iter() {
                    write!(f, "{}", e)?;
                    if e != param_names.last().unwrap() {
                        write!(f, ",")?;
                    }
                }
                
                writeln!(f, ")")?;
                
                for s in body.iter() {
                    if s == body.last().unwrap() {
                        match *s {
                            Statement::Expression(ref e) => { write!(f, "return {}\n", e)?; },
                            _ => { write!(f, "{}", s)?; },
                        }
                    } else {
                        write!(f, "{}", s)?;
                    }
                }
                
                write!(f, "end")
            },
        }
    }
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.lua(f)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Str, Num, Bool, Any, Nil, Array(Rc<Type>), Undefined,
    Fun(Rc<Vec<Type>>), Many(Rc<Type>),
}

#[allow(unused)]
impl Type {
    pub fn compare(&self, other: &Type) -> bool {
        if self == &Type::Any || other == &Type::Any {
            true
        } else {
            match self {
                &Type::Array(ref a) => match other {
                    &Type::Array(ref b) if **b != Type::Nil => a.compare(b),
                    _ => false,
                },

                _ => self == other,
            }
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

    pub fn lua(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.lua(f)
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

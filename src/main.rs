mod slug;

use std::rc::Rc;

use slug::syntax;
use syntax::lexer::{BlockTree, process_branch};
use syntax::parser::{Traveler, Parser, Expression};

fn main() {
    let test = r#"

    "#;

    let mut blocks = BlockTree::new(test, 0);
    let indents    = blocks.indents();

    let root = blocks.tree(&indents);
    let done = process_branch(&root);
    
    let traveler = Traveler::new(done.clone());
    let mut parser = Parser::new(traveler);
    
    let symtab  = Rc::new(syntax::SymTab::new_global());
    let typetab = Rc::new(syntax::TypeTab::new_global());

    match parser.parse() {
        Err(why)  => println!("error: {}", why),
        Ok(stuff) => {
            println!("{:#?}", stuff);
            
            for s in stuff.iter() {
                match s.visit(&symtab, &typetab) {
                    Ok(()) => (),
                    Err(e) => {
                        println!("{}", e);
                        return
                    },
                }
            }
            
            println!("{}", Expression::Block(Rc::new(stuff)))
        },
    }
        
    println!("{:?}\n{:?}", symtab, typetab);
}

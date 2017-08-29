mod slug;

use std::rc::Rc;

use slug::syntax;
use syntax::lexer::{BlockTree, process_branch};
use syntax::parser::{Traveler, Parser};

fn main() {
    let test = r#"
a = fun (b num) num:
    b + 10

c = a 10

b any .. = [
    a = fun (b num) num:
        b + 10
]

d = b.a 10

e = (fun num: 10)!
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
        Ok(stuff) => for s in stuff.iter() {
            println!("{:#?}", stuff);

            match s.visit(&symtab, &typetab) {
                Ok(()) => (),
                Err(e) => {
                    println!("{}", e);
                    return
                },
            }
        },
    }
    
    println!("{:?}\n{:?}", symtab, typetab);
}

mod slug;

use slug::syntax;
use syntax::lexer::{BlockTree, process_branch};
use syntax::parser::{Traveler, Parser};

fn main() {
    let test = r#"
a = 10
b = 10
c = 10
    "#;

    let mut blocks = BlockTree::new(test, 0);
    let indents    = blocks.indents();

    let root = blocks.tree(&indents);
    let done = process_branch(&root);
    
    let traveler = Traveler::new(done.clone());
    let mut parser = Parser::new(traveler);

    match parser.parse() {
        Err(why)  => println!("error: {}", why),
        Ok(stuff) => {
            println!("{:#?}", stuff)
        },
    }
}

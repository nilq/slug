mod slug;

use slug::syntax;
use syntax::lexer::{BlockTree, process_branch};
use syntax::parser::{Traveler, Parser};

fn main() {
    let test = r#"
a num = 10
b num = 10 + a

fun add (a num) num: a + 10
fun idk (a num): 1
fun hm (a): 10
fun ay num: 0
fun c: 0
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

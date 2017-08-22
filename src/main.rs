mod slug;

use slug::syntax;
use syntax::lexer::{BlockTree, process_branch};

fn main() {
    let test = r#"
a num = 10
    "#;
    
    let mut blocks = BlockTree::new(test, 0);
    let indents    = blocks.indents();

    let root = blocks.tree(&indents);
    let done = process_branch(&root);
    
    println!("#{:#?}", done);
}

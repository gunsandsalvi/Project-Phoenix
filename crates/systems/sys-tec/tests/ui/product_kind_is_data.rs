use if_base::ProductDecl;

// A branch on what kind of product it is: a product has no kind, only the declared data mechanisms read.
fn is_service(p: &ProductDecl) -> bool {
    matches!(p.kind, _)
}

fn main() {}

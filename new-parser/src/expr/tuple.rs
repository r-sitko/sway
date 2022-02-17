use crate::priv_prelude::*;

#[derive(Clone, Debug)]
pub struct ExprTuple {
    pub elems: Parens<Option<(Box<Expr>, CommaToken, Punctuated<Expr, CommaToken>)>>,
}

impl Spanned for ExprTuple {
    fn span(&self) -> Span {
        self.elems.span()
    }
}

pub fn expr_tuple() -> impl Parser<Output = ExprTuple> + Clone {
    parens(
        optional_leading_whitespace(lazy(|| expr()))
        .then_optional_whitespace()
        .then(comma_token())
        .then(punctuated(
            optional_leading_whitespace(lazy(|| expr())),
            optional_leading_whitespace(comma_token()),
        ))
        .optional()
        .then_optional_whitespace()
    )
    .map(|parens: Parens<Option<_>>| {
        let elems = parens.map(|elems_opt| {
            elems_opt.map(|((head, head_token), tail)| (Box::new(head), head_token, tail))
        });
        ExprTuple { elems }
    })
}

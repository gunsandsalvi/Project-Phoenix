use syn::visit::{self, Visit};
use syn::{BinOp, Expr, ExprCall, ImplItemFn, ItemFn, ItemMod};

use super::{Breach, attrs, unparsed};
use crate::workspace::{Source, Workspace};

const RULE: &str = "PC-17";

/// The crate that defines days and their civil dates.
const DAY_CRATE: &str = "phx-id";
/// The one module that places dates by the calendar, in its crate.
const CALENDAR: (&str, &str) = ("phx-core", "/src/calendar/");
const CIVIL: [&str; 2] = ["days_from_civil", "civil_from_days"];
const ARITHMETIC: [&str; 8] = [
    "checked_add",
    "checked_sub",
    "wrapping_add",
    "wrapping_sub",
    "saturating_add",
    "saturating_sub",
    "overflowing_add",
    "overflowing_sub",
];

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for c in ws.world_crates().filter(|c| c.name != DAY_CRATE) {
        let in_calendar =
            |s: &Source| c.name == CALENDAR.0 && s.path.strip_prefix(&c.dir).is_some_and(|p| p.starts_with(CALENDAR.1));
        for source in c.sources.iter().filter(|s| !s.is_test_or_bench() && !in_calendar(s)) {
            breaches.extend(check(source));
        }
    }
    breaches
}

fn check(source: &Source) -> Vec<Breach> {
    let file = match &source.file {
        Ok(file) => file,
        Err(error) => return vec![unparsed(RULE, &source.path, error)],
    };
    if attrs::is_test(&file.attrs) {
        return Vec::new();
    }
    let mut finder = Finder { found: Vec::new() };
    finder.visit_file(file);
    finder.found.into_iter().map(|(line, message)| Breach::new(RULE, &source.path, line, message)).collect()
}

#[derive(Debug)]
struct Finder {
    found: Vec<(usize, &'static str)>,
}

fn is_day_new(call: &ExprCall) -> bool {
    let Expr::Path(p) = &*call.func else { return false };
    let names: Vec<String> = p.path.segments.iter().map(|s| s.ident.to_string()).collect();
    names.ends_with(&["Day".to_owned(), "new".to_owned()])
}

/// An argument that computes a day by adding to or taking from a number.
fn is_arithmetic(expr: &Expr) -> bool {
    match expr {
        Expr::Binary(b) => matches!(b.op, BinOp::Add(_) | BinOp::Sub(_)),
        Expr::MethodCall(m) => ARITHMETIC.iter().any(|a| m.method == a) || is_arithmetic(&m.receiver),
        Expr::Paren(p) => is_arithmetic(&p.expr),
        Expr::Try(t) => is_arithmetic(&t.expr),
        _ => false,
    }
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_ident(&mut self, ident: &'ast proc_macro2::Ident) {
        if CIVIL.iter().any(|c| ident == c) {
            self.found.push((attrs::line(ident.span()), "a civil day count outside the calendar"));
        }
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if is_day_new(call) && call.args.iter().any(is_arithmetic) {
            self.found
                .push((attrs::line(call.paren_token.span.open()), "a number added to a day outside the calendar"));
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_mod(self, item);
        }
    }

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_item_fn(self, item);
        }
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        if !attrs::is_test(&item.attrs) {
            visit::visit_impl_item_fn(self, item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::workspace::fixture::{krate, with_source};
    use crate::workspace::{Layer, Workspace};

    #[test]
    fn days_are_placed_only_by_the_calendar() {
        let text = "use phx_id::days_from_civil;\n\
                    fn a(d: Day) -> Day { Day::new(d.get() + 3) }\n\
                    fn b(d: Day) -> Option<Day> { Some(phx_id::Day::new(d.get().checked_sub(1)?)) }\n\
                    fn c(d: Day) -> Day { d.succ() }\n\
                    fn e(n: u32) -> Day { Day::new(n) }\n\
                    #[cfg(test)]\nmod tests { fn f(d: Day) -> Day { Day::new(d.get() + 1) } }";
        let core = |path| with_source(krate("phx-core", Layer::Kernel), path, text);
        let lines: Vec<usize> = run(&Workspace::new(vec![core("src/agenda.rs")])).iter().map(|b| b.line).collect();
        assert_eq!(lines, vec![1, 2, 3]);
        assert!(run(&Workspace::new(vec![core("src/calendar/mod.rs")])).is_empty());
        let id = with_source(krate("phx-id", Layer::Foundation), "src/day.rs", text);
        assert!(run(&Workspace::new(vec![id])).is_empty());
    }
}

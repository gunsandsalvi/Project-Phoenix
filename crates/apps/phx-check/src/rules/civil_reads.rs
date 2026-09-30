//! PC-17 on the day's paths: a day's civil date, business-day test and period ends are read from its facts, computed
//! once at the day's open, so no day-path module converts a day to a date per call — neither `Calendar::date` nor the
//! calendar's civil conversions. Today's sites are admitted by the rule's exceptions file until their migrations.

use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ExprMethodCall, ImplItemFn, ItemFn, ItemImpl, ItemMod};

use super::hot_paths::is_hot;
use super::{Breach, attrs, unparsed};
use crate::exceptions::{Exceptions, Found};
use crate::workspace::Workspace;

pub const RULE: &str = "PC-17";

/// The calendar's civil conversions by name, and the receiver a day's date is read from.
const CIVIL_CALLS: &[&str] = &["civil_date", "days_after"];
const CALENDAR: &str = "calendar";
const DATE: &str = "date";
/// Functions not read: the opening, and an error path.
const NOT_READ: &[&str] = &["opening", "cold"];

/// Every civil read PC-17 finds on the day's paths, and a breach for each source that does not parse.
#[must_use]
pub fn found(ws: &Workspace) -> (Vec<Found>, Vec<Breach>) {
    let (mut sites, mut breaches) = (Vec::new(), Vec::new());
    for c in ws.world_crates() {
        for source in c.sources.iter().filter(|s| is_hot(c, s)) {
            let file = match &source.file {
                Ok(file) => file,
                Err(error) => {
                    breaches.push(unparsed(RULE, &source.path, error));
                    continue;
                }
            };
            if attrs::is_test(&file.attrs) {
                continue;
            }
            let mut finder = Finder::default();
            finder.visit_file(file);
            sites.extend(finder.found.into_iter().map(|(line, item, what)| Found {
                path: source.path.clone(),
                message: format!("{what} converts a day to a date on the day's path: read the day's facts"),
                item,
                what,
                line,
            }));
        }
    }
    (sites, breaches)
}

pub fn run(ws: &Workspace) -> Vec<Breach> {
    let (sites, mut breaches) = found(ws);
    match Exceptions::load(ws, RULE) {
        Ok(ex) => breaches.extend(ex.judge(&ws.ratchets, sites)),
        Err(b) => breaches.push(b),
    }
    breaches
}

#[derive(Debug, Default)]
struct Finder {
    found: Vec<(usize, String, String)>,
    items: Vec<String>,
}

fn not_read(attrs_: &[syn::Attribute]) -> bool {
    attrs::is_test(attrs_)
        || attrs_.iter().any(|a| a.path().segments.last().is_some_and(|s| NOT_READ.iter().any(|n| s.ident == n)))
}

/// Whether an expression names the calendar: a binding or a field called so.
fn is_calendar(e: &Expr) -> bool {
    match e {
        Expr::Path(p) => p.path.is_ident(CALENDAR),
        Expr::Field(f) => matches!(&f.member, syn::Member::Named(n) if n == CALENDAR),
        Expr::Reference(r) => is_calendar(&r.expr),
        Expr::Paren(p) => is_calendar(&p.expr),
        _ => false,
    }
}

impl Finder {
    fn item(&self) -> String {
        if self.items.is_empty() { "(module)".to_owned() } else { self.items.join("::") }
    }

    fn within(&mut self, name: String, visit: impl FnOnce(&mut Finder)) {
        self.items.push(name);
        visit(self);
        self.items.pop();
    }
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        if !attrs::is_test(&item.attrs) {
            self.within(format!("mod {}", item.ident), |f| visit::visit_item_mod(f, item));
        }
    }

    fn visit_item_impl(&mut self, item: &'ast ItemImpl) {
        if attrs::is_test(&item.attrs) {
            return;
        }
        let name = match &*item.self_ty {
            syn::Type::Path(p) => p.path.segments.last().map_or_else(String::new, |s| s.ident.to_string()),
            _ => "impl".to_owned(),
        };
        self.within(name, |f| visit::visit_item_impl(f, item));
    }

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        if !not_read(&item.attrs) {
            self.within(item.sig.ident.to_string(), |f| visit::visit_item_fn(f, item));
        }
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        if !not_read(&item.attrs) {
            self.within(item.sig.ident.to_string(), |f| visit::visit_impl_item_fn(f, item));
        }
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(p) = &*call.func
            && let Some(name) = p.path.segments.last().map(|s| s.ident.to_string())
            && CIVIL_CALLS.contains(&name.as_str())
        {
            self.found.push((attrs::line(call.paren_token.span.open()), self.item(), format!("`{name}`")));
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        if call.method == DATE && call.args.len() == 1 && is_calendar(&call.receiver) {
            self.found.push((attrs::line(call.method.span()), self.item(), "`Calendar::date`".to_owned()));
        }
        visit::visit_expr_method_call(self, call);
    }
}

#[cfg(test)]
mod tests {
    use super::found;
    use crate::workspace::fixture::{krate, with_source};
    use crate::workspace::{Layer, Workspace};

    #[test]
    fn day_paths_read_facts_not_dates() {
        let text = "fn a(ctx: &Ctx, day: Day) -> i32 { ctx.calendar.date(day).year() }\n\
                    fn b(calendar: &Calendar, day: Day) -> Date { calendar.date(day) }\n\
                    fn c(s: i64) -> Date { civil_date(s) }\n\
                    fn d(p: &Person, epoch: Date) -> Date { p.day.date(epoch) }\n\
                    #[opening] fn e(calendar: &Calendar, day: Day) -> Date { calendar.date(day) }";
        let hot = with_source(krate("phx-world", Layer::Assembly), "src/core_goods.rs", text);
        let (sites, breaches) = found(&Workspace::new(vec![hot]));
        assert!(breaches.is_empty());
        assert_eq!(sites.iter().map(|s| s.line).collect::<Vec<_>>(), vec![1, 2, 3]);
        let cold = with_source(krate("phx-world", Layer::Assembly), "src/opening/setup.rs", text);
        assert!(found(&Workspace::new(vec![cold])).0.is_empty(), "the opening's modules are not the day's paths");
    }
}

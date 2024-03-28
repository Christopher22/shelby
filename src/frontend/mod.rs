use std::collections::HashMap;

use rocket::serde::Serialize;
use rocket_dyn_templates::Template;

mod forms;
mod tables;
mod util;

#[cfg(test)]
mod tests;

use crate::backend::accounting::Amount;

pub use self::forms::{ForeignKeyStorage, InsertableDatabaseEntry};
pub use self::tables::RenderableDatabaseEntry;

pub trait Renderable: Sized {
    const TEMPLATE: &'static str;

    fn generate_context(self) -> impl Serialize;

    fn render(self) -> Template {
        Template::render(Self::TEMPLATE, self.generate_context())
    }
}

pub fn render_dashboard(
    database: &crate::backend::database::Database,
) -> Result<Template, crate::Error> {
    let summaries = crate::backend::accounting::AccountSummary::load_all(database)?;

    let mut cost_centers: HashMap<String, HashMap<String, Vec<(String, Amount)>>> = HashMap::new();
    for summary in summaries {
        cost_centers
            .entry(summary.cost_center)
            .or_default()
            .entry(summary.category)
            .or_default()
            .push((summary.account, summary.amount));
    }

    Ok(Template::render(
        "dashboard",
        rocket_dyn_templates::context! { cost_centers: cost_centers },
    ))
}

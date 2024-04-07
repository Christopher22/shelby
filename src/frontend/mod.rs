use std::collections::HashMap;

use rocket::serde::Serialize;
use rocket::{response::content::RawHtml, State};
use rocket_dyn_templates::Template;

mod forms;
mod overviews;
mod tables;
mod util;

#[cfg(test)]
mod tests;

use crate::backend::accounting::Amount;
use crate::backend::database::{PrimaryKey, SelectableByPrimaryKey};
use crate::backend::person::Group;
use crate::{
    auth::{AuthenticatedUser, Forward},
    Config, Error,
};

pub use self::forms::{ForeignKeyStorage, InsertableDatabaseEntry};
pub use self::tables::RenderableDatabaseEntry;

pub trait Renderable: Sized {
    const TEMPLATE: &'static str;

    fn generate_context(self) -> impl Serialize;

    fn render(self) -> Template {
        Template::render(Self::TEMPLATE, self.generate_context())
    }
}

#[get("/", rank = 1)]
pub async fn index_protected(
    _user: AuthenticatedUser<Forward>,
    config: &State<Config>,
) -> Result<Template, Error> {
    let summaries = {
        let database = &config.database();
        crate::backend::accounting::AccountSummary::load_all(database)?
    };

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

#[get("/groups/<group_id>", rank = 8)]
pub async fn group_overview(
    _user: AuthenticatedUser<Forward>,
    config: &State<Config>,
    group_id: i64,
    _expected_type: super::util::ExpectedFileType<super::util::Html>,
) -> Result<RawHtml<Template>, Error> {
    let database = &config.database();
    let group = Group::try_select(database, group_id)?.ok_or(Error::NotFound)?;
    let summaries = self::overviews::GroupOverview::load(database, group)?;
    Ok(RawHtml(summaries.render()))
}

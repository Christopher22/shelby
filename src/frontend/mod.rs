use rocket::serde::Serialize;
use rocket_dyn_templates::Template;

mod table_renderer;

pub use self::table_renderer::RenderableDatabaseEntry;

pub trait Renderable: Sized {
    const TEMPLATE: &'static str;

    fn generate_context(&self) -> impl Serialize;

    fn render(self) -> Template {
        Template::render(Self::TEMPLATE, self.generate_context())
    }
}

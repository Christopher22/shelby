use rocket::serde::Serialize;

use super::Renderable;

#[derive(Debug, Serialize)]
pub struct Field {
    id: &'static str,
    label: &'static str,
    attributes: &'static [&'static str],
}

pub trait InsertableDatabaseEntry<const N: usize>: Sized {
    const NAME: &'static str;
    const FIELDS: [Field; N];

    fn prepare_rendering(post_url: &'static str) -> InsertFormRenderer<N, Self> {
        InsertFormRenderer::new(post_url)
    }
}

pub struct InsertFormRenderer<const N: usize, T>(
    &'static str,
    std::marker::PhantomData<*const [T; N]>,
);

impl<const N: usize, T> InsertFormRenderer<N, T> {
    fn new(post_url: &'static str) -> Self {
        Self(post_url, std::marker::PhantomData)
    }
}

impl<const N: usize, T: InsertableDatabaseEntry<N>> Renderable for InsertFormRenderer<N, T>
where
    [Field; N]: Serialize,
{
    const TEMPLATE: &'static str = "form";

    fn generate_context(&self) -> impl rocket::serde::Serialize {
        rocket_dyn_templates::context! {
            name: &T::NAME,
            fields: &T::FIELDS,
            post_url: self.0
        }
    }
}

impl InsertableDatabaseEntry<5> for shelby_backend::person::Person {
    const NAME: &'static str = "New person";
    const FIELDS: [Field; 5] = [
        Field {
            id: "name",
            label: "Name",
            attributes: &[],
        },
        Field {
            id: "address",
            label: "Adress",
            attributes: &[],
        },
        Field {
            id: "email",
            label: "E-Mail",
            attributes: &[],
        },
        Field {
            id: "birthday",
            label: "Birthday",
            attributes: &[],
        },
        Field {
            id: "comment",
            label: "Comment",
            attributes: &[],
        },
    ];
}

impl InsertableDatabaseEntry<1> for shelby_backend::document::Document {
    const NAME: &'static str = "New document";
    const FIELDS: [Field; 1] = [Field {
        id: "name",
        label: "Name",
        attributes: &[],
    }];
}

impl InsertableDatabaseEntry<1> for shelby_backend::person::Group {
    const NAME: &'static str = "New group";
    const FIELDS: [Field; 1] = [Field {
        id: "description",
        label: "Description",
        attributes: &[
            "type=\"text\"",
            "class=\"form-control\"",
            "placeholder=\"Description of the new group\"",
            "required=\"\"",
        ],
    }];
}

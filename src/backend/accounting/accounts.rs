use crate::backend::{
    accounting::Category,
    database::{DefaultGenerator, Insertable, PrimaryKey},
};

crate::backend::database::make_struct!(
    #[derive(serde::Serialize, serde::Deserialize)]
    #[table("accounts")]
    #[dependencies(Category)]
    #[impl_select(true, testing: true, description: "description")]
    Account {
        code: u32,
        category: PrimaryKey<Category>,
        description: String
    }
);

impl DefaultGenerator for Account {
    fn create_default(database: &crate::backend::database::Database) -> Self {
        let category = Category::default()
            .insert(&database)
            .expect("valid category");
        Account {
            code: 1800,
            category,
            description: String::from("Exampke account"),
        }
    }
}

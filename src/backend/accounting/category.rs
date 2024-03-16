crate::backend::database::make_struct!(
    #[derive(Default, serde::Serialize, serde::Deserialize)]
    #[table("categories")]
    #[dependencies(())]
    #[impl_select(true, testing: true, description: "description")]
    Category {
        description: String
    }
);

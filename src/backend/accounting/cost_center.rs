crate::backend::database::make_struct!(
    #[derive(Default, serde::Serialize, serde::Deserialize)]
    #[table("cost_centers")]
    #[dependencies(())]
    #[impl_select(true, testing: true, description: "description")]
    CostCenter {
        description: String
    }
);

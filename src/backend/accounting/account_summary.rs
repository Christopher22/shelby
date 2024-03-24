use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountSummary {
    pub account: String,
    pub cost_center: String,
    pub category: String,
    pub amount: super::Amount,
}

impl AccountSummary {
    /// Load the summaries.
    pub fn load_all(
        database: &crate::backend::database::Database,
    ) -> Result<Vec<Self>, crate::backend::database::Error> {
        const QUERY: &'static str = r#"
            SELECT SUM(amount), accounts.description, cost_centers.description, categories.description FROM entries 
            INNER JOIN cost_centers ON cost_centers.id = cost_center 
            INNER JOIN accounts ON accounts.id = account 
            INNER JOIN categories ON categories.id = accounts.category 
            GROUP BY account, cost_center ORDER BY cost_center, categories.id, account"#;
        let mut stmt = database.connection.prepare(QUERY)?;
        let iterator = stmt.query_map((), |row| {
            <(super::Amount, String, String, String)>::try_from(row).map(|value| AccountSummary {
                account: value.1,
                cost_center: value.2,
                amount: value.0,
                category: value.3,
            })
        })?;
        Ok(iterator.filter_map(|value| value.ok()).collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::backend::database::{DefaultGenerator, Insertable};

    use super::super::*;

    #[test]
    fn test_loading() {
        let database = crate::backend::database::Database::in_memory().expect("valid database");

        let category_name = String::from("Category 1");
        let category_1 = Category {
            description: category_name.clone(),
        }
        .insert(&database)
        .expect("insert category failed");

        let evidence = crate::backend::document::Document::create_default(&database)
            .insert(&database)
            .expect("inserting document failed");

        // Add the cost centers
        let [(cost_center_1_name, cost_center_1), (cost_center_2_name, cost_center_2)] =
            ["Cost Center 1", "Cost Center 2"].map(|name| {
                let description = String::from(name);
                let center = CostCenter {
                    description: description.clone(),
                }
                .insert(&database)
                .expect("insert cost center failed");

                (description, center)
            });

        // Add accounts
        let [(account_1_name, account_1), (account_2_name, account_2)] = ["Account 1", "Account 2"]
            .map(|name| {
                let description = String::from(name);
                let account = Account {
                    code: 1,
                    category: category_1,
                    description: description.clone(),
                }
                .insert(&database)
                .expect("insert account failed");

                (description, account)
            });

        // Add the entries
        for (account, amount) in [(account_1, 100), (account_1, 200), (account_2, 140)] {
            Entry {
                evidence,
                account,
                cost_center: cost_center_1,
                amount: Amount::from(amount),
                description: String::new(),
            }
            .insert(&database)
            .expect("insert entry failed");
        }

        for (account, amount) in [(account_1, 50), (account_1, 80), (account_2, 300)] {
            Entry {
                evidence,
                account,
                cost_center: cost_center_2,
                amount: Amount::from(amount),
                description: String::new(),
            }
            .insert(&database)
            .expect("insert entry failed");
        }

        let summaries = AccountSummary::load_all(&database).expect("loading summary failed");
        assert_eq!(
            summaries.as_slice(),
            &[
                AccountSummary {
                    account: account_1_name.clone(),
                    cost_center: cost_center_1_name.clone(),
                    amount: Amount::from(300),
                    category: category_name.clone()
                },
                AccountSummary {
                    account: account_2_name.clone(),
                    cost_center: cost_center_1_name.clone(),
                    amount: Amount::from(140),
                    category: category_name.clone()
                },
                AccountSummary {
                    account: account_1_name.clone(),
                    cost_center: cost_center_2_name.clone(),
                    amount: Amount::from(130),
                    category: category_name.clone()
                },
                AccountSummary {
                    account: account_2_name.clone(),
                    cost_center: cost_center_2_name.clone(),
                    amount: Amount::from(300),
                    category: category_name.clone()
                }
            ]
        );
    }
}

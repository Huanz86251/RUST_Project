use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountType {
    Checking,
    Saving,
    Credit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum InOutType {
    Salary,
    Shop,
    Utility,
    Rent,
    Transfer,
    Hobby,
    Study,
    Invest,
    Etc,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: u64,
    pub account: AccountType,
    pub inout_type: InOutType,
    pub money: i64,
    pub desc: String,
    pub date: NaiveDate,
}
impl Transaction {
    pub fn new(
        id: u64,
        account: AccountType,
        inout_type: InOutType,
        money: i64,
        desc: impl Into<String>,
        date: NaiveDate,
    ) -> Self {
        Self {
            id,
            account,
            inout_type,
            money,
            desc: desc.into(),
            date,
        }
    }
}

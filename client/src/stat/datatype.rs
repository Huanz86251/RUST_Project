use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
pub type UserId = Uuid;
pub type AccountId = i64;
pub type CategoryId = i64;
pub type TransactionId = Uuid;
pub type EntryId = i64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountType {
    Checking,
    Cash,
    Credit,
    Other(String),
}
impl From<String> for AccountType {
    fn from(input: String) -> Self {
        match input.to_lowercase().as_str() {
            "checking" => AccountType::Checking,
            "credit" => AccountType::Credit,
            "cash" => AccountType::Cash,
            _ => AccountType::Other(input),
        }
    }
}

impl AccountType {
    pub fn to_cloud(&self) -> &str {
        match self {
            AccountType::Checking => "checking",
            AccountType::Credit => "credit",
            AccountType::Cash => "cash",
            AccountType::Other(_) => "other",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Currency(pub String);

impl Currency {
    pub fn new(input: &str) -> Self {
        Self(input.to_uppercase())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub email: String,
    pub create_date: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: AccountId,
    pub userid: UserId,
    pub name: String,
    pub account_type: AccountType,
    pub currency: Currency,
    pub balance: f64,
    pub create_date: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: CategoryId,
    pub userid: UserId,
    pub name: String,
    pub parentid: Option<CategoryId>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: TransactionId,
    pub userid: UserId,
    pub occur_date: NaiveDate,
    pub receiver: Option<String>,
    pub desc: Option<String>,
    pub create_date: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: EntryId,
    pub userid: UserId,
    pub tranid: TransactionId,
    pub accountid: AccountId,
    pub categoryid: Option<CategoryId>,
    pub amount: f64,
    pub desc: Option<String>,
}

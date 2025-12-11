use crate::stat::{Ledger, datatype::*};
use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json;
use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::Path,
};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clouduser {
    pub id: Uuid,
    pub email: String,
    #[serde(rename = "created_at")]
    pub create_date: DateTime<Utc>,
}
impl From<Clouduser> for User {
    fn from(value: Clouduser) -> Self {
        Self {
            id: value.id,
            email: value.email,
            create_date: value.create_date,
        }
    }
}
impl From<User> for Clouduser {
    fn from(v: User) -> Self {
        Self {
            id: v.id,
            email: v.email,
            create_date: v.create_date,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cloudaccount {
    pub id: i64,
    #[serde(rename = "user_id")]
    pub userid: Uuid,
    pub name: String,
    #[serde(rename = "account_type")]
    pub account_type: String,
    pub currency: String,
    #[serde(rename = "opening_balance")]
    pub opening_balance: f64,
    #[serde(rename = "created_at")]
    pub create_date: DateTime<Utc>,
}
impl From<Cloudaccount> for Account {
    fn from(v: Cloudaccount) -> Self {
        Self {
            id: v.id,
            userid: v.userid,
            name: v.name,
            account_type: AccountType::from(v.account_type),
            currency: Currency::new(&v.currency),
            balance: v.opening_balance,
            create_date: v.create_date,
        }
    }
}
impl From<Account> for Cloudaccount {
    fn from(v: Account) -> Self {
        Self {
            id: v.id,
            userid: v.userid,
            name: v.name,
            account_type: v.account_type.to_cloud().to_string(),
            currency: v.currency.0.clone(),
            opening_balance: v.balance,
            create_date: v.create_date,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cloudcategory {
    pub id: i64,
    #[serde(rename = "user_id")]
    pub userid: Uuid,
    pub name: String,
    #[serde(rename = "parent_id")]
    pub parentid: Option<i64>,
}
impl From<Cloudcategory> for Category {
    fn from(v: Cloudcategory) -> Self {
        Self {
            id: v.id,
            userid: v.userid,
            name: v.name,
            parentid: v.parentid,
        }
    }
}
impl From<Category> for Cloudcategory {
    fn from(v: Category) -> Self {
        Self {
            id: v.id,
            userid: v.userid,
            name: v.name,
            parentid: v.parentid,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cloudtransaction {
    pub id: Uuid,
    #[serde(rename = "user_id")]
    pub userid: Uuid,
    #[serde(rename = "occurred_at")]
    pub occur_date: NaiveDate,
    pub payee: Option<String>,
    #[serde(rename = "memo")]
    pub desc: Option<String>,
    #[serde(rename = "created_at")]
    pub create_date: DateTime<Utc>,
}
impl From<Cloudtransaction> for Transaction {
    fn from(v: Cloudtransaction) -> Self {
        Self {
            id: v.id,
            userid: v.userid,
            occur_date: v.occur_date,
            receiver: v.payee,
            desc: v.desc,
            create_date: v.create_date,
        }
    }
}
impl From<Transaction> for Cloudtransaction {
    fn from(v: Transaction) -> Self {
        Self {
            id: v.id,
            userid: v.userid,
            occur_date: v.occur_date,
            payee: v.receiver,
            desc: v.desc,
            create_date: v.create_date,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cloudentry {
    pub id: i64,
    #[serde(rename = "user_id")]
    pub userid: Uuid,
    #[serde(rename = "tx_id")]
    pub tranid: Uuid,
    #[serde(rename = "account_id")]
    pub accountid: i64,
    #[serde(rename = "category_id")]
    pub categoryid: Option<i64>,
    pub amount: f64,
    #[serde(rename = "note")]
    pub desc: Option<String>,
}
impl From<Cloudentry> for Entry {
    fn from(v: Cloudentry) -> Self {
        Self {
            id: v.id,
            userid: v.userid,
            tranid: v.tranid,
            accountid: v.accountid,
            categoryid: v.categoryid,
            amount: v.amount,
            desc: v.desc,
        }
    }
}
impl From<Entry> for Cloudentry {
    fn from(v: Entry) -> Self {
        Self {
            id: v.id,
            userid: v.userid,
            tranid: v.tranid,
            accountid: v.accountid,
            categoryid: v.categoryid,
            amount: v.amount,
            desc: v.desc,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cloudledger {
    pub users: Vec<Clouduser>,
    pub accounts: Vec<Cloudaccount>,
    pub categories: Vec<Cloudcategory>,
    pub transactions: Vec<Cloudtransaction>,
    pub entries: Vec<Cloudentry>,
}
impl From<Cloudledger> for Ledger {
    fn from(v: Cloudledger) -> Self {
        let mut user = Vec::new();
        let mut acc = Vec::new();
        let mut cat = Vec::new();
        let mut tran = Vec::new();
        let mut entry = Vec::new();
        for i in v.users {
            user.push(User::from(i));
        }
        for i in v.accounts {
            acc.push(Account::from(i));
        }
        for i in v.categories {
            cat.push(Category::from(i));
        }
        for i in v.transactions {
            tran.push(Transaction::from(i));
        }
        for i in v.entries {
            entry.push(Entry::from(i));
        }
        Ledger {
            user,
            account: acc,
            category: cat,
            transaction: tran,
            entry,
        }
    }
}
impl From<&Ledger> for Cloudledger {
    fn from(v: &Ledger) -> Self {
        let mut user = Vec::new();
        let mut acc = Vec::new();
        let mut cat = Vec::new();
        let mut tran = Vec::new();
        let mut entry = Vec::new();
        for i in &v.user {
            user.push(Clouduser::from(i.clone()));
        }
        for i in &v.account {
            acc.push(Cloudaccount::from(i.clone()));
        }
        for i in &v.category {
            cat.push(Cloudcategory::from(i.clone()));
        }
        for i in &v.transaction {
            tran.push(Cloudtransaction::from(i.clone()));
        }
        for i in &v.entry {
            entry.push(Cloudentry::from(i.clone()));
        }
        Cloudledger {
            users: user,
            accounts: acc,
            categories: cat,
            transactions: tran,
            entries: entry,
        }
    }
}
//for temp test
pub fn load_json_ledger(path: impl AsRef<Path>) -> Result<Ledger> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let cloud: Cloudledger = serde_json::from_reader(reader)?;
    Ok(Ledger::from(cloud))
}
//for temp test
pub fn save_ledger_to_json(path: impl AsRef<Path>, ledger: &Ledger) -> Result<()> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    let cloud = Cloudledger::from(ledger);
    serde_json::to_writer_pretty(writer, &cloud)?;
    Ok(())
}

pub async fn download_ledger_from_server(base_url: &str, token: &str) -> Result<Ledger> {
    let client = Client::new();
    let url = format!("{}/ledger/snapshot", base_url.trim_end_matches('/'));
    let resp = client
        .get(&url)
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?;
    let cloud: Cloudledger = resp.json().await?;
    Ok(Ledger::from(cloud))
}

pub async fn upload_ledger_to_server(base_url: &str, token: &str, ledger: &Ledger) -> Result<()> {
    let client = Client::new();
    let url = format!("{}/ledger/snapshot", base_url.trim_end_matches('/'));
    let cloud = Cloudledger::from(ledger);
    client
        .put(&url)
        .bearer_auth(token)
        .json(&cloud)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

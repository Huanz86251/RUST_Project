use crate::stat::{Ledger, datatype::*};
use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use reqwest::Client;
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
//funtions trans from local to cloud style or cloud to local style
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
    pub name: String,
    #[serde(rename = "account_type")]
    pub account_type: String,
    pub currency: String,
    #[serde(rename = "opening_balance")]
    pub opening_balance: Decimal,
    #[serde(rename = "created_at")]
    pub create_date: DateTime<Utc>,
}
impl From<(Cloudaccount, Uuid)> for Account {
    fn from((v, user_id): (Cloudaccount, Uuid)) -> Self {
        Self {
            id: v.id,
            userid: user_id,
            name: v.name,
            account_type: AccountType::from(v.account_type),
            currency: Currency::new(&v.currency),
            balance: v.opening_balance.to_f64().unwrap_or(0.0),
            create_date: v.create_date,
        }
    }
}
impl From<Account> for Cloudaccount {
    fn from(v: Account) -> Self {
        Self {
            id: v.id,
            name: v.name,
            account_type: v.account_type.to_cloud().to_string(),
            currency: v.currency.0.clone(),
            opening_balance: Decimal::from_f64(v.balance).unwrap_or(Decimal::ZERO),
            create_date: v.create_date,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cloudcategory {
    pub id: i64,
    pub name: String,
    #[serde(rename = "parent_id")]
    pub parentid: Option<i64>,
}
impl From<(Cloudcategory, Uuid)> for Category {
    fn from((v, user_id): (Cloudcategory, Uuid)) -> Self {
        Self {
            id: v.id,
            userid: user_id,
            name: v.name,
            parentid: v.parentid,
        }
    }
}
impl From<Category> for Cloudcategory {
    fn from(v: Category) -> Self {
        Self {
            id: v.id,
            name: v.name,
            parentid: v.parentid,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cloudtransaction {
    pub id: Uuid,
    #[serde(rename = "occurred_at")]
    pub occur_date: NaiveDate,
    pub payee: Option<String>,
    #[serde(rename = "memo")]
    pub desc: Option<String>,
    #[serde(rename = "created_at")]
    pub create_date: DateTime<Utc>,
    #[serde(default)]
    pub entries: Vec<Cloudentry>,
}
impl From<(Cloudtransaction, Uuid)> for Transaction {
    fn from((v, user_id): (Cloudtransaction, Uuid)) -> Self {
        Self {
            id: v.id,
            userid: user_id,
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
            occur_date: v.occur_date,
            payee: v.receiver,
            desc: v.desc,
            create_date: v.create_date,
            entries: Vec::new(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cloudentry {
    pub id: i64,
    #[serde(rename = "tx_id")]
    pub tranid: Uuid,
    #[serde(rename = "account_id")]
    pub accountid: i64,
    #[serde(rename = "category_id")]
    pub categoryid: Option<i64>,
    pub amount: Decimal,
    #[serde(rename = "note")]
    pub desc: Option<String>,
}
impl From<(Cloudentry, Uuid)> for Entry {
    fn from((v, user_id): (Cloudentry, Uuid)) -> Self {
        Self {
            id: v.id,
            userid: user_id,
            tranid: v.tranid,
            accountid: v.accountid,
            categoryid: v.categoryid,
            amount: v.amount.to_f64().unwrap_or(0.0),
            desc: v.desc,
        }
    }
}
impl From<Entry> for Cloudentry {
    fn from(v: Entry) -> Self {
        Self {
            id: v.id,
            tranid: v.tranid,
            accountid: v.accountid,
            categoryid: v.categoryid,
            amount: Decimal::from_f64(v.amount).unwrap_or(Decimal::ZERO),
            desc: v.desc,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cloudledger {
    pub user: Clouduser,
    #[serde(default)]
    pub accounts: Vec<Cloudaccount>,
    #[serde(default)]
    pub categories: Vec<Cloudcategory>,
    #[serde(default)]
    pub transactions: Vec<Cloudtransaction>,
    #[serde(default)]
    pub entries: Vec<Cloudentry>,
}
impl From<Cloudledger> for Ledger {
    fn from(v: Cloudledger) -> Self {
        let mut acc = Vec::new();
        let mut cat = Vec::new();
        let mut tran = Vec::new();
        let mut entry = Vec::new();
        let user_id = v.user.id;
        let user = vec![User::from(v.user)];
        for i in v.accounts {
            acc.push(Account::from((i, user_id)));
        }
        for i in v.categories {
            cat.push(Category::from((i, user_id)));
        }
        for i in v.transactions {
            tran.push(Transaction::from((i, user_id)));
        }
        for i in v.entries {
            entry.push(Entry::from((i, user_id)));
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
///GET full ledger
pub async fn download_ledger_from_server(base_url: &str, token: &str) -> Result<Ledger> {
    let client = Client::new();
    let url = format!("{}/ledger", base_url.trim_end_matches('/'));
    let resp = client
        .get(&url)
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?;
    let cloud: Cloudledger = resp.json().await?;
    Ok(Ledger::from(cloud))
}

#[derive(Debug, Clone, Serialize)]
pub struct ACCreq {
    pub name: String,
    pub account_type: String,
    pub currency: Option<String>,
    pub opening_balance: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Catreq {
    pub parent_id: Option<i64>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Entryreq {
    pub account_id: i64,
    pub category_id: Option<i64>,
    pub amount: Decimal,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Transreq {
    pub memo: Option<String>,
    pub payee: Option<String>,
    pub occurred_at: NaiveDate,
    pub entries: Vec<Entryreq>,
}
///base url+path
fn api_url(base_url: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}
pub async fn create_cloudaccount(
    base_url: &str,
    token: &str,
    name: &str,
    account_type: &AccountType,
    currency: Option<&str>,
    opening_balance: Option<f64>,
) -> Result<Cloudaccount> {
    let client = Client::new();
    let url = api_url(base_url, "/accounts");
    let currency = match currency {
        Some(c) => Some(c.to_string()),
        None => None,
    };
    let ob = match opening_balance {
        Some(o) => Decimal::from_f64(o),
        None => None,
    };
    let body = ACCreq {
        name: name.to_string(),
        account_type: account_type.to_cloud().to_string(),
        currency: currency,
        opening_balance: ob,
    };
    let resp = client
        .post(&url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;
    Ok(resp.json::<Cloudaccount>().await?)
}

pub async fn create_cloudcate(
    base_url: &str,
    token: &str,
    name: &str,
    parent_id: Option<i64>,
) -> Result<Cloudcategory> {
    let client = Client::new();
    let url = api_url(base_url, "/categories");
    let body = Catreq {
        parent_id,
        name: name.to_string(),
    };
    let resp = client
        .post(&url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;
    Ok(resp.json::<Cloudcategory>().await?)
}
pub async fn create_cloudtransaction(
    base_url: &str,
    token: &str,
    occurred_date: NaiveDate,
    reciver: Option<&str>,
    desc: Option<&str>,
    entries: Vec<Entryreq>,
) -> Result<Cloudtransaction> {
    let client = Client::new();
    let url = api_url(base_url, "/transactions");
    let rec = match reciver {
        Some(s) => Some(s.to_string()),
        None => None,
    };
    let dec = match desc {
        Some(s) => Some(s.to_string()),
        None => None,
    };
    let body = Transreq {
        payee: rec,
        memo: dec,
        occurred_at: occurred_date,
        entries,
    };
    let resp = client
        .post(&url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;
    Ok(resp.json::<Cloudtransaction>().await?)
}
pub async fn delete_transaction_on_server(base_url: &str, token: &str, tx_id: Uuid) -> Result<()> {
    let client = Client::new();
    let url = api_url(base_url, &format!("/transactions/{tx_id}"));
    client
        .delete(&url)
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
#[allow(dead_code)]
pub async fn delete_entry_on_server(base_url: &str, token: &str, entry_id: i64) -> Result<()> {
    let client = Client::new();
    let url = api_url(base_url, &format!("/entries/{entry_id}"));
    client
        .delete(&url)
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
#[allow(dead_code)]
pub async fn delete_category_on_server(
    base_url: &str,
    token: &str,
    category_id: i64,
) -> Result<()> {
    let client = Client::new();
    let url = api_url(base_url, &format!("/categories/{category_id}"));
    client
        .delete(&url)
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
#[allow(dead_code)]
pub async fn delete_account_on_server(base_url: &str, token: &str, account_id: i64) -> Result<()> {
    let client = Client::new();
    let url = api_url(base_url, &format!("/accounts/{account_id}"));
    client
        .delete(&url)
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
#[derive(Debug, Clone, Serialize)]
struct Loginreq {
    email: String,
    password: String,
}
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Loginget {
    pub token: String,
    pub user_id: Uuid,
}
pub async fn login(base_url: &str, email: &str, password: &str) -> Result<Loginget> {
    let client = Client::new();
    let url = api_url(base_url, "/auth/login");
    let body = Loginreq {
        email: email.to_string(),
        password: password.to_string(),
    };
    let resp = client.post(&url).json(&body).send().await?;
    if resp.status().is_success() {
        Ok(resp.json::<Loginget>().await?)
    } else {
        let status = resp.status();
        let msg = resp.text().await.unwrap_or_default();
        Err(anyhow::anyhow!("login error: {status} {msg}"))
    }
}

#[derive(Debug, Clone, Serialize)]
struct Registreq {
    email: String,
    password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Registerget {
    #[allow(dead_code)]
    pub user_id: Uuid,
}
pub async fn register(base_url: &str, email: &str, password: &str) -> Result<Registerget> {
    let client = Client::new();
    let url = api_url(base_url, "/auth/register");
    let body = Registreq {
        email: email.to_string(),
        password: password.to_string(),
    };
    let resp = client.post(&url).json(&body).send().await?;
    if resp.status().is_success() {
        Ok(resp.json::<Registerget>().await?)
    } else {
        let status = resp.status();
        let msg = resp.text().await.unwrap_or_default();
        Err(anyhow::anyhow!("register error: {status} {msg}"))
    }
}

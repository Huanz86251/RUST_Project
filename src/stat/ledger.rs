use super::datatype::*;
use crate::stat::datatype::{AccountId, CategoryId, UserId};
use chrono::*;
use std::collections::HashMap;
use uuid::Uuid;
fn expand_month_range(mut sy: i32, mut sm: u32, ey: i32, em: u32) -> Vec<(i32, u32)> {
    let mut result = Vec::new();

    while sy < ey || (sy == ey && sm <= em) {
        result.push((sy, sm));
        sm += 1;
        if sm > 12 {
            sm = 1;
            sy += 1;
        }
    }

    result
}
#[derive(Debug, Default)]
pub struct Ledger {
    pub user: Vec<User>,
    pub account: Vec<Account>,
    pub category: Vec<Category>,
    pub transaction: Vec<Transaction>,
    pub entry: Vec<Entry>,
}
#[derive(Debug, Clone)]
pub struct AccountSummary {
    pub accountid: AccountId,
    pub name: String,
    pub account_type: AccountType,
    pub balance: f64,
    pub currency: Currency,
}
#[derive(Debug, Clone, Default)]
pub struct Monthstats {
    pub income: f64,
    pub outcome: f64,
    pub summary: f64,
}
impl Ledger {
    pub fn cal_balance(&self, accountid: AccountId) -> f64 {
        let current = self
            .account
            .iter()
            .find(|i| i.id == accountid)
            .map(|i| i.balance)
            .unwrap_or(0.0);
        let temp: f64 = self
            .entry
            .iter()
            .filter(|i| i.accountid == accountid)
            .map(|i| i.amount)
            .sum();
        return current + temp;
    }
    pub fn all_account_summary(&self) -> Vec<AccountSummary> {
        self.account
            .iter()
            .map(|i| AccountSummary {
                accountid: i.id,
                name: i.name.clone(),
                account_type: i.account_type.clone(),
                balance: self.cal_balance(i.id),
                currency: i.currency.clone(),
            })
            .collect()
    }
    pub fn monthstats(
        &self,
        userid: UserId,
        category: Option<CategoryId>,
        timephase: ((i32, u32), (i32, u32)),
        accountid: Option<AccountId>,
    ) -> HashMap<(i32, u32), Monthstats> {
        let start = timephase.0;
        let end = timephase.1;
        let sy = start.0;
        let sm = start.1;
        let ey = end.0;
        let em = end.1;
        let phase = expand_month_range(sy, sm, ey, em);
        let mut trans: HashMap<TransactionId, (i32, u32)> = HashMap::new();
        for i in &self.transaction {
            trans.insert(i.id, (i.occur_date.year(), i.occur_date.month()));
        }
        let mut stats: HashMap<(i32, u32), Monthstats> = HashMap::new();
        for i in &self.entry {
            if i.userid != userid {
                continue;
            }
            if let Some(acc) = accountid {
                if i.accountid != acc {
                    continue;
                }
            }
            if let Some(cat) = category {
                if i.categoryid != Some(cat) {
                    continue;
                }
            }
            let (y, m) = match trans.get(&i.tranid) {
                Some(&(y1, m1)) => (y1, m1),
                None => continue,
            };
            if !phase.contains(&(y, m)) {
                continue;
            }
            let temp = stats.entry((y, m)).or_insert(Monthstats::default());
            if i.amount >= 0.0 {
                temp.income += i.amount;
            } else {
                temp.outcome += i.amount;
            }
            temp.summary = temp.income + temp.outcome;
        }
        for &(y, m) in &phase {
            stats.entry((y, m)).or_insert(Monthstats::default());
        }
        return stats;
    }
    /// `timephase`
    /// - if `timephase` is not provided，only search year/month data
    /// - if provide `timephase`, year/month will be overwritten, will search months between start (year,month) to end (year, month)
    ///
    /// `category`
    /// - if `category` is not provided, search all category instead
    ///
    /// `accountid`
    /// - if `accountid` is not provided, check user all account
    ///
    /// `onlyspend`
    /// - if `onlyspend` is not provided, return income+outcome, if equal to true, only count for spend and return negative value, if equal false, return positive value for income.
    pub fn month_summary(
        &self,
        userid: UserId,
        year: i32,
        month: u32,
        accountid: Option<AccountId>,
        category: Option<CategoryId>,
        onlyspend: Option<bool>,
        timephase: Option<((i32, u32), (i32, u32))>,
    ) -> f64 {
        let phase = timephase.unwrap_or(((year, month), (year, month)));

        let stat = self.monthstats(userid, category, phase, accountid);
        match onlyspend {
            None => stat.values().map(|s| s.summary).sum(),
            Some(true) => stat.values().map(|s| s.outcome).sum(),
            Some(false) => stat.values().map(|s| s.income).sum(),
        }
    }

    pub fn build_demo_ledger() -> Ledger {
        let userid = Uuid::new_v4();
        let now = Utc::now();

        let user = User {
            id: userid,
            email: "demo@example.com".to_string(),
            create_date: now,
        };

        let acc_checking = Account {
            id: 1,
            userid,
            name: "Chequing".to_string(),
            account_type: AccountType::Checking,
            currency: Currency::new("CAD"),
            balance: 1000.0,
            create_date: now,
        };

        let acc_credit = Account {
            id: 2,
            userid,
            name: "Visa".to_string(),
            account_type: AccountType::Credit,
            currency: Currency::new("CAD"),
            balance: 0.0,
            create_date: now,
        };

        let cat_food = Category {
            id: 1,
            userid,
            name: "Food".to_string(),
            parentid: None,
        };

        let cat_rent = Category {
            id: 2,
            userid,
            name: "Rent".to_string(),
            parentid: None,
        };

        let tx1 = Transaction {
            id: Uuid::new_v4(),
            userid,
            occur_date: NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
            receiver: Some("Supermarket".to_string()),
            desc: Some("Groceries".to_string()),
            create_date: now,
        };

        let tx2 = Transaction {
            id: Uuid::new_v4(),
            userid,
            occur_date: NaiveDate::from_ymd_opt(2025, 12, 2).unwrap(),
            receiver: Some("Landlord".to_string()),
            desc: Some("December rent".to_string()),
            create_date: now,
        };

        let tx3 = Transaction {
            id: Uuid::new_v4(),
            userid,
            occur_date: NaiveDate::from_ymd_opt(2025, 12, 3).unwrap(),
            receiver: Some("Cafe".to_string()),
            desc: None,
            create_date: now,
        };

        let e1 = Entry {
            id: 1,
            userid,
            tranid: tx1.id,
            accountid: acc_checking.id,
            categoryid: Some(cat_food.id),
            amount: -50.0,
            desc: None,
        };

        let e2 = Entry {
            id: 2,
            userid,
            tranid: tx2.id,
            accountid: acc_checking.id,
            categoryid: Some(cat_rent.id),
            amount: -700.0,
            desc: None,
        };

        let e3 = Entry {
            id: 3,
            userid,
            tranid: tx3.id,
            accountid: acc_credit.id,
            categoryid: Some(cat_food.id),
            amount: -10.0,
            desc: Some("Latte".to_string()),
        };

        Ledger {
            user: vec![user],
            account: vec![acc_checking, acc_credit],
            category: vec![cat_food, cat_rent],
            transaction: vec![tx1, tx2, tx3],
            entry: vec![e1, e2, e3],
        }
    }
}

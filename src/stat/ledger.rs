use super::datatype::*;
use crate::stat::datatype::{AccountId, CategoryId, UserId};
use chrono::*;
use std::collections::{HashMap, HashSet};
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
#[derive(Debug, Clone)]
pub struct ReconcileResult {
    pub good: bool,
    pub internal_balance: f64,
    pub external_balance: f64,
    pub difference: f64,
    pub suspicous_entry: Vec<Entry>,
}

#[derive(Debug, Clone, Copy)]
enum Purpose {
    All,
    Income,
    Outcome,
}
impl Purpose {
    fn trans(onlyspend: Option<bool>) -> Self {
        match onlyspend {
            None => Purpose::All,
            Some(true) => Purpose::Outcome,
            Some(false) => Purpose::Income,
        }
    }
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
pub struct Detailstats {
    pub income: f64,
    pub outcome: f64,
    pub summary: f64,
}
impl Detailstats {
    fn get(&self, purpose: Purpose) -> f64 {
        match purpose {
            Purpose::All => self.summary,
            Purpose::Income => self.income,
            Purpose::Outcome => self.outcome,
        }
    }
}
#[derive(Debug, Clone, Default)]
pub struct Monthstats {
    pub income: f64,
    pub outcome: f64,
    pub summary: f64,
    pub category: HashMap<Option<CategoryId>, Detailstats>,
    pub account: HashMap<AccountId, Detailstats>,
    pub account_category: HashMap<(AccountId, Option<CategoryId>), Detailstats>,
}
impl Monthstats {
    fn get(&self, purpose: Purpose) -> f64 {
        match purpose {
            Purpose::All => self.summary,
            Purpose::Income => self.income,
            Purpose::Outcome => self.outcome,
        }
    }
}
#[derive(Debug, Clone)]
pub struct Trend<K> {
    pub axis: Vec<K>,
    pub income: Vec<f64>,
    pub outcome: Vec<f64>,
    pub summary: Vec<f64>,
}
impl<K: Clone> Trend<K> {
    ///change Trend content to percentage, better for pie graph
    pub fn normalize(&self) -> Self {
        let mut inc_s = 0.0;
        let mut out_s = 0.0;
        let mut sum_s = 0.0;
        for i in &self.income {
            inc_s += *i;
        }
        let income = if inc_s == 0.0 {
            let mut temp = Vec::new();
            for _ in 0..self.income.len() {
                temp.push(0.0);
            }
            temp
        } else {
            let mut temp = Vec::new();
            for i in &self.income {
                temp.push(*i / inc_s);
            }
            temp
        };
        for i in &self.outcome {
            out_s += *i;
        }
        let outcome = if out_s == 0.0 {
            let mut temp = Vec::new();
            for _ in 0..self.outcome.len() {
                temp.push(0.0);
            }
            temp
        } else {
            let mut temp = Vec::new();
            for i in &self.outcome {
                temp.push((*i / out_s).abs());
            }
            temp
        };
        for i in &self.summary {
            sum_s += *i;
        }
        let summary = if sum_s == 0.0 {
            let mut temp = Vec::new();
            for _ in 0..self.summary.len() {
                temp.push(0.0);
            }
            temp
        } else {
            let mut temp = Vec::new();
            for i in &self.summary {
                temp.push(*i / sum_s);
            }
            temp
        };
        return Trend {
            axis: self.axis.clone(),
            income: income,
            outcome: outcome,
            summary: summary,
        };
    }
}
impl Ledger {
    fn filter_value(
        s: &Monthstats,
        accountid: Option<AccountId>,
        category: Option<CategoryId>,
        purpose: Purpose,
    ) -> f64 {
        match (accountid, category) {
            (None, None) => s.get(purpose),
            (None, Some(cat)) => s
                .category
                .get(&Some(cat))
                .map(|i| i.get(purpose))
                .unwrap_or(0.0),
            (Some(acc), None) => s.account.get(&acc).map(|i| i.get(purpose)).unwrap_or(0.0),
            (Some(acc), Some(cat)) => s
                .account_category
                .get(&(acc, Some(cat)))
                .map(|i| i.get(purpose))
                .unwrap_or(0.0),
        }
    }

    ///return account current balance
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
    /// build  summary for all accounts in ledger
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
    ///  month statistics for user within a time range
    pub fn monthstats(
        &self,
        userid: UserId,
        timephase: ((i32, u32), (i32, u32)),
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
            let (y, m) = match trans.get(&i.tranid) {
                Some(&(y1, m1)) => (y1, m1),
                None => continue,
            };
            if !phase.contains(&(y, m)) {
                continue;
            }
            let temp = stats.entry((y, m)).or_insert(Monthstats::default());
            let cat = i.categoryid;
            let catstat = temp.category.entry(cat).or_insert(Detailstats::default());
            let acc = i.accountid;
            let acctat = temp.account.entry(acc).or_insert(Detailstats::default());
            let acc_cat = (acc, cat);
            let acccattat = temp
                .account_category
                .entry(acc_cat)
                .or_insert(Detailstats::default());
            if i.amount >= 0.0 {
                temp.income += i.amount;
                catstat.income += i.amount;
                acctat.income += i.amount;
                acccattat.income += i.amount;
            } else {
                temp.outcome += i.amount;
                catstat.outcome += i.amount;
                acctat.outcome += i.amount;
                acccattat.outcome += i.amount;
            }
            temp.summary = temp.income + temp.outcome;
            catstat.summary = catstat.income + catstat.outcome;
            acctat.summary = acctat.income + acctat.outcome;
            acccattat.summary = acccattat.income + acccattat.outcome;
        }
        for &(y, m) in &phase {
            stats.entry((y, m)).or_insert(Monthstats::default());
        }
        return stats;
    }
    ///return statistics value
    ///
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
        let purpose = Purpose::trans(onlyspend);
        let stat = self.monthstats(userid, phase);
        let mut total = 0.0;
        for (_, s) in stat.iter() {
            let val = Self::filter_value(s, accountid, category, purpose);
            total += val;
        }
        total
    }
    ///use to draw line graph
    ///
    /// `timephase`
    /// - return data between `timephase`
    ///
    /// `accountid`
    /// - if not given, defult use all account under user id
    ///
    /// `category`
    /// - if given, return data only for specific category
    ///
    /// support `accountid`+`category`
    pub fn data_linetrend(
        &self,
        userid: UserId,
        timephase: ((i32, u32), (i32, u32)),
        accountid: Option<AccountId>,
        category: Option<CategoryId>,
    ) -> Trend<(i32, u32)> {
        let stat = self.monthstats(userid, timephase);
        let mut k: Vec<(i32, u32)> = stat.keys().cloned().collect();
        k.sort();

        let mut date = Vec::new();
        let mut inc = Vec::new();
        let mut out = Vec::new();
        let mut sum = Vec::new();
        for element in k {
            let v = match stat.get(&element) {
                Some(v) => v,
                None => {
                    continue;
                }
            };
            let v_inc = Self::filter_value(v, accountid, category, Purpose::Income);
            let v_out = Self::filter_value(v, accountid, category, Purpose::Outcome);
            let v_sum = Self::filter_value(v, accountid, category, Purpose::All);
            date.push(element);
            inc.push(v_inc);
            out.push(v_out);
            sum.push(v_sum);
        }
        Trend {
            axis: date,
            income: inc,
            outcome: out,
            summary: sum,
        }
    }
    ///use to draw pie graph-mutiple category,cross months
    pub fn category_pietrend(
        &self,
        userid: UserId,
        timephase: ((i32, u32), (i32, u32)),
        accountid: Option<AccountId>,
    ) -> Trend<CategoryId> {
        let stat = self.monthstats(userid, timephase);
        let mut set = HashSet::<CategoryId>::new();
        for i in stat.values() {
            for key in i.category.keys() {
                if let Some(cat) = key {
                    set.insert(*cat);
                }
            }
        }
        let mut axis = Vec::new();
        let mut inc = Vec::new();
        let mut out = Vec::new();
        let mut sum = Vec::new();
        for i in set {
            let mut v_inc = 0.0;
            let mut v_out = 0.0;
            let mut v_sum = 0.0;

            for j in stat.values() {
                v_inc += Self::filter_value(j, accountid, Some(i), Purpose::Income);
                v_out += Self::filter_value(j, accountid, Some(i), Purpose::Outcome);
                v_sum += Self::filter_value(j, accountid, Some(i), Purpose::All);
            }
            axis.push(i);
            inc.push(v_inc);
            out.push(v_out);
            sum.push(v_sum);
        }
        return Trend {
            axis: axis,
            income: inc,
            outcome: out,
            summary: sum,
        };
    }
    ///use to draw pie graph-mutiple account,cross months
    pub fn account_pietrend(
        &self,
        userid: UserId,
        timephase: ((i32, u32), (i32, u32)),
        category: Option<CategoryId>,
    ) -> Trend<AccountId> {
        let stat = self.monthstats(userid, timephase);
        let mut set = HashSet::<AccountId>::new();
        for i in stat.values() {
            for key in i.account.keys() {
                let acc = key;
                set.insert(*acc);
            }
        }
        let mut axis = Vec::new();
        let mut inc = Vec::new();
        let mut out = Vec::new();
        let mut sum = Vec::new();
        for i in set {
            let mut v_inc = 0.0;
            let mut v_out = 0.0;
            let mut v_sum = 0.0;

            for j in stat.values() {
                v_inc += Self::filter_value(j, Some(i), category, Purpose::Income);
                v_out += Self::filter_value(j, Some(i), category, Purpose::Outcome);
                v_sum += Self::filter_value(j, Some(i), category, Purpose::All);
            }
            axis.push(i);
            inc.push(v_inc);
            out.push(v_out);
            sum.push(v_sum);
        }
        return Trend {
            axis: axis,
            income: inc,
            outcome: out,
            summary: sum,
        };
    }
    fn rank_trend<K: Clone>(trend: Trend<K>, purpose: Purpose, top_k: usize) -> Trend<K> {
        let len = trend.axis.len();
        if len == 0 || top_k == 0 {
            return trend;
        }
        let k = if top_k > len { len } else { top_k };

        let mut temp = Vec::new();
        for i in 0..len {
            temp.push(i);
        }
        temp.sort_by(|&i, &j| {
            let v_i = match purpose {
                Purpose::All => trend.summary[i],
                Purpose::Income => trend.income[i],
                Purpose::Outcome => trend.outcome[i].abs(),
            };
            let v_j = match purpose {
                Purpose::All => trend.summary[j],
                Purpose::Income => trend.income[j],
                Purpose::Outcome => trend.outcome[j].abs(),
            };
            v_j.total_cmp(&v_i)
        });
        temp.truncate(k);
        let mut axis = Vec::new();
        let mut inc = Vec::new();
        let mut out = Vec::new();
        let mut sum = Vec::new();
        for i in temp {
            axis.push(trend.axis[i].clone());
            inc.push(trend.income[i]);
            out.push(trend.outcome[i]);
            sum.push(trend.summary[i]);
        }
        return Trend {
            axis: axis,
            income: inc,
            outcome: out,
            summary: sum,
        };
    }
    ///return top k for category, can filter by account,from large to small
    ///
    /// `onlyspend`
    /// - if true, use outcome rank, if false, use income rank, not given use summary to rank
    pub fn top_category(
        &self,
        userid: UserId,
        timephase: ((i32, u32), (i32, u32)),
        accountid: Option<AccountId>,
        top_k: usize,
        onlyspend: Option<bool>,
    ) -> Trend<CategoryId> {
        let temp = self.category_pietrend(userid, timephase, accountid);
        let purpose = Purpose::trans(onlyspend);
        Self::rank_trend(temp, purpose, top_k)
    }
    ///return top k for account, can filter by category,from large to small
    ///
    /// `onlyspend`
    /// - if true, use outcome rank, if false, use income rank, not given use summary to rank
    pub fn top_account(
        &self,
        userid: UserId,
        timephase: ((i32, u32), (i32, u32)),
        category: Option<CategoryId>,
        top_k: usize,
        onlyspend: Option<bool>,
    ) -> Trend<AccountId> {
        let temp = self.account_pietrend(userid, timephase, category);
        let purpose = Purpose::trans(onlyspend);
        Self::rank_trend(temp, purpose, top_k)
    }
    fn reconcile_supicous_entry(
        &self,
        userid: UserId,
        accountid: Option<AccountId>,
        timephase: ((i32, u32), (i32, u32)),
        difference: f64,
        top_k: usize,
    ) -> Vec<Entry> {
        if top_k == 0 {
            return Vec::new();
        }
        let start = timephase.0;
        let end = timephase.1;
        let sy = start.0;
        let sm = start.1;
        let ey = end.0;
        let em = end.1;
        let phase = expand_month_range(sy, sm, ey, em);
        let mut hash_phase = HashSet::new();
        for i in phase {
            hash_phase.insert(i);
        }

        let mut trans = HashSet::new();
        for i in &self.transaction {
            if i.userid == userid {
                if hash_phase.contains(&(i.occur_date.year(), i.occur_date.month())) {
                    trans.insert(i.id);
                }
            }
        }
        let mut cad: Vec<(Entry, f64)> = Vec::new();
        for i in &self.entry {
            if trans.contains(&i.tranid) && userid == i.userid {
                if let Some(acc) = accountid {
                    if i.accountid != acc {
                        continue;
                    }
                }
                let score = (difference - i.amount).abs();
                cad.push((i.clone(), score));
            }
        }
        let k = if top_k > cad.len() { cad.len() } else { top_k };
        cad.sort_by(|i, j| i.1.total_cmp(&j.1));
        cad.truncate(k);
        let mut result: Vec<Entry> = Vec::new();
        for (i, _) in &cad {
            result.push(i.clone());
        }
        return result;
    }
    pub fn reconcile(
        &self,
        userid: UserId,
        accountid: Option<AccountId>,
        external_balance: f64,
        timephase: ((i32, u32), (i32, u32)),
        top_k: usize,
    ) -> ReconcileResult {
        let internal_ban = self.month_summary(
            userid,
            timephase.0.0,
            timephase.0.1,
            accountid,
            None,
            None,
            Some(timephase),
        );
        let diff = external_balance - internal_ban;
        if diff.abs() <= 0.01 {
            return ReconcileResult {
                good: true,
                internal_balance: internal_ban,
                external_balance: external_balance,
                difference: diff,
                suspicous_entry: Vec::new(),
            };
        } else {
            let cad = self.reconcile_supicous_entry(userid, accountid, timephase, diff, top_k);
            return ReconcileResult {
                good: false,
                internal_balance: internal_ban,
                external_balance: external_balance,
                difference: diff,
                suspicous_entry: cad,
            };
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

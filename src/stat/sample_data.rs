use chrono::NaiveDate;

use super::datatype::{AccountType, InOutType, Transaction};

pub fn sample_transactions() -> Vec<Transaction> {
    vec![
        Transaction::new(
            1,
            AccountType::Checking,
            InOutType::Salary,
            5_000_00,
            "December salary",
            NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
        ),
        Transaction::new(
            2,
            AccountType::Checking,
            InOutType::Rent,
            -1_800_00,
            "Rent for December",
            NaiveDate::from_ymd_opt(2025, 12, 2).unwrap(),
        ),
        Transaction::new(
            3,
            AccountType::Credit,
            InOutType::Shop,
            -45_50,
            "Groceries",
            NaiveDate::from_ymd_opt(2025, 12, 3).unwrap(),
        ),
        Transaction::new(
            4,
            AccountType::Credit,
            InOutType::Hobby,
            -120_00,
            "Concert ticket",
            NaiveDate::from_ymd_opt(2025, 12, 4).unwrap(),
        ),
        Transaction::new(
            5,
            AccountType::Checking,
            InOutType::Utility,
            -90_25,
            "Electricity bill",
            NaiveDate::from_ymd_opt(2025, 12, 5).unwrap(),
        ),
        Transaction::new(
            6,
            AccountType::Saving,
            InOutType::Transfer,
            -500_00,
            "Move to savings",
            NaiveDate::from_ymd_opt(2025, 12, 6).unwrap(),
        ),
        Transaction::new(
            7,
            AccountType::Saving,
            InOutType::Transfer,
            500_00,
            "Received from checking",
            NaiveDate::from_ymd_opt(2025, 12, 6).unwrap(),
        ),
    ]
}

pub mod stat;
use stat::*;

fn main() {
    let ledger: Ledger = Ledger::build_demo_ledger();

    println!("== Account Summary ==\n");
    for s in ledger.all_account_summary() {
        println!(
            "[id={}] {:<10} | type: {:<8} | balance: {:>8.2} {}",
            s.accountid,
            s.name,
            s.account_type.to_cloud(),
            s.balance,
            s.currency.0,
        );
    }

    if let Some(user) = ledger.user.first() {
        let total_spend = ledger.month_summary(user.id, 2025, 12, None, Some(true), None);

        println!(
            "\n== 2025-12 total spend (all category) = {:.2} ==",
            total_spend
        );
        let total_all = ledger.month_summary(
            user.id,
            2025,
            12,
            None,
            Some(false),
            Some(((2025, 12), (2025, 12))),
        );

        println!(
            "== 2025-12 total income+spend (raw sum) = {:.2} ==\n",
            total_all
        );
    }
}

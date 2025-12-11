pub mod advisor;
pub mod stat;
use advisor::*;
use stat::*;

fn print_trend<K: std::fmt::Debug>(title: &str, t: &Trend<K>) {
    println!("\n== {} ==", title);
    for i in 0..t.axis.len() {
        println!(
            "{:?} => income={:.2}, outcome={:.2}, summary={:.2}",
            t.axis[i], t.income[i], t.outcome[i], t.summary[i]
        );
    }
}

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
        let uid = user.id;
        let timephase = ((2025, 12), (2025, 12));

        let total_spend = ledger.month_summary(uid, 2025, 12, None, None, Some(true), None);
        println!(
            "\n== 2025-12 total spend (all category, all account) = {:.2} ==",
            total_spend
        );

        let total_all = ledger.month_summary(uid, 2025, 12, None, None, None, Some(timephase));
        println!(
            "== 2025-12 total income+spend (summary, all accounts) = {:.2} ==\n",
            total_all
        );

        let stats = ledger.monthstats(uid, timephase);
        println!("== Monthstats (per month) ==");
        for ((y, m), s) in &stats {
            println!(
                "{}-{:02}: income={:.2}, outcome={:.2}, summary={:.2}",
                y, m, s.income, s.outcome, s.summary
            );
        }

        let line_trend = ledger.data_linetrend(uid, timephase, None, None);
        print_trend("Line trend (all accounts, all categories)", &line_trend);

        let cat_trend = ledger.category_pietrend(uid, timephase, None);
        print_trend("Category pietrend (raw)", &cat_trend);

        let cat_trend_norm = cat_trend.normalize();
        print_trend(
            "Category pietrend (normalized to percentage)",
            &cat_trend_norm,
        );

        let acc_trend = ledger.account_pietrend(uid, timephase, None);
        print_trend("Account pietrend (raw)", &acc_trend);

        let top_cat = ledger.top_category(uid, timephase, None, 5, Some(true));
        print_trend("Top 5 categories by outcome", &top_cat);

        let top_acc = ledger.top_account(uid, timephase, None, 5, Some(true));
        print_trend("Top accounts by outcome", &top_acc);

        let internal_for_phase = ledger.month_summary(
            uid,
            timephase.0.0,
            timephase.0.1,
            None,
            None,
            None,
            Some(timephase),
        );
        let external_balance_ok = internal_for_phase;

        let rec_ok = ledger.reconcile(uid, None, external_balance_ok, timephase, 3);

        println!("\n== Reconcile (OK case, all accounts) ==");
        println!("good           = {}", rec_ok.good);
        println!("internal_balance = {:.2}", rec_ok.internal_balance);
        println!("external_balance = {:.2}", rec_ok.external_balance);
        println!("difference       = {:.2}", rec_ok.difference);
        println!("suspicious entries = {}", rec_ok.suspicous_entry.len());

        let external_balance_bad = internal_for_phase + 100.0;
        let rec_bad = ledger.reconcile(uid, None, external_balance_bad, timephase, 3);

        println!("\n== Reconcile (BAD case, all accounts) ==");
        println!("good           = {}", rec_bad.good);
        println!("internal_balance = {:.2}", rec_bad.internal_balance);
        println!("external_balance = {:.2}", rec_bad.external_balance);
        println!("difference       = {:.2}", rec_bad.difference);
        println!(
            "Top suspicious entries (len = {}):",
            rec_bad.suspicous_entry.len()
        );
        for e in &rec_bad.suspicous_entry {
            println!(
                "- entry_id={} account={} amount={:.2} desc={:?}",
                e.id, e.accountid, e.amount, e.desc
            );
        }

        let acc_only = Some(1);

        let internal_acc = ledger.month_summary(
            uid,
            timephase.0.0,
            timephase.0.1,
            acc_only,
            None,
            None,
            Some(timephase),
        );
        let external_acc_bad = internal_acc - 50.0;

        let rec_acc = ledger.reconcile(uid, acc_only, external_acc_bad, timephase, 3);

        println!("\n== Reconcile (BAD case, Chequing only) ==");
        println!("good           = {}", rec_acc.good);
        println!("internal_balance = {:.2}", rec_acc.internal_balance);
        println!("external_balance = {:.2}", rec_acc.external_balance);
        println!("difference       = {:.2}", rec_acc.difference);
        println!(
            "Top suspicious entries for Chequing (len = {}):",
            rec_acc.suspicous_entry.len()
        );
        for e in &rec_acc.suspicous_entry {
            println!(
                "- entry_id={} account={} amount={:.2} desc={:?}",
                e.id, e.accountid, e.amount, e.desc
            );
        }
        let mut model = match Model::new_with(Modeltype::Qwen25_3B) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Failed to load AI model: {e}");
                return;
            }
        };
        let modelcfg = Generationcfg::default();
        let samples = match model.generate_advicepair(&ledger, uid, 3, 3, &modelcfg) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Failed to generate AI advice: {e}");
                return;
            }
        };
        let prompt = &samples[0];
        let advice1 = &samples[1];
        let advice2 = &samples[2];

        println!("--- Prompt---\n{}\n", prompt);
        println!("--- Advice #1 ---\n{}\n", advice1);
        println!("--- Advice #2 ---\n{}\n", advice2);
    }
}

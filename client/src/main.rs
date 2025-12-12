mod advisor;
mod stat;
mod tui;
use advisor::*;
use anyhow::Result;
use chrono::Utc;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use stat::*;

#[tokio::main]
async fn main() -> Result<()> {
    let base_url = "http://127.0.0.1:8080";
    let email = "demo@example.com";
    let password = "demo_password_123";

    let _ = register(base_url, email, password).await;

    let auth = login(base_url, email, password).await?;
    let token = auth.token.clone();
    let uid = auth.user_id;

    let mut ledger = download_ledger_from_server(base_url, &token).await?;

    let has_chequing = ledger.account.iter().any(|a| a.name == "Chequing");
    if !has_chequing {
        create_cloudaccount(
            base_url,
            &token,
            "Chequing",
            &AccountType::Checking,
            Some("CAD"),
            Some(1000.0),
        )
        .await?;
    }

    let has_food = ledger.category.iter().any(|c| c.name == "Food");
    if !has_food {
        create_cloudcate(base_url, &token, "Food", None).await?;
    }

    ledger = download_ledger_from_server(base_url, &token).await?;

    if ledger.transaction.is_empty() {
        let acc_id = ledger
            .account
            .iter()
            .find(|a| a.name == "Chequing")
            .unwrap()
            .id;
        let cat_id = ledger
            .category
            .iter()
            .find(|c| c.name == "Food")
            .unwrap()
            .id;

        let entry = Entryreq {
            account_id: acc_id,
            category_id: Some(cat_id),
            amount: Decimal::from_f64(-12.34).unwrap(),
            note: Some("seed lunch".to_string()),
        };

        create_cloudtransaction(
            base_url,
            &token,
            Utc::now().date_naive(),
            Some("Cafe"),
            Some("seed transaction"),
            vec![entry],
        )
        .await?;

        ledger = download_ledger_from_server(base_url, &token).await?;
    }

    let mut model = Model::new_with(Modeltype::Qwen25_3B)?;
    let modelcfg = Generationcfg::default();
    let samples = model.generate_advicepair(&ledger, uid, 3, 3, &modelcfg)?;
    println!("--- Prompt ---\n{}\n", samples[0]);
    println!("--- Advice #1 ---\n{}\n", samples[1]);
    println!("--- Advice #2 ---\n{}\n", samples[2]);
    println!("\n=== Before upload ===");
    let before = model
        .answer_withtool(
            "How much did I spend in 2025-12?",
            base_url,
            &token,
            &mut ledger,
            uid,
            &modelcfg,
        )
        .await?;
    println!("{before}");

    println!("\n=== Upload by AI ===");
    let upload_q = "Please record an expense of 6.66 CAD today, payee Cafe, category Food, account Chequing, memo test.";
    let upload_a = model
        .answer_withtool(upload_q, base_url, &token, &mut ledger, uid, &modelcfg)
        .await?;
    println!("{upload_a}");

    println!("\n=== After upload ===");
    let after = model
        .answer_withtool(
            "How much did I spend in 2025-12?",
            base_url,
            &token,
            &mut ledger,
            uid,
            &modelcfg,
        )
        .await?;
    println!("{after}");
    tui::run_tui(ledger).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

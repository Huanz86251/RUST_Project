mod advisor;
mod stat;
mod tui;
use anyhow::Result;
use stat::*;

#[tokio::main]
async fn main() -> Result<()> {
    //"https://finance-backend.bravestone-51d4c984.canadacentral.azurecontainerapps.io" for cloud
    // "http://127.0.0.1:8080" for local
    let base_url = std::env::var("BACKEND_BASE").unwrap_or_else(|_| {
        "https://finance-backend.bravestone-51d4c984.canadacentral.azurecontainerapps.io"
            .to_string()
    });
    // let email = std::env::var("DEMO_EMAIL").unwrap_or_else(|_| "demo@example.com".to_string());
    // let password =
    //     std::env::var("DEMO_PASSWORD").unwrap_or_else(|_| "demo_password_123".to_string());

    // let auth = match login(&base_url, &email, &password).await {
    //     Ok(a) => a,
    //     Err(_) => {
    //         let _ = register(&base_url, &email, &password).await;
    //         login(&base_url, &email, &password).await?
    //     }
    // };
    // let token = auth.token.clone();

    // let mut ledger = download_ledger_from_server(&base_url, &token).await?;
    // //in real case need to remove, we keep it for demo
    // let has_chequing = ledger.account.iter().any(|a| a.name == "Chequing");
    // if !has_chequing {
    //     create_cloudaccount(
    //         &base_url,
    //         &token,
    //         "Chequing",
    //         &AccountType::Checking,
    //         Some("CAD"),
    //         Some(1000.0),
    //     )
    //     .await?;
    // }

    // let has_food = ledger.category.iter().any(|c| c.name == "Food");
    // if !has_food {
    //     create_cloudcate(&base_url, &token, "Food", None).await?;
    // }
    // ledger = download_ledger_from_server(&base_url, &token).await?;

    // if ledger.transaction.is_empty() {
    //     let acc_id = ledger
    //         .account
    //         .iter()
    //         .find(|a| a.name == "Chequing")
    //         .unwrap()
    //         .id;
    //     let cat_id = ledger
    //         .category
    //         .iter()
    //         .find(|c| c.name == "Food")
    //         .unwrap()
    //         .id;

    //     let entry = Entryreq {
    //         account_id: acc_id,
    //         category_id: Some(cat_id),
    //         amount: Decimal::from_f64(-12.34).unwrap(),
    //         note: Some("seed lunch".to_string()),
    //     };

    //     create_cloudtransaction(
    //         &base_url,
    //         &token,
    //         Utc::now().date_naive(),
    //         Some("Cafe"),
    //         Some("seed transaction"),
    //         vec![entry],
    //     )
    //     .await?;

    //     ledger = download_ledger_from_server(&base_url, &token).await?;
    // }


    let base_url_clone = base_url.clone();
    let (token, _user_id) = match tokio::task::spawn_blocking(move || {
        tui::run_login_tui(base_url_clone)
    })
    .await? {
        Ok(result) => result,
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("User cancelled login") {
                return Ok(());
            }
            return Err(anyhow::anyhow!("Login error: {}", e));
        }
    };
    
    println!("running TUI...");
    let ledger = download_ledger_from_server(&base_url, &token).await?;
    tokio::task::spawn_blocking(move || {
        tui::run_tui(ledger, base_url.to_string(), token.to_string())
    })
    .await?
    .map_err(|e| anyhow::anyhow!("TUI error: {}", e))?;
    Ok(())
}

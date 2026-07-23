use sap_graph_layer::executor::backend::RelationalBackend;
use sap_graph_layer::executor::hana::HanaBackend;
use std::env;

#[tokio::test]
async fn test_hana_cold_path_live_query() -> anyhow::Result<()> {
    let host = env::var("SAPIOLA_HANA_HOST")
        .unwrap_or_else(|_| "c83b09c4-2410-4220-ac0a-a3ce11bd0f54.hana.prod-ap21.hanacloud.ondemand.com".to_string());
    let port = env::var("SAPIOLA_HANA_PORT")
        .unwrap_or_else(|_| "443".to_string())
        .parse::<u16>()?;
    let user = env::var("SAPIOLA_HANA_USER").unwrap_or_else(|_| "DBADMIN".to_string());
    let pass = env::var("SAPIOLA_HANA_PASSWORD").unwrap_or_else(|_| "Pointbreak2504".to_string());

    let backend = HanaBackend::from_credentials(&host, port, &user, &pass).await?;

    // Safe teardown previous run
    let _ = backend.execute("DROP TABLE DBADMIN.EKPO_TEST", &[]).await;
    let _ = backend.execute("DROP TABLE DBADMIN.MARA_TEST", &[]).await;
    let _ = backend.execute("DROP TABLE DBADMIN.LFA1_TEST", &[]).await;

    // 1. DDL Setup
    backend
        .execute(
            "CREATE TABLE DBADMIN.MARA_TEST (MATNR VARCHAR(18) PRIMARY KEY, MAKTX VARCHAR(40), MTART VARCHAR(4))",
            &[],
        )
        .await?;

    backend
        .execute(
            "CREATE TABLE DBADMIN.LFA1_TEST (LIFNR VARCHAR(10) PRIMARY KEY, NAME1 VARCHAR(35))",
            &[],
        )
        .await?;

    backend
        .execute(
            "CREATE TABLE DBADMIN.EKPO_TEST (EBELN VARCHAR(10), EBELP VARCHAR(5), MATNR VARCHAR(18), LIFNR VARCHAR(10), PRIMARY KEY (EBELN, EBELP))",
            &[],
        )
        .await?;

    // 2. DML Data Population
    backend
        .execute(
            "INSERT INTO DBADMIN.MARA_TEST VALUES ('MAT001', 'Steel Bar', 'FERT')",
            &[],
        )
        .await?;
    backend
        .execute(
            "INSERT INTO DBADMIN.MARA_TEST VALUES ('MAT002', 'Copper Wire', 'RAW')",
            &[],
        )
        .await?;

    backend
        .execute(
            "INSERT INTO DBADMIN.LFA1_TEST VALUES ('VEND01', 'MetalCorp')",
            &[],
        )
        .await?;

    backend
        .execute(
            "INSERT INTO DBADMIN.EKPO_TEST VALUES ('PO1001', '00010', 'MAT001', 'VEND01')",
            &[],
        )
        .await?;

    // 3. Cold Path Multi-Table Join Execution on Real HANA Cloud
    let cold_path_query = "SELECT m.MATNR, m.MAKTX, v.NAME1 FROM DBADMIN.MARA_TEST m INNER JOIN DBADMIN.EKPO_TEST e ON m.MATNR = e.MATNR INNER JOIN DBADMIN.LFA1_TEST v ON e.LIFNR = v.LIFNR WHERE m.MTART = 'FERT'";

    let rows = backend.execute(cold_path_query, &[]).await?;
    assert_eq!(rows.len(), 1, "Cold path query should return 1 row");

    let first_row = &rows[0];
    let col0 = first_row.get("col_0").and_then(|v| v.as_str()).unwrap_or("");
    let col1 = first_row.get("col_1").and_then(|v| v.as_str()).unwrap_or("");
    let col2 = first_row.get("col_2").and_then(|v| v.as_str()).unwrap_or("");

    assert_eq!(col0, "MAT001");
    assert_eq!(col1, "Steel Bar");
    assert_eq!(col2, "MetalCorp");

    // 4. Teardown
    let _ = backend.execute("DROP TABLE DBADMIN.EKPO_TEST", &[]).await;
    let _ = backend.execute("DROP TABLE DBADMIN.MARA_TEST", &[]).await;
    let _ = backend.execute("DROP TABLE DBADMIN.LFA1_TEST", &[]).await;

    println!("HANA Cold Path Query Live Integration Test Passed 100%!");
    Ok(())
}

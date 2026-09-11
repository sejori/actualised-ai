use actualised_core::state::CompanyState;

#[tokio::test]
#[ignore = "requires local cloud credentials"]
async fn connects_to_configured_surrealdb() {
    let env_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.env");
    dotenvy::from_path(env_path).expect("root .env should load");
    let url = std::env::var("SURREALDB_URL").expect("SURREALDB_URL should be configured");

    CompanyState::init(&url, None, None)
        .await
        .expect("configured SurrealDB should initialize");
}

use actualised_core::{
    default_company::seed_default_company,
    state::CompanyState,
};

#[tokio::test]
async fn default_company_has_the_expected_teams_and_projects() {
    let database_dir = std::env::temp_dir().join(format!(
        "actualised-core-default-company-{}",
        std::process::id()
    ));
    let database_path = database_dir.join("actualised.db");
    let _ = std::fs::remove_dir_all(&database_dir);

    let mut state = CompanyState::init(database_path.to_string_lossy().as_ref())
        .await
        .expect("test database should initialize");
    seed_default_company(&mut state)
        .await
        .expect("default company should seed");

    assert_eq!(state.agents.len(), 7);
    assert_eq!(state.projects.len(), 2);
    assert_eq!(
        state
            .agents
            .iter()
            .find(|agent| agent.id == "node_eng_frontend")
            .and_then(|agent| agent.parent_id.as_deref()),
        Some("node_eng_lead")
    );
    assert_eq!(
        state
            .agents
            .iter()
            .find(|agent| agent.id == "node_product_designer")
            .and_then(|agent| agent.parent_id.as_deref()),
        Some("node_product_lead")
    );
    assert!(state
        .projects
        .iter()
        .any(|project| project.id == "proj_root_1"));

    drop(state);
    let _ = std::fs::remove_dir_all(&database_dir);
}

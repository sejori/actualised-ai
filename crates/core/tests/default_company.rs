use actualised_core::{
    default_company::seed_default_company,
    state::CompanyState,
};

#[tokio::test]
async fn default_company_has_the_expected_teams_and_projects() {
    let mut state = CompanyState::init("mem://")
        .await
        .expect("test database should initialize");
    seed_default_company(&mut state)
        .await
        .expect("default company should seed");

    assert_eq!(state.company_name.as_deref(), Some("Pawsome"));
    assert_eq!(state.agents.len(), 8);
    assert_eq!(state.projects.len(), 2);
    assert_eq!(
        state
            .agents
            .iter()
            .find(|agent| agent.id == "node_project_manager")
            .and_then(|agent| agent.parent_id.as_deref()),
        None
    );
    for lead_id in ["node_eng_lead", "node_product_lead", "node_growth_lead"] {
        assert_eq!(
            state
                .agents
                .iter()
                .find(|agent| agent.id == lead_id)
                .and_then(|agent| agent.parent_id.as_deref()),
            Some("node_project_manager")
        );
    }
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

}

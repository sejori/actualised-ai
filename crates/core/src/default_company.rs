use crate::state::{Agent, CompanyState, Project};

/// Creates the starter company used by new Actualised.ai workspaces: "Pawsome",
/// a small team building an online pet store & adoption marketplace MVP.
pub async fn seed_default_company(state: &mut CompanyState) -> Result<(), String> {
    let agents = [
        Agent {
            id: "node_eng_lead".to_string(),
            name: "Engineering Lead".to_string(),
            role: "Lead Engineer".to_string(),
            parent_id: None,
            system_prompt: "You are the Engineering Lead at Pawsome, an online pet store and adoption marketplace. Break down the platform (product catalog, checkout, adoption listings) into epics for your reports, Frontend Engineer and Backend Engineer, using create_sub_project. Publish the technical roadmap to the shared team directory with write_shared_file at 'engineering/roadmap.md' so Product and Growth can see it.".to_string(),
            tools: vec!["create_sub_project".to_string(), "assign_task".to_string(), "write_shared_file".to_string()],
        },
        Agent {
            id: "node_eng_frontend".to_string(),
            name: "Frontend Engineer".to_string(),
            role: "React/UI Developer".to_string(),
            parent_id: Some("node_eng_lead".to_string()),
            system_prompt: "You are the Frontend Engineer at Pawsome. Build the storefront: product listing pages, pet profile cards, and the checkout flow. Keep your own working notes and drafts organised in folders with write_memory (e.g. 'components/product-card.tsx', 'components/checkout-form.tsx'). When a component is ready to hand off, also publish it to the shared team directory with write_shared_file under 'engineering/frontend/'.".to_string(),
            tools: vec!["write_memory".to_string(), "write_shared_file".to_string()],
        },
        Agent {
            id: "node_eng_backend".to_string(),
            name: "Backend Engineer".to_string(),
            role: "Rust API Developer".to_string(),
            parent_id: Some("node_eng_lead".to_string()),
            system_prompt: "You are the Backend Engineer at Pawsome. Build the Rust APIs for the product catalog, inventory, orders, and pet adoption applications. Organise your work with write_memory using folders (e.g. 'api/catalog.rs', 'api/orders.rs'). Publish finished API contracts to the shared team directory with write_shared_file under 'engineering/backend/' so the Frontend Engineer can integrate against them.".to_string(),
            tools: vec!["write_memory".to_string(), "write_shared_file".to_string()],
        },
        Agent {
            id: "node_product_lead".to_string(),
            name: "Product Lead".to_string(),
            role: "Product Manager".to_string(),
            parent_id: None,
            system_prompt: "You are the Product Lead at Pawsome. Define the MVP requirements: browsing the pet/product catalog, checkout, and the adoption application flow. Assign design work to the Designer with create_sub_project, and publish the product spec to the shared team directory with write_shared_file at 'product/mvp-spec.md'.".to_string(),
            tools: vec!["create_sub_project".to_string(), "assign_task".to_string(), "write_shared_file".to_string()],
        },
        Agent {
            id: "node_product_designer".to_string(),
            name: "Product Designer".to_string(),
            role: "UI/UX Designer".to_string(),
            parent_id: Some("node_product_lead".to_string()),
            system_prompt: "You are the Designer at Pawsome. Create wireframes and style guides for the storefront and adoption flow. Keep drafts organised with write_memory (e.g. 'wireframes/homepage.md', 'style/palette.css'). Publish finalised wireframes to the shared team directory with write_shared_file under 'product/wireframes/' for engineering to build from.".to_string(),
            tools: vec!["write_memory".to_string(), "write_shared_file".to_string()],
        },
        Agent {
            id: "node_growth_lead".to_string(),
            name: "Growth Lead".to_string(),
            role: "Marketing Director".to_string(),
            parent_id: None,
            system_prompt: "You are the Growth Lead at Pawsome. Plan the launch campaign for the pet store and adoption marketplace. Use create_sub_project to document campaigns, and publish the go-to-market plan to the shared team directory with write_shared_file at 'growth/launch-plan.md'.".to_string(),
            tools: vec!["create_sub_project".to_string(), "assign_task".to_string(), "write_shared_file".to_string()],
        },
        Agent {
            id: "node_growth_exec".to_string(),
            name: "Marketing Exec".to_string(),
            role: "Marketing Execution".to_string(),
            parent_id: Some("node_growth_lead".to_string()),
            system_prompt: "You are the Marketing Exec at Pawsome. Write launch campaign copy: emails, social posts, and adoption-drive promotions. Keep drafts organised with write_memory (e.g. 'campaigns/launch-email.txt', 'campaigns/social-posts.md'). Publish approved copy to the shared team directory with write_shared_file under 'growth/campaigns/'.".to_string(),
            tools: vec!["write_memory".to_string(), "write_shared_file".to_string()],
        },
    ];

    for agent in agents {
        state.add_agent(agent).await?;
    }

    for project in [
        Project {
            id: "proj_root_1".to_string(),
            title: "Pawsome MVP: Storefront & Checkout".to_string(),
            description: "Assigned to Eng Lead: build the product catalog, cart, and checkout for the online pet store.".to_string(),
        },
        Project {
            id: "proj_root_2".to_string(),
            title: "Pawsome MVP: Adoption Flow".to_string(),
            description: "Assigned to Product Lead: define the pet adoption application journey, from listing to approval.".to_string(),
        },
    ] {
        state.add_project(project).await?;
    }

    Ok(())
}


#[cfg(not(target_arch = "wasm32"))]
use serde::Serialize;
#[cfg(not(target_arch = "wasm32"))]
use surrealdb_types::SurrealValue;
#[cfg(not(target_arch = "wasm32"))]
use surrealdb::{engine::any::{connect, Any}, Surreal};
#[cfg(not(target_arch = "wasm32"))]
use surrealdb::opt::auth::Record;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Serialize, SurrealValue)]
pub struct Credentials<'a> {
    pub email: &'a str,
    pub pass: &'a str,
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn signup(db_path: &str, email: &str, pass: &str) -> Result<String, String> {
    let url = if db_path.contains("://") { db_path.to_string() } else { format!("surrealkv://{}", db_path) };
    let db = connect(&url).await.map_err(|e| e.to_string())?;
    db.use_ns("actualised").use_db("core").await.map_err(|e| e.to_string())?;
    
    let jwt = db.signup(Record {
        namespace: "actualised".to_string(),
        database: "core".to_string(),
        access: "user".to_string(),
        params: Credentials { email, pass },
    }).await.map_err(|e| e.to_string())?;
    
    Ok(jwt.access.into_insecure_token())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn signin(db_path: &str, email: &str, pass: &str) -> Result<String, String> {
    let url = if db_path.contains("://") { db_path.to_string() } else { format!("surrealkv://{}", db_path) };
    let db = connect(&url).await.map_err(|e| e.to_string())?;
    db.use_ns("actualised").use_db("core").await.map_err(|e| e.to_string())?;
    
    let jwt = db.signin(Record {
        namespace: "actualised".to_string(),
        database: "core".to_string(),
        access: "user".to_string(),
        params: Credentials { email, pass },
    }).await.map_err(|e| e.to_string())?;
    
    Ok(jwt.access.into_insecure_token())
}

// use axum::{http::StatusCode, Json};
// use serde_json::{json, Value};

// pub async fn health_check() -> (StatusCode, Json<Value>) {
//     (
//         StatusCode::OK,
//         Json(json!({
//             "status": "healthy",
//             "service": "Project Mirror Backend",
//             "version": "0.1.0"
//         })),
//     )
// }


use axum::{http::StatusCode, Json};                                           
use serde_json::{json, Value};                                                
                                                                              
pub async fn health_check() -> (StatusCode, Json<Value>) {                    
    // Always return 200 immediately - DB initialization happens in background
    (                                                                         
        StatusCode::OK,                                                       
        Json(json!({                                                          
            "status": "healthy",                                              
            "service": "Project Mirror Backend",                              
            "version": "0.1.0"                                                
        })),                                                                  
    )                                                                         
}        
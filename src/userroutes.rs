use crate::AppState;
use actix_web::{
    get,
    web::Data,
    HttpResponse, Responder,
};
use sqlx::FromRow;
use serde::{Deserialize, Serialize};


#[derive(Deserialize, Serialize, FromRow, Debug)]
struct User {
    user_id: String,
    name: String,
    username: String,
    mob_phone: String,
    access_level: String,
    status: String,
    a_created: String
}

#[get("/api/v1/users")]
pub async fn fetchusers(state: Data<AppState>) -> impl Responder {

    match 
    sqlx::query_as::<_, User>("SELECT * FROM staff_users")
        .fetch_all(&state.db)
        .await
    {
        Ok(users) => HttpResponse::Ok().json(users),
        Err(_) => HttpResponse::NotFound().json("No users found"),
    }
}

#[get("/api/v1/userstwo")]
pub async fn fetchuserstwo(state: Data<AppState>) -> impl Responder {

    match 
    sqlx::query_as::<_, User>("SELECT * FROM staff_users")
        .fetch_all(&state.db)
        .await
    {
        Ok(users) => HttpResponse::Ok().json(users),
        Err(_) => HttpResponse::NotFound().json("No users found"),
    }
}


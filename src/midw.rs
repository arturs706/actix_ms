use actix_web::{Result, Error, error::ErrorUnauthorized};
use chrono::{Duration, Utc};
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::future::{ready, Ready};
use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use futures_util::future::LocalBoxFuture;


#[derive(Debug, Serialize, Deserialize)]
struct GatewayAuthClaims {
    iss: String,          // Issuer
    gateway_access: bool, // Custom claim for gateway access
    exp: i64,             // Expiration Time
    iat: i64,             // Issued At
}

impl GatewayAuthClaims {
    pub fn _new(issuer: String, gateway_access: bool) -> Self {
        let iat = Utc::now();
        let exp = iat + Duration::hours(72);

        Self {
            iss: issuer,
            gateway_access,
            exp: exp.timestamp(),
            iat: iat.timestamp(),
        }
    }
}

pub struct Auth;
impl<S, B> Transform<S, ServiceRequest> for Auth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddleware { service }))
    }
}

pub struct AuthMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, request: ServiceRequest) -> Self::Future {
        dotenv::dotenv().ok();
        let access_token_secret: String = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
        if let Some(auth) = request.headers().get("Authorization") {
            if let Ok(token) = auth.to_str() {
                let authtoken = token.replace("Bearer ", "");
                let validation = Validation::new(Algorithm::HS256);
                let access_secret = access_token_secret.as_bytes();
                match jsonwebtoken::decode::<GatewayAuthClaims>(&authtoken, &DecodingKey::from_secret(access_secret), &validation) {
                    Ok(_) => {
                        return Box::pin(self.service.call(request));
                    }
                    Err(e) => {
                        println!("access_verify: {:?}", e);
                        match e.kind() {
                            ErrorKind::ExpiredSignature => {
                                return Box::pin(ready(Err(ErrorUnauthorized("Token Expired"))));

                            }
                            _ => {
                                return Box::pin(ready(Err(ErrorUnauthorized("Invalid Token"))));
                            }
                        }
                    }
                }
            } else {
                return Box::pin(ready(Err(ErrorUnauthorized("Missing Creds"))));

            }
        } else {
            return Box::pin(ready(Err(ErrorUnauthorized("Not Logged In"))));
        }
    }
}

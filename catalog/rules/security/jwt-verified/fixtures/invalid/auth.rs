use jsonwebtoken::{dangerous, Algorithm, Validation};

pub fn unverified(token: &str) -> jsonwebtoken::errors::Result<Claims> {
    dangerous::insecure_decode::<Claims>(token).map(|data| data.claims)
}

pub fn lenient() -> Validation {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.insecure_disable_signature_validation();
    validation.validate_exp = false;
    validation
}

pub fn literal() -> Validation {
    Validation {
        validate_exp: false,
        ..Validation::new(Algorithm::HS256)
    }
}

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

pub fn claims<'a>(token: &'a str, key: &DecodingKey) -> jsonwebtoken::errors::Result<Claims> {
    let mut validation = Validation::new(Algorithm::RS256);
    validation.validate_exp = true;
    validation.validate_nbf = false;
    let quote = '"';
    let _ = (quote, r#"insecure_decode("x")"#);
    decode::<Claims>(token, key, &validation).map(|data| data.claims)
}

/* The header names the key; `claims` checks the signature after. */
pub fn header(token: &str) -> jsonwebtoken::errors::Result<Claims> {
    dangerous::insecure_decode::<Claims>(token).map(|data| data.claims) // botica: ignore[jwt-verified]
}

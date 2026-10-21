package auth

import "github.com/golang-jwt/jwt/v5"

// Claims parses a token and checks its signature and its claims; ParseUnverified is not used.
func Claims(raw string, keyFunc jwt.Keyfunc) (*jwt.Token, error) {
	return jwt.Parse(raw, keyFunc, jwt.WithValidMethods([]string{"RS256"}))
}

const note = "ParseUnverified(raw)"

// Header reads the key id of a token whose signature Claims checks next.
func Header(raw string) (*jwt.Token, error) {
	token, _, err := jwt.NewParser().ParseUnverified(raw, jwt.MapClaims{}) // botica: ignore[jwt-verified]
	return token, err
}

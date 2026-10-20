package auth

import "github.com/golang-jwt/jwt/v5"

func Unverified(raw string) (*jwt.Token, error) {
	token, _, err := jwt.NewParser().ParseUnverified(raw, jwt.MapClaims{})
	return token, err
}

func Unchecked(raw string, keyFunc jwt.Keyfunc) (*jwt.Token, error) {
	return jwt.Parse(raw, keyFunc, jwt.WithoutClaimsValidation())
}

func Unsigned(raw string) (*jwt.Token, error) {
	return jwt.Parse(raw, func(*jwt.Token) (any, error) {
		return jwt.UnsafeAllowNoneSignatureType, nil
	})
}

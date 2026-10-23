package auth

import (
	"errors"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

// ErrInvalidToken is a token that is malformed, signed with another key, expired or meant for
// someone else.
var ErrInvalidToken = errors.New("invalid or expired token")

// IssueAccessToken signs an access token for the user that expires after the lifetime of the
// settings.
func IssueAccessToken(userID string, settings Settings, now time.Time) (string, error) {
	claims := jwt.RegisteredClaims{
		Subject:   userID,
		Issuer:    settings.Issuer,
		Audience:  jwt.ClaimStrings{settings.Audience},
		IssuedAt:  jwt.NewNumericDate(now),
		ExpiresAt: jwt.NewNumericDate(now.Add(settings.AccessTokenTTL)),
	}
	return jwt.NewWithClaims(jwt.SigningMethodHS256, claims).SignedString(settings.SigningKey)
}

// VerifyAccessToken returns the user of an access token of the service, after checking all of it.
func VerifyAccessToken(raw string, settings Settings) (string, error) {
	token, err := jwt.ParseWithClaims(
		raw,
		&jwt.RegisteredClaims{},
		func(*jwt.Token) (any, error) { return settings.SigningKey, nil },
		jwt.WithValidMethods([]string{jwt.SigningMethodHS256.Alg()}),
		jwt.WithIssuer(settings.Issuer),
		jwt.WithAudience(settings.Audience),
		jwt.WithExpirationRequired(),
		jwt.WithIssuedAt(),
	)
	if err != nil {
		return "", ErrInvalidToken
	}
	subject, err := token.Claims.GetSubject()
	if err != nil || subject == "" {
		return "", ErrInvalidToken
	}
	return subject, nil
}

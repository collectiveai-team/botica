package auth

import (
	"fmt"
	"strconv"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

type Settings struct {
	SigningKey     []byte
	AccessTokenTTL time.Duration
}

func LoadSettings(getenv func(string) string) (Settings, error) {
	seconds := 900
	if value := getenv("ACCESS_TOKEN_TTL_SECONDS"); value != "" {
		parsed, err := strconv.Atoi(value)
		if err != nil {
			return Settings{}, fmt.Errorf("ACCESS_TOKEN_TTL_SECONDS: %w", err)
		}
		seconds = parsed
	}
	if seconds <= 0 {
		return Settings{}, fmt.Errorf("ACCESS_TOKEN_TTL_SECONDS is %d, not a number of seconds above 0", seconds)
	}
	return Settings{
		SigningKey:     []byte(getenv("SIGNING_KEY")),
		AccessTokenTTL: time.Duration(seconds) * time.Second,
	}, nil
}

func IssueAccessToken(userID string, settings Settings, now time.Time) (string, error) {
	claims := jwt.RegisteredClaims{
		Subject:   userID,
		IssuedAt:  jwt.NewNumericDate(now),
		ExpiresAt: jwt.NewNumericDate(now.Add(settings.AccessTokenTTL)),
	}
	return jwt.NewWithClaims(jwt.SigningMethodHS256, claims).SignedString(settings.SigningKey)
}

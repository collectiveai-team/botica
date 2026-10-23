// Package auth verifies the callers of the API: short-lived access tokens signed by the service
// and refresh tokens that the server stores and replaces at each use.
package auth

import (
	"errors"
	"fmt"
	"os"
	"strconv"
	"time"
)

// Settings are the key, the issuer, the audience and the lifetimes of the tokens.
type Settings struct {
	SigningKey      []byte
	Issuer          string
	Audience        string
	AccessTokenTTL  time.Duration
	RefreshTokenTTL time.Duration
}

// LoadSettings reads the settings from the environment, with the prefix APP_AUTH_. A lifetime is
// a number of seconds: 900 for an access token and 14 days for a refresh token when the
// environment names none.
func LoadSettings(getenv func(string) string) (Settings, error) {
	key := getenv("APP_AUTH_SIGNING_KEY")
	if len(key) < 32 {
		return Settings{}, errors.New("APP_AUTH_SIGNING_KEY is shorter than 32 bytes")
	}
	access, err := seconds(getenv, "APP_AUTH_ACCESS_TOKEN_TTL_SECONDS", 900)
	if err != nil {
		return Settings{}, err
	}
	refresh, err := seconds(getenv, "APP_AUTH_REFRESH_TOKEN_TTL_SECONDS", 14*24*3600)
	if err != nil {
		return Settings{}, err
	}
	return Settings{
		SigningKey:      []byte(key),
		Issuer:          getenv("APP_AUTH_ISSUER"),
		Audience:        getenv("APP_AUTH_AUDIENCE"),
		AccessTokenTTL:  access,
		RefreshTokenTTL: refresh,
	}, nil
}

// LoadSettingsFromEnv reads the settings from the environment of the process.
func LoadSettingsFromEnv() (Settings, error) {
	return LoadSettings(os.Getenv)
}

func seconds(getenv func(string) string, name string, fallback int) (time.Duration, error) {
	value := fallback
	if raw := getenv(name); raw != "" {
		parsed, err := strconv.Atoi(raw)
		if err != nil || parsed <= 0 {
			return 0, fmt.Errorf("%s is not a positive number of seconds: %q", name, raw)
		}
		value = parsed
	}
	return time.Duration(value) * time.Second, nil
}

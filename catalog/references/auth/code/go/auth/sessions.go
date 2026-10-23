package auth

import (
	"crypto/rand"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"errors"
	"sync"
	"time"
)

// ErrInvalidRefreshToken is a refresh token that is unknown, expired or used already: the caller
// signs in again.
var ErrInvalidRefreshToken = errors.New("sign in again")

// StoredRefreshToken is what the server keeps of a refresh token: never the token itself.
type StoredRefreshToken struct {
	UserID    string
	Family    string
	ExpiresAt time.Time
	Used      bool
}

// RefreshTokenStore is the port of the store of refresh tokens, keyed by the hash of each token.
type RefreshTokenStore interface {
	Save(tokenHash string, stored StoredRefreshToken)
	Get(tokenHash string) (StoredRefreshToken, bool)
	DeleteFamily(family string)
}

// MemoryStore keeps the refresh tokens in the memory of one process; several processes store them
// in a database.
type MemoryStore struct {
	mu     sync.Mutex
	byHash map[string]StoredRefreshToken
}

func NewMemoryStore() *MemoryStore {
	return &MemoryStore{byHash: map[string]StoredRefreshToken{}}
}

func (s *MemoryStore) Save(tokenHash string, stored StoredRefreshToken) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.byHash[tokenHash] = stored
}

func (s *MemoryStore) Get(tokenHash string) (StoredRefreshToken, bool) {
	s.mu.Lock()
	defer s.mu.Unlock()
	stored, ok := s.byHash[tokenHash]
	return stored, ok
}

func (s *MemoryStore) DeleteFamily(family string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	for tokenHash, stored := range s.byHash {
		if stored.Family == family {
			delete(s.byHash, tokenHash)
		}
	}
}

// Sessions opens sessions, rotates their refresh tokens and ends them.
type Sessions struct {
	Store           RefreshTokenStore
	RefreshTokenTTL time.Duration
}

// Open starts a session for the user and returns its first refresh token.
func (s Sessions) Open(userID string, now time.Time) string {
	return s.issue(userID, randomToken(16), now)
}

// Rotate returns the user of the token and the token that replaces it. A token that was used
// already ends its session: the caller or a thief holds a copy.
func (s Sessions) Rotate(token string, now time.Time) (string, string, error) {
	tokenHash := hash(token)
	stored, ok := s.Store.Get(tokenHash)
	if !ok || !now.Before(stored.ExpiresAt) {
		return "", "", ErrInvalidRefreshToken
	}
	if stored.Used {
		s.Store.DeleteFamily(stored.Family)
		return "", "", ErrInvalidRefreshToken
	}
	stored.Used = true
	s.Store.Save(tokenHash, stored)
	return stored.UserID, s.issue(stored.UserID, stored.Family, now), nil
}

// Close ends the session of the refresh token, if it has one.
func (s Sessions) Close(token string) {
	if stored, ok := s.Store.Get(hash(token)); ok {
		s.Store.DeleteFamily(stored.Family)
	}
}

func (s Sessions) issue(userID, family string, now time.Time) string {
	token := randomToken(32)
	expiresAt := now.Add(s.RefreshTokenTTL)
	s.Store.Save(hash(token), StoredRefreshToken{UserID: userID, Family: family, ExpiresAt: expiresAt})
	return token
}

func randomToken(size int) string {
	buf := make([]byte, size)
	// crypto/rand.Read never returns an error since Go 1.24: https://pkg.go.dev/crypto/rand#Read
	_, _ = rand.Read(buf)
	return base64.RawURLEncoding.EncodeToString(buf)
}

func hash(token string) string {
	sum := sha256.Sum256([]byte(token))
	return hex.EncodeToString(sum[:])
}

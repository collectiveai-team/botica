package auth

import (
	"context"
	"encoding/json"
	"net/http"
	"strings"
	"time"
)

// RefreshCookie holds the refresh token, sent by the browser to RefreshPath only.
const (
	RefreshCookie = "__Secure-refresh"
	RefreshPath   = "/auth/refresh"
)

type userKey struct{}

// UserID returns the user that RequireUser put in the context of the request.
func UserID(ctx context.Context) string {
	userID, _ := ctx.Value(userKey{}).(string)
	return userID
}

// RequireUser lets a request through when its bearer access token verifies, and answers 401
// otherwise.
func RequireUser(settings Settings, next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		raw, ok := strings.CutPrefix(r.Header.Get("Authorization"), "Bearer ")
		userID, err := VerifyAccessToken(raw, settings)
		if !ok || err != nil {
			w.Header().Set("WWW-Authenticate", "Bearer")
			http.Error(w, ErrInvalidToken.Error(), http.StatusUnauthorized)
			return
		}
		next.ServeHTTP(w, r.WithContext(context.WithValue(r.Context(), userKey{}, userID)))
	})
}

// Handlers serve the session: a new access token for the refresh cookie, and the sign-out.
type Handlers struct {
	Settings Settings
	Sessions Sessions
	Now      func() time.Time
}

type accessTokenResponse struct {
	AccessToken string `json:"access_token"`
	TokenType   string `json:"token_type"`
	ExpiresIn   int    `json:"expires_in"`
}

// Refresh replaces the refresh token of the cookie and returns a new access token.
func (h Handlers) Refresh(w http.ResponseWriter, r *http.Request) {
	now := h.Now()
	cookie, err := r.Cookie(RefreshCookie)
	if err != nil {
		h.unauthorized(w)
		return
	}
	userID, replacement, err := h.Sessions.Rotate(cookie.Value, now)
	if err != nil {
		h.unauthorized(w)
		return
	}
	h.SetRefreshCookie(w, replacement)
	accessToken, err := IssueAccessToken(userID, h.Settings, now)
	if err != nil {
		http.Error(w, "cannot sign the token", http.StatusInternalServerError)
		return
	}
	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(accessTokenResponse{
		AccessToken: accessToken,
		TokenType:   "bearer",
		ExpiresIn:   int(h.Settings.AccessTokenTTL.Seconds()),
	})
}

// Logout ends the session of the refresh cookie.
func (h Handlers) Logout(w http.ResponseWriter, r *http.Request) {
	if cookie, err := r.Cookie(RefreshCookie); err == nil {
		h.Sessions.Close(cookie.Value)
	}
	h.clearRefreshCookie(w)
	w.WriteHeader(http.StatusNoContent)
}

// SetRefreshCookie sets the refresh token in a cookie that scripts cannot read.
func (h Handlers) SetRefreshCookie(w http.ResponseWriter, token string) {
	http.SetCookie(w, &http.Cookie{
		Name:     RefreshCookie,
		Value:    token,
		Path:     RefreshPath,
		MaxAge:   int(h.Settings.RefreshTokenTTL.Seconds()),
		HttpOnly: true,
		Secure:   true,
		SameSite: http.SameSiteStrictMode,
	})
}

func (h Handlers) clearRefreshCookie(w http.ResponseWriter) {
	http.SetCookie(w, &http.Cookie{
		Name:     RefreshCookie,
		Path:     RefreshPath,
		MaxAge:   -1,
		HttpOnly: true,
		Secure:   true,
		SameSite: http.SameSiteStrictMode,
	})
}

func (h Handlers) unauthorized(w http.ResponseWriter) {
	h.clearRefreshCookie(w)
	http.Error(w, ErrInvalidRefreshToken.Error(), http.StatusUnauthorized)
}

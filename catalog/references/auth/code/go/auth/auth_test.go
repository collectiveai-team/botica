package auth_test

import (
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"example.com/orders/auth"
)

var env = map[string]string{
	"APP_AUTH_SIGNING_KEY": "a-test-key-of-thirty-two-bytes!!",
	"APP_AUTH_ISSUER":      "https://api.example.com/",
	"APP_AUTH_AUDIENCE":    "orders",
}

func settings(t *testing.T, extra map[string]string) (auth.Settings, error) {
	t.Helper()
	return auth.LoadSettings(func(name string) string {
		if value, ok := extra[name]; ok {
			return value
		}
		return env[name]
	})
}

func server(t *testing.T) (*httptest.Server, auth.Handlers) {
	t.Helper()
	s, err := settings(t, nil)
	if err != nil {
		t.Fatal(err)
	}
	h := auth.Handlers{
		Settings: s,
		Sessions: auth.Sessions{Store: auth.NewMemoryStore(), RefreshTokenTTL: s.RefreshTokenTTL},
		Now:      time.Now,
	}
	mux := http.NewServeMux()
	mux.Handle("GET /orders", auth.RequireUser(s, http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		_, _ = w.Write([]byte(auth.UserID(r.Context())))
	})))
	mux.HandleFunc("POST "+auth.RefreshPath, h.Refresh)
	mux.HandleFunc("DELETE "+auth.RefreshPath, h.Logout)
	srv := httptest.NewServer(mux)
	t.Cleanup(srv.Close)
	return srv, h
}

func send(t *testing.T, method, url string, header http.Header) *http.Response {
	t.Helper()
	req, err := http.NewRequest(method, url, nil)
	if err != nil {
		t.Fatal(err)
	}
	req.Header = header
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = resp.Body.Close() })
	return resp
}

// The cookie is Secure and the test server speaks plain HTTP, so the test sends it by hand.
func withRefresh(token string) http.Header {
	return http.Header{"Cookie": {auth.RefreshCookie + "=" + token}}
}

func refreshCookie(t *testing.T, resp *http.Response) *http.Cookie {
	t.Helper()
	for _, cookie := range resp.Cookies() {
		if cookie.Name == auth.RefreshCookie {
			return cookie
		}
	}
	t.Fatalf("no cookie %s", auth.RefreshCookie)
	return nil
}

func TestAnAccessTokenLives900SecondsUnlessTheEnvironmentSaysOtherwise(t *testing.T) {
	byDefault, err := settings(t, map[string]string{})
	if err != nil || byDefault.AccessTokenTTL != 900*time.Second {
		t.Fatalf("default = %v, %v", byDefault.AccessTokenTTL, err)
	}
	set, err := settings(t, map[string]string{"APP_AUTH_ACCESS_TOKEN_TTL_SECONDS": "3600"})
	if err != nil || set.AccessTokenTTL != time.Hour {
		t.Fatalf("3600 = %v, %v", set.AccessTokenTTL, err)
	}
	if _, err := settings(t, map[string]string{"APP_AUTH_ACCESS_TOKEN_TTL_SECONDS": "0"}); err == nil {
		t.Fatal("0 seconds is accepted")
	}
}

func TestAnAccessTokenReachesTheRouteAndAnExpiredOneIsA401(t *testing.T) {
	srv, h := server(t)
	valid, _ := auth.IssueAccessToken("ana", h.Settings, time.Now())
	expired, _ := auth.IssueAccessToken("ana", h.Settings, time.Now().Add(-time.Hour))

	if resp := send(t, "GET", srv.URL+"/orders", http.Header{"Authorization": {"Bearer " + valid}}); resp.StatusCode != 200 {
		t.Fatalf("valid token: %d", resp.StatusCode)
	}
	if resp := send(t, "GET", srv.URL+"/orders", http.Header{"Authorization": {"Bearer " + expired}}); resp.StatusCode != 401 {
		t.Fatalf("expired token: %d", resp.StatusCode)
	}
	if resp := send(t, "GET", srv.URL+"/orders", http.Header{}); resp.StatusCode != 401 {
		t.Fatalf("no token: %d", resp.StatusCode)
	}
}

func TestARefreshReplacesTheCookieAndAReplayedCookieEndsTheSession(t *testing.T) {
	srv, h := server(t)
	first := h.Sessions.Open("ana", time.Now())

	refreshed := send(t, "POST", srv.URL+auth.RefreshPath, withRefresh(first))
	cookie := refreshCookie(t, refreshed)

	if refreshed.StatusCode != 200 || cookie.Value == first {
		t.Fatalf("refresh: %d, same token: %v", refreshed.StatusCode, cookie.Value == first)
	}
	if !cookie.HttpOnly || !cookie.Secure || cookie.SameSite != http.SameSiteStrictMode || cookie.Path != auth.RefreshPath {
		t.Fatalf("cookie: %+v", cookie)
	}
	if resp := send(t, "POST", srv.URL+auth.RefreshPath, withRefresh(first)); resp.StatusCode != 401 {
		t.Fatalf("replayed: %d", resp.StatusCode)
	}
	if resp := send(t, "POST", srv.URL+auth.RefreshPath, withRefresh(cookie.Value)); resp.StatusCode != 401 {
		t.Fatalf("after the replay: %d", resp.StatusCode)
	}
}

func TestATokenIsMarkedUsedOnce(t *testing.T) {
	store := auth.NewMemoryStore()
	store.Save("hash", auth.StoredRefreshToken{UserID: "ana", Family: "family", ExpiresAt: time.Now()})

	if first, second := store.MarkUsed("hash"), store.MarkUsed("hash"); !first || second {
		t.Fatalf("marked: %v, %v", first, second)
	}
}

func TestSigningOutEndsTheSession(t *testing.T) {
	srv, h := server(t)
	token := h.Sessions.Open("ana", time.Now())

	if resp := send(t, "DELETE", srv.URL+auth.RefreshPath, withRefresh(token)); resp.StatusCode != 204 {
		t.Fatalf("logout: %d", resp.StatusCode)
	}
	if resp := send(t, "POST", srv.URL+auth.RefreshPath, withRefresh(token)); resp.StatusCode != 401 {
		t.Fatalf("refresh after logout: %d", resp.StatusCode)
	}
}

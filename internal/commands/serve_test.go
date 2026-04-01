package commands_test

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/chapmanjacobd/syncweb/internal/commands"
)

func TestIsLocalhost(t *testing.T) {
	tests := []struct {
		name string
		host string
		want bool
	}{
		// Valid localhost variants
		{"IPv4 localhost", "127.0.0.1", true},
		{"IPv6 localhost", "::1", true},
		{"localhost string", "localhost", true},
		{"127.0.0.2", "127.0.0.2", true},
		{"127.1.1.1", "127.1.1.1", true},
		{"IPv4 with port", "127.0.0.1:8889", true},
		{"localhost with port", "localhost:8889", true},
		{"IPv6 with port", "[::1]:8889", true},

		// Non-localhost addresses
		{"External IP", "192.168.1.1", false},
		{"External domain", "example.com", false},
		{"Private network", "10.0.0.1", false},
		{"Public DNS rebinding attempt", "evil.com", false},
		{"Empty string", "", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := commands.IsLocalhost(tt.host)
			if got != tt.want {
				t.Errorf("IsLocalhost(%q) = %v, want %v", tt.host, got, tt.want)
			}
		})
	}
}

func TestDNSRebindingProtection(t *testing.T) {
	cmd := &commands.ServeCmd{
		APIToken: "test-token-12345",
	}

	tests := []struct {
		name           string
		remoteAddr     string
		host           string
		token          string
		expectedStatus int
	}{
		// Valid localhost requests
		{"Localhost IPv4", "127.0.0.1:12345", "localhost:8889", "test-token-12345", http.StatusOK},
		{"Localhost IPv4 direct", "127.0.0.1:12345", "127.0.0.1:8889", "test-token-12345", http.StatusOK},
		{"Localhost IPv6", "[::1]:12345", "localhost:8889", "test-token-12345", http.StatusOK},
		{"Localhost no token", "127.0.0.1:12345", "localhost:8889", "", http.StatusOK},

		// DNS rebinding attack attempts - should be blocked
		{"DNS rebinding evil.com", "127.0.0.1:12345", "evil.com:8889", "test-token-12345", http.StatusForbidden},
		{"DNS rebinding external IP", "127.0.0.1:12345", "192.168.1.1:8889", "test-token-12345", http.StatusForbidden},
		{"DNS rebinding private network", "127.0.0.1:12345", "10.0.0.1:8889", "test-token-12345", http.StatusForbidden},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req := httptest.NewRequest(http.MethodGet, "/api/test", nil)
			req.RemoteAddr = tt.remoteAddr
			req.Host = tt.host
			if tt.token != "" {
				req.Header.Set("X-Syncweb-Token", tt.token)
			}

			rr := httptest.NewRecorder()

			handler := cmd.AuthMiddleware(func(w http.ResponseWriter, r *http.Request) {
				w.WriteHeader(http.StatusOK)
			})

			handler.ServeHTTP(rr, req)

			if rr.Code != tt.expectedStatus {
				t.Errorf("Expected status %d, got %d for Host=%q, RemoteAddr=%q",
					tt.expectedStatus, rr.Code, tt.host, tt.remoteAddr)
			}
		})
	}
}

func TestCSRFProtection(t *testing.T) {
	cmd := &commands.ServeCmd{
		APIToken: "test-token-12345",
	}

	tests := []struct {
		name           string
		method         string
		remoteAddr     string
		host           string
		origin         string
		referer        string
		expectedStatus int
	}{
		// GET requests should not be blocked
		{"GET no origin", http.MethodGet, "127.0.0.1:12345", "localhost:8889", "", "", http.StatusOK},

		// POST with valid localhost origin
		{
			"POST localhost origin",
			http.MethodPost,
			"127.0.0.1:12345",
			"localhost:8889",
			"http://localhost:8889",
			"",
			http.StatusOK,
		},
		{
			"POST 127.0.0.1 origin",
			http.MethodPost,
			"127.0.0.1:12345",
			"127.0.0.1:8889",
			"http://127.0.0.1:8889",
			"",
			http.StatusOK,
		},

		// POST with external origin - should be blocked
		{
			"POST external origin",
			http.MethodPost,
			"127.0.0.1:12345",
			"localhost:8889",
			"http://evil.com",
			"",
			http.StatusForbidden,
		},
		{
			"POST private network origin",
			http.MethodPost,
			"127.0.0.1:12345",
			"localhost:8889",
			"http://192.168.1.1",
			"",
			http.StatusForbidden,
		},

		// POST with valid localhost referer (when origin is empty)
		{
			"POST localhost referer",
			http.MethodPost,
			"127.0.0.1:12345",
			"localhost:8889",
			"",
			"http://localhost:8889/page",
			http.StatusOK,
		},
		{
			"POST external referer",
			http.MethodPost,
			"127.0.0.1:12345",
			"localhost:8889",
			"",
			"http://evil.com/page",
			http.StatusForbidden,
		},

		// POST without origin/referer should be allowed (e.g., API clients)
		{"POST no origin from local", http.MethodPost, "127.0.0.1:12345", "localhost:8889", "", "", http.StatusOK},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req := httptest.NewRequest(tt.method, "/api/test", nil)
			req.RemoteAddr = tt.remoteAddr
			req.Host = tt.host

			if tt.origin != "" {
				req.Header.Set("Origin", tt.origin)
			}
			if tt.referer != "" {
				req.Header.Set("Referer", tt.referer)
			}

			rr := httptest.NewRecorder()

			handler := cmd.AuthMiddleware(func(w http.ResponseWriter, r *http.Request) {
				w.WriteHeader(http.StatusOK)
			})

			handler.ServeHTTP(rr, req)

			if rr.Code != tt.expectedStatus {
				t.Errorf("Expected status %d, got %d for Origin=%q, Referer=%q",
					tt.expectedStatus, rr.Code, tt.origin, tt.referer)
			}
		})
	}
}

func TestAuthMiddleware_TokenValidation(t *testing.T) {
	cmd := &commands.ServeCmd{
		APIToken: "test-token-12345",
	}

	tests := []struct {
		name           string
		remoteAddr     string
		host           string
		token          string
		expectedStatus int
	}{
		{"Valid token", "127.0.0.1:12345", "localhost:8889", "test-token-12345", http.StatusOK},
		{
			"Invalid token from remote",
			"192.168.1.100:12345",
			"example.com:8889",
			"wrong-token",
			http.StatusUnauthorized,
		},
		{"No token from remote", "192.168.1.100:12345", "example.com:8889", "", http.StatusUnauthorized},
		{"Local request without token", "127.0.0.1:12345", "localhost:8889", "", http.StatusOK},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req := httptest.NewRequest(http.MethodGet, "/api/test", nil)
			req.RemoteAddr = tt.remoteAddr
			req.Host = tt.host
			if tt.token != "" {
				req.Header.Set("X-Syncweb-Token", tt.token)
			}

			rr := httptest.NewRecorder()

			handler := cmd.AuthMiddleware(func(w http.ResponseWriter, r *http.Request) {
				w.WriteHeader(http.StatusOK)
			})

			handler.ServeHTTP(rr, req)

			if rr.Code != tt.expectedStatus {
				t.Errorf("Expected status %d, got %d for token=%q, remote=%q",
					tt.expectedStatus, rr.Code, tt.token, tt.remoteAddr)
			}
		})
	}
}

package maintenance_response_test

import (
	"context"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"testing"

	plugin "github.com/jannschu/maintenance-response"
)

func TestFilter(t *testing.T) {
	tests := []struct {
		name   string
		filter string
		query  string
		match  bool
	}{
		{
			name:   "fix",
			filter: "example.com",
			query:  "https://example.com:8000/path?query=param",
			match:  true,
		},
		{
			name:   "fix-subdomain",
			filter: "example.com",
			query:  "https://sub.example.com/path?query=param",
			match:  true,
		},
		{
			name:   "fix-suffix",
			filter: "example.com",
			query:  "https://someexample.com/path?query=param",
			match:  false,
		},
		{
			name:   "subdomain",
			filter: "sub.example.com",
			query:  "https://sub.example.com/path?query=param",
			match:  true,
		},
		{
			name:   "subdomain-negative",
			filter: "foo.example.com",
			query:  "https://bar.example.com/path?query=param",
			match:  false,
		},
		// {
		// 	name:   "path-prefix",
		// 	filter: "/path",
		// 	query:  "https://example.com/path?query=param",
		// 	match:  true,
		// },
		// {
		// 	name:   "path-prefix-slash",
		// 	filter: "/path",
		// 	query:  "https://example.com/path/?query=param",
		// 	match:  true,
		// },
		// {
		// 	name:   "path-prefix-unclean",
		// 	filter: "/path",
		// 	query:  "https://example.com/foo/../path/",
		// 	match:  true,
		// },
		// {
		// 	name:   "path-prefix2",
		// 	filter: "/path",
		// 	query:  "https://example.com/pathh?query=param",
		// 	match:  false,
		// },
		// {
		// 	name:   "path-prefix3",
		// 	filter: "/path",
		// 	query:  "https://example.com/path/sub?query=param",
		// 	match:  false,
		// },
		// {
		// 	name:   "path-prefix-glob",
		// 	filter: "/path/**",
		// 	query:  "https://example.com/path?query=param",
		// 	match:  true,
		// },
		// {
		// 	name:   "path-prefix-glob2",
		// 	filter: "/path/**",
		// 	query:  "https://example.com/path/foo/bar?query=param",
		// 	match:  true,
		// },
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			config := plugin.CreateConfig()
			config.Enabled = true
			config.QueryFilter = []string{tt.filter}

			ctx := context.Background()
			next := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				w.WriteHeader(http.StatusOK)
				fmt.Fprintln(w, "Next handler")
			})

			handler, err := plugin.New(ctx, next, config, "test")
			if err != nil {
				t.Fatalf("Failed to create handler: %v", err)
			}

			req, err := http.NewRequest("GET", tt.query, nil)
			req.Header.Set("Accept", "*/*")
			if err != nil {
				t.Fatalf("Failed to create request: %v", err)
			}

			rr := httptest.NewRecorder()
			handler.ServeHTTP(rr, req)

			if tt.match && rr.Code != http.StatusServiceUnavailable {
				t.Errorf("Expected status %d but got %d", http.StatusServiceUnavailable, rr.Code)
			} else if !tt.match && rr.Code == http.StatusServiceUnavailable {
				t.Errorf("Expected status not to be %d but got %d", http.StatusServiceUnavailable, rr.Code)
			}
		})
	}
}

func TestAcceptNegotiation(t *testing.T) {
	tmpDir := t.TempDir()

	if err := os.WriteFile(tmpDir+"/maintenance.html", []byte("<html></html>"), 0644); err != nil {
		t.Fatalf("Failed to create maintenance file: %v", err)
	}
	if err := os.WriteFile(tmpDir+"/maintenance.json", []byte("{}"), 0644); err != nil {
		t.Fatalf("Failed to create maintenance file: %v", err)
	}

	config := plugin.CreateConfig()
	config.Enabled = true
	config.Content = tmpDir + "/maintenance.*"

	ctx := context.Background()
	next := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		fmt.Fprintln(w, "Next handler")
	})

	handler, err := plugin.New(ctx, next, config, "test")
	if err != nil {
		t.Fatalf("Failed to create handler: %v", err)
	}

	req, err := http.NewRequest("GET", "/", nil)
	req.Header.Set("Accept", "text/html,application/json;q=0.9,*/*;q=0.8")
	if err != nil {
		t.Fatalf("Failed to create request: %v", err)
	}

	rr := httptest.NewRecorder()
	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusServiceUnavailable {
		t.Errorf("Expected status %d but got %d", http.StatusServiceUnavailable, rr.Code)
	}

	contentType := rr.Header().Get("Content-Type")
	if contentType != "text/html" {
		t.Errorf("Expected Content-Type 'text/html' but got '%s'", contentType)
	}

	if rr.Body.String() != "<html></html>" {
		t.Errorf("Expected body '<html></html>' but got '%s'", rr.Body.String())
	}
}

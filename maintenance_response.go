package maintenance_response

import (
	"context"
	"log"
	"net/http"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"time"

	"github.com/elnormous/contenttype"
	"github.com/h2non/filetype"
)

type Config struct {
	Enabled     bool     `json:"enabled"`
	Content     string   `json:"content"`
	QueryFilter []string `json:"query_filter"`
}

type MaintenanceCheck struct {
	config         *Config
	mimeMap        map[string]string
	availableTypes []contenttype.MediaType
	next           http.Handler
	filter         []Filter
}

func CreateConfig() *Config {
	return &Config{
		Enabled:     false,
		Content:     "",
		QueryFilter: []string{},
	}
}

var maintenanceStatusCode = http.StatusServiceUnavailable

func New(ctx context.Context, next http.Handler, config *Config, name string) (http.Handler, error) {
	if !config.Enabled {
		return next, nil
	}

	mimeMapRaw, err := getMimeMap(config.Content)
	if err != nil {
		log.Printf("Error getting mime map: %v", err)
		return next, nil
	}

	availableTypes := make([]contenttype.MediaType, 0, len(mimeMapRaw))
	mimeMap := make(map[string]string, len(mimeMapRaw))
	for mimeType, filePath := range mimeMapRaw {
		mediaType := contenttype.NewMediaType(mimeType)
		availableTypes = append(availableTypes, mediaType)
		mimeMap[mediaType.String()] = filePath
	}

	filter, err := getFilter(config.QueryFilter)
	if err != nil {
		log.Printf("Error creating filter: %v", err)
		return next, nil
	}

	m := &MaintenanceCheck{
		config:         config,
		mimeMap:        mimeMap,
		availableTypes: availableTypes,
		next:           next,
		filter:         filter,
	}

	return m, nil
}

type Filter struct {
	host string
}

func (f *Filter) Match(req *http.Request) bool {
	if f.host != "" {
		host := req.Host
		// Remove port number if present
		if colon := strings.IndexByte(host, ':'); colon != -1 {
			host = host[:colon]
		}
		if host != f.host && !strings.HasSuffix(host, "."+f.host) {
			return false
		}
	}
	return true
}

func (f *Filter) AlwaysFulfilled() bool {
	return f.host == ""
}

func GetFilter(pattern string) (*Filter, error) {
	var hostPattern, pathPattern string
	if strings.HasPrefix(pattern, "/") {
		pathPattern = pattern
	} else {
		if idx := strings.Index(pattern, "/"); idx != -1 {
			hostPattern = pattern[:idx]
			pathPattern = pattern[idx:]
		} else {
			hostPattern = pattern
		}
	}

	var host string
	if pathPattern != "" {
		log.Printf("Path pattern detected: %s, which is not implemented", pathPattern)
	}
	if hostPattern != "" {
		if strings.Contains(hostPattern, "*") {
			log.Printf("Wildcard in host pattern: %s, not implemented", hostPattern)
		}
		re := regexp.MustCompile(`^https?://`)
		host = re.ReplaceAllString(hostPattern, "")
	}
	return &Filter{
		host: host,
	}, nil
}

func getFilter(queryFilter []string) ([]Filter, error) {
	patterns := make([]Filter, 0, len(queryFilter))
	for _, pattern := range queryFilter {
		filter, err := GetFilter(pattern)
		if err != nil {
			log.Printf("Error creating filter for pattern %s: %v", pattern, err)
			continue
		}
		if !filter.AlwaysFulfilled() {
			patterns = append(patterns, *filter)
		}
	}
	return patterns, nil
}

func getMimeMap(contentPath string) (map[string]string, error) {
	matches, err := filepath.Glob(contentPath)
	if err != nil {
		return nil, err
	}
	mimeMap := make(map[string]string)
	for _, match := range matches {
		f, err := os.Stat(match)
		if err != nil || f.IsDir() {
			log.Printf("Skipping %s: %v", match, err)
			continue
		}
		var mime string

		ext := strings.ToLower(filepath.Ext(match))
		switch ext {
		case ".html":
			mime = "text/html"
		case ".json":
			mime = "application/json"
		default:
			mimeType, err := filetype.MatchFile(match)
			if err != nil {
				log.Printf("Error detecting mime type for %s: %v", match, err)
				continue
			}
			if mimeType != filetype.Unknown {
				mime = mimeType.MIME.Type + "/" + mimeType.MIME.Subtype
			}
		}

		if mime == "" {
			log.Printf("Could not determine mime type for file %s", match)
			continue
		}

		if _, exists := mimeMap[mime]; exists {
			log.Printf("Warning: duplicate mime type %s for file %s", mime, match)
		}
		log.Printf("Adding mime type %s for file %s", mime, match)
		mimeMap[mime] = match
	}
	return mimeMap, nil
}

func (m *MaintenanceCheck) ServeHTTP(rw http.ResponseWriter, req *http.Request) {
	if len(m.filter) > 0 {
		matched := false
		for _, f := range m.filter {
			if f.Match(req) {
				matched = true
				break
			}
		}
		if !matched {
			m.next.ServeHTTP(rw, req)
			return
		}
	}
	if len(m.availableTypes) == 0 {
		fallback(rw)
		return
	}
	accepted, _, err := contenttype.GetAcceptableMediaType(req, m.availableTypes)
	if err != nil {
		log.Printf("Error getting acceptable media type: %v", err)
		fallback(rw)
		return
	}
	path, ok := m.mimeMap[accepted.String()]
	if !ok {
		log.Printf("No media type found for %s", accepted)
		fallback(rw)
		return
	}

	rw.Header().Set("Content-Type", accepted.String())
	rw.WriteHeader(maintenanceStatusCode)
	file, err := os.Open(path)
	if err != nil {
		log.Printf("Error opening file %s: %v", path, err)
		fallback(rw)
		return
	}
	defer file.Close()
	stat, err := file.Stat()
	var modTime time.Time
	if err != nil {
		log.Printf("Error getting file stat for %s: %v", path, err)
		modTime = time.Time{}
	} else {
		modTime = stat.ModTime()
	}
	http.ServeContent(rw, req, "", modTime, file)
}

func fallback(rw http.ResponseWriter) {
	rw.Header().Set("Content-Type", "text/plain; charset=utf-8")
	rw.WriteHeader(maintenanceStatusCode)
	_, err := rw.Write([]byte("Service is in maintenance mode"))
	if err != nil {
		log.Printf("Error writing response: %v", err)
	}
}

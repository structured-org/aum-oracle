package main

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"strings"
	"time"

	"go.uber.org/zap"
)

type lastStateEntry struct {
	Present    bool       `json:"present"`
	CapturedAt *time.Time `json:"captured_at,omitempty"`
	Data       any        `json:"data,omitempty"`
}

type lastStateResponse struct {
	Mode     string                    `json:"mode"`
	Now      time.Time                 `json:"now"`
	Networks map[string]lastStateEntry `json:"networks"`
}

func normalizeListenAddr(input string) (string, error) {
	in := strings.TrimSpace(input)
	if in == "" {
		return "", fmt.Errorf("empty rest-laddr")
	}

	if strings.Contains(in, "://") {
		u, err := url.Parse(in)
		if err != nil {
			return "", fmt.Errorf("invalid url %q: %w", in, err)
		}
		if u.Host == "" {
			return "", fmt.Errorf("invalid url %q: missing host", in)
		}
		return u.Host, nil
	}

	// Assume host:port
	return in, nil
}

func startRestServer(
	ctx context.Context,
	logger *zap.Logger,
	restAddr string,
	getState func() lastStateResponse,
) {
	addr, err := normalizeListenAddr(restAddr)
	if err != nil {
		logger.Error("invalid rest listen address", zap.String("rest_laddr", restAddr), zap.Error(err))
		return
	}

	h := newRestMux(getState)

	srv := &http.Server{
		Addr:              addr,
		Handler:           h,
		ReadHeaderTimeout: 5 * time.Second,
	}

	go func() {
		<-ctx.Done()
		shutdownCtx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
		defer cancel()
		_ = srv.Shutdown(shutdownCtx)
	}()

	go func() {
		logger.Info("REST server started", zap.String("listen", addr))
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			logger.Error("REST server stopped unexpectedly", zap.Error(err))
		}
	}()
}

func newRestMux(getState func() lastStateResponse) http.Handler {
	mux := http.NewServeMux()

	withCORS := func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			w.Header().Set("Access-Control-Allow-Origin", "*")
			w.Header().Set("Access-Control-Allow-Methods", "GET, OPTIONS")
			w.Header().Set("Access-Control-Allow-Headers", "Content-Type")
			w.Header().Set("Access-Control-Max-Age", "600")

			if r.Method == http.MethodOptions {
				w.WriteHeader(http.StatusNoContent)
				return
			}
			next.ServeHTTP(w, r)
		})
	}

	handle := func(pattern string, handler http.Handler) {
		mux.Handle(pattern, withCORS(handler))
	}

	handleFunc := func(pattern string, handler func(http.ResponseWriter, *http.Request)) {
		handle(pattern, http.HandlerFunc(handler))
	}

	handleFunc("/health", func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "text/plain; charset=utf-8")
		_, _ = w.Write([]byte("ok"))
	})

	lastHandler := func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "application/json; charset=utf-8")
		enc := json.NewEncoder(w)
		_ = enc.Encode(getState())
	}
	handleFunc("/last", lastHandler)

	return mux
}

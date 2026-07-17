package main

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/downMark/github-profile/app/todo-service/internal/auth"
	"github.com/downMark/github-profile/app/todo-service/internal/config"
	"github.com/downMark/github-profile/app/todo-service/internal/httpapi"
	"github.com/downMark/github-profile/app/todo-service/internal/postgres"
	"github.com/downMark/github-profile/app/todo-service/internal/profileclient"
	"github.com/downMark/github-profile/app/todo-service/internal/service"
)

func main() {
	logger := slog.New(slog.NewJSONHandler(os.Stdout, nil))
	if err := run(logger); err != nil {
		logger.Error("todo service stopped", "error", err)
		os.Exit(1)
	}
}

func run(logger *slog.Logger) error {
	config, err := config.Load()
	if err != nil {
		return err
	}
	ctx, cancel := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer cancel()

	pool, err := postgres.Open(ctx, config.DatabaseURL, config.DatabaseSchema, config.DatabaseMaxConnections)
	if err != nil {
		return err
	}
	defer pool.Close()
	if err := postgres.Migrate(ctx, pool); err != nil {
		return fmt.Errorf("migrate todo database: %w", err)
	}

	profiles, err := profileclient.New(config.ProfileGRPCAddress)
	if err != nil {
		return err
	}
	defer profiles.Close()

	todos := service.New(profiles, postgres.NewRepository(pool))
	verifier := auth.New(config.AuthIssuer, config.AuthAudience, config.AuthJWKSURL)
	handler := httpapi.New(todos, verifier, logger, config.AllowedOrigin, config.APIBasePath)
	server := &http.Server{
		Addr:              fmt.Sprintf(":%d", config.Port),
		Handler:           handler,
		ReadHeaderTimeout: 5 * time.Second,
		IdleTimeout:       60 * time.Second,
	}

	serverErrors := make(chan error, 1)
	go func() {
		logger.Info("todo service listening", "address", server.Addr, "profile_grpc", config.ProfileGRPCAddress)
		serverErrors <- server.ListenAndServe()
	}()

	select {
	case <-ctx.Done():
		shutdownContext, shutdownCancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer shutdownCancel()
		return server.Shutdown(shutdownContext)
	case err := <-serverErrors:
		if errors.Is(err, http.ErrServerClosed) {
			return nil
		}
		return err
	}
}

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

	awsconfig "github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/service/sns"
	"github.com/aws/aws-sdk-go-v2/service/sqs"
	"github.com/downMark/github-profile/app/todo-service/internal/auth"
	"github.com/downMark/github-profile/app/todo-service/internal/config"
	"github.com/downMark/github-profile/app/todo-service/internal/httpapi"
	"github.com/downMark/github-profile/app/todo-service/internal/messaging"
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

	repository := postgres.NewRepositoryWithEnvironment(pool, config.DeployEnvironment)
	todos := service.New(profiles, repository)
	if config.TodoEventsTopicARN != "" {
		awsConfiguration, err := awsconfig.LoadDefaultConfig(ctx)
		if err != nil {
			return fmt.Errorf("load AWS messaging configuration: %w", err)
		}
		go messaging.NewPublisher(repository, sns.NewFromConfig(awsConfiguration), config.TodoEventsTopicARN, logger).Run(ctx)
		go messaging.NewConsumer(repository, sqs.NewFromConfig(awsConfiguration), config.TodoEventsQueueURL, logger).Run(ctx)
		logger.Info("todo event messaging enabled")
	}
	verifier := auth.New(config.AuthIssuer, config.AuthAudience, config.AuthJWKSURL)
	handler := httpapi.NewWithDeployment(
		todos, verifier, logger, config.AllowedOrigin, config.APIBasePath,
		config.DeployEnvironment, config.ServiceRevision,
	)
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

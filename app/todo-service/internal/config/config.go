package config

import (
	"fmt"
	"net/url"
	"os"
	"strconv"
	"strings"
)

type Config struct {
	Port                   uint16
	DatabaseURL            string
	DatabaseSchema         string
	DatabaseMaxConnections int32
	AllowedOrigin          string
	APIBasePath            string
	ProfileGRPCAddress     string
	AuthIssuer             string
	AuthAudience           string
	AuthJWKSURL            string
	DeployEnvironment      string
	ServiceRevision        string
	TodoEventsTopicARN     string
	TodoEventsQueueURL     string
}

func Load() (Config, error) {
	port, err := uint16Value("PORT", 3001)
	if err != nil {
		return Config{}, err
	}
	maxConnections, err := int32Value("DATABASE_MAX_CONNECTIONS", 5)
	if err != nil || maxConnections < 1 {
		return Config{}, fmt.Errorf("DATABASE_MAX_CONNECTIONS must be a positive integer")
	}

	schema := os.Getenv("DB_SCHEMA")
	if schema != "" && !validSchema(schema) {
		return Config{}, fmt.Errorf("DB_SCHEMA must be prod, staging or pr_<number>")
	}
	basePath := os.Getenv("API_BASE_PATH")
	if basePath != "" && (!strings.HasPrefix(basePath, "/") || strings.HasSuffix(basePath, "/")) {
		return Config{}, fmt.Errorf("API_BASE_PATH must be empty or start with / and must not end with /")
	}

	databaseURL, err := databaseURLFromEnv()
	if err != nil {
		return Config{}, err
	}
	topicARN := os.Getenv("TODO_EVENTS_TOPIC_ARN")
	queueURL := os.Getenv("TODO_EVENTS_QUEUE_URL")
	if (topicARN == "") != (queueURL == "") {
		return Config{}, fmt.Errorf("TODO_EVENTS_TOPIC_ARN and TODO_EVENTS_QUEUE_URL must be configured together")
	}
	return Config{
		Port:                   port,
		DatabaseURL:            databaseURL,
		DatabaseSchema:         schema,
		DatabaseMaxConnections: maxConnections,
		AllowedOrigin:          envOr("ALLOWED_ORIGIN", "http://localhost:5173"),
		APIBasePath:            basePath,
		ProfileGRPCAddress:     envOr("PROFILE_GRPC_ADDR", "localhost:50051"),
		AuthIssuer:             envOr("AUTH_ISSUER", "http://localhost:3002"),
		AuthAudience:           envOr("AUTH_AUDIENCE", "github-profile"),
		AuthJWKSURL:            envOr("AUTH_JWKS_URL", "http://localhost:3002/.well-known/jwks.json"),
		DeployEnvironment:      envOr("DEPLOY_ENVIRONMENT", "local"),
		ServiceRevision:        envOr("SERVICE_REVISION", "development"),
		TodoEventsTopicARN:     topicARN,
		TodoEventsQueueURL:     queueURL,
	}, nil
}

func databaseURLFromEnv() (string, error) {
	if value := os.Getenv("DATABASE_URL"); value != "" {
		return value, nil
	}
	host, err := required("DB_HOST")
	if err != nil {
		return "", err
	}
	username, err := required("DB_USERNAME")
	if err != nil {
		return "", err
	}
	password, err := required("DB_PASSWORD")
	if err != nil {
		return "", err
	}
	port := envOr("DB_PORT", "5432")
	database := envOr("DB_NAME", "postgres")
	sslMode := envOr("DB_SSL_MODE", "require")
	u := &url.URL{
		Scheme: "postgres",
		User:   url.UserPassword(username, password),
		Host:   host + ":" + port,
		Path:   database,
	}
	query := u.Query()
	query.Set("sslmode", sslMode)
	u.RawQuery = query.Encode()
	return u.String(), nil
}

func uint16Value(name string, fallback uint16) (uint16, error) {
	value := os.Getenv(name)
	if value == "" {
		return fallback, nil
	}
	number, err := strconv.ParseUint(value, 10, 16)
	if err != nil {
		return 0, fmt.Errorf("%s must be a valid port", name)
	}
	return uint16(number), nil
}

func int32Value(name string, fallback int32) (int32, error) {
	value := os.Getenv(name)
	if value == "" {
		return fallback, nil
	}
	number, err := strconv.ParseInt(value, 10, 32)
	if err != nil {
		return 0, err
	}
	return int32(number), nil
}

func required(name string) (string, error) {
	value := os.Getenv(name)
	if value == "" {
		return "", fmt.Errorf("environment variable %s must be set", name)
	}
	return value, nil
}

func envOr(name, fallback string) string {
	if value := os.Getenv(name); value != "" {
		return value
	}
	return fallback
}

func validPRSchema(schema string) bool {
	number := strings.TrimPrefix(schema, "pr_")
	if number == schema || number == "" {
		return false
	}
	for _, char := range number {
		if char < '0' || char > '9' {
			return false
		}
	}
	return true
}

func validSchema(schema string) bool {
	return schema == "prod" || schema == "staging" || validPRSchema(schema)
}

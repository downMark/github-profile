package config

import "testing"

func TestValidPRSchema(t *testing.T) {
	t.Parallel()
	tests := map[string]bool{
		"pr_123": true,
		"pr_":    false,
		"public": false,
		"pr_1;x": false,
	}
	for value, expected := range tests {
		if actual := validPRSchema(value); actual != expected {
			t.Fatalf("validPRSchema(%q) = %v, want %v", value, actual, expected)
		}
	}
}

func TestMessagingConfigurationRequiresTopicAndQueue(t *testing.T) {
	t.Setenv("DATABASE_URL", "postgres://test:test@localhost/test")
	t.Setenv("TODO_EVENTS_TOPIC_ARN", "arn:aws:sns:us-east-1:123456789012:todo")
	t.Setenv("TODO_EVENTS_QUEUE_URL", "")
	if _, err := Load(); err == nil {
		t.Fatal("Load accepted a topic without a queue")
	}
}

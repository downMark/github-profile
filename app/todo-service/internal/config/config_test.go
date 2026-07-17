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

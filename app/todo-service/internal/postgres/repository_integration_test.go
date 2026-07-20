package postgres

import (
	"context"
	"errors"
	"fmt"
	"os"
	"testing"
	"time"

	"github.com/downMark/github-profile/app/todo-service/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

func TestRepositoryIntegration(t *testing.T) {
	databaseURL := os.Getenv("TEST_DATABASE_URL")
	if databaseURL == "" {
		t.Skip("set TEST_DATABASE_URL to run PostgreSQL integration tests")
	}

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	schema := fmt.Sprintf("pr_%d", time.Now().UnixNano())

	pool, err := Open(ctx, databaseURL, schema, 2)
	if err != nil {
		t.Fatalf("open database: %v", err)
	}
	t.Cleanup(func() {
		pool.Close()
		admin, adminErr := pgxpool.New(context.Background(), databaseURL)
		if adminErr == nil {
			defer admin.Close()
			_, _ = admin.Exec(context.Background(), `DROP SCHEMA IF EXISTS `+schema+` CASCADE`)
		}
	})

	// Simulate Rust SQLx metadata in the same schema before applying Todo migrations.
	if _, err := pool.Exec(ctx, `CREATE TABLE _sqlx_migrations (version BIGINT PRIMARY KEY)`); err != nil {
		t.Fatalf("create SQLx metadata table: %v", err)
	}
	if err := Migrate(ctx, pool); err != nil {
		t.Fatalf("migrate: %v", err)
	}

	var sqlxTable, todoMigrationTable, todosTable bool
	if err := pool.QueryRow(ctx, `
		SELECT
			to_regclass('_sqlx_migrations') IS NOT NULL,
			to_regclass('todo_schema_migrations') IS NOT NULL,
			to_regclass('todos') IS NOT NULL`).Scan(&sqlxTable, &todoMigrationTable, &todosTable); err != nil {
		t.Fatalf("inspect tables: %v", err)
	}
	if !sqlxTable || !todoMigrationTable || !todosTable {
		t.Fatalf("expected SQLx, Todo migration, and Todo tables to coexist")
	}

	repository := NewRepository(pool)
	userA := uuid.New()
	userB := uuid.New()
	description := "first description"
	created, err := repository.Create(ctx, userA, domain.CreateInput{Title: "first", Description: &description})
	if err != nil {
		t.Fatalf("create todo: %v", err)
	}
	if _, err := repository.Create(ctx, userA, domain.CreateInput{Title: "second"}); err != nil {
		t.Fatalf("create second todo: %v", err)
	}
	if _, err := repository.Create(ctx, userB, domain.CreateInput{Title: "other user"}); err != nil {
		t.Fatalf("create isolated todo: %v", err)
	}

	page, err := repository.List(ctx, userA, 1, 1)
	if err != nil {
		t.Fatalf("list todos: %v", err)
	}
	if page.Total != 2 || len(page.Items) != 1 || page.Page != 1 || page.Limit != 1 {
		t.Fatalf("unexpected page: %#v", page)
	}
	if _, err := repository.Get(ctx, userB, created.ID); !errors.Is(err, domain.ErrTodoNotFound) {
		t.Fatalf("cross-user read must return not found, got %v", err)
	}

	completed := true
	updated, err := repository.Update(ctx, userA, created.ID, domain.UpdateInput{DescriptionSet: true, Completed: &completed})
	if err != nil {
		t.Fatalf("update todo: %v", err)
	}
	if !updated.Completed || updated.Description != nil {
		t.Fatalf("PATCH null semantics not persisted: %#v", updated)
	}
	if err := repository.Delete(ctx, userA, created.ID); err != nil {
		t.Fatalf("delete todo: %v", err)
	}
	if _, err := repository.Get(ctx, userA, created.ID); !errors.Is(err, domain.ErrTodoNotFound) {
		t.Fatalf("deleted todo must return not found, got %v", err)
	}
	events, err := repository.ClaimOutbox(ctx, "integration-test", 20)
	if err != nil {
		t.Fatalf("claim outbox: %v", err)
	}
	if len(events) != 5 {
		t.Fatalf("outbox event count = %d, want 5", len(events))
	}
	if err := repository.RecordAudit(ctx, events[0]); err != nil {
		t.Fatalf("record audit: %v", err)
	}
	if err := repository.RecordAudit(ctx, events[0]); err != nil {
		t.Fatalf("record duplicate audit: %v", err)
	}
	audit, err := repository.ListAudit(ctx, events[0].GithubUserID, 1, 20)
	if err != nil {
		t.Fatalf("list audit: %v", err)
	}
	if audit.Total != 1 || len(audit.Items) != 1 {
		t.Fatalf("audit idempotency failed: %#v", audit)
	}
}

package postgres

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"time"

	"github.com/downMark/github-profile/app/todo-service/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

type Repository struct {
	pool        *pgxpool.Pool
	environment string
}

func NewRepository(pool *pgxpool.Pool) *Repository {
	return NewRepositoryWithEnvironment(pool, "local")
}

func NewRepositoryWithEnvironment(pool *pgxpool.Pool, environment string) *Repository {
	return &Repository{pool: pool, environment: environment}
}

func (r *Repository) Create(ctx context.Context, userID uuid.UUID, input domain.CreateInput) (domain.Todo, error) {
	tx, err := r.pool.Begin(ctx)
	if err != nil {
		return domain.Todo{}, err
	}
	defer tx.Rollback(ctx)
	item, err := scanTodo(tx.QueryRow(ctx, `
        INSERT INTO todos (id, github_user_id, title, description)
        VALUES ($1, $2, $3, $4)
        RETURNING id, github_user_id, title, description, completed, created_at, updated_at`,
		uuid.New(), userID, input.Title, input.Description))
	if err != nil {
		return domain.Todo{}, err
	}
	if err := r.insertEvent(ctx, tx, "todo.created", item); err != nil {
		return domain.Todo{}, err
	}
	return item, tx.Commit(ctx)
}

func (r *Repository) List(ctx context.Context, userID uuid.UUID, page, limit uint32) (domain.ListResult, error) {
	var total int64
	if err := r.pool.QueryRow(ctx, "SELECT COUNT(*) FROM todos WHERE github_user_id=$1", userID).Scan(&total); err != nil {
		return domain.ListResult{}, err
	}
	rows, err := r.pool.Query(ctx, `
        SELECT id, github_user_id, title, description, completed, created_at, updated_at
        FROM todos WHERE github_user_id=$1
        ORDER BY updated_at DESC, id DESC LIMIT $2 OFFSET $3`,
		userID, limit, int64(page-1)*int64(limit))
	if err != nil {
		return domain.ListResult{}, err
	}
	defer rows.Close()
	items := make([]domain.Todo, 0)
	for rows.Next() {
		item, err := scanTodo(rows)
		if err != nil {
			return domain.ListResult{}, err
		}
		items = append(items, item)
	}
	if err := rows.Err(); err != nil {
		return domain.ListResult{}, err
	}
	return domain.ListResult{Items: items, Total: total, Page: page, Limit: limit}, nil
}

func (r *Repository) Get(ctx context.Context, userID, todoID uuid.UUID) (domain.Todo, error) {
	item, err := scanTodo(r.pool.QueryRow(ctx, `
        SELECT id, github_user_id, title, description, completed, created_at, updated_at
        FROM todos WHERE id=$1 AND github_user_id=$2`, todoID, userID))
	if errors.Is(err, pgx.ErrNoRows) {
		return domain.Todo{}, domain.ErrTodoNotFound
	}
	return item, err
}

func (r *Repository) Update(ctx context.Context, userID, todoID uuid.UUID, input domain.UpdateInput) (domain.Todo, error) {
	tx, err := r.pool.Begin(ctx)
	if err != nil {
		return domain.Todo{}, err
	}
	defer tx.Rollback(ctx)
	item, err := scanTodo(tx.QueryRow(ctx, `
        UPDATE todos SET
            title = CASE WHEN $3::boolean THEN $4 ELSE title END,
            description = CASE WHEN $5::boolean THEN $6 ELSE description END,
            completed = CASE WHEN $7::boolean THEN $8 ELSE completed END,
            updated_at = NOW()
        WHERE id=$1 AND github_user_id=$2
        RETURNING id, github_user_id, title, description, completed, created_at, updated_at`,
		todoID, userID,
		input.Title != nil, input.Title,
		input.DescriptionSet, input.Description,
		input.Completed != nil, input.Completed))
	if errors.Is(err, pgx.ErrNoRows) {
		return domain.Todo{}, domain.ErrTodoNotFound
	}
	if err != nil {
		return domain.Todo{}, err
	}
	if err := r.insertEvent(ctx, tx, "todo.updated", item); err != nil {
		return domain.Todo{}, err
	}
	return item, tx.Commit(ctx)
}

func (r *Repository) Delete(ctx context.Context, userID, todoID uuid.UUID) error {
	tx, err := r.pool.Begin(ctx)
	if err != nil {
		return err
	}
	defer tx.Rollback(ctx)
	item, err := scanTodo(tx.QueryRow(ctx, `
		DELETE FROM todos WHERE id=$1 AND github_user_id=$2
		RETURNING id, github_user_id, title, description, completed, created_at, updated_at`, todoID, userID))
	if errors.Is(err, pgx.ErrNoRows) {
		return domain.ErrTodoNotFound
	}
	if err != nil {
		return err
	}
	if err := r.insertEvent(ctx, tx, "todo.deleted", item); err != nil {
		return err
	}
	return tx.Commit(ctx)
}

func (r *Repository) insertEvent(ctx context.Context, tx pgx.Tx, eventType string, item domain.Todo) error {
	payload, err := json.Marshal(item)
	if err != nil {
		return fmt.Errorf("marshal todo event: %w", err)
	}
	_, err = tx.Exec(ctx, `INSERT INTO todo_event_outbox
		(event_id, schema_version, event_type, occurred_at, environment, github_user_id, todo_id, payload)
		VALUES ($1, 1, $2, $3, $4, $5, $6, $7)`,
		uuid.New(), eventType, time.Now().UTC(), r.environment, item.GithubUserID, item.ID, payload)
	return err
}

func (r *Repository) ClaimOutbox(ctx context.Context, workerID string, limit int) ([]domain.TodoEvent, error) {
	rows, err := r.pool.Query(ctx, `WITH pending AS (
		SELECT event_id FROM todo_event_outbox
		WHERE published_at IS NULL AND available_at <= NOW()
		  AND (locked_until IS NULL OR locked_until < NOW())
		ORDER BY occurred_at LIMIT $1 FOR UPDATE SKIP LOCKED
	) UPDATE todo_event_outbox o SET locked_by=$2, locked_until=NOW()+INTERVAL '60 seconds', attempts=attempts+1
	FROM pending WHERE o.event_id=pending.event_id
	RETURNING o.event_id,o.schema_version,o.event_type,o.occurred_at,o.environment,o.github_user_id,o.todo_id,o.payload`, limit, workerID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	events := make([]domain.TodoEvent, 0)
	for rows.Next() {
		var event domain.TodoEvent
		if err := rows.Scan(&event.EventID, &event.SchemaVersion, &event.EventType, &event.OccurredAt, &event.Environment, &event.GithubUserID, &event.TodoID, &event.Todo); err != nil {
			return nil, err
		}
		events = append(events, event)
	}
	return events, rows.Err()
}

func (r *Repository) MarkPublished(ctx context.Context, eventID uuid.UUID) error {
	_, err := r.pool.Exec(ctx, `UPDATE todo_event_outbox SET published_at=NOW(),locked_by=NULL,locked_until=NULL,last_error=NULL WHERE event_id=$1`, eventID)
	return err
}

func (r *Repository) RetryOutbox(ctx context.Context, eventID uuid.UUID, cause string) error {
	_, err := r.pool.Exec(ctx, `UPDATE todo_event_outbox SET locked_by=NULL,locked_until=NULL,last_error=$2,
		available_at=NOW()+(LEAST(attempts,10)*INTERVAL '5 seconds') WHERE event_id=$1`, eventID, cause)
	return err
}

func (r *Repository) RecordAudit(ctx context.Context, event domain.TodoEvent) error {
	_, err := r.pool.Exec(ctx, `INSERT INTO todo_event_audit
		(event_id,schema_version,event_type,occurred_at,environment,github_user_id,todo_id,payload)
		VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(event_id) DO NOTHING`,
		event.EventID, event.SchemaVersion, event.EventType, event.OccurredAt, event.Environment, event.GithubUserID, event.TodoID, event.Todo)
	return err
}

func (r *Repository) ListAudit(ctx context.Context, userID uuid.UUID, page, limit uint32) (domain.EventAuditListResult, error) {
	var result domain.EventAuditListResult
	result.Page, result.Limit = page, limit
	if err := r.pool.QueryRow(ctx, `SELECT COUNT(*) FROM todo_event_audit WHERE github_user_id=$1`, userID).Scan(&result.Total); err != nil {
		return result, err
	}
	rows, err := r.pool.Query(ctx, `SELECT event_id,schema_version,event_type,occurred_at,processed_at,environment,github_user_id,todo_id,payload
		FROM todo_event_audit WHERE github_user_id=$1 ORDER BY occurred_at DESC,event_id DESC LIMIT $2 OFFSET $3`, userID, limit, int64(page-1)*int64(limit))
	if err != nil {
		return result, err
	}
	defer rows.Close()
	result.Items = make([]domain.TodoEventAudit, 0)
	for rows.Next() {
		var item domain.TodoEventAudit
		if err := rows.Scan(&item.EventID, &item.SchemaVersion, &item.EventType, &item.OccurredAt, &item.ProcessedAt, &item.Environment, &item.GithubUserID, &item.TodoID, &item.Todo); err != nil {
			return result, err
		}
		result.Items = append(result.Items, item)
	}
	return result, rows.Err()
}

type todoScanner interface {
	Scan(dest ...any) error
}

func scanTodo(row todoScanner) (domain.Todo, error) {
	var item domain.Todo
	err := row.Scan(&item.ID, &item.GithubUserID, &item.Title, &item.Description, &item.Completed, &item.CreatedAt, &item.UpdatedAt)
	return item, err
}

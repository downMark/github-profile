package postgres

import (
	"context"
	"errors"

	"github.com/downMark/github-profile/app/todo-service/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

type Repository struct {
	pool *pgxpool.Pool
}

func NewRepository(pool *pgxpool.Pool) *Repository {
	return &Repository{pool: pool}
}

func (r *Repository) Create(ctx context.Context, userID uuid.UUID, input domain.CreateInput) (domain.Todo, error) {
	return scanTodo(r.pool.QueryRow(ctx, `
        INSERT INTO todos (id, github_user_id, title, description)
        VALUES ($1, $2, $3, $4)
        RETURNING id, github_user_id, title, description, completed, created_at, updated_at`,
		uuid.New(), userID, input.Title, input.Description))
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
	item, err := scanTodo(r.pool.QueryRow(ctx, `
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
	return item, err
}

func (r *Repository) Delete(ctx context.Context, userID, todoID uuid.UUID) error {
	result, err := r.pool.Exec(ctx, "DELETE FROM todos WHERE id=$1 AND github_user_id=$2", todoID, userID)
	if err != nil {
		return err
	}
	if result.RowsAffected() == 0 {
		return domain.ErrTodoNotFound
	}
	return nil
}

type todoScanner interface {
	Scan(dest ...any) error
}

func scanTodo(row todoScanner) (domain.Todo, error) {
	var item domain.Todo
	err := row.Scan(&item.ID, &item.GithubUserID, &item.Title, &item.Description, &item.Completed, &item.CreatedAt, &item.UpdatedAt)
	return item, err
}

package domain

import (
	"errors"
	"time"

	"github.com/google/uuid"
)

var (
	ErrInvalidInput       = errors.New("invalid input")
	ErrUserNotFound       = errors.New("user not found")
	ErrTodoNotFound       = errors.New("todo not found")
	ErrProfileUnavailable = errors.New("profile service unavailable")
	ErrUnauthorized       = errors.New("unauthorized")
	ErrAuthUnavailable    = errors.New("authentication service unavailable")
)

type Todo struct {
	ID           uuid.UUID `json:"id"`
	GithubUserID uuid.UUID `json:"github_user_id"`
	Title        string    `json:"title"`
	Description  *string   `json:"description"`
	Completed    bool      `json:"completed"`
	CreatedAt    time.Time `json:"created_at"`
	UpdatedAt    time.Time `json:"updated_at"`
}

type ListResult struct {
	Items []Todo `json:"items"`
	Total int64  `json:"total"`
	Page  uint32 `json:"page"`
	Limit uint32 `json:"limit"`
}

type CreateInput struct {
	Title       string
	Description *string
}

type UpdateInput struct {
	Title          *string
	DescriptionSet bool
	Description    *string
	Completed      *bool
}

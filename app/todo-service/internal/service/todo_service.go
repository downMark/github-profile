package service

import (
	"context"
	"fmt"
	"strings"

	"github.com/downMark/github-profile/app/todo-service/internal/domain"
	"github.com/google/uuid"
)

type ProfileValidator interface {
	AuthorizeGithubUser(context.Context, uuid.UUID) error
}

type TodoRepository interface {
	Create(context.Context, uuid.UUID, domain.CreateInput) (domain.Todo, error)
	List(context.Context, uuid.UUID, uint32, uint32) (domain.ListResult, error)
	Get(context.Context, uuid.UUID, uuid.UUID) (domain.Todo, error)
	Update(context.Context, uuid.UUID, uuid.UUID, domain.UpdateInput) (domain.Todo, error)
	Delete(context.Context, uuid.UUID, uuid.UUID) error
	ListAudit(context.Context, uuid.UUID, uint32, uint32) (domain.EventAuditListResult, error)
}

func (s *TodoService) ListAudit(ctx context.Context, userID uuid.UUID, page, limit uint32) (domain.EventAuditListResult, error) {
	if page == 0 || limit == 0 || limit > 100 {
		return domain.EventAuditListResult{}, fmt.Errorf("%w: page must be >= 1 and limit must be between 1 and 100", domain.ErrInvalidInput)
	}
	if err := s.profiles.AuthorizeGithubUser(ctx, userID); err != nil {
		return domain.EventAuditListResult{}, err
	}
	return s.todos.ListAudit(ctx, userID, page, limit)
}

type TodoService struct {
	profiles ProfileValidator
	todos    TodoRepository
}

func New(profiles ProfileValidator, todos TodoRepository) *TodoService {
	return &TodoService{profiles: profiles, todos: todos}
}

func (s *TodoService) Create(ctx context.Context, userID uuid.UUID, input domain.CreateInput) (domain.Todo, error) {
	if err := validateCreate(&input); err != nil {
		return domain.Todo{}, err
	}
	if err := s.profiles.AuthorizeGithubUser(ctx, userID); err != nil {
		return domain.Todo{}, err
	}
	return s.todos.Create(ctx, userID, input)
}

func (s *TodoService) List(ctx context.Context, userID uuid.UUID, page, limit uint32) (domain.ListResult, error) {
	if page == 0 || limit == 0 || limit > 100 {
		return domain.ListResult{}, fmt.Errorf("%w: page must be >= 1 and limit must be between 1 and 100", domain.ErrInvalidInput)
	}
	if err := s.profiles.AuthorizeGithubUser(ctx, userID); err != nil {
		return domain.ListResult{}, err
	}
	return s.todos.List(ctx, userID, page, limit)
}

func (s *TodoService) Get(ctx context.Context, userID, todoID uuid.UUID) (domain.Todo, error) {
	if err := s.profiles.AuthorizeGithubUser(ctx, userID); err != nil {
		return domain.Todo{}, err
	}
	return s.todos.Get(ctx, userID, todoID)
}

func (s *TodoService) Update(ctx context.Context, userID, todoID uuid.UUID, input domain.UpdateInput) (domain.Todo, error) {
	if err := validateUpdate(&input); err != nil {
		return domain.Todo{}, err
	}
	if err := s.profiles.AuthorizeGithubUser(ctx, userID); err != nil {
		return domain.Todo{}, err
	}
	return s.todos.Update(ctx, userID, todoID, input)
}

func (s *TodoService) Delete(ctx context.Context, userID, todoID uuid.UUID) error {
	if err := s.profiles.AuthorizeGithubUser(ctx, userID); err != nil {
		return err
	}
	return s.todos.Delete(ctx, userID, todoID)
}

func validateCreate(input *domain.CreateInput) error {
	input.Title = strings.TrimSpace(input.Title)
	if input.Title == "" || len([]rune(input.Title)) > 200 {
		return fmt.Errorf("%w: title must contain between 1 and 200 characters", domain.ErrInvalidInput)
	}
	return validateDescription(input.Description)
}

func validateUpdate(input *domain.UpdateInput) error {
	if input.Title == nil && !input.DescriptionSet && input.Completed == nil {
		return fmt.Errorf("%w: at least one field must be provided", domain.ErrInvalidInput)
	}
	if input.Title != nil {
		trimmed := strings.TrimSpace(*input.Title)
		if trimmed == "" || len([]rune(trimmed)) > 200 {
			return fmt.Errorf("%w: title must contain between 1 and 200 characters", domain.ErrInvalidInput)
		}
		input.Title = &trimmed
	}
	return validateDescription(input.Description)
}

func validateDescription(description *string) error {
	if description != nil && len([]rune(*description)) > 2000 {
		return fmt.Errorf("%w: description must not exceed 2000 characters", domain.ErrInvalidInput)
	}
	return nil
}

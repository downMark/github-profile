package service

import (
	"context"
	"errors"
	"testing"

	"github.com/downMark/github-profile/app/todo-service/internal/domain"
	"github.com/google/uuid"
)

type fakeProfiles struct{ err error }

func (f fakeProfiles) AuthorizeGithubUser(context.Context, uuid.UUID) error { return f.err }

type fakeTodos struct{ createCalls int }

func (f *fakeTodos) Create(_ context.Context, userID uuid.UUID, input domain.CreateInput) (domain.Todo, error) {
	f.createCalls++
	return domain.Todo{ID: uuid.New(), GithubUserID: userID, Title: input.Title}, nil
}
func (*fakeTodos) List(context.Context, uuid.UUID, uint32, uint32) (domain.ListResult, error) {
	return domain.ListResult{}, nil
}
func (*fakeTodos) Get(context.Context, uuid.UUID, uuid.UUID) (domain.Todo, error) {
	return domain.Todo{}, nil
}
func (*fakeTodos) Update(context.Context, uuid.UUID, uuid.UUID, domain.UpdateInput) (domain.Todo, error) {
	return domain.Todo{}, nil
}
func (*fakeTodos) Delete(context.Context, uuid.UUID, uuid.UUID) error { return nil }
func (*fakeTodos) ListAudit(context.Context, uuid.UUID, uint32, uint32) (domain.EventAuditListResult, error) {
	return domain.EventAuditListResult{}, nil
}

func TestCreateValidatesProfileBeforeWriting(t *testing.T) {
	t.Parallel()
	for _, profileError := range []error{domain.ErrUserNotFound, domain.ErrProfileUnavailable} {
		profileError := profileError
		t.Run(profileError.Error(), func(t *testing.T) {
			t.Parallel()
			repo := &fakeTodos{}
			svc := New(fakeProfiles{err: profileError}, repo)
			_, err := svc.Create(context.Background(), uuid.New(), domain.CreateInput{Title: "todo"})
			if !errors.Is(err, profileError) {
				t.Fatalf("Create error = %v, want %v", err, profileError)
			}
			if repo.createCalls != 0 {
				t.Fatalf("repository called %d times, want 0", repo.createCalls)
			}
		})
	}
}

func TestUpdateAllowsNullDescription(t *testing.T) {
	t.Parallel()
	input := domain.UpdateInput{DescriptionSet: true, Description: nil}
	if err := validateUpdate(&input); err != nil {
		t.Fatalf("validateUpdate returned %v", err)
	}
}

func TestCreateTrimsTitle(t *testing.T) {
	t.Parallel()
	repo := &fakeTodos{}
	svc := New(fakeProfiles{}, repo)
	item, err := svc.Create(context.Background(), uuid.New(), domain.CreateInput{Title: "  hello  "})
	if err != nil {
		t.Fatal(err)
	}
	if item.Title != "hello" {
		t.Fatalf("title = %q, want hello", item.Title)
	}
}

package httpapi

import (
	"context"
	"encoding/json"
	"io"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/downMark/github-profile/app/todo-service/internal/domain"
	"github.com/google/uuid"
)

type fakeService struct {
	updated domain.UpdateInput
}
type fakeAuth struct{}

func (fakeAuth) Authenticate(context.Context, string) (string, error) { return "Bearer test", nil }

func (*fakeService) Create(context.Context, uuid.UUID, domain.CreateInput) (domain.Todo, error) {
	return domain.Todo{}, nil
}
func (*fakeService) List(context.Context, uuid.UUID, uint32, uint32) (domain.ListResult, error) {
	return domain.ListResult{Items: []domain.Todo{}, Page: 1, Limit: 20}, nil
}
func (*fakeService) Get(context.Context, uuid.UUID, uuid.UUID) (domain.Todo, error) {
	return domain.Todo{}, nil
}
func (f *fakeService) Update(_ context.Context, _, _ uuid.UUID, input domain.UpdateInput) (domain.Todo, error) {
	f.updated = input
	return domain.Todo{}, nil
}
func (*fakeService) Delete(context.Context, uuid.UUID, uuid.UUID) error { return nil }
func (*fakeService) ListAudit(context.Context, uuid.UUID, uint32, uint32) (domain.EventAuditListResult, error) {
	return domain.EventAuditListResult{Items: []domain.TodoEventAudit{}, Page: 1, Limit: 20}, nil
}

func TestPatchNullClearsDescription(t *testing.T) {
	t.Parallel()
	service := &fakeService{}
	handler := New(service, fakeAuth{}, slog.New(slog.NewTextHandler(io.Discard, nil)), "http://localhost", "")
	request := httptest.NewRequest(http.MethodPatch,
		"/api/users/"+uuid.NewString()+"/todos/"+uuid.NewString(), strings.NewReader(`{"description":null}`))
	response := httptest.NewRecorder()
	handler.ServeHTTP(response, request)
	if response.Code != http.StatusOK {
		t.Fatalf("status = %d, body = %s", response.Code, response.Body.String())
	}
	if !service.updated.DescriptionSet || service.updated.Description != nil {
		t.Fatalf("description update = %#v, want explicitly set nil", service.updated)
	}
}

func TestListResponseHasFixedShape(t *testing.T) {
	t.Parallel()
	handler := New(&fakeService{}, fakeAuth{}, slog.New(slog.NewTextHandler(io.Discard, nil)), "http://localhost", "")
	request := httptest.NewRequest(http.MethodGet, "/api/users/"+uuid.NewString()+"/todos", nil)
	response := httptest.NewRecorder()
	handler.ServeHTTP(response, request)
	var body map[string]any
	if err := json.Unmarshal(response.Body.Bytes(), &body); err != nil {
		t.Fatal(err)
	}
	for _, key := range []string{"items", "total", "page", "limit"} {
		if _, ok := body[key]; !ok {
			t.Fatalf("missing response field %q", key)
		}
	}
}

func TestEventAuditResponseHasFixedShape(t *testing.T) {
	t.Parallel()
	handler := New(&fakeService{}, fakeAuth{}, slog.New(slog.NewTextHandler(io.Discard, nil)), "http://localhost", "")
	request := httptest.NewRequest(http.MethodGet, "/api/users/"+uuid.NewString()+"/todos/events", nil)
	response := httptest.NewRecorder()
	handler.ServeHTTP(response, request)
	if response.Code != http.StatusOK {
		t.Fatalf("status = %d, body = %s", response.Code, response.Body.String())
	}
	var body map[string]any
	if err := json.Unmarshal(response.Body.Bytes(), &body); err != nil {
		t.Fatal(err)
	}
	for _, key := range []string{"items", "total", "page", "limit"} {
		if _, ok := body[key]; !ok {
			t.Fatalf("missing response field %q", key)
		}
	}
}

func TestMockChecksAuthenticationAndReturnsServiceStatus(t *testing.T) {
	t.Parallel()
	handler := New(&fakeService{}, fakeAuth{}, slog.New(slog.NewTextHandler(io.Discard, nil)), "http://localhost", "")
	request := httptest.NewRequest(http.MethodGet, "/api/users/mock/todos/mock", nil)
	response := httptest.NewRecorder()
	handler.ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("status = %d, body = %s", response.Code, response.Body.String())
	}
	var body serviceMockResponse
	if err := json.Unmarshal(response.Body.Bytes(), &body); err != nil {
		t.Fatal(err)
	}
	if body.Service != "todo" || body.Status != "ok" || body.Message == "" || body.Environment != "local" || body.Revision != "development" {
		t.Fatalf("response = %#v", body)
	}
}

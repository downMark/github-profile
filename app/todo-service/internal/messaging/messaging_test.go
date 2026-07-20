package messaging

import (
	"context"
	"encoding/json"
	"io"
	"log/slog"
	"testing"
	"time"

	"github.com/aws/aws-sdk-go-v2/service/sns"
	"github.com/downMark/github-profile/app/todo-service/internal/domain"
	"github.com/google/uuid"
)

type fakeStore struct {
	events []domain.TodoEvent
	marked uuid.UUID
}

func (f *fakeStore) ClaimOutbox(context.Context, string, int) ([]domain.TodoEvent, error) {
	return f.events, nil
}
func (f *fakeStore) MarkPublished(_ context.Context, id uuid.UUID) error { f.marked = id; return nil }
func (*fakeStore) RetryOutbox(context.Context, uuid.UUID, string) error  { return nil }
func (*fakeStore) RecordAudit(context.Context, domain.TodoEvent) error   { return nil }

type fakeSNS struct{ input *sns.PublishInput }

func (f *fakeSNS) Publish(_ context.Context, input *sns.PublishInput, _ ...func(*sns.Options)) (*sns.PublishOutput, error) {
	f.input = input
	return &sns.PublishOutput{}, nil
}

func TestValidEvent(t *testing.T) {
	t.Parallel()
	event := domain.TodoEvent{
		EventID: uuid.New(), SchemaVersion: 1, EventType: "todo.created", OccurredAt: time.Now(),
		GithubUserID: uuid.New(), TodoID: uuid.New(), Todo: json.RawMessage(`{"title":"test"}`),
	}
	if !validEvent(event) {
		t.Fatal("validEvent rejected schema version 1 event")
	}
	event.SchemaVersion = 2
	if validEvent(event) {
		t.Fatal("validEvent accepted unsupported schema version")
	}
}

func TestPublisherMarksSuccessfullyPublishedEvent(t *testing.T) {
	t.Parallel()
	event := domain.TodoEvent{EventID: uuid.New(), SchemaVersion: 1, EventType: "todo.created", GithubUserID: uuid.New(), TodoID: uuid.New(), Todo: json.RawMessage(`{}`)}
	store, client := &fakeStore{events: []domain.TodoEvent{event}}, &fakeSNS{}
	publisher := NewPublisher(store, client, "arn:aws:sns:us-east-1:123456789012:todo", slog.New(slog.NewTextHandler(io.Discard, nil)))
	if err := publisher.publishBatch(context.Background()); err != nil {
		t.Fatal(err)
	}
	if store.marked != event.EventID {
		t.Fatalf("marked event = %s, want %s", store.marked, event.EventID)
	}
	if client.input == nil || client.input.MessageAttributes["event_type"].StringValue == nil {
		t.Fatal("SNS publish input missing event_type")
	}
}

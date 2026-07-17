package profileclient

import (
	"context"
	"errors"
	"testing"

	"github.com/downMark/github-profile/app/todo-service/internal/domain"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func TestMapProfileError(t *testing.T) {
	t.Parallel()
	tests := []struct {
		name string
		err  error
		want error
	}{
		{name: "not found", err: status.Error(codes.NotFound, "missing"), want: domain.ErrUserNotFound},
		{name: "invalid", err: status.Error(codes.InvalidArgument, "invalid"), want: domain.ErrInvalidInput},
		{name: "unavailable", err: status.Error(codes.Unavailable, "down"), want: domain.ErrProfileUnavailable},
		{name: "deadline", err: status.Error(codes.DeadlineExceeded, "late"), want: domain.ErrProfileUnavailable},
		{name: "context deadline", err: context.DeadlineExceeded, want: domain.ErrProfileUnavailable},
	}
	for _, test := range tests {
		test := test
		t.Run(test.name, func(t *testing.T) {
			t.Parallel()
			if err := mapProfileError(test.err); !errors.Is(err, test.want) {
				t.Fatalf("mapProfileError() = %v, want %v", err, test.want)
			}
		})
	}
}

func TestInternalProfileErrorRemainsInternal(t *testing.T) {
	t.Parallel()
	err := mapProfileError(status.Error(codes.Internal, "database detail"))
	if errors.Is(err, domain.ErrProfileUnavailable) || errors.Is(err, domain.ErrUserNotFound) {
		t.Fatalf("internal error was incorrectly mapped to a public domain error: %v", err)
	}
}

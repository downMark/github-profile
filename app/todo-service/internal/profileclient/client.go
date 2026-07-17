package profileclient

import (
	"context"
	"errors"
	"fmt"
	"time"

	"github.com/downMark/github-profile/app/todo-service/internal/domain"
	profilev1 "github.com/downMark/github-profile/app/todo-service/internal/gen/profile/v1"
	"github.com/downMark/github-profile/app/todo-service/internal/requestauth"
	"github.com/google/uuid"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/grpc/metadata"
	"google.golang.org/grpc/status"
)

const requestTimeout = 2 * time.Second

type Client struct {
	connection *grpc.ClientConn
	profiles   profilev1.ProfileServiceClient
}

func New(address string) (*Client, error) {
	connection, err := grpc.NewClient(address, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		return nil, fmt.Errorf("create profile gRPC client: %w", err)
	}
	return &Client{
		connection: connection,
		profiles:   profilev1.NewProfileServiceClient(connection),
	}, nil
}

func (c *Client) Close() error {
	return c.connection.Close()
}

func (c *Client) AuthorizeGithubUser(ctx context.Context, userID uuid.UUID) error {
	requestContext, cancel := context.WithTimeout(ctx, requestTimeout)
	defer cancel()
	bearer, ok := requestauth.Bearer(ctx)
	if !ok {
		return domain.ErrUnauthorized
	}
	requestContext = metadata.AppendToOutgoingContext(requestContext, "authorization", bearer)
	_, err := c.profiles.AuthorizeGithubUser(requestContext, &profilev1.AuthorizeGithubUserRequest{UserId: userID.String()})
	if err == nil {
		return nil
	}
	return mapProfileError(err)
}

func mapProfileError(err error) error {
	switch status.Code(err) {
	case codes.NotFound, codes.PermissionDenied:
		return domain.ErrUserNotFound
	case codes.InvalidArgument:
		return fmt.Errorf("%w: invalid user id", domain.ErrInvalidInput)
	case codes.Unavailable, codes.DeadlineExceeded, codes.Canceled:
		return domain.ErrProfileUnavailable
	case codes.Unauthenticated:
		return domain.ErrUnauthorized
	default:
		if errors.Is(err, context.DeadlineExceeded) {
			return domain.ErrProfileUnavailable
		}
		return fmt.Errorf("profile validation failed: %w", err)
	}
}

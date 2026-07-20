package messaging

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/sns"
	"github.com/aws/aws-sdk-go-v2/service/sns/types"
	"github.com/aws/aws-sdk-go-v2/service/sqs"
	"github.com/downMark/github-profile/app/todo-service/internal/domain"
	"github.com/google/uuid"
)

type Store interface {
	ClaimOutbox(context.Context, string, int) ([]domain.TodoEvent, error)
	MarkPublished(context.Context, uuid.UUID) error
	RetryOutbox(context.Context, uuid.UUID, string) error
	RecordAudit(context.Context, domain.TodoEvent) error
}

type SNSClient interface {
	Publish(context.Context, *sns.PublishInput, ...func(*sns.Options)) (*sns.PublishOutput, error)
}

type SQSClient interface {
	ReceiveMessage(context.Context, *sqs.ReceiveMessageInput, ...func(*sqs.Options)) (*sqs.ReceiveMessageOutput, error)
	DeleteMessage(context.Context, *sqs.DeleteMessageInput, ...func(*sqs.Options)) (*sqs.DeleteMessageOutput, error)
}

type Publisher struct {
	store    Store
	client   SNSClient
	topicARN string
	workerID string
	logger   *slog.Logger
}

func NewPublisher(store Store, client SNSClient, topicARN string, logger *slog.Logger) *Publisher {
	return &Publisher{store: store, client: client, topicARN: topicARN, workerID: uuid.NewString(), logger: logger}
}

func (p *Publisher) Run(ctx context.Context) {
	ticker := time.NewTicker(2 * time.Second)
	defer ticker.Stop()
	for {
		if err := p.publishBatch(ctx); err != nil && ctx.Err() == nil {
			p.logger.Error("publish todo outbox batch", "error", err)
		}
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
		}
	}
}

func (p *Publisher) publishBatch(ctx context.Context) error {
	events, err := p.store.ClaimOutbox(ctx, p.workerID, 10)
	if err != nil {
		return err
	}
	for _, event := range events {
		body, err := json.Marshal(event)
		if err == nil {
			_, err = p.client.Publish(ctx, &sns.PublishInput{
				TopicArn: aws.String(p.topicARN), Message: aws.String(string(body)),
				MessageAttributes: map[string]types.MessageAttributeValue{
					"event_type":     {DataType: aws.String("String"), StringValue: aws.String(event.EventType)},
					"schema_version": {DataType: aws.String("Number"), StringValue: aws.String(fmt.Sprint(event.SchemaVersion))},
				},
			})
		}
		if err != nil {
			if retryErr := p.store.RetryOutbox(ctx, event.EventID, err.Error()); retryErr != nil {
				return retryErr
			}
			continue
		}
		if err := p.store.MarkPublished(ctx, event.EventID); err != nil {
			return err
		}
	}
	return nil
}

type Consumer struct {
	store    Store
	client   SQSClient
	queueURL string
	logger   *slog.Logger
}

func NewConsumer(store Store, client SQSClient, queueURL string, logger *slog.Logger) *Consumer {
	return &Consumer{store: store, client: client, queueURL: queueURL, logger: logger}
}

func (c *Consumer) Run(ctx context.Context) {
	for ctx.Err() == nil {
		output, err := c.client.ReceiveMessage(ctx, &sqs.ReceiveMessageInput{
			QueueUrl: aws.String(c.queueURL), MaxNumberOfMessages: 10, WaitTimeSeconds: 20, VisibilityTimeout: 60,
		})
		if err != nil {
			if ctx.Err() == nil {
				c.logger.Error("receive todo events", "error", err)
			}
			continue
		}
		for _, message := range output.Messages {
			var event domain.TodoEvent
			if err := json.Unmarshal([]byte(aws.ToString(message.Body)), &event); err != nil || !validEvent(event) {
				c.logger.Error("reject invalid todo event", "message_id", aws.ToString(message.MessageId), "error", err)
				continue
			}
			if err := c.store.RecordAudit(ctx, event); err != nil {
				c.logger.Error("record todo event audit", "event_id", event.EventID, "error", err)
				continue
			}
			if _, err := c.client.DeleteMessage(ctx, &sqs.DeleteMessageInput{QueueUrl: aws.String(c.queueURL), ReceiptHandle: message.ReceiptHandle}); err != nil {
				c.logger.Error("delete processed todo event", "event_id", event.EventID, "error", err)
			}
		}
	}
}

func validEvent(event domain.TodoEvent) bool {
	return event.SchemaVersion == 1 && event.EventID != uuid.Nil && event.GithubUserID != uuid.Nil &&
		event.TodoID != uuid.Nil && event.EventType != "" && len(event.Todo) > 0
}
